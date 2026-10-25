// src-tauri/src/batch.rs
// Cuddle - Phase 5.1: Normalization & Bulk Rename Rules Engine & Dry-run Verification

use chrono::{DateTime, Local};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;
use ts_rs::TS;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub enum NormalizationForm {
    Nfc,
    Nfd,
    Nfkc,
    Nfkd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub enum SequencePosition {
    Prefix,
    SuffixBeforeExtension,
    SuffixAfterExtension,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub enum DateTimeSource {
    CurrentTime,
    FileModifiedTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export, export_to = "../../src/types/")]
pub enum RenameRule {
    RegexReplace {
        pattern: String,
        replacement: String,
    },
    UnicodeNormalize {
        form: NormalizationForm,
    },
    ConvertCharacter {
        half_width_alnum: bool,
        half_width_kana: bool,
        space_to_underscore: bool,
        underscore_to_space: bool,
        to_lowercase: bool,
        to_uppercase: bool,
    },
    AddPrefixSuffix {
        prefix: String,
        suffix: String,
        preserve_extension: bool,
    },
    AddSequence {
        start: u32,
        step: u32,
        padding: usize,
        position: SequencePosition,
        preserve_extension: bool,
    },
    AddDateTime {
        format: String,
        source: DateTimeSource,
        position: SequencePosition,
        preserve_extension: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub enum OperationStatus {
    Ok,
    Unchanged,
    ConflictDuplicate,
    ConflictExistsOnDisk,
    InvalidFilename,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct PlannedOperationDto {
    pub item_id: String,
    pub original_path: String,
    pub display_name: String,
    pub raw_base64: String,
    pub proposed_name: String,
    pub target_path: String,
    pub status: OperationStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct BatchPlanDto {
    pub batch_id: String,
    pub operations: Vec<PlannedOperationDto>,
    pub total_count: usize,
    pub changed_count: usize,
    pub conflict_count: usize,
    pub error_count: usize,
    pub is_executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct BatchExecutionResultDto {
    pub batch_id: String,
    pub success: bool,
    pub total_count: usize,
    pub succeeded_count: usize,
    pub failed_count: usize,
    pub is_rolled_back: bool,
    pub error_message: Option<String>,
    pub affected_items: Vec<PlannedOperationDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct BatchJournalEntry {
    pub batch_id: String,
    pub executed_at_ms: i64,
    pub operations: Vec<PlannedOperationDto>,
    pub is_undone: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct RenameTargetInputDto {
    pub item_id: String,
    pub path_or_base64: String,
    pub display_name: String,
    pub mtime_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct BulkRenameOptions {
    pub target_items: Vec<RenameTargetInputDto>,
    pub rules: Vec<RenameRule>,
}

/// Helper to split filename into stem and extension properly, handling dotfiles
pub fn split_stem_and_ext(filename: &str) -> (String, Option<String>) {
    if filename.is_empty() {
        return (String::new(), None);
    }

    // Dotfiles like .gitignore, .bashrc (no extension, stem is the whole dotfile)
    if let Some(rest) = filename.strip_prefix('.') {
        if rest.find('.').is_none() {
            return (filename.to_string(), None);
        }
    }

    // Dotfiles with extra extension like .config.json
    let (is_dotfile, search_slice) = if let Some(rest) = filename.strip_prefix('.') {
        (true, rest)
    } else {
        (false, filename)
    };

    if let Some(dot_idx) = search_slice.rfind('.') {
        let actual_idx = if is_dotfile { dot_idx + 1 } else { dot_idx };
        let stem = filename[..actual_idx].to_string();
        let ext = filename[actual_idx + 1..].to_string();
        (stem, Some(ext))
    } else {
        (filename.to_string(), None)
    }
}

/// Recombine stem and extension
pub fn join_stem_and_ext(stem: &str, ext: Option<&str>) -> String {
    match ext {
        Some(e) if !e.is_empty() => format!("{}.{}", stem, e),
        _ => stem.to_string(),
    }
}

/// Convert full-width ASCII characters to half-width
pub fn convert_fullwidth_alnum_to_halfwidth(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            // Full-width space (U+3000) -> standard space
            '\u{3000}' => out.push(' '),
            // Full-width ASCII: ！(U+FF01) to ～(U+FF5E) -> !(U+0021) to ~(U+007E)
            '\u{FF01}'..='\u{FF5E}' => {
                let ascii = (c as u32 - 0xFEE0) as u8 as char;
                out.push(ascii);
            }
            _ => out.push(c),
        }
    }
    out
}

/// Apply a single rename rule to a filename
pub fn apply_rule(
    filename: &str,
    rule: &RenameRule,
    index: usize,
    modified_time: Option<DateTime<Local>>,
) -> Result<String, String> {
    match rule {
        RenameRule::RegexReplace {
            pattern,
            replacement,
        } => {
            let re = Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;
            Ok(re.replace_all(filename, replacement.as_str()).to_string())
        }
        RenameRule::UnicodeNormalize { form } => {
            let normalized = match form {
                NormalizationForm::Nfc => filename.nfc().collect::<String>(),
                NormalizationForm::Nfd => filename.nfd().collect::<String>(),
                NormalizationForm::Nfkc => filename.nfkc().collect::<String>(),
                NormalizationForm::Nfkd => filename.nfkd().collect::<String>(),
            };
            Ok(normalized)
        }
        RenameRule::ConvertCharacter {
            half_width_alnum,
            half_width_kana: _,
            space_to_underscore,
            underscore_to_space,
            to_lowercase,
            to_uppercase,
        } => {
            let mut result = filename.to_string();
            if *half_width_alnum {
                result = convert_fullwidth_alnum_to_halfwidth(&result);
            }
            if *space_to_underscore {
                result = result.replace(' ', "_");
            }
            if *underscore_to_space {
                result = result.replace('_', " ");
            }
            if *to_lowercase {
                result = result.to_lowercase();
            } else if *to_uppercase {
                result = result.to_uppercase();
            }
            Ok(result)
        }
        RenameRule::AddPrefixSuffix {
            prefix,
            suffix,
            preserve_extension,
        } => {
            if *preserve_extension {
                let (stem, ext) = split_stem_and_ext(filename);
                let new_stem = format!("{}{}{}", prefix, stem, suffix);
                Ok(join_stem_and_ext(&new_stem, ext.as_deref()))
            } else {
                Ok(format!("{}{}{}", prefix, filename, suffix))
            }
        }
        RenameRule::AddSequence {
            start,
            step,
            padding,
            position,
            preserve_extension,
        } => {
            let seq_num = start.saturating_add((index as u32).saturating_mul(*step));
            let seq_str = format!("{:0>width$}", seq_num, width = *padding);

            if *preserve_extension {
                let (stem, ext) = split_stem_and_ext(filename);
                let new_stem = match position {
                    SequencePosition::Prefix => format!("{}{}", seq_str, stem),
                    SequencePosition::SuffixBeforeExtension
                    | SequencePosition::SuffixAfterExtension => {
                        format!("{}{}", stem, seq_str)
                    }
                };
                Ok(join_stem_and_ext(&new_stem, ext.as_deref()))
            } else {
                match position {
                    SequencePosition::Prefix => Ok(format!("{}{}", seq_str, filename)),
                    SequencePosition::SuffixBeforeExtension
                    | SequencePosition::SuffixAfterExtension => {
                        Ok(format!("{}{}", filename, seq_str))
                    }
                }
            }
        }
        RenameRule::AddDateTime {
            format,
            source,
            position,
            preserve_extension,
        } => {
            let dt = match source {
                DateTimeSource::CurrentTime => Local::now(),
                DateTimeSource::FileModifiedTime => modified_time.unwrap_or_else(Local::now),
            };
            let dt_str = dt.format(format).to_string();

            if *preserve_extension {
                let (stem, ext) = split_stem_and_ext(filename);
                let new_stem = match position {
                    SequencePosition::Prefix => format!("{}{}", dt_str, stem),
                    SequencePosition::SuffixBeforeExtension
                    | SequencePosition::SuffixAfterExtension => {
                        format!("{}{}", stem, dt_str)
                    }
                };
                Ok(join_stem_and_ext(&new_stem, ext.as_deref()))
            } else {
                match position {
                    SequencePosition::Prefix => Ok(format!("{}{}", dt_str, filename)),
                    SequencePosition::SuffixBeforeExtension
                    | SequencePosition::SuffixAfterExtension => {
                        Ok(format!("{}{}", filename, dt_str))
                    }
                }
            }
        }
    }
}

/// Apply a pipeline of rename rules to a filename sequentially
pub fn apply_rule_chain(
    initial_name: &str,
    rules: &[RenameRule],
    index: usize,
    modified_time: Option<DateTime<Local>>,
) -> Result<String, String> {
    let mut current_name = initial_name.to_string();
    for rule in rules {
        current_name = apply_rule(&current_name, rule, index, modified_time)?;
    }
    Ok(current_name)
}

/// Validate if a proposed filename is legal in Linux filesystem
pub fn validate_filename(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("Filename cannot be empty");
    }
    if name.contains('\0') {
        return Err("Filename cannot contain null byte");
    }
    if name.contains('/') {
        return Err("Filename cannot contain directory separator '/'");
    }
    if name == "." || name == ".." {
        return Err("Filename cannot be '.' or '..'");
    }
    Ok(())
}

/// Item input representation for generating a batch rename plan
#[derive(Debug, Clone)]
pub struct RenameTargetItem {
    pub item_id: String,
    pub original_path: PathBuf,
    pub display_name: String,
    pub raw_base64: String,
    pub modified_time: Option<DateTime<Local>>,
}

/// Generate a BatchPlan with full dry-run collision detection
/// Common validation and collision detection engine that constructs BatchPlanDto.
pub fn build_batch_plan_from_proposed_names(
    items: Vec<RenameTargetItem>,
    proposed_names: HashMap<String, Result<String, String>>,
) -> BatchPlanDto {
    let batch_id = uuid::Uuid::new_v4().to_string();
    let mut operations = Vec::with_capacity(items.len());

    // 1. Process each item with its proposed name
    for item in &items {
        let parent_dir = item.original_path.parent().unwrap_or_else(|| Path::new(""));

        // If the item in staging has a "{item_id}_" prefix on disk, maintain it in the destination path on disk
        let orig_file_name = item
            .original_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let prefix_with_underscore = format!("{}_", item.item_id);
        let has_staged_prefix = orig_file_name.starts_with(&prefix_with_underscore);

        let proposed_res = proposed_names
            .get(&item.item_id)
            .cloned()
            .unwrap_or_else(|| Ok(item.display_name.clone()));

        match proposed_res {
            Ok(proposed_name) => {
                let disk_target_filename = if has_staged_prefix {
                    format!("{}_{}", item.item_id, proposed_name)
                } else {
                    proposed_name.clone()
                };
                let target_path = parent_dir.join(disk_target_filename);

                if let Err(err_msg) = validate_filename(&proposed_name) {
                    operations.push(PlannedOperationDto {
                        item_id: item.item_id.clone(),
                        original_path: item.original_path.to_string_lossy().to_string(),
                        display_name: item.display_name.clone(),
                        raw_base64: item.raw_base64.clone(),
                        proposed_name,
                        target_path: target_path.to_string_lossy().to_string(),
                        status: OperationStatus::InvalidFilename,
                        message: Some(err_msg.to_string()),
                    });
                } else {
                    let is_unchanged = proposed_name == item.display_name;
                    let initial_status = if is_unchanged {
                        OperationStatus::Unchanged
                    } else {
                        OperationStatus::Ok
                    };

                    operations.push(PlannedOperationDto {
                        item_id: item.item_id.clone(),
                        original_path: item.original_path.to_string_lossy().to_string(),
                        display_name: item.display_name.clone(),
                        raw_base64: item.raw_base64.clone(),
                        proposed_name,
                        target_path: target_path.to_string_lossy().to_string(),
                        status: initial_status,
                        message: None,
                    });
                }
            }
            Err(e) => {
                operations.push(PlannedOperationDto {
                    item_id: item.item_id.clone(),
                    original_path: item.original_path.to_string_lossy().to_string(),
                    display_name: item.display_name.clone(),
                    raw_base64: item.raw_base64.clone(),
                    proposed_name: item.display_name.clone(),
                    target_path: item.original_path.to_string_lossy().to_string(),
                    status: OperationStatus::Error,
                    message: Some(e),
                });
            }
        }
    }

    // 2. Intra-batch collision detection (Duplicate target paths within the batch)
    let mut target_path_counts: HashMap<String, usize> = HashMap::new();
    for op in &operations {
        if op.status == OperationStatus::Ok || op.status == OperationStatus::Unchanged {
            *target_path_counts
                .entry(op.target_path.clone())
                .or_insert(0) += 1;
        }
    }

    let batch_original_paths: HashSet<String> = items
        .iter()
        .map(|it| it.original_path.to_string_lossy().to_string())
        .collect();

    // 3. Mark duplicate conflicts and check against disk collision
    for op in operations.iter_mut() {
        if op.status == OperationStatus::Ok || op.status == OperationStatus::Unchanged {
            if let Some(&count) = target_path_counts.get(&op.target_path) {
                if count > 1 {
                    op.status = OperationStatus::ConflictDuplicate;
                    op.message = Some(format!(
                        "Multiple items would be renamed to the same path: {}",
                        op.proposed_name
                    ));
                    continue;
                }
            }

            // Check if target already exists on disk and is NOT one of the batch's original items
            if op.status == OperationStatus::Ok {
                let target_p = Path::new(&op.target_path);
                if target_p.exists() && !batch_original_paths.contains(&op.target_path) {
                    op.status = OperationStatus::ConflictExistsOnDisk;
                    op.message = Some(format!(
                        "A file with name '{}' already exists on disk in destination directory",
                        op.proposed_name
                    ));
                }
            }
        }
    }

    let total_count = operations.len();
    let mut changed_count = 0;
    let mut conflict_count = 0;
    let mut error_count = 0;

    for op in &operations {
        match op.status {
            OperationStatus::Ok => changed_count += 1,
            OperationStatus::ConflictDuplicate | OperationStatus::ConflictExistsOnDisk => {
                conflict_count += 1;
            }
            OperationStatus::InvalidFilename | OperationStatus::Error => {
                error_count += 1;
            }
            OperationStatus::Unchanged => {}
        }
    }

    let is_executable = conflict_count == 0 && error_count == 0 && changed_count > 0;

    BatchPlanDto {
        batch_id,
        operations,
        total_count,
        changed_count,
        conflict_count,
        error_count,
        is_executable,
    }
}

/// Generate a BatchPlan with full dry-run collision detection using rules
pub fn generate_batch_plan(items: Vec<RenameTargetItem>, rules: &[RenameRule]) -> BatchPlanDto {
    let mut proposed_map = HashMap::with_capacity(items.len());

    for (idx, item) in items.iter().enumerate() {
        let res = apply_rule_chain(&item.display_name, rules, idx, item.modified_time);
        proposed_map.insert(item.item_id.clone(), res);
    }

    build_batch_plan_from_proposed_names(items, proposed_map)
}

/// Convert input DTOs to resolved internal items
pub fn resolve_rename_target_items(
    target_items: Vec<RenameTargetInputDto>,
) -> Result<Vec<RenameTargetItem>, String> {
    use chrono::TimeZone;
    let mut items = Vec::with_capacity(target_items.len());

    for input in target_items {
        let resolved = crate::models::resolve_path_str_or_b64(&input.path_or_base64)
            .map_err(|e| format!("Failed to resolve path for item {}: {}", input.item_id, e))?;

        let path = PathBuf::from(&resolved.path_str);
        let raw_base64 = crate::models::encode_path_to_base64(&path);

        let modified_time = input
            .mtime_ms
            .and_then(|ms| Local.timestamp_millis_opt(ms).single());

        items.push(RenameTargetItem {
            item_id: input.item_id,
            original_path: path,
            display_name: input.display_name,
            raw_base64,
            modified_time,
        });
    }

    Ok(items)
}

/// Generate vidir text buffer content from target items.
/// Format (Design 9.4):
/// [ID]<TAB>[Filename]
pub fn generate_vidir_buffer(items: &[RenameTargetItem]) -> String {
    let mut buf = String::new();
    buf.push_str(
        "# ==============================================================================\n",
    );
    buf.push_str("# Cuddle - vidir Editor Rename (Phase 9.1 / Design 9.4)\n");
    buf.push_str("# Format: [ID]<TAB>[Filename]\n");
    buf.push_str("# ・Edit the filename on the right and save/apply.\n");
    buf.push_str("# ・Deleted lines are treated as UNCHANGED (files are NEVER deleted).\n");
    buf.push_str("# ・Do NOT change the ID on the left.\n");
    buf.push_str(
        "# ==============================================================================\n",
    );

    for item in items {
        buf.push_str(&format!("{}\t{}\n", item.item_id, item.display_name));
    }
    buf
}

/// Parse vidir text buffer and construct a BatchPlanDto with collision detection.
/// Follows Design 9.4: items omitted/deleted from the buffer are preserved as Unchanged.
pub fn parse_vidir_buffer(items: Vec<RenameTargetItem>, buffer: &str) -> BatchPlanDto {
    let mut proposed_map: HashMap<String, Result<String, String>> = HashMap::new();

    for line in buffer.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Split by tab first, or by 2+ spaces if tab is not present
        let (id_part, name_part) = if let Some(tab_pos) = line.find('\t') {
            (
                line[..tab_pos].trim().to_string(),
                line[tab_pos + 1..].trim().to_string(),
            )
        } else if let Some(space_pos) = line.find("  ") {
            (
                line[..space_pos].trim().to_string(),
                line[space_pos..].trim().to_string(),
            )
        } else {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                (parts[0].to_string(), parts[1..].join(" "))
            } else if parts.len() == 1 {
                (parts[0].to_string(), String::new())
            } else {
                continue;
            }
        };

        if !id_part.is_empty() {
            proposed_map.insert(id_part, Ok(name_part));
        }
    }

    build_batch_plan_from_proposed_names(items, proposed_map)
}

/// Detect preferred external text editor command
pub fn detect_preferred_editor(custom_editor: Option<&str>) -> Option<Vec<String>> {
    if let Some(cmd) = custom_editor {
        let trimmed = cmd.trim();
        if !trimmed.is_empty() {
            let parts: Vec<String> = trimmed.split_whitespace().map(|s| s.to_string()).collect();
            return Some(parts);
        }
    }

    if let Ok(visual) = std::env::var("VISUAL") {
        let trimmed = visual.trim();
        if !trimmed.is_empty() {
            let parts: Vec<String> = trimmed.split_whitespace().map(|s| s.to_string()).collect();
            return Some(parts);
        }
    }

    if let Ok(editor) = std::env::var("EDITOR") {
        let trimmed = editor.trim();
        if !trimmed.is_empty() {
            let parts: Vec<String> = trimmed.split_whitespace().map(|s| s.to_string()).collect();
            return Some(parts);
        }
    }

    // Common GUI / terminal editor candidates on Linux
    let candidates = [
        "code --wait",
        "gedit --wait",
        "kate",
        "gnome-text-editor --wait",
        "xed",
        "mousepad",
        "featherpad",
        "nano",
    ];

    for c in &candidates {
        let parts: Vec<String> = c.split_whitespace().map(|s| s.to_string()).collect();
        if let Some(bin) = parts.first() {
            if crate::pipeline::ToolDetector::find_binary_in_path(bin).is_some() {
                return Some(parts);
            }
        }
    }

    None
}

/// Spawns external editor with temporary TSV file and waits for completion.
pub fn launch_external_editor_vidir(
    items: Vec<RenameTargetItem>,
    custom_editor: Option<String>,
) -> Result<BatchPlanDto, String> {
    let editor_cmd = detect_preferred_editor(custom_editor.as_deref())
        .ok_or_else(|| "No suitable text editor found ($VISUAL, $EDITOR, code, gedit, nano). Please specify an editor command.".to_string())?;

    let temp_dir = std::env::temp_dir();
    let temp_filename = format!("cuddle_vidir_{}.tsv", uuid::Uuid::new_v4());
    let temp_file_path = temp_dir.join(temp_filename);

    let initial_content = generate_vidir_buffer(&items);
    std::fs::write(&temp_file_path, &initial_content)
        .map_err(|e| format!("Failed to create temporary editor file: {}", e))?;

    let bin = &editor_cmd[0];
    let mut cmd = std::process::Command::new(bin);
    for arg in &editor_cmd[1..] {
        cmd.arg(arg);
    }
    cmd.arg(&temp_file_path);

    let status = cmd.status().map_err(|e| {
        let _ = std::fs::remove_file(&temp_file_path);
        format!("Failed to execute editor '{}': {}", bin, e)
    })?;

    if !status.success() {
        let _ = std::fs::remove_file(&temp_file_path);
        return Err(format!(
            "Editor exited with non-zero exit code: {:?}",
            status.code()
        ));
    }

    let edited_content = std::fs::read_to_string(&temp_file_path)
        .map_err(|e| format!("Failed to read edited temporary file: {}", e))?;

    let _ = std::fs::remove_file(&temp_file_path);

    Ok(parse_vidir_buffer(items, &edited_content))
}

/// Convenience entry point to generate BatchPlanDto from BulkRenameOptions
pub fn generate_plan_from_options(options: BulkRenameOptions) -> Result<BatchPlanDto, String> {
    let items = resolve_rename_target_items(options.target_items)?;
    Ok(generate_batch_plan(items, &options.rules))
}

/// Convenience entry point to generate vidir buffer string
pub fn generate_vidir_text(target_items: Vec<RenameTargetInputDto>) -> Result<String, String> {
    let items = resolve_rename_target_items(target_items)?;
    Ok(generate_vidir_buffer(&items))
}

/// Convenience entry point to parse vidir buffer string into BatchPlanDto
pub fn parse_vidir_text(
    target_items: Vec<RenameTargetInputDto>,
    content: String,
) -> Result<BatchPlanDto, String> {
    let items = resolve_rename_target_items(target_items)?;
    Ok(parse_vidir_buffer(items, &content))
}

/// Convenience entry point to launch external editor and parse result
pub fn launch_external_editor(
    target_items: Vec<RenameTargetInputDto>,
    custom_editor: Option<String>,
) -> Result<BatchPlanDto, String> {
    let items = resolve_rename_target_items(target_items)?;
    launch_external_editor_vidir(items, custom_editor)
}

/// Executes a two-phase bulk rename plan with atomic rollback on any failure.
/// Phase 1: Parks each target file into a temporary intermediate UUID file.
/// Executes a two-phase bulk rename plan with atomic rollback on any failure.
/// Records successful operations into the rollback journal for one-click undo.
pub fn execute_batch_rename(plan: BatchPlanDto) -> Result<BatchExecutionResultDto, String> {
    execute_batch_rename_internal(plan, true)
}

/// Internal execution engine supporting optional journal recording.
pub fn execute_batch_rename_internal(
    plan: BatchPlanDto,
    record_in_journal: bool,
) -> Result<BatchExecutionResultDto, String> {
    if !plan.is_executable {
        return Err(
            "Cannot execute plan: batch contains conflicts or validation errors".to_string(),
        );
    }

    let changed_ops: Vec<PlannedOperationDto> = plan
        .operations
        .into_iter()
        .filter(|op| op.status == OperationStatus::Ok && op.original_path != op.target_path)
        .collect();

    if changed_ops.is_empty() {
        return Ok(BatchExecutionResultDto {
            batch_id: plan.batch_id,
            success: true,
            total_count: 0,
            succeeded_count: 0,
            failed_count: 0,
            is_rolled_back: false,
            error_message: None,
            affected_items: Vec::new(),
        });
    }

    // Verify all original source files exist
    for op in &changed_ops {
        let p = Path::new(&op.original_path);
        if !p.exists() {
            return Err(format!("Source file does not exist: {}", op.original_path));
        }
    }

    let mut phase1_moves: Vec<(&PlannedOperationDto, PathBuf)> =
        Vec::with_capacity(changed_ops.len());

    // --- PHASE 1: Move each file to intermediate temporary UUID name ---
    for op in &changed_ops {
        let orig_path = PathBuf::from(&op.original_path);
        let parent_dir = orig_path.parent().unwrap_or_else(|| Path::new("."));
        let temp_name = format!(".cuddle_tmp_{}_{}.tmp", plan.batch_id, uuid::Uuid::new_v4());
        let temp_path = parent_dir.join(temp_name);

        if let Err(e) = std::fs::rename(&orig_path, &temp_path) {
            // ROLLBACK PHASE 1: Revert all moves done so far in Phase 1
            for (prev_op, prev_temp) in phase1_moves.iter().rev() {
                let _ = std::fs::rename(prev_temp, &prev_op.original_path);
            }
            return Err(format!(
                "Failed in Phase 1 (temp parking) for '{}': {}. Entire batch rolled back.",
                op.display_name, e
            ));
        }

        phase1_moves.push((op, temp_path));
    }

    // --- PHASE 2: Move from intermediate temporary names to final target paths ---
    let mut phase2_committed: Vec<(&PlannedOperationDto, PathBuf)> =
        Vec::with_capacity(changed_ops.len());

    for (op, temp_path) in &phase1_moves {
        let target_path = PathBuf::from(&op.target_path);

        if let Err(e) = std::fs::rename(temp_path, &target_path) {
            // ROLLBACK PHASE 2: Revert committed target paths back to temp names
            for (c_op, c_target) in phase2_committed.iter().rev() {
                if let Some((_, orig_temp)) = phase1_moves
                    .iter()
                    .find(|(m_op, _)| m_op.item_id == c_op.item_id)
                {
                    let _ = std::fs::rename(c_target, orig_temp);
                }
            }

            // ROLLBACK PHASE 1: Revert all temp names back to original paths
            for (p1_op, p1_temp) in phase1_moves.iter().rev() {
                let _ = std::fs::rename(p1_temp, &p1_op.original_path);
            }

            return Err(format!(
                "Failed in Phase 2 (final renaming) for '{}' -> '{}': {}. Entire batch safely rolled back.",
                op.display_name, op.proposed_name, e
            ));
        }

        phase2_committed.push((op, target_path));
    }

    // --- PHASE 3: Update WorkspaceManager registry ---
    let ws_mgr = crate::workspace::get_workspace_manager();
    let mut affected_items = Vec::with_capacity(changed_ops.len());

    for op in changed_ops {
        let target_p = PathBuf::from(&op.target_path);
        let _ = ws_mgr.update_staged_path(&op.item_id, &target_p, &op.proposed_name);
        affected_items.push(op);
    }

    let count = affected_items.len();

    // Record in journal if requested
    if record_in_journal && count > 0 {
        let now_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        get_journal_manager().record_batch(BatchJournalEntry {
            batch_id: plan.batch_id.clone(),
            executed_at_ms: now_ms,
            operations: affected_items.clone(),
            is_undone: false,
        });

        // Record persistent operation history in SQLite WAL database
        let hist_mgr = crate::history::get_history_manager();
        let _ = hist_mgr.record_operation(&crate::history::NewHistoryEntry {
            operation_id: plan.batch_id.clone(),
            operation_type: "BulkRename".to_string(),
            display_name: format!("{} files renamed", count),
            original_path: None,
            staged_path: None,
            inode: None,
            status: "Success".to_string(),
            can_undo: true,
            details: Some(format!("Batch ID: {}", plan.batch_id)),
        });
    }

    Ok(BatchExecutionResultDto {
        batch_id: plan.batch_id,
        success: true,
        total_count: count,
        succeeded_count: count,
        failed_count: 0,
        is_rolled_back: false,
        error_message: None,
        affected_items,
    })
}

/// Rollback Journal Manager to allow atomic undo of bulk rename operations
pub struct BatchJournalManager {
    history: Mutex<Vec<BatchJournalEntry>>,
}

impl Default for BatchJournalManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BatchJournalManager {
    pub fn new() -> Self {
        Self {
            history: Mutex::new(Vec::new()),
        }
    }

    pub fn record_batch(&self, entry: BatchJournalEntry) {
        let mut hist = self.history.lock().unwrap();
        hist.push(entry);
    }

    pub fn can_undo(&self) -> bool {
        let hist = self.history.lock().unwrap();
        hist.iter().any(|entry| !entry.is_undone)
    }

    pub fn clear(&self) {
        let mut hist = self.history.lock().unwrap();
        hist.clear();
    }

    pub fn undo_last(&self) -> Result<BatchExecutionResultDto, String> {
        let hist = self.history.lock().unwrap();
        let last_idx = hist
            .iter()
            .rposition(|entry| !entry.is_undone)
            .ok_or_else(|| "No bulk rename operation available to undo".to_string())?;
        let target_batch_id = hist[last_idx].batch_id.clone();
        drop(hist);

        self.undo_batch_by_id(&target_batch_id)
    }

    /// Selectively undo a bulk rename operation by its batch_id (Design 6.2)
    pub fn undo_batch_by_id(
        &self,
        target_batch_id: &str,
    ) -> Result<BatchExecutionResultDto, String> {
        let mut hist = self.history.lock().unwrap();
        let target_idx = hist
            .iter()
            .rposition(|entry| entry.batch_id == target_batch_id && !entry.is_undone)
            .ok_or_else(|| {
                format!(
                    "No active undo journal entry found for batch ID: {}",
                    target_batch_id
                )
            })?;

        let operations_to_reverse = hist[target_idx].operations.clone();

        // Construct reverse BatchPlanDto
        let undo_batch_id = format!("undo_{}", target_batch_id);
        let mut reverse_ops = Vec::with_capacity(operations_to_reverse.len());

        for op in &operations_to_reverse {
            reverse_ops.push(PlannedOperationDto {
                item_id: op.item_id.clone(),
                original_path: op.target_path.clone(), // Current path on disk is target_path
                display_name: op.proposed_name.clone(), // Current display name
                raw_base64: op.raw_base64.clone(),
                proposed_name: op.display_name.clone(), // Revert strictly to original user display name (NOT internal disk filename)
                target_path: op.original_path.clone(),  // Revert to original path on disk
                status: OperationStatus::Ok,
                message: None,
            });
        }

        let reverse_plan = BatchPlanDto {
            batch_id: undo_batch_id.clone(),
            operations: reverse_ops,
            total_count: operations_to_reverse.len(),
            changed_count: operations_to_reverse.len(),
            conflict_count: 0,
            error_count: 0,
            is_executable: true,
        };

        // Execute reverse rename without re-recording it into the journal
        let result = execute_batch_rename_internal(reverse_plan, false)?;

        // Mark entry as undone in memory and SQLite persistent history
        hist[target_idx].is_undone = true;

        let hist_mgr = crate::history::get_history_manager();
        let _ = hist_mgr.mark_as_undone(target_batch_id);
        let _ = hist_mgr.record_operation(&crate::history::NewHistoryEntry {
            operation_id: undo_batch_id,
            operation_type: "UndoBulkRename".to_string(),
            display_name: format!("Restored {} files", result.succeeded_count),
            original_path: None,
            staged_path: None,
            inode: None,
            status: "Success".to_string(),
            can_undo: false,
            details: Some(format!("Undid batch: {}", target_batch_id)),
        });

        Ok(result)
    }
}

pub fn get_journal_manager() -> &'static BatchJournalManager {
    static INSTANCE: OnceLock<BatchJournalManager> = OnceLock::new();
    INSTANCE.get_or_init(BatchJournalManager::new)
}

pub fn can_undo_batch() -> bool {
    get_journal_manager().can_undo()
}

pub fn undo_last_batch() -> Result<BatchExecutionResultDto, String> {
    get_journal_manager().undo_last()
}

pub fn undo_batch_by_id(batch_id: &str) -> Result<BatchExecutionResultDto, String> {
    get_journal_manager().undo_batch_by_id(batch_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_and_join_stem_and_ext() {
        assert_eq!(
            split_stem_and_ext("report.pdf"),
            ("report".to_string(), Some("pdf".to_string()))
        );
        assert_eq!(
            split_stem_and_ext(".gitignore"),
            (".gitignore".to_string(), None)
        );
        assert_eq!(
            split_stem_and_ext(".config.json"),
            (".config".to_string(), Some("json".to_string()))
        );
        assert_eq!(
            split_stem_and_ext("Makefile"),
            ("Makefile".to_string(), None)
        );

        assert_eq!(join_stem_and_ext("report", Some("pdf")), "report.pdf");
        assert_eq!(join_stem_and_ext("Makefile", None), "Makefile");
    }

    #[test]
    fn test_unicode_normalization_nfc_nfd() {
        // Japanese Dakuten separation test (Mac style NFD -> NFC)
        // 'ガ' in NFC is '\u{30AC}', in NFD is 'カ' ('\u{30AB}') + dakuten ('\u{3099}')
        let nfd_str = "\u{30AB}\u{3099}ストロ.txt";
        let rule_nfc = RenameRule::UnicodeNormalize {
            form: NormalizationForm::Nfc,
        };
        let result = apply_rule(nfd_str, &rule_nfc, 0, None).unwrap();
        assert_eq!(result, "ガストロ.txt");
    }

    #[test]
    fn test_regex_replace() {
        let rule = RenameRule::RegexReplace {
            pattern: r"^IMG_(\d+)".to_string(),
            replacement: "Photo_$1".to_string(),
        };
        let result = apply_rule("IMG_1234.JPG", &rule, 0, None).unwrap();
        assert_eq!(result, "Photo_1234.JPG");
    }

    #[test]
    fn test_character_conversion_fullwidth_and_space() {
        let rule = RenameRule::ConvertCharacter {
            half_width_alnum: true,
            half_width_kana: false,
            space_to_underscore: true,
            underscore_to_space: false,
            to_lowercase: true,
            to_uppercase: false,
        };
        // 'ＡＢＣ １２３.txt' -> 'abc_123.txt'
        let result = apply_rule("ＡＢＣ １２３.txt", &rule, 0, None).unwrap();
        assert_eq!(result, "abc_123.txt");
    }

    #[test]
    fn test_sequence_number_with_padding() {
        let rule = RenameRule::AddSequence {
            start: 1,
            step: 1,
            padding: 3,
            position: SequencePosition::SuffixBeforeExtension,
            preserve_extension: true,
        };
        let result0 = apply_rule("document.pdf", &rule, 0, None).unwrap();
        let result1 = apply_rule("document.pdf", &rule, 1, None).unwrap();
        assert_eq!(result0, "document001.pdf");
        assert_eq!(result1, "document002.pdf");
    }

    #[test]
    fn test_dry_run_duplicate_conflict_detection() {
        let temp_dir =
            std::env::temp_dir().join(format!("cuddle_batch_dup_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let p1 = temp_dir.join("a.txt");
        let p2 = temp_dir.join("b.txt");

        let items = vec![
            RenameTargetItem {
                item_id: "1".to_string(),
                original_path: p1,
                display_name: "a.txt".to_string(),
                raw_base64: "YS50eHQ=".to_string(),
                modified_time: None,
            },
            RenameTargetItem {
                item_id: "2".to_string(),
                original_path: p2,
                display_name: "b.txt".to_string(),
                raw_base64: "Yi50eHQ=".to_string(),
                modified_time: None,
            },
        ];

        // Rule: change all names to fixed "fixed.txt"
        let rule = RenameRule::RegexReplace {
            pattern: ".*".to_string(),
            replacement: "fixed.txt".to_string(),
        };

        let plan = generate_batch_plan(items, &[rule]);
        assert_eq!(plan.conflict_count, 2);
        assert!(!plan.is_executable);
        assert_eq!(
            plan.operations[0].status,
            OperationStatus::ConflictDuplicate
        );
        assert_eq!(
            plan.operations[1].status,
            OperationStatus::ConflictDuplicate
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_dry_run_disk_conflict_detection() {
        let temp_dir =
            std::env::temp_dir().join(format!("cuddle_batch_disk_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let p1 = temp_dir.join("source.txt");
        let target_on_disk = temp_dir.join("existing.txt");
        std::fs::write(&target_on_disk, "already exists").unwrap();

        let items = vec![RenameTargetItem {
            item_id: "1".to_string(),
            original_path: p1,
            display_name: "source.txt".to_string(),
            raw_base64: "c291cmNlLnR4dA==".to_string(),
            modified_time: None,
        }];

        let rule = RenameRule::RegexReplace {
            pattern: "source".to_string(),
            replacement: "existing".to_string(),
        };

        let plan = generate_batch_plan(items, &[rule]);
        assert_eq!(plan.conflict_count, 1);
        assert!(!plan.is_executable);
        assert_eq!(
            plan.operations[0].status,
            OperationStatus::ConflictExistsOnDisk
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_two_phase_rename_swap_success() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle_swap_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let p1 = temp_dir.join("first.txt");
        let p2 = temp_dir.join("second.txt");
        std::fs::write(&p1, "content-first").unwrap();
        std::fs::write(&p2, "content-second").unwrap();

        // Swap plan: first.txt -> second.txt, second.txt -> first.txt
        let plan = BatchPlanDto {
            batch_id: "swap-batch".to_string(),
            operations: vec![
                PlannedOperationDto {
                    item_id: "1".to_string(),
                    original_path: p1.to_string_lossy().to_string(),
                    display_name: "first.txt".to_string(),
                    raw_base64: "dummy1".to_string(),
                    proposed_name: "second.txt".to_string(),
                    target_path: p2.to_string_lossy().to_string(),
                    status: OperationStatus::Ok,
                    message: None,
                },
                PlannedOperationDto {
                    item_id: "2".to_string(),
                    original_path: p2.to_string_lossy().to_string(),
                    display_name: "second.txt".to_string(),
                    raw_base64: "dummy2".to_string(),
                    proposed_name: "first.txt".to_string(),
                    target_path: p1.to_string_lossy().to_string(),
                    status: OperationStatus::Ok,
                    message: None,
                },
            ],
            total_count: 2,
            changed_count: 2,
            conflict_count: 0,
            error_count: 0,
            is_executable: true,
        };

        let result = execute_batch_rename(plan).expect("Two-phase swap must succeed");
        assert_eq!(result.succeeded_count, 2);
        assert!(!result.is_rolled_back);

        // Verify contents swapped without loss
        assert_eq!(std::fs::read_to_string(&p1).unwrap(), "content-second");
        assert_eq!(std::fs::read_to_string(&p2).unwrap(), "content-first");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_two_phase_rename_missing_file_safety() {
        let temp_dir =
            std::env::temp_dir().join(format!("cuddle_missing_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let p1 = temp_dir.join("exist.txt");
        let p2 = temp_dir.join("non_existent.txt");
        std::fs::write(&p1, "safe-content").unwrap();

        let plan = BatchPlanDto {
            batch_id: "missing-batch".to_string(),
            operations: vec![
                PlannedOperationDto {
                    item_id: "1".to_string(),
                    original_path: p1.to_string_lossy().to_string(),
                    display_name: "exist.txt".to_string(),
                    raw_base64: "dummy".to_string(),
                    proposed_name: "exist_renamed.txt".to_string(),
                    target_path: temp_dir
                        .join("exist_renamed.txt")
                        .to_string_lossy()
                        .to_string(),
                    status: OperationStatus::Ok,
                    message: None,
                },
                PlannedOperationDto {
                    item_id: "2".to_string(),
                    original_path: p2.to_string_lossy().to_string(),
                    display_name: "non_existent.txt".to_string(),
                    raw_base64: "dummy".to_string(),
                    proposed_name: "failed.txt".to_string(),
                    target_path: temp_dir.join("failed.txt").to_string_lossy().to_string(),
                    status: OperationStatus::Ok,
                    message: None,
                },
            ],
            total_count: 2,
            changed_count: 2,
            conflict_count: 0,
            error_count: 0,
            is_executable: true,
        };

        // Execution should fail pre-flight and leave original file intact
        let res = execute_batch_rename(plan);
        assert!(res.is_err());
        assert!(
            p1.exists(),
            "Original file must remain completely untouched"
        );
        assert_eq!(std::fs::read_to_string(&p1).unwrap(), "safe-content");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_batch_rename_and_atomic_undo() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle_undo_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let p1 = temp_dir.join("alpha.txt");
        let p2 = temp_dir.join("beta.txt");
        std::fs::write(&p1, "alpha-data").unwrap();
        std::fs::write(&p2, "beta-data").unwrap();

        let plan = BatchPlanDto {
            batch_id: "undo-lifecycle-batch".to_string(),
            operations: vec![
                PlannedOperationDto {
                    item_id: "1".to_string(),
                    original_path: p1.to_string_lossy().to_string(),
                    display_name: "alpha.txt".to_string(),
                    raw_base64: "dummy1".to_string(),
                    proposed_name: "ALPHA_RENAMED.txt".to_string(),
                    target_path: temp_dir
                        .join("ALPHA_RENAMED.txt")
                        .to_string_lossy()
                        .to_string(),
                    status: OperationStatus::Ok,
                    message: None,
                },
                PlannedOperationDto {
                    item_id: "2".to_string(),
                    original_path: p2.to_string_lossy().to_string(),
                    display_name: "beta.txt".to_string(),
                    raw_base64: "dummy2".to_string(),
                    proposed_name: "BETA_RENAMED.txt".to_string(),
                    target_path: temp_dir
                        .join("BETA_RENAMED.txt")
                        .to_string_lossy()
                        .to_string(),
                    status: OperationStatus::Ok,
                    message: None,
                },
            ],
            total_count: 2,
            changed_count: 2,
            conflict_count: 0,
            error_count: 0,
            is_executable: true,
        };

        // 1. Execute rename with dedicated journal
        let journal = BatchJournalManager::new();
        let res = execute_batch_rename_internal(plan, false).expect("Initial rename must succeed");
        assert_eq!(res.succeeded_count, 2);

        journal.record_batch(BatchJournalEntry {
            batch_id: "undo-lifecycle-batch".to_string(),
            executed_at_ms: 1000,
            operations: res.affected_items,
            is_undone: false,
        });

        // Verify renamed files exist on disk
        let p1_renamed = temp_dir.join("ALPHA_RENAMED.txt");
        let p2_renamed = temp_dir.join("BETA_RENAMED.txt");
        assert!(p1_renamed.exists());
        assert!(p2_renamed.exists());
        assert!(!p1.exists());
        assert!(!p2.exists());

        // 2. Check can_undo
        assert!(journal.can_undo());

        // 3. Execute undo
        let undo_res = journal.undo_last().expect("Undo must succeed");
        assert_eq!(undo_res.succeeded_count, 2);

        // Verify original files are completely restored
        assert!(p1.exists(), "Original alpha.txt must be restored");
        assert!(p2.exists(), "Original beta.txt must be restored");
        assert!(!p1_renamed.exists());
        assert!(!p2_renamed.exists());
        assert_eq!(std::fs::read_to_string(&p1).unwrap(), "alpha-data");
        assert_eq!(std::fs::read_to_string(&p2).unwrap(), "beta-data");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_staged_item_undo_restores_display_name_without_uuid() {
        let temp_dir =
            std::env::temp_dir().join(format!("cuddle_staged_undo_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let stage_id = "test-stage-uuid-456".to_string();
        let staged_filename = format!("{}_04.txt", stage_id);
        let disk_path = temp_dir.join(&staged_filename);
        std::fs::write(&disk_path, "04-content").unwrap();

        let item = RenameTargetItem {
            item_id: stage_id.clone(),
            original_path: disk_path.clone(),
            display_name: "04.txt".to_string(), // User-facing display name
            raw_base64: "dummy".to_string(),
            modified_time: None,
        };

        // Add a prefix rule
        let rules = vec![RenameRule::AddPrefixSuffix {
            prefix: "renamed_".to_string(),
            suffix: "".to_string(),
            preserve_extension: true,
        }];

        let plan = generate_batch_plan(vec![item], &rules);
        assert_eq!(plan.operations[0].proposed_name, "renamed_04.txt");
        // Target path on disk should retain the stage_id prefix
        let expected_target_path = temp_dir.join(format!("{}_renamed_04.txt", stage_id));
        assert_eq!(
            plan.operations[0].target_path,
            expected_target_path.to_string_lossy().to_string()
        );

        // Execute rename with dedicated journal
        let journal = BatchJournalManager::new();
        let res = execute_batch_rename_internal(plan, false).expect("Bulk rename must succeed");
        assert_eq!(res.succeeded_count, 1);
        assert!(expected_target_path.exists());
        assert!(!disk_path.exists());

        journal.record_batch(BatchJournalEntry {
            batch_id: "staged-undo-batch".to_string(),
            executed_at_ms: 1000,
            operations: res.affected_items,
            is_undone: false,
        });

        // Undo
        let undo_res = journal.undo_last().expect("Undo must succeed");
        assert_eq!(undo_res.succeeded_count, 1);
        assert_eq!(
            undo_res.affected_items[0].proposed_name, "04.txt",
            "Display name in undo plan must be original display name, NEVER uuid prefix"
        );
        assert!(disk_path.exists(), "Original disk path must be restored");
        assert!(!expected_target_path.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_vidir_buffer_generation_and_parsing() {
        let temp_dir =
            std::env::temp_dir().join(format!("cuddle_test_vidir_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let item1 = RenameTargetItem {
            item_id: "id-101".to_string(),
            original_path: temp_dir.join("photo1.jpg"),
            display_name: "photo1.jpg".to_string(),
            raw_base64: "dummy1".to_string(),
            modified_time: None,
        };
        let item2 = RenameTargetItem {
            item_id: "id-102".to_string(),
            original_path: temp_dir.join("photo2.jpg"),
            display_name: "photo2.jpg".to_string(),
            raw_base64: "dummy2".to_string(),
            modified_time: None,
        };
        let item3 = RenameTargetItem {
            item_id: "id-103".to_string(),
            original_path: temp_dir.join("document.pdf"),
            display_name: "document.pdf".to_string(),
            raw_base64: "dummy3".to_string(),
            modified_time: None,
        };

        let items = vec![item1, item2, item3];

        // 1. Generate buffer
        let buf = generate_vidir_buffer(&items);
        assert!(buf.contains("id-101\tphoto1.jpg"));
        assert!(buf.contains("id-102\tphoto2.jpg"));
        assert!(buf.contains("id-103\tdocument.pdf"));

        // 2. Simulate user editing:
        // - reorder items (id-103 first, then id-101)
        // - delete id-102 (line deletion must be treated as UNCHANGED, NEVER file deletion)
        // - modify id-101 to "photo_renamed.jpg"
        // - leave id-103 as "document.pdf" (unchanged)
        let edited = r#"
# Custom comment
id-103	document.pdf
id-101	photo_renamed.jpg
"#;

        let plan = parse_vidir_buffer(items, edited);
        assert_eq!(plan.total_count, 3);
        assert_eq!(plan.changed_count, 1);
        assert_eq!(plan.conflict_count, 0);
        assert_eq!(plan.error_count, 0);
        assert!(plan.is_executable);

        let op101 = plan
            .operations
            .iter()
            .find(|op| op.item_id == "id-101")
            .unwrap();
        assert_eq!(op101.proposed_name, "photo_renamed.jpg");
        assert_eq!(op101.status, OperationStatus::Ok);

        let op102 = plan
            .operations
            .iter()
            .find(|op| op.item_id == "id-102")
            .unwrap();
        assert_eq!(op102.proposed_name, "photo2.jpg");
        assert_eq!(
            op102.status,
            OperationStatus::Unchanged,
            "Deleted line MUST be kept as Unchanged"
        );

        let op103 = plan
            .operations
            .iter()
            .find(|op| op.item_id == "id-103")
            .unwrap();
        assert_eq!(op103.proposed_name, "document.pdf");
        assert_eq!(op103.status, OperationStatus::Unchanged);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_vidir_collision_detection() {
        let temp_dir =
            std::env::temp_dir().join(format!("cuddle_test_vidir_col_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Existing file on disk
        std::fs::write(temp_dir.join("existing_on_disk.txt"), "hello").unwrap();

        let item1 = RenameTargetItem {
            item_id: "id-1".to_string(),
            original_path: temp_dir.join("f1.txt"),
            display_name: "f1.txt".to_string(),
            raw_base64: "d1".to_string(),
            modified_time: None,
        };
        let item2 = RenameTargetItem {
            item_id: "id-2".to_string(),
            original_path: temp_dir.join("f2.txt"),
            display_name: "f2.txt".to_string(),
            raw_base64: "d2".to_string(),
            modified_time: None,
        };

        let items = vec![item1, item2];

        // 1. Duplicate conflict
        let edited_dup = "id-1\tsame_name.txt\nid-2\tsame_name.txt\n";
        let plan_dup = parse_vidir_buffer(items.clone(), edited_dup);
        assert_eq!(plan_dup.conflict_count, 2);
        assert!(!plan_dup.is_executable);

        // 2. On-disk conflict
        let edited_disk = "id-1\texisting_on_disk.txt\n";
        let plan_disk = parse_vidir_buffer(items.clone(), edited_disk);
        let op1 = plan_disk
            .operations
            .iter()
            .find(|o| o.item_id == "id-1")
            .unwrap();
        assert_eq!(op1.status, OperationStatus::ConflictExistsOnDisk);
        assert!(!plan_disk.is_executable);

        // 3. Invalid filename
        let edited_invalid = "id-1\tinvalid/name.txt\n";
        let plan_invalid = parse_vidir_buffer(items, edited_invalid);
        let op_inv = plan_invalid
            .operations
            .iter()
            .find(|o| o.item_id == "id-1")
            .unwrap();
        assert_eq!(op_inv.status, OperationStatus::InvalidFilename);
        assert!(!plan_invalid.is_executable);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_selective_undo_by_batch_id() {
        let temp_dir =
            std::env::temp_dir().join(format!("cuddle_sel_undo_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let f1 = temp_dir.join("origin_a.txt");
        let f2 = temp_dir.join("origin_b.txt");
        std::fs::write(&f1, "data A").unwrap();
        std::fs::write(&f2, "data B").unwrap();

        let batch_id = format!("batch-sel-{}", uuid::Uuid::new_v4());
        let plan = BatchPlanDto {
            batch_id: batch_id.clone(),
            operations: vec![
                PlannedOperationDto {
                    item_id: "item-a".to_string(),
                    original_path: f1.to_string_lossy().to_string(),
                    display_name: "origin_a.txt".to_string(),
                    raw_base64: "dummy_a".to_string(),
                    proposed_name: "renamed_a.txt".to_string(),
                    target_path: temp_dir.join("renamed_a.txt").to_string_lossy().to_string(),
                    status: OperationStatus::Ok,
                    message: None,
                },
                PlannedOperationDto {
                    item_id: "item-b".to_string(),
                    original_path: f2.to_string_lossy().to_string(),
                    display_name: "origin_b.txt".to_string(),
                    raw_base64: "dummy_b".to_string(),
                    proposed_name: "renamed_b.txt".to_string(),
                    target_path: temp_dir.join("renamed_b.txt").to_string_lossy().to_string(),
                    status: OperationStatus::Ok,
                    message: None,
                },
            ],
            total_count: 2,
            changed_count: 2,
            conflict_count: 0,
            error_count: 0,
            is_executable: true,
        };

        // Execute batch
        let res = execute_batch_rename(plan).expect("Execute must succeed");
        assert_eq!(res.succeeded_count, 2);
        assert!(!f1.exists());
        assert!(temp_dir.join("renamed_a.txt").exists());

        // Selective Undo by Batch ID
        let undo_res = undo_batch_by_id(&batch_id).expect("Selective undo must succeed");
        assert_eq!(undo_res.succeeded_count, 2);
        assert!(f1.exists(), "Original A must be restored");
        assert!(f2.exists(), "Original B must be restored");
        assert!(!temp_dir.join("renamed_a.txt").exists());
        assert!(!temp_dir.join("renamed_b.txt").exists());

        // Trying to undo again should fail
        assert!(undo_batch_by_id(&batch_id).is_err());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_export_typescript_bindings_batch() {
        let _ = std::fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        BatchPlanDto::export(&cfg).expect("Failed to export BatchPlanDto");
        BatchExecutionResultDto::export(&cfg).expect("Failed to export BatchExecutionResultDto");
        BatchJournalEntry::export(&cfg).expect("Failed to export BatchJournalEntry");
        RenameRule::export(&cfg).expect("Failed to export RenameRule");
        NormalizationForm::export(&cfg).expect("Failed to export NormalizationForm");
        SequencePosition::export(&cfg).expect("Failed to export SequencePosition");
        DateTimeSource::export(&cfg).expect("Failed to export DateTimeSource");
        OperationStatus::export(&cfg).expect("Failed to export OperationStatus");
        PlannedOperationDto::export(&cfg).expect("Failed to export PlannedOperationDto");
        RenameTargetInputDto::export(&cfg).expect("Failed to export RenameTargetInputDto");
        BulkRenameOptions::export(&cfg).expect("Failed to export BulkRenameOptions");
    }
}
