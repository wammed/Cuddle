// src-tauri/src/chmod.rs
// Cuddle - Phase 9.2: Bulk Permission Modification (chmod) Engine & Dry-run Verification
// Adheres to Design Document 9.4:
// - Supports both Octal (e.g. 644, 755) and Symbolic (e.g. u+x, go-w, a+r) modes
// - Highlights explicit warnings for special bits (setuid, setgid, sticky)
// - Prohibits privilege escalation (operates strictly within current user permissions)
// - Supports Dry-run verification and atomic rollback journal for one-click Undo

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Input target item representation for chmod operation
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct ChmodTargetInputDto {
    pub item_id: String,
    pub path_or_base64: String,
    pub display_name: String,
}

/// Chmod mode specification (Octal or Symbolic)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "value")]
#[ts(export, export_to = "../../src/types/")]
pub enum ChmodModeSpec {
    Octal(String),
    Symbolic(String),
}

/// Single item in planned chmod operation
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct PlannedChmodItemDto {
    pub item_id: String,
    pub path: String,
    pub display_name: String,
    pub is_dir: bool,
    pub current_mode: u32,
    pub current_mode_str: String,
    pub current_octal: String,
    pub target_mode: u32,
    pub target_mode_str: String,
    pub target_octal: String,
    pub is_changed: bool,
    pub has_special_bits: bool,
    pub warning: Option<String>,
}

/// Complete plan DTO for bulk chmod with dry-run results
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct ChmodPlanDto {
    pub plan_id: String,
    pub items: Vec<PlannedChmodItemDto>,
    pub total_count: usize,
    pub changed_count: usize,
    pub has_warnings: bool,
    pub has_special_bits: bool,
    pub is_executable: bool,
    pub error_message: Option<String>,
}

/// Result DTO after executing bulk chmod
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct ChmodExecutionResultDto {
    pub plan_id: String,
    pub success: bool,
    pub total_count: usize,
    pub succeeded_count: usize,
    pub failed_count: usize,
    pub is_rolled_back: bool,
    pub error_message: Option<String>,
}

/// Convert unix mode to human readable string (e.g. "-rwxr-xr-x" or "drw-r--r--")
pub fn mode_to_string(mode: u32, is_dir: bool) -> String {
    let mut s = String::with_capacity(10);
    s.push(if is_dir { 'd' } else { '-' });

    // User (Owner)
    s.push(if mode & 0o400 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o200 != 0 { 'w' } else { '-' });
    s.push(if mode & 0o4000 != 0 {
        if mode & 0o100 != 0 { 's' } else { 'S' }
    } else if mode & 0o100 != 0 {
        'x'
    } else {
        '-'
    });

    // Group
    s.push(if mode & 0o040 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o020 != 0 { 'w' } else { '-' });
    s.push(if mode & 0o2000 != 0 {
        if mode & 0o010 != 0 { 's' } else { 'S' }
    } else if mode & 0o010 != 0 {
        'x'
    } else {
        '-'
    });

    // Others
    s.push(if mode & 0o004 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o002 != 0 { 'w' } else { '-' });
    s.push(if mode & 0o1000 != 0 {
        if mode & 0o001 != 0 { 't' } else { 'T' }
    } else if mode & 0o001 != 0 {
        'x'
    } else {
        '-'
    });

    s
}

/// Format mode as a standard 3-digit or 4-digit octal string (e.g. "0644" or "4755")
pub fn mode_to_octal_string(mode: u32) -> String {
    let masked = mode & 0o7777;
    if masked > 0o777 {
        format!("{:04o}", masked)
    } else {
        format!("{:03o}", masked)
    }
}

/// Parse octal string like "644", "0755", "4755" into numeric mode
pub fn parse_octal_mode(octal_str: &str) -> Result<u32, String> {
    let trimmed = octal_str.trim();
    if trimmed.is_empty() {
        return Err("Octal mode string cannot be empty".to_string());
    }

    let val = u32::from_str_radix(trimmed, 8)
        .map_err(|_| format!("Invalid octal number: '{}'", trimmed))?;

    if val > 0o7777 {
        return Err(format!("Octal mode out of range (max 7777): '{:o}'", val));
    }

    Ok(val)
}

/// Parse symbolic mode expressions like "u+x", "go-w", "a+r", "u=rwx,go=rx", "+x"
pub fn parse_symbolic_mode(current_mode: u32, expr: &str) -> Result<u32, String> {
    let mut mode = current_mode & 0o7777;
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return Err("Symbolic mode expression cannot be empty".to_string());
    }

    // Support comma-separated clauses: e.g. "u=rw,go=r"
    for clause in trimmed.split(',') {
        let c = clause.trim();
        if c.is_empty() {
            continue;
        }

        // Find operator position (+, -, =)
        let op_pos = c.find(|ch| ch == '+' || ch == '-' || ch == '=')
            .ok_or_else(|| format!("Missing operator (+, -, =) in symbolic clause: '{}'", c))?;

        let who_str = &c[..op_pos];
        let op = c.as_bytes()[op_pos] as char;
        let perm_str = &c[op_pos + 1..];

        // Determine targets: 'u', 'g', 'o', 'a' (default is 'a' if who is empty)
        let (apply_u, apply_g, apply_o) = if who_str.is_empty() || who_str.contains('a') {
            (true, true, true)
        } else {
            (who_str.contains('u'), who_str.contains('g'), who_str.contains('o'))
        };

        if !apply_u && !apply_g && !apply_o {
            return Err(format!("Invalid target in clause '{}'. Use u, g, o, or a.", c));
        }

        // Parse permission bits
        let mut bits_u = 0u32;
        let mut bits_g = 0u32;
        let mut bits_o = 0u32;

        for ch in perm_str.chars() {
            match ch {
                'r' => {
                    if apply_u { bits_u |= 0o400; }
                    if apply_g { bits_g |= 0o040; }
                    if apply_o { bits_o |= 0o004; }
                }
                'w' => {
                    if apply_u { bits_u |= 0o200; }
                    if apply_g { bits_g |= 0o020; }
                    if apply_o { bits_o |= 0o002; }
                }
                'x' => {
                    if apply_u { bits_u |= 0o100; }
                    if apply_g { bits_g |= 0o010; }
                    if apply_o { bits_o |= 0o001; }
                }
                's' => {
                    // setuid / setgid
                    if apply_u { bits_u |= 0o4000; }
                    if apply_g { bits_g |= 0o2000; }
                }
                't' => {
                    // sticky bit
                    bits_o |= 0o1000;
                }
                _ => return Err(format!("Unknown permission character '{}' in clause '{}'", ch, c)),
            }
        }

        let mask_target = bits_u | bits_g | bits_o;

        match op {
            '+' => {
                mode |= mask_target;
            }
            '-' => {
                mode &= !mask_target;
            }
            '=' => {
                // Clear existing permissions for selected who, then set new bits
                if apply_u { mode &= !(0o700 | 0o4000); }
                if apply_g { mode &= !(0o070 | 0o2000); }
                if apply_o { mode &= !(0o007 | 0o1000); }
                mode |= mask_target;
            }
            _ => unreachable!(),
        }
    }

    Ok(mode & 0o7777)
}

/// Compute target mode from ChmodModeSpec given current mode
pub fn compute_target_mode(current_mode: u32, spec: &ChmodModeSpec) -> Result<u32, String> {
    match spec {
        ChmodModeSpec::Octal(octal) => parse_octal_mode(octal),
        ChmodModeSpec::Symbolic(symbolic) => parse_symbolic_mode(current_mode, symbolic),
    }
}

/// Check if a mode contains special bits (setuid: 4000, setgid: 2000, sticky: 1000)
pub fn has_special_permission_bits(mode: u32) -> bool {
    (mode & 0o7000) != 0
}

/// Generate a complete Dry-run ChmodPlanDto
pub fn generate_chmod_plan(
    target_items: Vec<ChmodTargetInputDto>,
    spec: ChmodModeSpec,
) -> Result<ChmodPlanDto, String> {
    let plan_id = uuid::Uuid::new_v4().to_string();
    let mut planned_items = Vec::with_capacity(target_items.len());
    let mut changed_count = 0;
    let mut has_warnings = false;
    let mut has_special_bits = false;

    for input in target_items {
        let resolved = crate::models::resolve_path_str_or_b64(&input.path_or_base64)
            .map_err(|e| format!("Failed to resolve path for item {}: {}", input.item_id, e))?;

        let path = PathBuf::from(&resolved.path_str);
        if !path.exists() {
            return Err(format!("File does not exist: {}", path.to_string_lossy()));
        }

        let meta = fs::symlink_metadata(&path)
            .map_err(|e| format!("Failed to read metadata for {}: {}", path.to_string_lossy(), e))?;

        let is_dir = meta.is_dir();
        let current_mode = meta.permissions().mode() & 0o7777;
        let current_mode_str = mode_to_string(current_mode, is_dir);
        let current_octal = mode_to_octal_string(current_mode);

        let target_mode = compute_target_mode(current_mode, &spec)?;
        let target_mode_str = mode_to_string(target_mode, is_dir);
        let target_octal = mode_to_octal_string(target_mode);

        let is_changed = current_mode != target_mode;
        if is_changed {
            changed_count += 1;
        }

        let item_has_special = has_special_permission_bits(target_mode);
        if item_has_special {
            has_special_bits = true;
            has_warnings = true;
        }

        // Warning generation
        let mut warning = None;
        if item_has_special {
            let mut bits_desc = Vec::new();
            if target_mode & 0o4000 != 0 { bits_desc.push("setuid"); }
            if target_mode & 0o2000 != 0 { bits_desc.push("setgid"); }
            if target_mode & 0o1000 != 0 { bits_desc.push("sticky"); }
            warning = Some(format!("Special permission bits enabled ({}) - ensure this is intentional", bits_desc.join(", ")));
        } else if is_dir && (target_mode & 0o111 == 0) {
            warning = Some("Directory has no execute (search) bit set, which will make it inaccessible".to_string());
            has_warnings = true;
        } else if target_mode & 0o400 == 0 {
            warning = Some("Owner read permission removed".to_string());
            has_warnings = true;
        }

        planned_items.push(PlannedChmodItemDto {
            item_id: input.item_id,
            path: path.to_string_lossy().to_string(),
            display_name: input.display_name,
            is_dir,
            current_mode,
            current_mode_str,
            current_octal,
            target_mode,
            target_mode_str,
            target_octal,
            is_changed,
            has_special_bits: item_has_special,
            warning,
        });
    }

    let total_count = planned_items.len();
    let is_executable = total_count > 0;

    Ok(ChmodPlanDto {
        plan_id,
        items: planned_items,
        total_count,
        changed_count,
        has_warnings,
        has_special_bits,
        is_executable,
        error_message: None,
    })
}

/// Executes a chmod plan with atomic rollback if any item fails
pub fn execute_chmod_plan(plan: ChmodPlanDto) -> Result<ChmodExecutionResultDto, String> {
    if !plan.is_executable {
        return Err("Cannot execute chmod plan: plan is marked as non-executable".to_string());
    }

    let mut applied_items: Vec<(&PlannedChmodItemDto, u32)> = Vec::with_capacity(plan.items.len());

    for item in &plan.items {
        if !item.is_changed {
            continue;
        }

        let p = Path::new(&item.path);
        if !p.exists() {
            // Rollback previously applied
            rollback_chmod(&applied_items);
            return Ok(ChmodExecutionResultDto {
                plan_id: plan.plan_id,
                success: false,
                total_count: plan.total_count,
                succeeded_count: 0,
                failed_count: plan.changed_count,
                is_rolled_back: true,
                error_message: Some(format!("Target file does not exist: {}", item.path)),
            });
        }

        let permissions = fs::Permissions::from_mode(item.target_mode);
        if let Err(e) = fs::set_permissions(p, permissions) {
            // Atomic Rollback
            rollback_chmod(&applied_items);
            return Ok(ChmodExecutionResultDto {
                plan_id: plan.plan_id,
                success: false,
                total_count: plan.total_count,
                succeeded_count: 0,
                failed_count: plan.changed_count,
                is_rolled_back: true,
                error_message: Some(format!("Failed to chmod '{}' to {:04o}: {}", item.display_name, item.target_mode, e)),
            });
        }

        applied_items.push((item, item.current_mode));
    }

    let succeeded_count = applied_items.len();

    // Record in operation history
    let history_mgr = crate::history::get_history_manager();
    for (item, _) in &applied_items {
        let _ = history_mgr.record_operation(&crate::history::NewHistoryEntry {
            operation_id: uuid::Uuid::new_v4().to_string(),
            operation_type: "chmod".to_string(),
            display_name: item.display_name.clone(),
            original_path: Some(item.path.clone()),
            staged_path: Some(item.path.clone()),
            inode: None,
            status: "Success".to_string(),
            can_undo: true,
            details: Some(format!(
                "Mode changed: {} ({}) -> {} ({})",
                item.current_octal, item.current_mode_str, item.target_octal, item.target_mode_str
            )),
        });
    }

    Ok(ChmodExecutionResultDto {
        plan_id: plan.plan_id,
        success: true,
        total_count: plan.total_count,
        succeeded_count,
        failed_count: 0,
        is_rolled_back: false,
        error_message: None,
    })
}

/// Helper function to rollback applied chmod permissions in reverse order
fn rollback_chmod(applied: &[(&PlannedChmodItemDto, u32)]) {
    for (item, original_mode) in applied.iter().rev() {
        let p = Path::new(&item.path);
        if p.exists() {
            let perms = fs::Permissions::from_mode(*original_mode);
            let _ = fs::set_permissions(p, perms);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_to_string_conversions() {
        assert_eq!(mode_to_string(0o644, false), "-rw-r--r--");
        assert_eq!(mode_to_string(0o755, true), "drwxr-xr-x");
        assert_eq!(mode_to_string(0o700, true), "drwx------");
        assert_eq!(mode_to_string(0o600, false), "-rw-------");
        // Special bits
        assert_eq!(mode_to_string(0o4755, false), "-rwsr-xr-x");
        assert_eq!(mode_to_string(0o2755, false), "-rwxr-sr-x");
        assert_eq!(mode_to_string(0o1777, true), "drwxrwxrwt");
    }

    #[test]
    fn test_parse_octal_mode() {
        assert_eq!(parse_octal_mode("644").unwrap(), 0o644);
        assert_eq!(parse_octal_mode("755").unwrap(), 0o755);
        assert_eq!(parse_octal_mode("0644").unwrap(), 0o644);
        assert_eq!(parse_octal_mode("4755").unwrap(), 0o4755);

        assert!(parse_octal_mode("").is_err());
        assert!(parse_octal_mode("999").is_err());
        assert!(parse_octal_mode("8888").is_err());
    }

    #[test]
    fn test_parse_symbolic_mode() {
        // u+x
        assert_eq!(parse_symbolic_mode(0o644, "u+x").unwrap(), 0o744);
        // go-r
        assert_eq!(parse_symbolic_mode(0o644, "go-r").unwrap(), 0o600);
        // a+x
        assert_eq!(parse_symbolic_mode(0o644, "a+x").unwrap(), 0o755);
        // +x (default who is a)
        assert_eq!(parse_symbolic_mode(0o644, "+x").unwrap(), 0o755);
        // Multiple clauses: u=rwx,go=rx
        assert_eq!(parse_symbolic_mode(0o600, "u=rwx,go=rx").unwrap(), 0o755);
        // Special bit: u+s
        assert_eq!(parse_symbolic_mode(0o755, "u+s").unwrap(), 0o4755);
    }

    #[test]
    fn test_generate_chmod_plan_and_execution() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle_test_chmod_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("test_file.txt");
        fs::write(&file_path, "chmod test").unwrap();
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o644)).unwrap();

        let target = ChmodTargetInputDto {
            item_id: "chmod-1".to_string(),
            path_or_base64: file_path.to_string_lossy().to_string(),
            display_name: "test_file.txt".to_string(),
        };

        // 1. Dry run with 755
        let plan = generate_chmod_plan(vec![target], ChmodModeSpec::Octal("755".to_string())).unwrap();
        assert_eq!(plan.total_count, 1);
        assert_eq!(plan.changed_count, 1);
        assert_eq!(plan.items[0].current_mode, 0o644);
        assert_eq!(plan.items[0].target_mode, 0o755);
        assert!(!plan.has_special_bits);
        assert!(plan.is_executable);

        // 2. Execute plan
        let res = execute_chmod_plan(plan).unwrap();
        assert!(res.success);
        assert_eq!(res.succeeded_count, 1);

        let updated_mode = fs::metadata(&file_path).unwrap().permissions().mode() & 0o7777;
        assert_eq!(updated_mode, 0o755);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_export_typescript_bindings_chmod() {
        let _ = std::fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        ChmodTargetInputDto::export(&cfg).expect("Failed to export ChmodTargetInputDto");
        ChmodModeSpec::export(&cfg).expect("Failed to export ChmodModeSpec");
        PlannedChmodItemDto::export(&cfg).expect("Failed to export PlannedChmodItemDto");
        ChmodPlanDto::export(&cfg).expect("Failed to export ChmodPlanDto");
        ChmodExecutionResultDto::export(&cfg).expect("Failed to export ChmodExecutionResultDto");
    }
}
