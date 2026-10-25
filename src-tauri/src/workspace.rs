use crate::models::{decode_path_from_base64, encode_path_to_base64, FileItem, ModelError};
use crate::policy::{check_disk_space, canonicalize_path, is_external_or_mount_boundary, OperationPolicy, OriginalSnapshotDto, PolicyError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Model error: {0}")]
    Model(#[from] ModelError),
    #[error("Policy error: {0}")]
    Policy(#[from] PolicyError),
    #[error("Diff error: {0}")]
    Diff(#[from] crate::diff::DiffError),
    #[error("Staged item not found: {0}")]
    NotFound(String),
    #[error("Failed to determine staging directory")]
    StagingDirUnavailable,
    #[error("Insufficient disk space in staging destination")]
    InsufficientSpace,
}

impl WorkspaceError {
    pub fn error_code(&self) -> &'static str {
        match self {
            WorkspaceError::Policy(p) => p.error_code(),
            WorkspaceError::NotFound(_) => "ERR_STAGE_NOT_FOUND",
            WorkspaceError::StagingDirUnavailable => "ERR_STAGING_DIR_UNAVAILABLE",
            WorkspaceError::InsufficientSpace => "ERR_INSUFFICIENT_SPACE",
            WorkspaceError::Io(_) => "ERR_IO",
            WorkspaceError::Model(_) => "ERR_INVALID_PATH",
            WorkspaceError::Diff(_) => "ERR_DIFF_FAILED",
        }
    }
}

/// The cloning mechanism utilized to create the working copy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub enum CloneMethod {
    /// Instant zero-storage Copy-on-Write clone using Linux FICLONE ioctl
    Reflink,
    /// Byte-for-byte stream fallback copy
    StandardCopy,
}

/// Current status of the staged working copy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub enum StageStatus {
    /// Working copy is staged and pristine (matches original snapshot)
    Staged,
    /// Working copy has been mutated in workspace-temp
    Modified,
    /// Changes have been safely reflected back to the original
    Reflected,
}

/// DTO representing an isolated staged file in the workbench.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct StagedItemDto {
    pub stage_id: String,
    pub original_item_id: String,
    pub raw_base64: String,
    pub display_name: String,
    pub staged_raw_base64: String,
    pub staged_path: String,
    pub size: u64,
    pub clone_method: CloneMethod,
    pub status: StageStatus,
    pub staged_at_ms: i64,
    pub original_snapshot: OriginalSnapshotDto,
}

/// Resolves the global fallback staging directory (`$XDG_CACHE_HOME/Cuddle/staging/`).
pub fn get_global_staging_directory() -> PathBuf {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(std::env::temp_dir).join(".cache"));
    cache_dir.join("Cuddle").join("staging")
}

/// Determines the optimal staging directory for an original file.
/// Prioritizes a local `.cuddle-temp` on the same volume/filesystem.
/// Falls back to `$XDG_CACHE_HOME/Cuddle/staging/` if same-volume is not writable or safe.
pub fn resolve_staging_directory<P: AsRef<Path>>(original_path: P) -> PathBuf {
    let orig = original_path.as_ref();
    if let Some(parent) = orig.parent() {
        // Try same-directory or nearest parent .cuddle-temp
        let local_temp = parent.join(".cuddle-temp");
        if !is_external_or_mount_boundary(&local_temp) {
            if let Ok(()) = fs::create_dir_all(&local_temp) {
                // Test write permissions
                let probe_file = local_temp.join(format!(".probe_{}", Uuid::new_v4()));
                if fs::write(&probe_file, b"probe").is_ok() {
                    let _ = fs::remove_file(probe_file);
                    return local_temp;
                }
            }
        }
    }

    let global_staging = get_global_staging_directory();
    let _ = fs::create_dir_all(&global_staging);
    global_staging
}

/// Clones a source file to destination, attempting Reflink (ioctl FICLONE) first,
/// falling back to standard streaming byte copy.
pub fn clone_file<P: AsRef<Path>, Q: AsRef<Path>>(
    src: P,
    dest: Q,
) -> Result<CloneMethod, WorkspaceError> {
    let src_path = src.as_ref();
    let dest_path = dest.as_ref();

    let src_file = File::open(src_path)?;
    let dest_file = File::create(dest_path)?;

    // Attempt Reflink (CoW) using Linux FICLONE ioctl
    #[cfg(target_os = "linux")]
    {
        let src_fd = src_file.as_raw_fd();
        let dest_fd = dest_file.as_raw_fd();

        let ret = unsafe { libc::ioctl(dest_fd, libc::FICLONE, src_fd) };
        if ret == 0 {
            return Ok(CloneMethod::Reflink);
        }
    }

    // Fallback: standard byte copy
    drop(dest_file);
    drop(src_file);
    fs::copy(src_path, dest_path)?;

    Ok(CloneMethod::StandardCopy)
}

/// Thread-safe in-memory manager tracking all staged working copies.
pub struct WorkspaceManager {
    staged_items: Mutex<HashMap<String, StagedItemDto>>,
}

impl Default for WorkspaceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceManager {
    pub fn new() -> Self {
        Self {
            staged_items: Mutex::new(HashMap::new()),
        }
    }

    /// Stages a file: validates capacity, takes snapshot, clones to staging area.
    pub fn stage_file<P: AsRef<Path>>(&self, path: P) -> Result<StagedItemDto, WorkspaceError> {
        let p = path.as_ref();
        let canonical_src = canonicalize_path(p)?;
        let item = FileItem::from_path(&canonical_src)?;

        // 1. Take original snapshot with SHA-256 for TOCTOU and conflict tracking
        let snapshot = OperationPolicy::create_snapshot(&item, true)?;

        // 2. Select staging directory (Same-volume preferred, global fallback)
        let staging_dir = resolve_staging_directory(&canonical_src);

        // 3. Pre-flight check: ensure at least 1.5x disk capacity is available
        check_disk_space(&staging_dir, snapshot.size, 1.5)?;

        // 4. Generate unique staged filename preserving original name
        let stage_id = Uuid::new_v4().to_string();
        let staged_filename = format!("{}_{}", stage_id, item.display_name);
        let staged_target_path = staging_dir.join(staged_filename);

        // 5. Perform safe cloning (Reflink CoW preferred, standard stream fallback)
        let clone_method = clone_file(&canonical_src, &staged_target_path)?;

        let now_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let staged_dto = StagedItemDto {
            stage_id: stage_id.clone(),
            original_item_id: item.item_id.clone(),
            raw_base64: item.raw_base64.clone(),
            display_name: item.display_name.clone(),
            staged_raw_base64: encode_path_to_base64(&staged_target_path),
            staged_path: staged_target_path.to_string_lossy().to_string(),
            size: snapshot.size,
            clone_method,
            status: StageStatus::Staged,
            staged_at_ms: now_ms,
            original_snapshot: snapshot,
        };

        // 6. Record in active staging registry
        let mut map = self.staged_items.lock().unwrap();
        map.insert(stage_id.clone(), staged_dto.clone());

        // 7. Record operation in SQLite persistent history
        let hist_mgr = crate::history::get_history_manager();
        let _ = hist_mgr.record_operation(&crate::history::NewHistoryEntry {
            operation_id: stage_id,
            operation_type: "Stage".to_string(),
            display_name: item.display_name.clone(),
            original_path: Some(canonical_src.to_string_lossy().to_string()),
            staged_path: Some(staged_dto.staged_path.clone()),
            inode: Some(staged_dto.original_snapshot.inode as i64),
            status: "Success".to_string(),
            can_undo: true,
            details: Some(format!("Cloned via {:?}", staged_dto.clone_method)),
        });

        Ok(staged_dto)
    }

    /// Unstages a file: securely deletes the staged copy (ORIGINAL IS NEVER TOUCHED).
    pub fn unstage_file(&self, stage_id: &str) -> Result<(), WorkspaceError> {
        let mut map = self.staged_items.lock().unwrap();
        let target_staged = if let Some(staged) = map.remove(stage_id) {
            Some(staged)
        } else {
            let found_key = map.iter().find_map(|(k, v)| {
                if v.original_item_id == stage_id
                    || v.raw_base64 == stage_id
                    || v.staged_raw_base64 == stage_id
                    || v.display_name == stage_id
                {
                    Some(k.clone())
                } else {
                    None
                }
            });
            found_key.and_then(|k| map.remove(&k))
        };

        if let Some(staged) = target_staged {
            let path = decode_path_from_base64(&staged.staged_raw_base64)?;
            if path.exists() {
                let _ = fs::remove_file(path);
            }

            let hist_mgr = crate::history::get_history_manager();
            let _ = hist_mgr.record_operation(&crate::history::NewHistoryEntry {
                operation_id: uuid::Uuid::new_v4().to_string(),
                operation_type: "Unstage".to_string(),
                display_name: staged.display_name.clone(),
                original_path: Some(staged.original_snapshot.canonical_path.clone()),
                staged_path: Some(staged.staged_path.clone()),
                inode: Some(staged.original_snapshot.inode as i64),
                status: "Success".to_string(),
                can_undo: false,
                details: None,
            });

            return Ok(());
        }

        Err(WorkspaceError::NotFound(stage_id.to_string()))
    }

    /// Retrieves a single staged item by stage_id.
    pub fn get_staged(&self, stage_id: &str) -> Result<StagedItemDto, WorkspaceError> {
        let map = self.staged_items.lock().unwrap();
        map.get(stage_id)
            .cloned()
            .ok_or_else(|| WorkspaceError::NotFound(stage_id.to_string()))
    }

    /// Updates the staged copy path and display name after a successful batch operation.
    pub fn update_staged_path(
        &self,
        identifier: &str,
        new_path: &Path,
        new_display_name: &str,
    ) -> Result<StagedItemDto, WorkspaceError> {
        let mut map = self.staged_items.lock().unwrap();
        let target_key = if map.contains_key(identifier) {
            Some(identifier.to_string())
        } else {
            map.iter().find_map(|(k, v)| {
                if v.stage_id == identifier
                    || v.original_item_id == identifier
                    || v.staged_path == identifier
                    || v.display_name == identifier
                {
                    Some(k.clone())
                } else {
                    None
                }
            })
        };

        if let Some(key) = target_key {
            if let Some(item) = map.get_mut(&key) {
                item.staged_path = new_path.to_string_lossy().to_string();
                item.staged_raw_base64 = encode_path_to_base64(new_path);
                item.display_name = new_display_name.to_string();
                item.status = StageStatus::Modified;
                return Ok(item.clone());
            }
        }
        Err(WorkspaceError::NotFound(identifier.to_string()))
    }

    /// Resets a modified staged working copy back to pristine original state.
    pub fn reset_staged_file(&self, stage_id: &str) -> Result<StagedItemDto, WorkspaceError> {
        let mut map = self.staged_items.lock().unwrap();
        let staged = map
            .get_mut(stage_id)
            .ok_or_else(|| WorkspaceError::NotFound(stage_id.to_string()))?;

        let orig_path = decode_path_from_base64(&staged.raw_base64)?;
        let staged_path = decode_path_from_base64(&staged.staged_raw_base64)?;

        if !orig_path.exists() {
            return Err(WorkspaceError::Policy(PolicyError::NotFound(
                orig_path.to_string_lossy().to_string(),
            )));
        }

        // Re-clone from pristine original
        if staged_path.exists() {
            let _ = fs::remove_file(&staged_path);
        }
        let method = clone_file(&orig_path, &staged_path)?;

        staged.clone_method = method;
        staged.status = StageStatus::Staged;

        let hist_mgr = crate::history::get_history_manager();
        let _ = hist_mgr.record_operation(&crate::history::NewHistoryEntry {
            operation_id: uuid::Uuid::new_v4().to_string(),
            operation_type: "Reset".to_string(),
            display_name: staged.display_name.clone(),
            original_path: Some(staged.original_snapshot.canonical_path.clone()),
            staged_path: Some(staged.staged_path.clone()),
            inode: Some(staged.original_snapshot.inode as i64),
            status: "Success".to_string(),
            can_undo: false,
            details: Some("Reset staged copy back to pristine original".to_string()),
        });

        Ok(staged.clone())
    }

    /// Computes the diff between the original file and its staged working copy.
    pub fn get_diff(&self, stage_id: &str) -> Result<crate::diff::FileDiffResultDto, WorkspaceError> {
        let (raw_b64, staged_b64) = {
            let map = self.staged_items.lock().unwrap();
            let staged = map
                .get(stage_id)
                .ok_or_else(|| WorkspaceError::NotFound(stage_id.to_string()))?;
            (staged.raw_base64.clone(), staged.staged_raw_base64.clone())
        };

        let orig_path = decode_path_from_base64(&raw_b64)?;
        let staged_path = decode_path_from_base64(&staged_b64)?;

        let diff_result = crate::diff::compute_file_diff(stage_id, &orig_path, &staged_path)?;
        Ok(diff_result)
    }

    /// Reflects staged working copy changes back to the original file safely.
    /// STRICT PROTOCOL:
    /// 1. Policy validation: Forbidden Zone checks (outside $HOME, mount boundaries, external storage)
    /// 2. TOCTOU conflict verification: verifies original inode, dev, size, mtime, and SHA-256 against snapshot
    /// 3. Atomic replace: creates temporary file in the original's directory, clones/copies staged data, and renames
    /// 4. Updates snapshot & status to Reflected
    pub fn reflect_staged_file(&self, stage_id: &str) -> Result<StagedItemDto, WorkspaceError> {
        let mut map = self.staged_items.lock().unwrap();
        let staged = map
            .get_mut(stage_id)
            .ok_or_else(|| WorkspaceError::NotFound(stage_id.to_string()))?;

        let orig_path = decode_path_from_base64(&staged.raw_base64)?;
        let staged_path = decode_path_from_base64(&staged.staged_raw_base64)?;

        // 1. Policy Validation: Forbidden Zone
        OperationPolicy::validate_reflection(&orig_path)?;

        // 2. TOCTOU Race & External Conflict Verification
        OperationPolicy::verify_conflict(&staged.original_snapshot, true)?;

        // 3. Atomic Replace via Temporary File in same directory
        let orig_parent = orig_path
            .parent()
            .ok_or(WorkspaceError::StagingDirUnavailable)?;
        let tmp_reflect_name = format!(".cuddle_reflect_{}.tmp", Uuid::new_v4());
        let tmp_reflect_path = orig_parent.join(tmp_reflect_name);

        // Copy staged content to temporary file in destination volume
        if let Err(e) = fs::copy(&staged_path, &tmp_reflect_path) {
            let _ = fs::remove_file(&tmp_reflect_path);
            return Err(WorkspaceError::Io(e));
        }

        // Atomic replace via POSIX rename
        if let Err(e) = fs::rename(&tmp_reflect_path, &orig_path) {
            let _ = fs::remove_file(&tmp_reflect_path);
            return Err(WorkspaceError::Io(e));
        }

        // 4. Update snapshot and status
        let updated_item = FileItem::from_path(&orig_path)?;
        let new_snapshot = OperationPolicy::create_snapshot(&updated_item, true)?;

        staged.original_snapshot = new_snapshot;
        staged.status = StageStatus::Reflected;

        let hist_mgr = crate::history::get_history_manager();
        let _ = hist_mgr.record_operation(&crate::history::NewHistoryEntry {
            operation_id: uuid::Uuid::new_v4().to_string(),
            operation_type: "Reflect".to_string(),
            display_name: staged.display_name.clone(),
            original_path: Some(staged.original_snapshot.canonical_path.clone()),
            staged_path: Some(staged.staged_path.clone()),
            inode: Some(updated_item.inode as i64),
            status: "Success".to_string(),
            can_undo: false,
            details: Some(format!("Reflected atomic replace to {}", orig_path.display())),
        });

        Ok(staged.clone())
    }

    /// Lists all currently active staged items.
    pub fn list_staged(&self) -> Vec<StagedItemDto> {
        let map = self.staged_items.lock().unwrap();
        map.values().cloned().collect()
    }

    /// Clears all staged files from the staging area.
    pub fn clear_all(&self) -> Result<(), WorkspaceError> {
        let mut map = self.staged_items.lock().unwrap();
        for (_, staged) in map.drain() {
            if let Ok(path) = decode_path_from_base64(&staged.staged_raw_base64) {
                if path.exists() {
                    let _ = fs::remove_file(path);
                }
            }
        }

        let hist_mgr = crate::history::get_history_manager();
        let _ = hist_mgr.record_operation(&crate::history::NewHistoryEntry {
            operation_id: uuid::Uuid::new_v4().to_string(),
            operation_type: "ClearWorkspace".to_string(),
            display_name: "Cleared workspace".to_string(),
            original_path: None,
            staged_path: None,
            inode: None,
            status: "Success".to_string(),
            can_undo: false,
            details: None,
        });

        Ok(())
    }
}

// Global shared WorkspaceManager instance
static WORKSPACE_MANAGER: std::sync::OnceLock<Arc<WorkspaceManager>> = std::sync::OnceLock::new();

pub fn get_workspace_manager() -> Arc<WorkspaceManager> {
    WORKSPACE_MANAGER
        .get_or_init(|| Arc::new(WorkspaceManager::new()))
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_clone_file_standard_and_reflink_fallback() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle_ws_test_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let src_file = temp_dir.join("src_original.txt");
        let dest_file = temp_dir.join("dest_staged.txt");
        fs::write(&src_file, b"important original content").unwrap();

        let _method = clone_file(&src_file, &dest_file).unwrap();
        // Method should be Reflink or StandardCopy, both valid
        assert!(dest_file.exists());
        assert_eq!(fs::read(&dest_file).unwrap(), b"important original content");

        // Verify mutability separation: modifying dest must not change src
        fs::write(&dest_file, b"mutated in staging").unwrap();
        assert_eq!(fs::read(&src_file).unwrap(), b"important original content");
        assert_eq!(fs::read(&dest_file).unwrap(), b"mutated in staging");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_workspace_manager_staging_lifecycle() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle_ws_mgr_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let orig_file = temp_dir.join("report.md");
        fs::write(&orig_file, b"# Annual Report\nOriginal content").unwrap();

        let mgr = WorkspaceManager::new();

        // 1. Stage file
        let staged = mgr.stage_file(&orig_file).expect("Staging must succeed");
        assert_eq!(staged.display_name, "report.md");
        assert_eq!(staged.status, StageStatus::Staged);
        assert!(Path::new(&staged.staged_path).exists());

        // 2. List staged
        let list = mgr.list_staged();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].stage_id, staged.stage_id);

        // 3. Unstage file (Staged copy removed, original remains untouched)
        mgr.unstage_file(&staged.stage_id).expect("Unstaging must succeed");
        assert!(!Path::new(&staged.staged_path).exists(), "Staged file must be deleted");
        assert!(orig_file.exists(), "Original file must remain intact!");

        assert_eq!(mgr.list_staged().len(), 0);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_stage_non_utf8_filename() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let temp_dir = std::env::temp_dir().join(format!("cuddle_ws_nonutf8_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        // Invalid UTF-8 filename with 0xFF, 0xFE
        let mut os_name = OsString::from("data_");
        os_name.push(OsString::from_vec(vec![0xff, 0xfe]));
        os_name.push(".bin");

        let non_utf8_file = temp_dir.join(os_name);
        fs::write(&non_utf8_file, b"binary non-utf8 data").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&non_utf8_file).expect("Must stage non-UTF8 file without panic");

        assert!(staged.display_name.contains('\u{FFFD}'));
        assert!(Path::new(&staged.staged_path).exists());

        mgr.clear_all().unwrap();
        assert_eq!(mgr.list_staged().len(), 0);
        assert!(non_utf8_file.exists(), "Original non-UTF8 file must remain intact");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_workspace_manager_reset_to_original() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle_ws_reset_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let orig_file = temp_dir.join("code.rs");
        fs::write(&orig_file, b"fn main() { println!(\"original\"); }").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        // Mutate staged working copy
        fs::write(&staged.staged_path, b"fn main() { println!(\"mutated\"); }").unwrap();
        assert_eq!(fs::read(&staged.staged_path).unwrap(), b"fn main() { println!(\"mutated\"); }");

        // Verify diff before reset
        let diff = mgr.get_diff(&staged.stage_id).unwrap();
        assert!(diff.has_changes);

        // Reset staged copy to pristine original
        let reset_item = mgr.reset_staged_file(&staged.stage_id).unwrap();
        assert_eq!(reset_item.status, StageStatus::Staged);
        assert_eq!(fs::read(&staged.staged_path).unwrap(), b"fn main() { println!(\"original\"); }");

        // Diff after reset must be clean
        let diff_after = mgr.get_diff(&staged.stage_id).unwrap();
        assert!(!diff_after.has_changes);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_workspace_manager_reflect_success() {
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_reflect_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("document.txt");
        fs::write(&orig_file, b"Original v1").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        // Mutate staged working copy
        fs::write(&staged.staged_path, b"Mutated in staging v2").unwrap();

        // Reflect back to original
        let reflected = mgr.reflect_staged_file(&staged.stage_id).unwrap();
        assert_eq!(reflected.status, StageStatus::Reflected);

        // Verify original content is updated
        assert_eq!(fs::read(&orig_file).unwrap(), b"Mutated in staging v2");

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_manager_reflect_conflict_blocked() {
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_conflict_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("protected.txt");
        fs::write(&orig_file, b"Original content").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        // Simulate external process modifying the original behind our back
        std::thread::sleep(std::time::Duration::from_millis(15));
        fs::write(&orig_file, b"External rogue mutation").unwrap();

        // Attempt reflection: must be blocked by conflict verification
        let err = mgr.reflect_staged_file(&staged.stage_id);
        assert!(err.is_err());
        assert_eq!(err.unwrap_err().error_code(), "ERR_CONFLICT_DETECTED");

        // Original content must not be overwritten by staged content
        assert_eq!(fs::read(&orig_file).unwrap(), b"External rogue mutation");

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_export_typescript_bindings_workspace() {
        let _ = fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        StagedItemDto::export(&cfg).expect("Failed to export StagedItemDto");
        CloneMethod::export(&cfg).expect("Failed to export CloneMethod");
        StageStatus::export(&cfg).expect("Failed to export StageStatus");
    }
}
