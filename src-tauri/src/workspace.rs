use crate::models::{decode_path_from_base64, encode_path_to_base64, FileItem, ModelError};
use crate::policy::{
    canonicalize_path, check_disk_space, compute_file_sha256, is_external_or_mount_boundary,
    OperationPolicy, OriginalSnapshotDto, PolicyError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

/// Atomically swaps two file paths on the filesystem using Linux `renameat2(..., RENAME_EXCHANGE)`.
/// This eliminates TOCTOU races by atomically swapping the candidate into place
/// and quarantining the original for verification in a single kernel syscall.
#[cfg(target_os = "linux")]
pub fn atomic_exchange_files(path_a: &Path, path_b: &Path) -> std::io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path_a = CString::new(path_a.as_os_str().as_bytes())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let c_path_b = CString::new(path_b.as_os_str().as_bytes())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let ret = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            c_path_a.as_ptr(),
            libc::AT_FDCWD,
            c_path_b.as_ptr(),
            libc::RENAME_EXCHANGE,
        )
    };

    if ret == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(target_os = "linux"))]
pub fn atomic_exchange_files(_path_a: &Path, _path_b: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "atomic renameat2 exchange is only supported on Linux",
    ))
}

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
    #[error("Unstage rollback failed for stage '{stage_id}': {cause}")]
    RollbackFailed {
        stage_id: String,
        path: PathBuf,
        trash_path: PathBuf,
        cause: String,
    },
    #[error("Atomic exchange unsupported: {0}")]
    AtomicExchangeUnsupported(String),
    #[error("Failed to preserve concurrent external write: unable to move temporary file '{tmp_path}' to backup '{lost_write_path}': {cause}")]
    LostWritePreserveFailed {
        tmp_path: PathBuf,
        lost_write_path: PathBuf,
        cause: String,
    },
    #[error("Reflection rollback swap failed for stage '{stage_id}': unable to restore original file '{path}' from quarantined file '{quarantined_path}': {cause}")]
    ReflectRollbackFailed {
        stage_id: String,
        path: PathBuf,
        quarantined_path: PathBuf,
        cause: String,
    },
}

impl WorkspaceError {
    pub fn error_code(&self) -> &'static str {
        match self {
            WorkspaceError::Policy(p) => p.error_code(),
            WorkspaceError::NotFound(_) => "ERR_STAGE_NOT_FOUND",
            WorkspaceError::StagingDirUnavailable => "ERR_STAGING_DIR_UNAVAILABLE",
            WorkspaceError::InsufficientSpace => "ERR_INSUFFICIENT_SPACE",
            WorkspaceError::RollbackFailed { .. } => "ERR_UNSTAGE_ROLLBACK_FAILED",
            WorkspaceError::ReflectRollbackFailed { .. } => "ERR_REFLECT_ROLLBACK_FAILED",
            WorkspaceError::AtomicExchangeUnsupported(_) => "ERR_ATOMIC_EXCHANGE_UNSUPPORTED",
            WorkspaceError::LostWritePreserveFailed { .. } => "ERR_LOST_WRITE_PRESERVE_FAILED",
            WorkspaceError::Io(_) => "ERR_IO",
            WorkspaceError::Model(_) => "ERR_INVALID_PATH",
            WorkspaceError::Diff(_) => "ERR_DIFF_FAILED",
        }
    }
}

#[cfg(test)]
pub mod test_hooks {
    use std::cell::{Cell, RefCell};
    use std::path::Path;

    pub type HookFn = Box<dyn Fn(&Path)>;

    thread_local! {
        static POST_SWAP_HOOK: RefCell<Option<HookFn>> = const { RefCell::new(None) };
        static POST_VERIFY_HOOK: RefCell<Option<HookFn>> = const { RefCell::new(None) };
        static POST_STAGE_CLONE_HOOK: RefCell<Option<HookFn>> = const { RefCell::new(None) };
        static FAIL_STAGE_DB: Cell<bool> = const { Cell::new(false) };
        static FAIL_RESTORE_DB: Cell<bool> = const { Cell::new(false) };
        static FAIL_LOAD_SESSION_DB: Cell<bool> = const { Cell::new(false) };
    }

    pub fn set_fail_load_session_db(fail: bool) {
        FAIL_LOAD_SESSION_DB.with(|c| c.set(fail));
    }

    pub fn is_fail_load_session_db() -> bool {
        FAIL_LOAD_SESSION_DB.with(|c| c.get())
    }

    pub fn set_fail_restore_db(fail: bool) {
        FAIL_RESTORE_DB.with(|c| c.set(fail));
    }

    pub fn is_fail_restore_db() -> bool {
        FAIL_RESTORE_DB.with(|c| c.get())
    }

    pub fn set_fail_stage_db(fail: bool) {
        FAIL_STAGE_DB.with(|c| c.set(fail));
    }

    pub fn is_fail_stage_db() -> bool {
        FAIL_STAGE_DB.with(|c| c.get())
    }

    pub fn set_post_stage_clone_hook<F: Fn(&Path) + 'static>(f: F) {
        POST_STAGE_CLONE_HOOK.with(|h| {
            *h.borrow_mut() = Some(Box::new(f));
        });
    }

    pub fn clear_post_stage_clone_hook() {
        POST_STAGE_CLONE_HOOK.with(|h| {
            *h.borrow_mut() = None;
        });
    }

    pub fn trigger_post_stage_clone_hook(path: &Path) {
        POST_STAGE_CLONE_HOOK.with(|h| {
            if let Some(ref hook) = *h.borrow() {
                hook(path);
            }
        });
    }

    pub fn set_post_swap_hook<F: Fn(&Path) + 'static>(f: F) {
        POST_SWAP_HOOK.with(|h| {
            *h.borrow_mut() = Some(Box::new(f));
        });
    }

    pub fn clear_post_swap_hook() {
        POST_SWAP_HOOK.with(|h| {
            *h.borrow_mut() = None;
        });
    }

    pub fn trigger_post_swap_hook(path: &Path) {
        POST_SWAP_HOOK.with(|h| {
            if let Some(ref hook) = *h.borrow() {
                hook(path);
            }
        });
    }

    pub fn set_post_verify_hook<F: Fn(&Path) + 'static>(f: F) {
        POST_VERIFY_HOOK.with(|h| {
            *h.borrow_mut() = Some(Box::new(f));
        });
    }

    pub fn clear_post_verify_hook() {
        POST_VERIFY_HOOK.with(|h| {
            *h.borrow_mut() = None;
        });
    }

    pub fn trigger_post_verify_hook(path: &Path) {
        POST_VERIFY_HOOK.with(|h| {
            if let Some(ref hook) = *h.borrow() {
                hook(path);
            }
        });
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
    /// Changes are currently being reflected to the original file (WAL journal state)
    Applying,
    /// Changes have been safely reflected back to the original
    Reflected,
    /// Original file was modified externally after staging (Conflict detected on session restore)
    Conflict,
}

/// Mode for reflecting staged working copies back to original destination (Design 5.2)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub enum ReflectionMode {
    /// 方式 A: 原本を直接上書き (Atomic Overwrite)
    Overwrite,
    /// 方式 B: 原本をタイムスタンプ付き .origin に退避保存してから配置 (Backup Origin)
    BackupOrigin,
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
    let cache_dir = dirs::cache_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join(".cache")
    });
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
        let staged_target_path = staging_dir.join(&staged_filename);

        // Pre-commit temporary filename with .tmp_stage_ prefix for safe orphan detection and GC
        let tmp_stage_filename = format!(".tmp_stage_{}_{}", stage_id, item.display_name);
        let tmp_stage_target_path = staging_dir.join(&tmp_stage_filename);

        // 5. Perform safe cloning into pre-commit temporary copy (Reflink CoW preferred, standard stream fallback)
        let clone_method = clone_file(&canonical_src, &tmp_stage_target_path)?;

        #[cfg(test)]
        test_hooks::trigger_post_stage_clone_hook(&canonical_src);

        // 5.1 TOCTOU Defense: Verify original file was not modified during cloning
        if let Err(e) = OperationPolicy::verify_conflict_on_path(&canonical_src, &snapshot, true) {
            let _ = fs::remove_file(&tmp_stage_target_path);
            return Err(WorkspaceError::Policy(e));
        }

        // 5.2 Verify staged copy integrity against original snapshot
        let staged_item = FileItem::from_path(&tmp_stage_target_path)?;
        let staged_snapshot = OperationPolicy::create_snapshot(&staged_item, true)?;
        if staged_snapshot.sha256 != snapshot.sha256 {
            let _ = fs::remove_file(&tmp_stage_target_path);
            return Err(WorkspaceError::Policy(PolicyError::ConflictDetected(
                format!(
                    "Original file '{}' content changed concurrently during staging copy",
                    canonical_src.display()
                ),
            )));
        }

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

        // 6. Record operation in SQLite persistent history & save session atomically BEFORE registering in memory
        #[cfg(test)]
        let db_res = if test_hooks::is_fail_stage_db() {
            Err("Simulated SQLite persistence failure".to_string())
        } else {
            let hist_mgr = crate::history::get_history_manager();
            let history_entry = crate::history::NewHistoryEntry {
                operation_id: stage_id.clone(),
                operation_type: "Stage".to_string(),
                display_name: item.display_name.clone(),
                original_path: Some(canonical_src.to_string_lossy().to_string()),
                staged_path: Some(staged_dto.staged_path.clone()),
                inode: Some(staged_dto.original_snapshot.inode as i64),
                status: "Success".to_string(),
                can_undo: true,
                details: Some(format!("Cloned via {:?}", staged_dto.clone_method)),
            };
            hist_mgr.record_stage_operation_and_session(&history_entry, &staged_dto)
        };

        #[cfg(not(test))]
        let db_res = {
            let hist_mgr = crate::history::get_history_manager();
            let history_entry = crate::history::NewHistoryEntry {
                operation_id: stage_id.clone(),
                operation_type: "Stage".to_string(),
                display_name: item.display_name.clone(),
                original_path: Some(canonical_src.to_string_lossy().to_string()),
                staged_path: Some(staged_dto.staged_path.clone()),
                inode: Some(staged_dto.original_snapshot.inode as i64),
                status: "Success".to_string(),
                can_undo: true,
                details: Some(format!("Cloned via {:?}", staged_dto.clone_method)),
            };
            hist_mgr.record_stage_operation_and_session(&history_entry, &staged_dto)
        };

        if let Err(e) = db_res {
            if let Err(rm_err) = fs::remove_file(&tmp_stage_target_path) {
                eprintln!(
                    "[WARN] Staging DB persistence failed ({}) and failed to clean up temporary staged working copy at '{}': {}. The orphaned artifact has been queued for background maintenance.",
                    e,
                    tmp_stage_target_path.display(),
                    rm_err
                );
                let hist_mgr = crate::history::get_history_manager();
                let _ = hist_mgr.queue_cleanup_path(
                    &tmp_stage_target_path.to_string_lossy(),
                    Some(&rm_err.to_string()),
                );
                return Err(WorkspaceError::Io(std::io::Error::other(format!(
                    "Failed to persist stage session/history to SQLite: {}. Cleanup of temporary staged copy also failed: {}",
                    e, rm_err
                ))));
            }
            return Err(WorkspaceError::Io(std::io::Error::other(e)));
        }

        // Commit stage copy atomically: rename .tmp_stage_* to final staged_target_path
        if let Err(rn_err) = fs::rename(&tmp_stage_target_path, &staged_target_path) {
            eprintln!(
                "[ERROR] Failed to commit staged copy from '{}' to '{}': {}. Rolling back session and history.",
                tmp_stage_target_path.display(),
                staged_target_path.display(),
                rn_err
            );
            let hist_mgr = crate::history::get_history_manager();
            if let Err(rb_err) =
                hist_mgr.rollback_stage_operation_and_session(&stage_id, &rn_err.to_string())
            {
                eprintln!(
                    "[CRITICAL ERROR] Failed to roll back stage session/history in SQLite for '{}': {}",
                    stage_id, rb_err
                );
            }
            if let Err(rm_err) = fs::remove_file(&tmp_stage_target_path) {
                let _ = hist_mgr.queue_cleanup_path(
                    &tmp_stage_target_path.to_string_lossy(),
                    Some(&rm_err.to_string()),
                );
            }
            return Err(WorkspaceError::Io(rn_err));
        }

        // 7. Record in active staging registry only after SQLite persistence and atomic file commit succeeded
        let mut map = self.staged_items.lock().unwrap();
        map.insert(stage_id, staged_dto.clone());

        Ok(staged_dto)
    }

    /// Unstages a file: securely deletes the staged copy (ORIGINAL IS NEVER TOUCHED).
    pub fn unstage_file(&self, stage_id: &str) -> Result<(), WorkspaceError> {
        let mut map = self.staged_items.lock().unwrap();
        let target_key = if map.contains_key(stage_id) {
            Some(stage_id.to_string())
        } else {
            map.iter().find_map(|(k, v)| {
                if v.original_item_id == stage_id
                    || v.raw_base64 == stage_id
                    || v.staged_raw_base64 == stage_id
                {
                    Some(k.clone())
                } else {
                    None
                }
            })
        };

        if let Some(key) = target_key {
            let staged = map
                .get(&key)
                .cloned()
                .ok_or_else(|| WorkspaceError::NotFound(key.clone()))?;
            let path = decode_path_from_base64(&staged.staged_raw_base64)?;

            // 1. Two-phase deletion with atomic rollback:
            // Quarantine staged copy by renaming to temporary path before touching database.
            // Preserve the original full filename (with extension) and include stage_id for reliable recovery.
            let temp_trash_path = if path.exists() {
                let orig_file_name = path.file_name().unwrap_or_default().to_string_lossy();
                let trash_name = format!(
                    "{}.unstage_trash_{}_{}",
                    orig_file_name,
                    staged.stage_id,
                    Uuid::new_v4()
                );
                let trash = path.with_file_name(trash_name);
                fs::rename(&path, &trash).map_err(WorkspaceError::Io)?;
                Some(trash)
            } else {
                None
            };

            // 2. Remove from persistent SQLite session table
            let hist_mgr = crate::history::get_history_manager();
            if let Err(e) = hist_mgr.remove_session_item(&staged.stage_id) {
                // Rollback: restore quarantined file if DB removal fails
                if let Some(ref trash) = temp_trash_path {
                    if let Err(rb_err) = fs::rename(trash, &path) {
                        eprintln!(
                            "[CRITICAL] Failed to rollback unstage: unable to restore '{}' to '{}': {}",
                            trash.display(),
                            path.display(),
                            rb_err
                        );
                        return Err(WorkspaceError::RollbackFailed {
                            stage_id: staged.stage_id.clone(),
                            path: path.clone(),
                            trash_path: trash.clone(),
                            cause: format!(
                                "DB removal failed: {}. Rollback file restore also failed: {}. Quarantined staged file remains at {}",
                                e, rb_err, trash.display()
                            ),
                        });
                    }
                }
                return Err(WorkspaceError::Io(std::io::Error::other(format!(
                    "Failed to remove session item from SQLite: {}. Rolled back unstage operation.",
                    e
                ))));
            }

            // 3. Purge quarantined temporary file now that DB update is confirmed
            if let Some(ref trash) = temp_trash_path {
                if let Err(rm_err) = fs::remove_file(trash) {
                    eprintln!(
                        "[WARN] Unstaged copy removed from DB but failed to delete trash file '{}': {}",
                        trash.display(),
                        rm_err
                    );
                }
            }

            let _ = hist_mgr.record_operation(&crate::history::NewHistoryEntry {
                operation_id: uuid::Uuid::new_v4().to_string(),
                operation_type: "Unstage".to_string(),
                display_name: staged.display_name.clone(),
                original_path: Some(staged.original_snapshot.canonical_path.clone()),
                staged_path: Some(staged.staged_path.clone()),
                inode: Some(staged.original_snapshot.inode as i64),
                status: "Success".to_string(),
                can_undo: false,
                details: Some("Unstaged and safely removed working copy".to_string()),
            });

            // 4. Remove from memory state only after both disk and DB succeed
            map.remove(&key);
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
                let cloned = item.clone();
                let hist_mgr = crate::history::get_history_manager();
                let _ = hist_mgr.save_session_item(&cloned);
                return Ok(cloned);
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
        hist_mgr
            .save_session_item(staged)
            .map_err(|e| WorkspaceError::Io(std::io::Error::other(e)))?;
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
    pub fn get_diff(
        &self,
        stage_id: &str,
    ) -> Result<crate::diff::FileDiffResultDto, WorkspaceError> {
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
    /// Supports Mode A (Atomic Overwrite) and Mode B (Rename Original to .origin and place deliverable).
    pub fn reflect_staged_file(&self, stage_id: &str) -> Result<StagedItemDto, WorkspaceError> {
        self.reflect_staged_file_with_mode(stage_id, ReflectionMode::Overwrite)
            .map(|(item, _)| item)
    }

    /// Reflects staged working copy changes with specified ReflectionMode (Design 5.2).
    /// Returns (updated StagedItemDto, Option<PathBuf of .origin backup if Mode B was used>)
    pub fn reflect_staged_file_with_mode(
        &self,
        stage_id: &str,
        mode: ReflectionMode,
    ) -> Result<(StagedItemDto, Option<PathBuf>), WorkspaceError> {
        let mut map = self.staged_items.lock().unwrap();
        let staged = map
            .get_mut(stage_id)
            .ok_or_else(|| WorkspaceError::NotFound(stage_id.to_string()))?;

        let orig_path = decode_path_from_base64(&staged.raw_base64)?;
        let staged_path = decode_path_from_base64(&staged.staged_raw_base64)?;

        // 1. Policy Validation: Forbidden Zone (Outside $HOME, mount boundary, external storage)
        OperationPolicy::validate_reflection(&orig_path)?;

        // 2. Initial TOCTOU Race & External Conflict Verification
        if let Err(e) = OperationPolicy::verify_conflict(&staged.original_snapshot, true) {
            staged.status = StageStatus::Conflict;
            let hist_mgr = crate::history::get_history_manager();
            let _ = hist_mgr.save_session_item(staged);
            return Err(WorkspaceError::Policy(e));
        }

        let orig_parent = orig_path
            .parent()
            .ok_or(WorkspaceError::StagingDirUnavailable)?;

        // Temporary file in same directory for atomic POSIX rename (includes stage_id for crash recovery tracking)
        let tmp_reflect_name =
            format!(".cuddle_reflect_{}_{}.tmp", staged.stage_id, Uuid::new_v4());
        let tmp_reflect_path = orig_parent.join(tmp_reflect_name);

        // Copy staged content to temporary file in destination volume
        if let Err(e) = fs::copy(&staged_path, &tmp_reflect_path) {
            let _ = fs::remove_file(&tmp_reflect_path);
            return Err(WorkspaceError::Io(e));
        }

        // --- Write-Ahead Log (WAL) Journal State ---
        // Transition staged item to `StageStatus::Applying` and persist to SQLite BEFORE initiating physical exchange.
        // If crash or power failure occurs during or immediately after atomic_exchange_files,
        // restore_session() will detect `Applying` state and automatically roll forward or roll back safely.
        let previous_status = staged.status;
        staged.status = StageStatus::Applying;
        let hist_mgr = crate::history::get_history_manager();
        hist_mgr.save_session_item(staged).map_err(|e| {
            let _ = fs::remove_file(&tmp_reflect_path);
            staged.status = previous_status;
            WorkspaceError::Io(std::io::Error::other(e))
        })?;

        // --- TOCTOU Prevention: Atomic Exchange & Quarantined Verification ---
        // Atomically swap candidate into place and quarantine the original file in tmp_reflect_path.
        // Then verify the quarantined original file in isolation against the snapshot.
        //
        // --- Technical Note on POSIX Open File Descriptor & Inode Semantics ---
        // Under POSIX semantics, open file descriptors are bound directly to the underlying inode,
        // not the directory pathname. When `atomic_exchange_files` exchanges path references:
        // 1. Any external process with an open FD to `orig_path` continues writing to the quarantined
        //    inode at `tmp_reflect_path` without interruption.
        // 2. We verify `tmp_reflect_path` post-swap to detect such concurrent writes and safely
        //    preserve them as `.concurrent_write` backups rather than unlinking.
        // 3. Without cooperative advisory locks or mandatory kernel locks (which risk system-wide deadlocks
        //    and are disabled on modern Linux), no userspace program can close the sub-microsecond gap
        //    between a final check and the `unlink` syscall if an external process writes to the open FD.
        // Cuddle's multi-tier verification and fail-closed preservation provide the strongest practical
        // original data protection permitted by the POSIX filesystem architecture.
        let mut backup_origin_path: Option<PathBuf> = None;
        let mut concurrent_write_path: Option<PathBuf> = None;

        let swap_result = atomic_exchange_files(&tmp_reflect_path, &orig_path);
        match swap_result {
            Ok(()) => {
                // Swap succeeded:
                // `orig_path` now holds the newly reflected content.
                // `tmp_reflect_path` now holds the exact pre-reflection original file.
                // Verify the quarantined original file against snapshot in full isolation.
                match OperationPolicy::verify_conflict_on_path(
                    &tmp_reflect_path,
                    &staged.original_snapshot,
                    true,
                ) {
                    Ok(()) => {
                        // Pre-swap verification SUCCESS: Original file was strictly untouched up to the swap!
                        // Check for concurrent writes from external processes holding an open file descriptor:
                        // In POSIX systems, an open FD continues pointing to the swapped inode at `tmp_reflect_path`.
                        // Re-verifying right before removing ensures any unflushed or concurrent writes are detected.

                        #[cfg(test)]
                        test_hooks::trigger_post_swap_hook(&tmp_reflect_path);

                        let concurrent_check = OperationPolicy::verify_conflict_on_path(
                            &tmp_reflect_path,
                            &staged.original_snapshot,
                            true,
                        );

                        if concurrent_check.is_err() {
                            // Concurrent external FD write detected!
                            // CRITICAL DEFENSE: Never delete the quarantined file!
                            // Safely preserve the external process's modified content in a timestamped backup.
                            let lost_write_target = generate_lost_write_backup_path(&orig_path);
                            if let Err(e) = fs::rename(&tmp_reflect_path, &lost_write_target) {
                                eprintln!(
                                    "[CRITICAL] Failed to preserve concurrent external write '{}': {}",
                                    lost_write_target.display(),
                                    e
                                );
                                return Err(WorkspaceError::LostWritePreserveFailed {
                                    tmp_path: tmp_reflect_path.clone(),
                                    lost_write_path: lost_write_target.clone(),
                                    cause: format!("Failed to move quarantined file containing concurrent external writes: {}", e),
                                });
                            } else {
                                eprintln!(
                                    "[SECURITY] Concurrent external FD write detected and preserved to '{}'",
                                    lost_write_target.display()
                                );
                                concurrent_write_path = Some(lost_write_target);
                            }
                        } else if mode == ReflectionMode::BackupOrigin {
                            let backup_target = generate_origin_backup_path(&orig_path);
                            if let Err(e) = fs::rename(&tmp_reflect_path, &backup_target) {
                                // If saving backup fails, swap original back to recover
                                if let Err(rb_err) =
                                    atomic_exchange_files(&tmp_reflect_path, &orig_path)
                                {
                                    eprintln!(
                                        "[CRITICAL] Reflection rollback exchange failed! Quarantined original file at '{}', candidate at '{}': {}",
                                        tmp_reflect_path.display(),
                                        orig_path.display(),
                                        rb_err
                                    );
                                    // CRITICAL: NEVER remove tmp_reflect_path when exchange fails, as it contains the original data!
                                    staged.status = StageStatus::Conflict;
                                    let _ = hist_mgr.save_session_item(staged);
                                    return Err(WorkspaceError::ReflectRollbackFailed {
                                        stage_id: staged.stage_id.clone(),
                                        path: orig_path.clone(),
                                        quarantined_path: tmp_reflect_path.clone(),
                                        cause: format!(
                                            "Failed to save backup ({}). Subsequent rollback swap also failed: {}. Original data preserved in quarantined file at {}",
                                            e, rb_err, tmp_reflect_path.display()
                                        ),
                                    });
                                }
                                // Rollback exchange succeeded: orig_path has the original back, tmp_reflect_path has staging candidate
                                let _ = fs::remove_file(&tmp_reflect_path);
                                staged.status = previous_status;
                                let _ = hist_mgr.save_session_item(staged);
                                return Err(WorkspaceError::Io(e));
                            }
                            backup_origin_path = Some(backup_target);
                        } else {
                            // Mode A (Overwrite):
                            // 1. Initial verification before removal:
                            let final_check = OperationPolicy::verify_conflict_on_path(
                                &tmp_reflect_path,
                                &staged.original_snapshot,
                                true,
                            );

                            if final_check.is_err() {
                                let lost_write_target = generate_lost_write_backup_path(&orig_path);
                                if let Err(e) = fs::rename(&tmp_reflect_path, &lost_write_target) {
                                    eprintln!(
                                        "[CRITICAL] Failed to preserve external write '{}': {}",
                                        lost_write_target.display(),
                                        e
                                    );
                                    return Err(WorkspaceError::LostWritePreserveFailed {
                                        tmp_path: tmp_reflect_path.clone(),
                                        lost_write_path: lost_write_target.clone(),
                                        cause: format!("Failed to preserve concurrent write before removal: {}", e),
                                    });
                                }
                                eprintln!(
                                    "[SECURITY] Concurrent external write detected and preserved to '{}'",
                                    lost_write_target.display()
                                );
                                concurrent_write_path = Some(lost_write_target);
                            } else {
                                // 2. Trigger post-verify hook AFTER initial verification passed, immediately before unlinking
                                #[cfg(test)]
                                test_hooks::trigger_post_verify_hook(&tmp_reflect_path);

                                // 3. Double-check right before unlinking to catch last-millisecond concurrent writes:
                                let pre_unlink_check = OperationPolicy::verify_conflict_on_path(
                                    &tmp_reflect_path,
                                    &staged.original_snapshot,
                                    true,
                                );

                                if pre_unlink_check.is_err() {
                                    let lost_write_target =
                                        generate_lost_write_backup_path(&orig_path);
                                    if let Err(e) =
                                        fs::rename(&tmp_reflect_path, &lost_write_target)
                                    {
                                        eprintln!(
                                            "[CRITICAL] Failed to preserve last-millisecond external write '{}': {}",
                                            lost_write_target.display(),
                                            e
                                        );
                                        return Err(WorkspaceError::LostWritePreserveFailed {
                                            tmp_path: tmp_reflect_path.clone(),
                                            lost_write_path: lost_write_target.clone(),
                                            cause: format!("Failed to preserve last-millisecond concurrent write before removal: {}", e),
                                        });
                                    }
                                    eprintln!(
                                        "[SECURITY] Last-millisecond concurrent write detected after verification and preserved to '{}'",
                                        lost_write_target.display()
                                    );
                                    concurrent_write_path = Some(lost_write_target);
                                } else {
                                    // Safely remove the quarantined old original
                                    let _ = fs::remove_file(&tmp_reflect_path);
                                }
                            }
                        }
                    }
                    Err(conflict_err) => {
                        // CONFLICT DETECTED!
                        // An external modification occurred prior to the swap.
                        // Rollback immediately: swap the original file back into place!
                        if let Err(rb_err) = atomic_exchange_files(&tmp_reflect_path, &orig_path) {
                            eprintln!(
                                "[CRITICAL] Conflict rollback swap failed! Quarantined original file at '{}', candidate at '{}': {}",
                                tmp_reflect_path.display(),
                                orig_path.display(),
                                rb_err
                            );
                            // CRITICAL: NEVER remove tmp_reflect_path when exchange fails, as it contains the user's original data!
                            staged.status = StageStatus::Conflict;
                            let hist_mgr = crate::history::get_history_manager();
                            let _ = hist_mgr.save_session_item(staged);
                            return Err(WorkspaceError::ReflectRollbackFailed {
                                stage_id: staged.stage_id.clone(),
                                path: orig_path.clone(),
                                quarantined_path: tmp_reflect_path.clone(),
                                cause: format!(
                                    "Conflict detected ({}). Rollback swap failed: {}. Original data preserved in quarantined file at {}",
                                    conflict_err, rb_err, tmp_reflect_path.display()
                                ),
                            });
                        }
                        // Rollback exchange succeeded: original file restored to orig_path, tmp_reflect_path has staging candidate
                        let _ = fs::remove_file(&tmp_reflect_path);
                        staged.status = StageStatus::Conflict;
                        let hist_mgr = crate::history::get_history_manager();
                        let _ = hist_mgr.save_session_item(staged);
                        return Err(WorkspaceError::Policy(conflict_err));
                    }
                }
            }
            Err(swap_err) => {
                // Remove temporary staging candidate and revert WAL status
                let _ = fs::remove_file(&tmp_reflect_path);
                staged.status = previous_status;
                let hist_mgr = crate::history::get_history_manager();
                let _ = hist_mgr.save_session_item(staged);

                // Classify atomic_exchange_files error
                let raw_err = swap_err.raw_os_error();
                let is_unsupported_fs = match raw_err {
                    #[cfg(target_os = "linux")]
                    Some(code) => {
                        code == libc::EINVAL
                            || code == libc::ENOSYS
                            || code == libc::EOPNOTSUPP
                            || code == libc::EXDEV
                    }
                    #[cfg(target_os = "linux")]
                    None => false,
                    #[cfg(not(target_os = "linux"))]
                    _ => true,
                };

                if !is_unsupported_fs {
                    // Environmental or permission error (EACCES, EPERM, ENOENT, EROFS, etc.)
                    // FAIL IMMEDIATELY. Never attempt insecure fallback.
                    return Err(WorkspaceError::Io(swap_err));
                }

                // If filesystem does not support atomic RENAME_EXCHANGE,
                // enforce FAIL-CLOSED safety: refuse non-atomic direct overwrite reflection
                // to prevent TOCTOU race conditions and preserve original data.
                return Err(WorkspaceError::AtomicExchangeUnsupported(format!(
                    "Filesystem does not support atomic RENAME_EXCHANGE ({}). \
                    Direct overwrite is blocked to protect original data from TOCTOU corruption. \
                    Please use 'Save as Copy' (reflect_staged_item_as_copy) instead.",
                    swap_err
                )));
            }
        }

        // 4. Update snapshot and status
        let updated_item = match FileItem::from_path(&orig_path) {
            Ok(item) => item,
            Err(e) => {
                // If taking snapshot fails, rollback Mode B if possible
                if let Some(ref bkp) = backup_origin_path {
                    let _ = fs::rename(&orig_path, &tmp_reflect_path);
                    let _ = fs::rename(bkp, &orig_path);
                    let _ = fs::remove_file(&tmp_reflect_path);
                    staged.status = previous_status;
                    let hist_mgr = crate::history::get_history_manager();
                    let _ = hist_mgr.save_session_item(staged);
                }
                return Err(WorkspaceError::Model(e));
            }
        };
        let new_snapshot = OperationPolicy::create_snapshot(&updated_item, true)?;

        staged.original_snapshot = new_snapshot;
        staged.status = StageStatus::Reflected;

        let mut details = match &backup_origin_path {
            Some(bkp) => format!(
                "Reflected with .origin backup: {} -> original: {}",
                bkp.display(),
                orig_path.display()
            ),
            None => format!("Reflected atomic replace to {}", orig_path.display()),
        };

        if let Some(ref cw) = concurrent_write_path {
            details.push_str(&format!(
                " (Concurrent external FD writes preserved to {})",
                cw.display()
            ));
        }

        let hist_mgr = crate::history::get_history_manager();
        hist_mgr
            .save_session_item(staged)
            .map_err(|e| WorkspaceError::Io(std::io::Error::other(e)))?;
        hist_mgr
            .record_operation(&crate::history::NewHistoryEntry {
                operation_id: uuid::Uuid::new_v4().to_string(),
                operation_type: "Reflect".to_string(),
                display_name: staged.display_name.clone(),
                original_path: Some(staged.original_snapshot.canonical_path.clone()),
                staged_path: Some(staged.staged_path.clone()),
                inode: Some(updated_item.inode as i64),
                status: "Success".to_string(),
                can_undo: false,
                details: Some(details),
            })
            .map_err(|e| WorkspaceError::Io(std::io::Error::other(e)))?;

        Ok((staged.clone(), backup_origin_path.or(concurrent_write_path)))
    }

    /// Saves the staged working copy as an alternate copy (e.g., .edit.ext) without modifying the original.
    /// Used when a conflict is detected or when the user explicitly requests non-destructive export.
    /// Returns (updated StagedItemDto, PathBuf of saved copy).
    pub fn reflect_staged_item_as_copy(
        &self,
        stage_id: &str,
        custom_suffix: Option<String>,
    ) -> Result<(StagedItemDto, PathBuf), WorkspaceError> {
        let mut map = self.staged_items.lock().unwrap();
        let staged = map
            .get_mut(stage_id)
            .ok_or_else(|| WorkspaceError::NotFound(stage_id.to_string()))?;

        let orig_path = decode_path_from_base64(&staged.raw_base64)?;
        let staged_path = decode_path_from_base64(&staged.staged_raw_base64)?;

        // 1. Policy Validation: Validate target parent directory for non-destructive copy
        OperationPolicy::validate_alternate_copy(&orig_path)?;

        let orig_parent = orig_path
            .parent()
            .ok_or(WorkspaceError::StagingDirUnavailable)?;

        // 2. Generate collision-resistant alternate copy path
        let target_copy_path = generate_alternate_copy_path(&orig_path, custom_suffix.as_deref());

        // 3. Atomic placement via temporary file in same directory
        let tmp_copy_name = format!(".cuddle_copy_{}.tmp", Uuid::new_v4());
        let tmp_copy_path = orig_parent.join(tmp_copy_name);

        if let Err(e) = fs::copy(&staged_path, &tmp_copy_path) {
            let _ = fs::remove_file(&tmp_copy_path);
            return Err(WorkspaceError::Io(e));
        }

        if let Err(e) = fs::rename(&tmp_copy_path, &target_copy_path) {
            let _ = fs::remove_file(&tmp_copy_path);
            return Err(WorkspaceError::Io(e));
        }

        // 4. Prepare updated item candidate without mutating in-memory map yet
        let mut updated_item = staged.clone();
        updated_item.status = StageStatus::Reflected;

        let details = format!(
            "Saved alternate copy: {} (original {} preserved untouched)",
            target_copy_path.display(),
            orig_path.display()
        );

        let hist_mgr = crate::history::get_history_manager();
        let history_entry = crate::history::NewHistoryEntry {
            operation_id: uuid::Uuid::new_v4().to_string(),
            operation_type: "ReflectAsCopy".to_string(),
            display_name: updated_item.display_name.clone(),
            original_path: Some(updated_item.original_snapshot.canonical_path.clone()),
            staged_path: Some(target_copy_path.to_string_lossy().to_string()),
            inode: None,
            status: "Success".to_string(),
            can_undo: false,
            details: Some(details),
        };

        #[cfg(test)]
        let db_res = if test_hooks::is_fail_stage_db() {
            Err("Simulated SQLite persistence failure".to_string())
        } else {
            hist_mgr.record_stage_operation_and_session(&history_entry, &updated_item)
        };

        #[cfg(not(test))]
        let db_res = hist_mgr.record_stage_operation_and_session(&history_entry, &updated_item);

        if let Err(e) = db_res {
            eprintln!(
                "[CRITICAL ERROR] Failed to persist alternate copy session to SQLite for stage '{}': {}. Marking Conflict to alert user.",
                stage_id, e
            );
            // Alternate copy was created on disk (protect user data!), but DB failed to update.
            // Mark in-memory staged item as Conflict to prevent ghost state.
            staged.status = StageStatus::Conflict;
            return Err(WorkspaceError::Io(std::io::Error::other(format!(
                "Failed to persist alternate copy session to SQLite: {}. Staged item marked Conflict.",
                e
            ))));
        }

        // 5. Update memory only AFTER successful DB persistence
        *staged = updated_item;

        Ok((staged.clone(), target_copy_path))
    }

    /// Generates timestamped .origin backup path (Design 5.2 Mode B)
    pub fn get_origin_backup_preview(&self, orig_path: &Path) -> PathBuf {
        generate_origin_backup_path(orig_path)
    }

    /// Generates alternate copy preview path (Task 3-2)
    pub fn get_alternate_copy_preview(&self, orig_path: &Path, suffix: Option<&str>) -> PathBuf {
        generate_alternate_copy_path(orig_path, suffix)
    }

    /// Reads the text content of a staged working copy (Max 2 MiB, UTF-8 text only).
    pub fn read_staged_text(&self, stage_id: &str) -> Result<String, WorkspaceError> {
        let map = self.staged_items.lock().unwrap();
        let staged = map
            .get(stage_id)
            .ok_or_else(|| WorkspaceError::NotFound(stage_id.to_string()))?;

        let staged_path = decode_path_from_base64(&staged.staged_raw_base64)?;
        if !staged_path.exists() {
            return Err(WorkspaceError::NotFound(
                staged_path.to_string_lossy().to_string(),
            ));
        }

        let metadata = fs::metadata(&staged_path).map_err(WorkspaceError::Io)?;
        const MAX_TEXT_SIZE: u64 = 2 * 1024 * 1024; // 2 MiB limit for configuration text
        if metadata.len() > MAX_TEXT_SIZE {
            return Err(WorkspaceError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "File size ({} bytes) exceeds the maximum editable size of 2 MiB",
                    metadata.len()
                ),
            )));
        }

        let mut file = fs::File::open(&staged_path).map_err(WorkspaceError::Io)?;
        let mut buffer = Vec::with_capacity(metadata.len() as usize);
        file.read_to_end(&mut buffer).map_err(WorkspaceError::Io)?;

        // Quick binary detection: check first 8 KiB for NUL bytes
        let sample_len = buffer.len().min(8192);
        if buffer[..sample_len].contains(&0) {
            return Err(WorkspaceError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Binary file cannot be edited in quick text editor",
            )));
        }

        String::from_utf8(buffer).map_err(|e| {
            WorkspaceError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("File is not valid UTF-8 text: {}", e),
            ))
        })
    }

    /// Writes text content to a staged working copy atomically, updating session status to Modified.
    pub fn write_staged_text(
        &self,
        stage_id: &str,
        content: &str,
    ) -> Result<StagedItemDto, WorkspaceError> {
        let mut map = self.staged_items.lock().unwrap();
        let staged = map
            .get_mut(stage_id)
            .ok_or_else(|| WorkspaceError::NotFound(stage_id.to_string()))?;

        let staged_path = decode_path_from_base64(&staged.staged_raw_base64)?;
        let parent = staged_path
            .parent()
            .ok_or(WorkspaceError::StagingDirUnavailable)?;

        // 1. Atomic write via temporary file
        let tmp_name = format!(".cuddle_edit_{}.tmp", Uuid::new_v4());
        let tmp_path = parent.join(tmp_name);

        {
            let mut tmp_file = fs::File::create(&tmp_path).map_err(WorkspaceError::Io)?;
            std::io::Write::write_all(&mut tmp_file, content.as_bytes())
                .map_err(WorkspaceError::Io)?;
            tmp_file.sync_all().map_err(WorkspaceError::Io)?;
        }

        if let Err(e) = fs::rename(&tmp_path, &staged_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(WorkspaceError::Io(e));
        }

        // 2. Refresh metadata
        let metadata = fs::metadata(&staged_path).map_err(WorkspaceError::Io)?;

        // Prepare updated item candidate without mutating in-memory map yet
        let mut updated_item = staged.clone();
        updated_item.size = metadata.len();
        updated_item.status = StageStatus::Modified;

        let details = format!(
            "Edited staged copy '{}' ({} bytes)",
            updated_item.display_name, updated_item.size
        );

        // 3. Persist to SQLite session & history atomically
        let hist_mgr = crate::history::get_history_manager();
        let history_entry = crate::history::NewHistoryEntry {
            operation_id: Uuid::new_v4().to_string(),
            operation_type: "EditStaged".to_string(),
            display_name: updated_item.display_name.clone(),
            original_path: Some(updated_item.original_snapshot.canonical_path.clone()),
            staged_path: Some(updated_item.staged_path.clone()),
            inode: None,
            status: "Success".to_string(),
            can_undo: false,
            details: Some(details),
        };

        #[cfg(test)]
        let db_res = if test_hooks::is_fail_stage_db() {
            Err("Simulated SQLite persistence failure".to_string())
        } else {
            hist_mgr.record_stage_operation_and_session(&history_entry, &updated_item)
        };

        #[cfg(not(test))]
        let db_res = hist_mgr.record_stage_operation_and_session(&history_entry, &updated_item);

        if let Err(e) = db_res {
            eprintln!(
                "[CRITICAL ERROR] Failed to persist edited session to SQLite for stage '{}': {}. Marking Conflict to alert user.",
                stage_id, e
            );
            // Protect user data: DO NOT delete the edited file!
            // Mark in-memory staged item as Conflict to flag the DB-filesystem discrepancy.
            staged.size = metadata.len();
            staged.status = StageStatus::Conflict;
            return Err(WorkspaceError::Io(std::io::Error::other(format!(
                "Failed to persist edited session to SQLite: {}. Staged item marked Conflict.",
                e
            ))));
        }

        // 4. Update memory only AFTER successful DB persistence
        *staged = updated_item;

        Ok(staged.clone())
    }

    /// Lists all currently active staged items.
    pub fn list_staged(&self) -> Vec<StagedItemDto> {
        let map = self.staged_items.lock().unwrap();
        map.values().cloned().collect()
    }

    /// Restores previous staged session from SQLite persistent session table.
    /// Performs TOCTOU conflict verification against original files.
    pub fn restore_session(&self) -> Result<Vec<StagedItemDto>, WorkspaceError> {
        let hist_mgr = crate::history::get_history_manager();
        let saved_items = hist_mgr
            .load_all_session_items()
            .map_err(|e| WorkspaceError::Io(std::io::Error::other(e)))?;

        let mut restored = Vec::new();
        let mut map = self.staged_items.lock().unwrap();

        for mut item in saved_items {
            let staged_path = Path::new(&item.staged_path);
            if !staged_path.exists() {
                // Check if an interrupted unstage rollback, quarantined trash file,
                // or interrupted pre-commit temporary stage copy exists
                let mut recovered = false;
                if let Some(parent) = staged_path.parent() {
                    if let Ok(entries) = fs::read_dir(parent) {
                        let unstage_marker = format!("unstage_trash_{}", item.stage_id);
                        let tmp_stage_marker = format!(".tmp_stage_{}", item.stage_id);

                        for entry in entries.flatten() {
                            let entry_name = entry.file_name().to_string_lossy().to_string();

                            // 1. Recover from interrupted unstage rollback
                            if entry_name.contains(&unstage_marker)
                                && fs::rename(entry.path(), staged_path).is_ok()
                            {
                                eprintln!(
                                    "[RECOVERY] Successfully restored staged file '{}' from uncompleted unstage trash '{}'",
                                    staged_path.display(),
                                    entry.path().display()
                                );
                                recovered = true;
                                break;
                            }

                            // 2. Roll-forward from interrupted stage copy (crashed between SQLite commit and rename)
                            if entry_name.contains(&tmp_stage_marker) {
                                let tmp_path = entry.path();
                                let tmp_hash = compute_file_sha256(&tmp_path).ok();
                                let is_intact = match (&tmp_hash, &item.original_snapshot.sha256) {
                                    (Some(th), Some(oh)) => th == oh,
                                    (Some(_), None) => true,
                                    _ => false,
                                };

                                if is_intact {
                                    if fs::rename(&tmp_path, staged_path).is_ok() {
                                        eprintln!(
                                            "[RECOVERY] Successfully rolled forward temporary stage copy '{}' to formal staged file '{}'",
                                            tmp_path.display(),
                                            staged_path.display()
                                        );
                                        recovered = true;
                                        break;
                                    }
                                } else {
                                    eprintln!(
                                        "[WARN] Found temporary stage copy '{}' for stage '{}' but integrity verification failed. Retaining as Conflict.",
                                        tmp_path.display(),
                                        item.stage_id
                                    );
                                }
                            }
                        }
                    }
                }

                if !recovered && !staged_path.exists() {
                    // Staged working copy was missing from disk:
                    // For original data protection, DO NOT silently purge DB session item!
                    // Retain in Conflict status so user can inspect and resolve.
                    eprintln!(
                        "[WARN] Staged working copy for stage '{}' is missing at '{}'. Retaining in Conflict status to prevent silent data loss.",
                        item.stage_id,
                        staged_path.display()
                    );
                    item.status = StageStatus::Conflict;
                    if let Err(e) = hist_mgr.save_session_item(&item) {
                        eprintln!(
                            "[ERROR] Failed to save Conflict status to session DB for stage '{}': {}",
                            item.stage_id, e
                        );
                    }
                    map.insert(item.stage_id.clone(), item.clone());
                    restored.push(item);
                    continue;
                }
            }

            // Verify original file and detect conflict
            let orig_path = PathBuf::from(&item.original_snapshot.canonical_path);

            // Handle WAL Journal State (StageStatus::Applying):
            // An interrupted reflection was recorded in WAL journal state!
            if item.status == StageStatus::Applying {
                let mut resolved = false;

                if orig_path.exists() && staged_path.exists() {
                    let orig_hash = compute_file_sha256(&orig_path).ok();
                    let staged_hash = compute_file_sha256(staged_path).ok();

                    if let (Some(ref oh), Some(ref sh)) = (&orig_hash, &staged_hash) {
                        if oh == sh {
                            // CASE A: orig_path hash matches staged copy hash!
                            // Physical reflection completed successfully prior to SQLite commit.
                            // Idempotently roll-forward to Reflected status and refresh snapshot.
                            if let Ok(updated_item) = FileItem::from_path(&orig_path) {
                                if let Ok(new_snapshot) =
                                    OperationPolicy::create_snapshot(&updated_item, true)
                                {
                                    let old_snapshot_hash = item.original_snapshot.sha256.clone();
                                    item.original_snapshot = new_snapshot;
                                    item.status = StageStatus::Reflected;

                                    // P1: Strict DB transaction - verify DB write succeeds before file inspection/cleanup!
                                    #[cfg(test)]
                                    let db_save_res = if test_hooks::is_fail_restore_db() {
                                        Err("Simulated restore SQLite persistence failure"
                                            .to_string())
                                    } else {
                                        hist_mgr.save_session_item(&item)
                                    };

                                    #[cfg(not(test))]
                                    let db_save_res = hist_mgr.save_session_item(&item);

                                    if let Err(db_err) = db_save_res {
                                        eprintln!(
                                            "[ERROR] Failed to save Reflected status in session DB for stage '{}': {}. Retaining all recovery files.",
                                            item.stage_id, db_err
                                        );
                                    } else {
                                        if let Err(rec_err) = hist_mgr.record_operation(&crate::history::NewHistoryEntry {
                                            operation_id: Uuid::new_v4().to_string(),
                                            operation_type: "RecoveryRollForward".to_string(),
                                            display_name: item.display_name.clone(),
                                            original_path: Some(item.original_snapshot.canonical_path.clone()),
                                            staged_path: Some(item.staged_path.clone()),
                                            inode: Some(updated_item.inode as i64),
                                            status: "Success".to_string(),
                                            can_undo: false,
                                            details: Some("Interrupted reflection auto-recovered and rolled forward to Reflected".to_string()),
                                        }) {
                                            eprintln!(
                                                "[ERROR] Failed to record RecoveryRollForward operation in history for stage '{}': {}",
                                                item.stage_id, rec_err
                                            );
                                        }

                                        // P0: Inspect leftover quarantine file .cuddle_reflect_{stage_id}_*.tmp
                                        // CRITICAL: DO NOT unconditionally delete it! It may hold concurrent writes from an open FD.
                                        let mut quarantine_failure = false;
                                        if let Some(parent) = orig_path.parent() {
                                            match fs::read_dir(parent) {
                                                Ok(entries) => {
                                                    let marker = format!(
                                                        ".cuddle_reflect_{}",
                                                        item.stage_id
                                                    );
                                                    let mut matching_entries: Vec<PathBuf> =
                                                        Vec::new();
                                                    for entry_res in entries {
                                                        let entry = match entry_res {
                                                            Ok(e) => e,
                                                            Err(e) => {
                                                                eprintln!(
                                                                    "[ERROR] WAL recovery: Error reading directory entry in '{}' for stage '{}': {}. Marking Conflict for safety.",
                                                                    parent.display(),
                                                                    item.stage_id,
                                                                    e
                                                                );
                                                                quarantine_failure = true;
                                                                break;
                                                            }
                                                        };
                                                        let entry_name = entry
                                                            .file_name()
                                                            .to_string_lossy()
                                                            .to_string();
                                                        if entry_name.contains(&marker) {
                                                            matching_entries.push(entry.path());
                                                        }
                                                    }

                                                    if !quarantine_failure {
                                                        if matching_entries.len() > 1 {
                                                            // Ambiguous state: multiple quarantine files exist!
                                                            // CRITICAL: Retain all files without deleting or renaming!
                                                            eprintln!(
                                                                "[SECURITY RECOVERY] Found multiple ({}) quarantine files for stage '{}'. Ambiguous state: retaining all files without deletion and marking Conflict.",
                                                                matching_entries.len(),
                                                                item.stage_id
                                                            );
                                                            quarantine_failure = true;
                                                        } else {
                                                            for q_path in matching_entries {
                                                                let q_hash =
                                                                    compute_file_sha256(&q_path)
                                                                        .ok();
                                                                if q_hash.is_some()
                                                                    && q_hash == old_snapshot_hash
                                                                {
                                                                    // CRITICAL DEFENSE (Quarantine Safety Fix):
                                                                    // Do NOT delete the quarantined original file based merely on SHA-256 match!
                                                                    // An external process may still hold an open FD to this inode.
                                                                    // Retain the quarantine file and mark Conflict for safety.
                                                                    eprintln!(
                                                                        "[SECURITY RECOVERY] Clean quarantine file exists at '{}' for stage '{}'. Retaining file and marking Conflict to prevent TOCTOU data loss.",
                                                                        q_path.display(),
                                                                        item.stage_id
                                                                    );
                                                                    quarantine_failure = true;
                                                                } else {
                                                                    // Quarantined file has external modifications (e.g. concurrent FD write)
                                                                    // or cannot be verified! MUST PRESERVE as .concurrent_write backup!
                                                                    let lost_write_target = generate_lost_write_backup_path(&orig_path);
                                                                    if let Err(mv_err) = fs::rename(
                                                                        &q_path,
                                                                        &lost_write_target,
                                                                    ) {
                                                                        eprintln!(
                                                                            "[CRITICAL RECOVERY] Failed to preserve concurrent external write '{}' -> '{}': {}. Retaining quarantine file and marking Conflict.",
                                                                            q_path.display(),
                                                                            lost_write_target.display(),
                                                                            mv_err
                                                                        );
                                                                        quarantine_failure = true;
                                                                    } else {
                                                                        eprintln!(
                                                                            "[SECURITY RECOVERY] Preserved concurrent external write during WAL restore: '{}' -> '{}'",
                                                                            q_path.display(),
                                                                            lost_write_target.display()
                                                                        );
                                                                        let _ = hist_mgr.record_operation(&crate::history::NewHistoryEntry {
                                                                            operation_id: Uuid::new_v4().to_string(),
                                                                            operation_type: "QuarantinePreserved".to_string(),
                                                                            display_name: item.display_name.clone(),
                                                                            original_path: Some(item.original_snapshot.canonical_path.clone()),
                                                                            staged_path: Some(lost_write_target.to_string_lossy().to_string()),
                                                                            inode: None,
                                                                            status: "Success".to_string(),
                                                                            can_undo: false,
                                                                            details: Some(format!(
                                                                                "Concurrent external write detected during WAL restore and preserved to '{}'",
                                                                                lost_write_target.display()
                                                                            )),
                                                                        });
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    eprintln!(
                                                        "[ERROR] WAL recovery: Failed to read parent directory '{}' for stage '{}': {}. Retaining quarantine files and marking Conflict for safety.",
                                                        parent.display(),
                                                        item.stage_id,
                                                        e
                                                    );
                                                    quarantine_failure = true;
                                                }
                                            }
                                        } else {
                                            quarantine_failure = true;
                                        }

                                        if !quarantine_failure {
                                            resolved = true;
                                        } else {
                                            item.status = StageStatus::Conflict;
                                            if let Err(e) = hist_mgr.save_session_item(&item) {
                                                eprintln!(
                                                    "[ERROR] WAL recovery: Failed to save Conflict status to session DB for stage '{}': {}. Reverting to Applying so subsequent restarts retry recovery.",
                                                    item.stage_id, e
                                                );
                                                let mut retry_item = item.clone();
                                                retry_item.status = StageStatus::Applying;
                                                let _ = hist_mgr.save_session_item(&retry_item);
                                            } else {
                                                resolved = true;
                                            }
                                        }
                                    }
                                }
                            }
                        } else if item.original_snapshot.sha256.as_ref() == Some(oh) {
                            // CASE B: orig_path still matches pre-reflection snapshot (swap was not completed or rolled back).
                            let mut has_leftover_quarantine = false;
                            let mut dir_read_failed = false;

                            if let Some(parent) = orig_path.parent() {
                                match fs::read_dir(parent) {
                                    Ok(entries) => {
                                        let marker = format!(".cuddle_reflect_{}", item.stage_id);
                                        for entry in entries.flatten() {
                                            if entry.file_name().to_string_lossy().contains(&marker)
                                            {
                                                has_leftover_quarantine = true;
                                                break;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "[ERROR] WAL recovery: Failed reading parent directory in CASE B for stage '{}': {}. Retaining files and marking Conflict for safety.",
                                            item.stage_id, e
                                        );
                                        dir_read_failed = true;
                                    }
                                }
                            } else {
                                dir_read_failed = true;
                            }

                            if dir_read_failed || has_leftover_quarantine {
                                // CRITICAL SAFETY (Quarantine Safety Fix for CASE B):
                                // If quarantine files exist, do NOT delete them based on filename alone!
                                // Safety of deletion cannot be proven -> retain files and mark Conflict!
                                eprintln!(
                                    "[SECURITY RECOVERY] Leftover quarantine/reflection file exists in CASE B for stage '{}'. Retaining all files and marking Conflict for safety.",
                                    item.stage_id
                                );
                                item.status = StageStatus::Conflict;
                                #[cfg(test)]
                                let db_save_res = if test_hooks::is_fail_restore_db() {
                                    Err("Simulated restore SQLite persistence failure".to_string())
                                } else {
                                    hist_mgr.save_session_item(&item)
                                };
                                #[cfg(not(test))]
                                let db_save_res = hist_mgr.save_session_item(&item);

                                if let Err(db_err) = db_save_res {
                                    eprintln!(
                                        "[ERROR] Failed to save Conflict status in CASE B for stage '{}': {}",
                                        item.stage_id, db_err
                                    );
                                } else {
                                    resolved = true;
                                }
                            } else {
                                // Clean rollback: no quarantine files exist, original untouched on disk.
                                item.status = StageStatus::Modified;
                                #[cfg(test)]
                                let db_save_res = if test_hooks::is_fail_restore_db() {
                                    Err("Simulated restore SQLite persistence failure".to_string())
                                } else {
                                    hist_mgr.save_session_item(&item)
                                };
                                #[cfg(not(test))]
                                let db_save_res = hist_mgr.save_session_item(&item);

                                if let Err(db_err) = db_save_res {
                                    eprintln!(
                                        "[ERROR] Failed to save rollback Modified status for stage '{}': {}",
                                        item.stage_id, db_err
                                    );
                                } else {
                                    resolved = true;
                                }
                            }
                        }
                    }
                }

                if !resolved {
                    // Cannot confirm clean roll-forward or clean rollback -> mark Conflict for safety
                    #[cfg(test)]
                    let is_failing_db = test_hooks::is_fail_restore_db();
                    #[cfg(not(test))]
                    let is_failing_db = false;

                    if !is_failing_db {
                        item.status = StageStatus::Conflict;
                        if let Err(e) = hist_mgr.save_session_item(&item) {
                            eprintln!(
                                "[ERROR] WAL recovery: Failed to save Conflict status in session DB for unresolved stage '{}': {}",
                                item.stage_id, e
                            );
                        }
                    } else {
                        item.status = StageStatus::Applying;
                    }
                }

                map.insert(item.stage_id.clone(), item.clone());
                restored.push(item);
                continue;
            }

            // Check if an interrupted reflection or failed rollback left a quarantined original file (.cuddle_reflect_{stage_id}_*.tmp)
            if let Some(orig_parent) = orig_path.parent() {
                match fs::read_dir(orig_parent) {
                    Ok(entries) => {
                        let reflect_marker = format!(".cuddle_reflect_{}", item.stage_id);
                        for entry_res in entries {
                            match entry_res {
                                Ok(entry) => {
                                    let entry_name =
                                        entry.file_name().to_string_lossy().to_string();
                                    if entry_name.contains(&reflect_marker) {
                                        eprintln!(
                                            "[CRITICAL RECOVERY] Found uncompleted reflection quarantine file '{}' for stage '{}'. Retaining original data and marking Conflict.",
                                            entry.path().display(),
                                            item.stage_id
                                        );
                                        item.status = StageStatus::Conflict;
                                        if let Err(e) = hist_mgr.save_session_item(&item) {
                                            eprintln!(
                                                "[ERROR] Failed to save Conflict status to session DB for stage '{}' with leftover quarantine file: {}",
                                                item.stage_id, e
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!(
                                        "[WARN] Failed to read directory entry in '{}' while checking quarantine files for stage '{}': {}. Marking Conflict for safety.",
                                        orig_parent.display(),
                                        item.stage_id,
                                        e
                                    );
                                    item.status = StageStatus::Conflict;
                                    let _ = hist_mgr.save_session_item(&item);
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "[WARN] Failed to read parent directory '{}' for stage '{}': {}. Marking Conflict for safety.",
                            orig_parent.display(),
                            item.stage_id,
                            e
                        );
                        item.status = StageStatus::Conflict;
                        let _ = hist_mgr.save_session_item(&item);
                    }
                }
            }

            if !orig_path.exists() {
                item.status = StageStatus::Conflict;
                if let Err(e) = hist_mgr.save_session_item(&item) {
                    eprintln!(
                        "[ERROR] Failed to save Conflict status for missing original '{}': {}",
                        item.stage_id, e
                    );
                }
            } else {
                match OperationPolicy::verify_conflict(&item.original_snapshot, true) {
                    Ok(_) => {}
                    Err(PolicyError::ConflictDetected(_)) => {
                        // External modification detected! Mark as Conflict per user decision
                        item.status = StageStatus::Conflict;
                        if let Err(e) = hist_mgr.save_session_item(&item) {
                            eprintln!(
                                "[ERROR] Failed to save Conflict status for detected conflict '{}': {}",
                                item.stage_id, e
                            );
                        }
                    }
                    Err(e) => {
                        // Permission denied or IO error: do not assume Clean! Mark as Conflict for safety
                        eprintln!(
                            "Warning: Error verifying conflict during session restore for '{}': {}",
                            orig_path.display(),
                            e
                        );
                        item.status = StageStatus::Conflict;
                        if let Err(db_err) = hist_mgr.save_session_item(&item) {
                            eprintln!(
                                "[ERROR] Failed to save Conflict status on IO verification error '{}': {}",
                                item.stage_id, db_err
                            );
                        }
                    }
                }
            }

            // Verify whether staged working copy has unpersisted edits
            // (e.g. from crash or DB write failure during write_staged_text)
            if item.status == StageStatus::Staged && staged_path.exists() {
                if let Ok(staged_meta) = fs::metadata(staged_path) {
                    let disk_size = staged_meta.len();
                    let mut unpersisted_edit = disk_size != item.size;
                    if !unpersisted_edit {
                        if let (Some(ref orig_hash), Ok(staged_hash)) = (
                            &item.original_snapshot.sha256,
                            compute_file_sha256(staged_path),
                        ) {
                            if orig_hash != &staged_hash {
                                unpersisted_edit = true;
                            }
                        }
                    }
                    if unpersisted_edit {
                        eprintln!(
                            "[RECOVERY] Detected unpersisted edits in staged copy '{}' for stage '{}'. Recovering as Modified status.",
                            staged_path.display(),
                            item.stage_id
                        );
                        item.size = disk_size;
                        item.status = StageStatus::Modified;
                        if let Err(e) = hist_mgr.save_session_item(&item) {
                            eprintln!(
                                "[ERROR] Failed to persist recovered Modified status for stage '{}': {}",
                                item.stage_id, e
                            );
                        }
                    }
                }
            }

            map.insert(item.stage_id.clone(), item.clone());
            restored.push(item);
        }

        Ok(restored)
    }

    /// Injects or replaces a staged item in the active workspace registry.
    pub fn insert_staged_item(&self, item: StagedItemDto) {
        let mut map = self.staged_items.lock().unwrap();
        map.insert(item.stage_id.clone(), item);
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
        let _ = hist_mgr.clear_session_items();
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

/// Generates a timestamped .origin backup path (Design 5.2 Mode B).
/// e.g. report.pdf -> report_20261025_201530.origin.pdf
pub fn generate_origin_backup_path(orig_path: &Path) -> PathBuf {
    let parent = orig_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = orig_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = orig_path.extension().and_then(|s| s.to_str());
    let now = chrono::Local::now();
    let timestamp = now.format("%Y%m%d_%H%M%S").to_string();

    let backup_filename = if let Some(e) = ext {
        format!("{}_{}.origin.{}", stem, timestamp, e)
    } else {
        format!("{}_{}.origin", stem, timestamp)
    };

    let mut candidate = parent.join(backup_filename);
    let mut counter = 1;
    while candidate.exists() {
        let unique_name = if let Some(e) = ext {
            format!("{}_{}_{}.origin.{}", stem, timestamp, counter, e)
        } else {
            format!("{}_{}_{}.origin", stem, timestamp, counter)
        };
        candidate = parent.join(unique_name);
        counter += 1;
    }
    candidate
}

/// Generates a timestamped backup path for files that received concurrent external writes
/// from an open file descriptor during or after reflection.
/// e.g. report.pdf -> report_20261025_201530.concurrent_write.pdf
pub fn generate_lost_write_backup_path(orig_path: &Path) -> PathBuf {
    let parent = orig_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = orig_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = orig_path.extension().and_then(|s| s.to_str());
    let now = chrono::Local::now();
    let timestamp = now.format("%Y%m%d_%H%M%S").to_string();

    let backup_filename = if let Some(e) = ext {
        format!("{}_{}.concurrent_write.{}", stem, timestamp, e)
    } else {
        format!("{}_{}.concurrent_write", stem, timestamp)
    };

    let mut candidate = parent.join(backup_filename);
    let mut counter = 1;
    while candidate.exists() {
        let unique_name = if let Some(e) = ext {
            format!("{}_{}_{}.concurrent_write.{}", stem, timestamp, counter, e)
        } else {
            format!("{}_{}_{}.concurrent_write", stem, timestamp, counter)
        };
        candidate = parent.join(unique_name);
        counter += 1;
    }
    candidate
}

/// Validates whether a filename strictly conforms to Cuddle's generated staging artifact naming conventions,
/// including structural full-UUID verification to prevent accidental deletion of user or third-party files.
pub fn is_cuddle_staging_artifact_name(fname: &str) -> bool {
    // 1. .tmp_stage_{UUID}_{display_name}
    // Must start with .tmp_stage_, followed by a valid UUID, followed by '_' and a non-empty display name.
    if let Some(rest) = fname.strip_prefix(".tmp_stage_") {
        if let Some(idx) = rest.find('_') {
            let uuid_part = &rest[..idx];
            let display_name = &rest[idx + 1..];
            if !display_name.is_empty() && Uuid::parse_str(uuid_part).is_ok() {
                return true;
            }
        }
    }

    // 2. .cuddle_reflect_{UUID}_{UUID}.tmp
    // Must start with .cuddle_reflect_ and end with .tmp, containing EXACTLY two valid UUIDs separated by '_'.
    if fname.starts_with(".cuddle_reflect_") && fname.ends_with(".tmp") {
        let core = &fname[".cuddle_reflect_".len()..fname.len() - ".tmp".len()];
        let parts: Vec<&str> = core.split('_').collect();
        if parts.len() == 2
            && Uuid::parse_str(parts[0]).is_ok()
            && Uuid::parse_str(parts[1]).is_ok()
        {
            return true;
        }
    }

    // 3. {prefix}.unstage_trash_{UUID}_{UUID} or unstage_trash_{UUID}_{UUID}
    // Must contain unstage_trash_, and the remainder must contain EXACTLY two valid UUIDs separated by '_'.
    if let Some(idx) = fname.find("unstage_trash_") {
        let rest = &fname[idx + "unstage_trash_".len()..];
        let parts: Vec<&str> = rest.split('_').collect();
        if parts.len() == 2
            && Uuid::parse_str(parts[0]).is_ok()
            && Uuid::parse_str(parts[1]).is_ok()
        {
            return true;
        }
    }

    // 4. .probe_{UUID}
    // Must start with .probe_ and the remainder must be exactly a valid UUID.
    if let Some(uuid_part) = fname.strip_prefix(".probe_") {
        if Uuid::parse_str(uuid_part).is_ok() {
            return true;
        }
    }

    // 5. .cuddle_edit_{UUID}.tmp
    // Temporary file used during quick text editing in write_staged_text()
    if fname.starts_with(".cuddle_edit_") && fname.ends_with(".tmp") {
        let uuid_part = &fname[".cuddle_edit_".len()..fname.len() - ".tmp".len()];
        if Uuid::parse_str(uuid_part).is_ok() {
            return true;
        }
    }

    // 6. .cuddle_copy_{UUID}.tmp
    // Temporary file used during alternate copy creation in reflect_staged_item_as_copy()
    if fname.starts_with(".cuddle_copy_") && fname.ends_with(".tmp") {
        let uuid_part = &fname[".cuddle_copy_".len()..fname.len() - ".tmp".len()];
        if Uuid::parse_str(uuid_part).is_ok() {
            return true;
        }
    }

    false
}

/// Cleans up stale unstage trash and temporary files from all known staging directories.
/// CRITICAL SAFETY GUARANTEES:
/// 1. Processes and retries persisted failed-cleanup artifacts from SQLite cleanup_queue.
/// 2. Gathers both global and same-volume local staging directories (.cuddle-temp).
/// 3. Queries active DB sessions and NEVER removes artifacts belonging to active sessions (by stage_id).
/// 4. Only deletes orphan artifacts whose last modification time is older than 24 hours (86,400s).
pub fn cleanup_stale_staging_artifacts() {
    cleanup_stale_staging_artifacts_with_threshold(24 * 60 * 60);
}

/// Helper for cleanup with a custom stale age threshold (in seconds) against all known staging directories.
pub fn cleanup_stale_staging_artifacts_with_threshold(threshold_secs: u64) {
    let hist_mgr = crate::history::get_history_manager();

    // 1. Gather all known staging directories (global + local .cuddle-temp directories)
    let mut target_dirs = vec![get_global_staging_directory()];
    if let Ok(known_dirs) = hist_mgr.get_known_staging_directories() {
        for dir in known_dirs {
            if !target_dirs.iter().any(|d| d == &dir) {
                target_dirs.push(dir);
            }
        }
    }

    #[cfg(test)]
    let load_res = if test_hooks::is_fail_load_session_db() {
        Err("Simulated load_all_session_items failure".to_string())
    } else {
        hist_mgr.load_all_session_items()
    };
    #[cfg(not(test))]
    let load_res = hist_mgr.load_all_session_items();

    let session_items = match load_res {
        Ok(items) => items,
        Err(e) => {
            eprintln!(
                "[ERROR] cleanup_stale_staging_artifacts: Failed to load session items from database: {}. Aborting cleanup for data safety.",
                e
            );
            return;
        }
    };

    let active_stage_ids: std::collections::HashSet<String> = session_items
        .into_iter()
        .map(|item| item.stage_id)
        .collect();

    // 2. Process persisted retry cleanup queue with strict defense-in-depth guardrails
    if let Ok(queue) = hist_mgr.list_cleanup_queue() {
        for (id, path_str, _retries) in queue {
            let path = Path::new(&path_str);

            if !path.exists() {
                let _ = hist_mgr.remove_cleanup_queue_item(id);
                continue;
            }

            // Guardrail 1: Path must reside strictly inside one of the managed staging directories,
            // or be a validated local .cuddle-temp directory located directly under an existing parent directory.
            let is_in_managed_dir = path
                .parent()
                .map(|p| {
                    if target_dirs
                        .iter()
                        .any(|td| p == td || p.canonicalize().ok() == td.canonicalize().ok())
                    {
                        return true;
                    }
                    // Defense-in-depth: validate local .cuddle-temp
                    let dir_name = p.file_name().and_then(|f| f.to_str()).unwrap_or("");
                    if dir_name == ".cuddle-temp" {
                        if let Some(grandparent) = p.parent() {
                            let expected_temp = grandparent.join(".cuddle-temp");
                            return p == expected_temp
                                || (p.canonicalize().ok().is_some()
                                    && p.canonicalize().ok() == expected_temp.canonicalize().ok());
                        }
                    }
                    false
                })
                .unwrap_or(false);

            if !is_in_managed_dir {
                eprintln!(
                    "[SECURITY ALERT] Refusing to clean up path '{}': outside managed staging directories",
                    path.display()
                );
                let _ = hist_mgr.update_cleanup_queue_error(
                    id,
                    "Security violation: path is outside managed staging directories",
                );
                continue;
            }

            // Guardrail 2: Filename must match safe temporary/orphan naming conventions
            let fname = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
            let is_allowed_artifact = is_cuddle_staging_artifact_name(fname);

            if !is_allowed_artifact {
                eprintln!(
                    "[SECURITY ALERT] Refusing to clean up path '{}': unrecognized artifact filename pattern",
                    path.display()
                );
                let _ = hist_mgr.update_cleanup_queue_error(
                    id,
                    "Security violation: unrecognized artifact filename pattern",
                );
                continue;
            }

            // Guardrail 3: Never delete an artifact belonging to an active session
            let belongs_to_active = active_stage_ids
                .iter()
                .any(|stage_id| fname.contains(stage_id));
            if belongs_to_active {
                continue;
            }

            // Guardrail 4: Inspect symlink metadata to safely remove symlink without traversing target
            if let Ok(meta) = fs::symlink_metadata(path) {
                if meta.file_type().is_symlink() {
                    if fs::remove_file(path).is_ok() {
                        let _ = hist_mgr.remove_cleanup_queue_item(id);
                    }
                    continue;
                }
            }

            // Guardrail 5: Safely remove standard file with granular error recording
            match fs::remove_file(path) {
                Ok(()) => {
                    let _ = hist_mgr.remove_cleanup_queue_item(id);
                }
                Err(e) => {
                    let err_msg = format!("Failed to remove file during maintenance retry: {}", e);
                    let _ = hist_mgr.update_cleanup_queue_error(id, &err_msg);
                }
            }
        }
    }

    // 3. Perform maintenance against each staging directory
    for dir in target_dirs {
        cleanup_stale_staging_artifacts_with_dir_and_threshold(&dir, threshold_secs);
    }
}

/// Helper for cleanup with an explicit staging directory and custom stale age threshold (in seconds).
pub fn cleanup_stale_staging_artifacts_with_dir_and_threshold(
    staging_dir: &Path,
    threshold_secs: u64,
) {
    if !staging_dir.exists() {
        return;
    }

    let hist_mgr = crate::history::get_history_manager();
    #[cfg(test)]
    let load_res = if test_hooks::is_fail_load_session_db() {
        Err("Simulated load_all_session_items failure".to_string())
    } else {
        hist_mgr.load_all_session_items()
    };
    #[cfg(not(test))]
    let load_res = hist_mgr.load_all_session_items();

    let session_items = match load_res {
        Ok(items) => items,
        Err(e) => {
            eprintln!(
                "[ERROR] cleanup_stale_staging_artifacts: Failed to load session items from database: {}. Aborting cleanup for data safety.",
                e
            );
            return;
        }
    };

    let active_stage_ids: std::collections::HashSet<String> = session_items
        .into_iter()
        .map(|item| item.stage_id)
        .collect();

    let now = SystemTime::now();

    if let Ok(entries) = fs::read_dir(staging_dir) {
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            let is_candidate = is_cuddle_staging_artifact_name(&fname);

            if !is_candidate {
                continue;
            }

            // Never delete an artifact belonging to any active session item
            let belongs_to_active_session = active_stage_ids
                .iter()
                .any(|stage_id| fname.contains(stage_id));
            if belongs_to_active_session {
                continue;
            }

            // Check file modification time: only remove if older than the threshold
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age.as_secs() >= threshold_secs {
                            let _ = fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }
}

/// Generates an alternate non-destructive copy path with a custom or default suffix (e.g. .edit).
/// Automatically resolves collisions: filename.ext -> filename.edit.ext -> filename.edit.1.ext
pub fn generate_alternate_copy_path(orig_path: &Path, suffix: Option<&str>) -> PathBuf {
    let parent = orig_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = orig_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = orig_path.extension().and_then(|s| s.to_str());
    let raw_suffix = suffix.unwrap_or("edit").trim();
    let clean_suffix = raw_suffix.trim_start_matches('.');
    let effective_suffix = if clean_suffix.is_empty() {
        "edit"
    } else {
        clean_suffix
    };

    let candidate_name = if let Some(e) = ext {
        format!("{}.{}.{}", stem, effective_suffix, e)
    } else {
        format!("{}.{}", stem, effective_suffix)
    };

    let mut candidate = parent.join(&candidate_name);
    let mut counter = 1;
    while candidate.exists() {
        let unique_name = if let Some(e) = ext {
            format!("{}.{}.{}.{}", stem, effective_suffix, counter, e)
        } else {
            format!("{}.{}.{}", stem, effective_suffix, counter)
        };
        candidate = parent.join(unique_name);
        counter += 1;
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    fn lock_test_mutex() -> std::sync::MutexGuard<'static, ()> {
        TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
    }

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
        let _guard = lock_test_mutex();
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
        mgr.unstage_file(&staged.stage_id)
            .expect("Unstaging must succeed");
        assert!(
            !Path::new(&staged.staged_path).exists(),
            "Staged file must be deleted"
        );
        assert!(orig_file.exists(), "Original file must remain intact!");

        assert_eq!(mgr.list_staged().len(), 0);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_stage_non_utf8_filename() {
        let _guard = lock_test_mutex();
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
        let staged = mgr
            .stage_file(&non_utf8_file)
            .expect("Must stage non-UTF8 file without panic");

        assert!(staged.display_name.contains('\u{FFFD}'));
        assert!(Path::new(&staged.staged_path).exists());

        mgr.clear_all().unwrap();
        assert_eq!(mgr.list_staged().len(), 0);
        assert!(
            non_utf8_file.exists(),
            "Original non-UTF8 file must remain intact"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_workspace_manager_reset_to_original() {
        let _guard = lock_test_mutex();
        let temp_dir = std::env::temp_dir().join(format!("cuddle_ws_reset_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let orig_file = temp_dir.join("code.rs");
        fs::write(&orig_file, b"fn main() { println!(\"original\"); }").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        // Mutate staged working copy
        fs::write(&staged.staged_path, b"fn main() { println!(\"mutated\"); }").unwrap();
        assert_eq!(
            fs::read(&staged.staged_path).unwrap(),
            b"fn main() { println!(\"mutated\"); }"
        );

        // Verify diff before reset
        let diff = mgr.get_diff(&staged.stage_id).unwrap();
        assert!(diff.has_changes);

        // Reset staged copy to pristine original
        let reset_item = mgr.reset_staged_file(&staged.stage_id).unwrap();
        assert_eq!(reset_item.status, StageStatus::Staged);
        assert_eq!(
            fs::read(&staged.staged_path).unwrap(),
            b"fn main() { println!(\"original\"); }"
        );

        // Diff after reset must be clean
        let diff_after = mgr.get_diff(&staged.stage_id).unwrap();
        assert!(!diff_after.has_changes);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_workspace_manager_reflect_success() {
        let _guard = lock_test_mutex();
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
        let _guard = lock_test_mutex();
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
    fn test_workspace_session_restore_and_conflict_detection() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_session_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("session_test.txt");
        fs::write(&orig_file, b"Session original content").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        // 1. Restore on a new WorkspaceManager instance
        let new_mgr = WorkspaceManager::new();
        let restored_list = new_mgr.restore_session().unwrap();
        assert!(restored_list
            .iter()
            .any(|item| item.stage_id == staged.stage_id));

        let restored_item = restored_list
            .into_iter()
            .find(|i| i.stage_id == staged.stage_id)
            .unwrap();
        assert_eq!(restored_item.status, StageStatus::Staged);

        // 2. Simulate external modification of the original file
        std::thread::sleep(std::time::Duration::from_millis(15));
        fs::write(&orig_file, b"External modified session content").unwrap();

        // 3. Restore again: external modification must be detected as Conflict!
        let third_mgr = WorkspaceManager::new();
        let restored_list_2 = third_mgr.restore_session().unwrap();
        let conflict_item = restored_list_2
            .into_iter()
            .find(|i| i.stage_id == staged.stage_id)
            .unwrap();
        assert_eq!(conflict_item.status, StageStatus::Conflict);

        // Clean up
        let _ = mgr.unstage_file(&staged.stage_id);
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_manager_reflect_mode_b_backup_origin() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_reflect_mode_b_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("report.pdf");
        fs::write(&orig_file, b"Original PDF Version 1").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        // Mutate staged working copy
        fs::write(&staged.staged_path, b"Mutated Deliverable Version 2").unwrap();

        // Reflect with Mode B (BackupOrigin)
        let (reflected, backup_path) = mgr
            .reflect_staged_file_with_mode(&staged.stage_id, ReflectionMode::BackupOrigin)
            .unwrap();
        assert_eq!(reflected.status, StageStatus::Reflected);
        assert!(backup_path.is_some());

        let bkp = backup_path.unwrap();
        assert!(bkp.exists());
        assert!(bkp.to_string_lossy().contains(".origin.pdf"));
        // Backup contains original content
        assert_eq!(fs::read(&bkp).unwrap(), b"Original PDF Version 1");

        // Original location contains mutated deliverable
        assert_eq!(
            fs::read(&orig_file).unwrap(),
            b"Mutated Deliverable Version 2"
        );

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_reflect_staged_item_as_copy_and_collision() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_reflect_copy_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("document.pdf");
        fs::write(&orig_file, b"Original PDF Content").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        // Mutate staged working copy
        fs::write(&staged.staged_path, b"Edited Deliverable Staged").unwrap();

        // Simulate external modification to original creating a conflict
        std::thread::sleep(std::time::Duration::from_millis(15));
        fs::write(&orig_file, b"Externally Mutated Original").unwrap();

        // Standard reflection must fail with Conflict
        let reflect_err = mgr.reflect_staged_file(&staged.stage_id);
        assert!(reflect_err.is_err());

        // Reflect as alternate copy with default .edit suffix
        let (saved_item, copy_path) = mgr
            .reflect_staged_item_as_copy(&staged.stage_id, None)
            .expect("Alternate copy save must succeed even when conflict exists");

        assert_eq!(saved_item.status, StageStatus::Reflected);
        assert!(copy_path.exists());
        assert_eq!(copy_path, test_dir.join("document.edit.pdf"));
        assert_eq!(fs::read(&copy_path).unwrap(), b"Edited Deliverable Staged");

        // Verify original file was preserved untouched with external modification!
        assert_eq!(
            fs::read(&orig_file).unwrap(),
            b"Externally Mutated Original"
        );

        // Verify collision handling: second alternate copy should get sequence number .edit.1.pdf
        let staged_2 = mgr.stage_file(&orig_file).unwrap();
        fs::write(&staged_2.staged_path, b"Second Edit Deliverable").unwrap();

        let (_, copy_path_2) = mgr
            .reflect_staged_item_as_copy(&staged_2.stage_id, Some("edit".to_string()))
            .unwrap();

        assert_eq!(copy_path_2, test_dir.join("document.edit.1.pdf"));
        assert_eq!(fs::read(&copy_path_2).unwrap(), b"Second Edit Deliverable");

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_reflect_staged_item_as_copy_outside_home_directory() {
        let _guard = lock_test_mutex();
        let temp_dir =
            std::env::temp_dir().join(format!("cuddle_outside_home_test_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let orig_file = temp_dir.join("contract.txt");
        fs::write(&orig_file, b"Initial Contract Version").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        // Mutate staged working copy
        fs::write(&staged.staged_path, b"Edited Contract Staged Version").unwrap();

        // Standard reflection must fail because /tmp is outside home (ForbiddenZone)
        let reflect_err = mgr.reflect_staged_file(&staged.stage_id);
        assert!(reflect_err.is_err());

        // But alternate copy must SUCCEED as a non-destructive safe export!
        let (saved_item, copy_path) = mgr
            .reflect_staged_item_as_copy(&staged.stage_id, Some("edit".to_string()))
            .expect("Alternate copy save must succeed even for files outside home directory");

        assert_eq!(saved_item.status, StageStatus::Reflected);
        assert!(copy_path.exists());
        assert_eq!(copy_path, temp_dir.join("contract.edit.txt"));
        assert_eq!(
            fs::read(&copy_path).unwrap(),
            b"Edited Contract Staged Version"
        );

        // Verify original file is untouched
        assert_eq!(fs::read(&orig_file).unwrap(), b"Initial Contract Version");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_unstage_removes_physical_file_and_session() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_unstage_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("to_unstage.txt");
        fs::write(&orig_file, b"Unstage content").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();
        let staged_copy_path = PathBuf::from(&staged.staged_path);
        assert!(staged_copy_path.exists());

        // Unstage file
        mgr.unstage_file(&staged.stage_id)
            .expect("Unstage should succeed");

        // Staged physical file must be deleted from disk
        assert!(!staged_copy_path.exists());

        // Must be removed from active memory
        assert!(mgr.get_staged(&staged.stage_id).is_err());

        // Original file must remain untouched
        assert!(orig_file.exists());
        assert_eq!(fs::read(&orig_file).unwrap(), b"Unstage content");

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_read_and_write_staged_text() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_editor_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join(".bashrc");
        fs::write(
            &orig_file,
            b"# Initial bashrc\nexport PATH=\"$HOME/bin:$PATH\"\n",
        )
        .unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();
        assert_eq!(staged.status, StageStatus::Staged);

        // Read initial text
        let content = mgr.read_staged_text(&staged.stage_id).unwrap();
        assert_eq!(
            content,
            "# Initial bashrc\nexport PATH=\"$HOME/bin:$PATH\"\n"
        );

        // Write edited text
        let updated_content =
            "# Initial bashrc\nexport PATH=\"$HOME/bin:$PATH\"\nalias ll='ls -la'\n";
        let updated_staged = mgr
            .write_staged_text(&staged.stage_id, updated_content)
            .unwrap();

        assert_eq!(updated_staged.status, StageStatus::Modified);
        assert_eq!(updated_staged.size, updated_content.len() as u64);

        // Read again and verify persistence
        let read_back = mgr.read_staged_text(&staged.stage_id).unwrap();
        assert_eq!(read_back, updated_content);

        // Original file must be completely untouched
        assert_eq!(
            fs::read(&orig_file).unwrap(),
            b"# Initial bashrc\nexport PATH=\"$HOME/bin:$PATH\"\n"
        );

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_read_staged_text_rejects_binary() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_bin_editor_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("binary.dat");
        fs::write(&orig_file, [0x7f, b'E', b'L', b'F', 0x00, 0x01, 0x02]).unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        let result = mgr.read_staged_text(&staged.stage_id);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Binary file"));

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_reflect_toctou_atomic_exchange_conflict_protection() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_toctou_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("concurrent_doc.txt");
        fs::write(&orig_file, b"Original document content V1").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        // Mutate staged working copy to V2
        fs::write(&staged.staged_path, b"Staged document content V2").unwrap();

        // Simulate external process concurrently modifying the original document right before/during reflection
        std::thread::sleep(std::time::Duration::from_millis(15));
        fs::write(&orig_file, b"External process modified content V1.5").unwrap();

        // Attempt reflection: must be detected as conflict and blocked!
        let reflect_result =
            mgr.reflect_staged_file_with_mode(&staged.stage_id, ReflectionMode::BackupOrigin);
        assert!(
            reflect_result.is_err(),
            "Reflection must fail when external conflict is detected"
        );

        // Verify original file was protected and retains the external modification V1.5
        let current_orig_content = fs::read(&orig_file).unwrap();
        assert_eq!(
            current_orig_content, b"External process modified content V1.5",
            "Original file must retain external process modifications intact without data loss"
        );

        // Verify status changed to Conflict
        let fetched_staged = mgr.get_staged(&staged.stage_id).unwrap();
        assert_eq!(fetched_staged.status, StageStatus::Conflict);

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_reflect_concurrent_open_fd_write_protection() {
        let _guard = lock_test_mutex();
        use std::io::Write;

        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_open_fd_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("open_fd_doc.txt");
        fs::write(&orig_file, b"Original FD file content V1").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        // Mutate staged working copy
        fs::write(&staged.staged_path, b"Staged reflected content V2").unwrap();

        // Simulate external process holding an open write file descriptor
        let external_fd = Arc::new(Mutex::new(
            fs::OpenOptions::new()
                .write(true)
                .open(&orig_file)
                .expect("External process opens file"),
        ));

        // Use post-swap hook to write via open FD *after* atomic exchange, right before concurrent_check!
        let fd_clone = Arc::clone(&external_fd);
        test_hooks::set_post_swap_hook(move |_quarantined_path| {
            let mut fd = fd_clone.lock().unwrap();
            fd.write_all(b"Concurrent external write through open FD post-swap")
                .unwrap();
            fd.flush().unwrap();
        });

        // Attempt reflection in Overwrite mode
        let reflect_res =
            mgr.reflect_staged_file_with_mode(&staged.stage_id, ReflectionMode::Overwrite);
        test_hooks::clear_post_swap_hook();

        assert!(
            reflect_res.is_ok(),
            "Reflection succeeds while safely preserving concurrent FD write"
        );
        let (reflected_item, backup_path) = reflect_res.unwrap();
        assert_eq!(reflected_item.status, StageStatus::Reflected);
        assert!(
            backup_path.is_some(),
            "Backup path must be returned pointing to preserved concurrent writes"
        );

        let lost_write_path = backup_path.unwrap();
        assert!(
            lost_write_path.exists(),
            "Preserved lost-write backup file must exist on disk"
        );
        let lost_bytes = fs::read(&lost_write_path).unwrap();
        let expected_payload = b"Concurrent external write through open FD post-swap";
        assert!(
            lost_bytes
                .windows(expected_payload.len())
                .any(|w| w == expected_payload),
            "Preserved backup file must contain the complete concurrent write payload"
        );

        // Original file must have the staged content V2
        let orig_bytes = fs::read(&orig_file).unwrap();
        assert_eq!(orig_bytes, b"Staged reflected content V2");

        drop(external_fd);
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_reflect_concurrent_write_before_unlink_protection() {
        let _guard = lock_test_mutex();
        use std::io::Write;

        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_post_verify_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("post_verify_doc.txt");
        fs::write(&orig_file, b"Original V1").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        fs::write(&staged.staged_path, b"Staged V2").unwrap();

        let external_fd = Arc::new(Mutex::new(
            fs::OpenOptions::new()
                .write(true)
                .open(&orig_file)
                .expect("External process opens file"),
        ));

        // Use post-verify hook to simulate write occurring immediately before unlink
        let fd_clone = Arc::clone(&external_fd);
        test_hooks::set_post_verify_hook(move |_quarantined_path| {
            let mut fd = fd_clone.lock().unwrap();
            fd.write_all(b"Last-millisecond write before unlink")
                .unwrap();
            fd.flush().unwrap();
        });

        let reflect_res =
            mgr.reflect_staged_file_with_mode(&staged.stage_id, ReflectionMode::Overwrite);
        test_hooks::clear_post_verify_hook();

        assert!(reflect_res.is_ok());
        let (_item, backup_path) = reflect_res.unwrap();
        assert!(backup_path.is_some());
        let lost_path = backup_path.unwrap();
        assert!(lost_path.exists());
        let lost_bytes = fs::read(&lost_path).unwrap();
        let expected_payload = b"Last-millisecond write before unlink";
        assert!(
            lost_bytes
                .windows(expected_payload.len())
                .any(|w| w == expected_payload),
            "Preserved backup file must contain the complete last-millisecond payload"
        );

        drop(external_fd);
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_unstage_recovery_preserves_extension_and_restores_by_stage_id() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_unstage_rec_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("document.txt");
        fs::write(&orig_file, b"Original document text").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        let staged_path = PathBuf::from(&staged.staged_path);
        assert!(staged_path.exists());

        // Simulate interrupted unstage: staged copy was renamed to trash with extension intact and stage_id
        let orig_name = staged_path.file_name().unwrap().to_string_lossy();
        let trash_name = format!(
            "{}.unstage_trash_{}_{}",
            orig_name,
            staged.stage_id,
            Uuid::new_v4()
        );
        let trash_path = staged_path.with_file_name(trash_name);
        fs::rename(&staged_path, &trash_path).unwrap();

        // Staged copy does not exist at original path, but trash exists
        assert!(!staged_path.exists());
        assert!(trash_path.exists());

        // Trigger session restore: must recover trash by stage_id
        let restored = mgr.restore_session().unwrap();
        let found = restored.iter().find(|i| i.stage_id == staged.stage_id);
        assert!(found.is_some(), "Session item must be restored");
        assert!(
            staged_path.exists(),
            "Staged file must be recovered to original staged path"
        );
        assert!(!trash_path.exists(), "Trash file must be renamed away");

        // Verify missing file retention (anti-silent purge protection):
        // If staged file is deleted and no trash exists, it must be retained as Conflict, NOT purged
        fs::remove_file(&staged_path).unwrap();
        let restored_again = mgr.restore_session().unwrap();
        let found_conflict = restored_again
            .iter()
            .find(|i| i.stage_id == staged.stage_id);
        assert!(
            found_conflict.is_some(),
            "Item must NOT be silently purged from session"
        );
        assert_eq!(found_conflict.unwrap().status, StageStatus::Conflict);

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_reflect_lost_write_rename_failure_returns_error() {
        let _guard = lock_test_mutex();
        use std::io::Write;

        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_lost_write_err_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("lost_write_fail.txt");
        fs::write(&orig_file, b"Original V1").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        fs::write(&staged.staged_path, b"Staged V2").unwrap();

        let external_fd = Arc::new(Mutex::new(
            fs::OpenOptions::new()
                .write(true)
                .open(&orig_file)
                .expect("External process opens file"),
        ));

        // Hook simulates concurrent write and then restricts directory permissions to force rename failure
        let fd_clone = Arc::clone(&external_fd);
        let test_dir_clone = test_dir.clone();
        test_hooks::set_post_swap_hook(move |_quarantined_path| {
            let mut fd = fd_clone.lock().unwrap();
            fd.write_all(b"Concurrent write data that must be preserved")
                .unwrap();
            fd.flush().unwrap();
            // Restrict permissions on destination parent directory to force rename failure
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&test_dir_clone, fs::Permissions::from_mode(0o555));
            }
        });

        let reflect_res =
            mgr.reflect_staged_file_with_mode(&staged.stage_id, ReflectionMode::Overwrite);
        test_hooks::clear_post_swap_hook();

        // Restore permissions for inspection and cleanup
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&test_dir, fs::Permissions::from_mode(0o755));
        }

        // Must FAIL with LostWritePreserveFailed, NOT succeed!
        assert!(
            reflect_res.is_err(),
            "Must not succeed when lost-write preservation fails"
        );
        let err = reflect_res.unwrap_err();
        assert_eq!(err.error_code(), "ERR_LOST_WRITE_PRESERVE_FAILED");
        assert!(err
            .to_string()
            .contains("Failed to preserve concurrent external write"));

        // CRITICAL DATA PRESERVATION CHECK:
        // The quarantined temporary file must NOT have been deleted and must still exist with the data!
        if let WorkspaceError::LostWritePreserveFailed { tmp_path, .. } = &err {
            assert!(
                tmp_path.exists(),
                "Quarantined temporary file containing concurrent writes must remain on disk!"
            );
            let preserved_bytes = fs::read(tmp_path).unwrap();
            let expected_payload = b"Concurrent write data that must be preserved";
            assert!(
                preserved_bytes
                    .windows(expected_payload.len())
                    .any(|w| w == expected_payload),
                "Quarantined file must retain the full uncorrupted external write payload"
            );
        } else {
            panic!("Expected LostWritePreserveFailed error variant");
        }

        drop(external_fd);
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_reflect_rollback_failure_preserves_quarantined_original() {
        let _guard = lock_test_mutex();
        use std::os::unix::fs::PermissionsExt;

        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_rb_fail_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("original_precious_data.txt");
        let original_data = b"Precious original data that must NEVER be lost";
        fs::write(&orig_file, original_data).unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        // Mutate staged file to candidate content
        fs::write(&staged.staged_path, b"Candidate content to be reflected").unwrap();

        // Use post-swap hook to make directory read-only,
        // which forces the backup rename to fail AND rollback atomic_exchange_files to fail with EACCES!
        let test_dir_clone = test_dir.clone();
        test_hooks::set_post_swap_hook(move |_quarantined_path| {
            // Restrict permissions on test_dir so both backup rename and atomic_exchange_files fail
            let _ = fs::set_permissions(&test_dir_clone, fs::Permissions::from_mode(0o555));
        });

        let reflect_res =
            mgr.reflect_staged_file_with_mode(&staged.stage_id, ReflectionMode::BackupOrigin);
        test_hooks::clear_post_swap_hook();

        // Restore permissions for inspection and cleanup
        let _ = fs::set_permissions(&test_dir, fs::Permissions::from_mode(0o755));

        // Must FAIL with ReflectRollbackFailed, NOT delete the quarantined file!
        assert!(reflect_res.is_err(), "Reflection must fail");
        let err = reflect_res.unwrap_err();
        assert_eq!(err.error_code(), "ERR_REFLECT_ROLLBACK_FAILED");

        // CRITICAL DEFENSE: Verify quarantined original file was NOT deleted and retains original content!
        if let WorkspaceError::ReflectRollbackFailed {
            quarantined_path, ..
        } = &err
        {
            assert!(
                quarantined_path.exists(),
                "CRITICAL: Quarantined original file must NEVER be removed when rollback exchange fails!"
            );
            let preserved = fs::read(quarantined_path).unwrap();
            assert_eq!(
                preserved, original_data,
                "Quarantined original file must retain the user's precious original data intact"
            );
        } else {
            panic!(
                "Expected ReflectRollbackFailed error variant, got: {:?}",
                err
            );
        }

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_stage_file_detects_concurrent_modification_during_copy() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_stage_toctou_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("concurrent_test.txt");
        fs::write(&orig_file, b"Original file content before clone").unwrap();

        let mgr = WorkspaceManager::new();

        // Simulate external modification to original file during copy
        let orig_file_clone = orig_file.clone();
        test_hooks::set_post_stage_clone_hook(move |_path| {
            let _ = fs::write(
                &orig_file_clone,
                b"Externally mutated content during clone!",
            );
        });

        let stage_res = mgr.stage_file(&orig_file);
        test_hooks::clear_post_stage_clone_hook();

        assert!(
            stage_res.is_err(),
            "Must reject stage if original file was concurrently modified"
        );
        let err = stage_res.unwrap_err();
        assert_eq!(err.error_code(), "ERR_CONFLICT_DETECTED");

        // Verify no orphan staged files leaked into staging
        let staging_dir = resolve_staging_directory(&orig_file);
        if staging_dir.exists() {
            let entries: Vec<_> = fs::read_dir(&staging_dir)
                .unwrap()
                .flatten()
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .contains("concurrent_test.txt")
                })
                .collect();
            assert!(
                entries.is_empty(),
                "Staged file candidate must be removed on conflict detection"
            );
        }

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_session_restore_auto_recovers_applying_journal_state() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_wal_rec_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file_a = test_dir.join("file_a.txt");
        let orig_file_b = test_dir.join("file_b.txt");
        fs::write(&orig_file_a, b"Original A initial").unwrap();
        fs::write(&orig_file_b, b"Original B initial").unwrap();

        let mgr = WorkspaceManager::new();
        let staged_a = mgr.stage_file(&orig_file_a).unwrap();
        let staged_b = mgr.stage_file(&orig_file_b).unwrap();

        let hist_mgr = crate::history::get_history_manager();

        // Scenario 1: Physical swap completed, but DB crashed in `Applying` state.
        // Orig file on disk now matches the staged copy.
        fs::write(&staged_a.staged_path, b"Reflected A content").unwrap();
        fs::write(&orig_file_a, b"Reflected A content").unwrap();
        let mut item_a = staged_a.clone();
        item_a.status = StageStatus::Applying;
        hist_mgr.save_session_item(&item_a).unwrap();

        // Scenario 2: Physical swap failed or did not run, DB left in `Applying` state.
        // Orig file on disk still matches original snapshot.
        fs::write(&staged_b.staged_path, b"Candidate B content").unwrap();
        // orig_file_b remains b"Original B initial"
        let mut item_b = staged_b.clone();
        item_b.status = StageStatus::Applying;
        hist_mgr.save_session_item(&item_b).unwrap();

        // Restore session
        let restored = mgr.restore_session().unwrap();

        let restored_a = restored
            .iter()
            .find(|i| i.stage_id == staged_a.stage_id)
            .unwrap();
        assert_eq!(
            restored_a.status,
            StageStatus::Reflected,
            "Item A must auto-roll-forward to Reflected because disk content matches staged copy"
        );
        assert_eq!(
            restored_a.original_snapshot.sha256,
            Some(compute_file_sha256(&orig_file_a).unwrap()),
            "Snapshot must be refreshed on roll forward"
        );

        let restored_b = restored
            .iter()
            .find(|i| i.stage_id == staged_b.stage_id)
            .unwrap();
        assert_eq!(
            restored_b.status,
            StageStatus::Modified,
            "Item B must auto-roll-back to Modified because original file was untouched on disk"
        );

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_cleanup_stale_artifacts_protects_active_sessions() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_cleanup_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let test_staging = test_dir.join("staging");
        fs::create_dir_all(&test_staging).unwrap();

        let orig_file = test_dir.join("active_session_file.txt");
        fs::write(&orig_file, b"Active session data").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        // Ensure session item is registered in DB even if another parallel test modified session table
        let hist_mgr = crate::history::get_history_manager();
        let _ = hist_mgr.save_session_item(&staged);

        // 1. Create artifact containing active session's stage_id
        let active_artifact = test_staging.join(format!(
            "active.unstage_trash_{}_{}",
            staged.stage_id,
            Uuid::new_v4()
        ));
        fs::write(&active_artifact, b"Active session recovery trash").unwrap();

        // 2. Create orphan artifact with unknown stage_id
        let orphan_artifact = test_staging.join(format!(
            "orphan.unstage_trash_{}_{}",
            Uuid::new_v4(),
            Uuid::new_v4()
        ));
        fs::write(&orphan_artifact, b"Stale orphan trash").unwrap();

        // Run cleanup with 0-second threshold against test_staging
        cleanup_stale_staging_artifacts_with_dir_and_threshold(&test_staging, 0);

        // Active session artifact MUST NOT be deleted even with threshold 0!
        assert!(
            active_artifact.exists(),
            "Artifact belonging to active session must NEVER be deleted"
        );

        // Orphan artifact MUST be deleted because it exceeded the 0-second threshold
        assert!(
            !orphan_artifact.exists(),
            "Orphan artifact older than threshold must be cleaned up"
        );

        // 3. Create a fresh orphan artifact and test 24h threshold
        let fresh_orphan = test_staging.join(format!(
            "fresh.unstage_trash_{}_{}",
            Uuid::new_v4(),
            Uuid::new_v4()
        ));
        fs::write(&fresh_orphan, b"Fresh orphan trash").unwrap();

        // Run standard cleanup with 24h threshold (86400s)
        cleanup_stale_staging_artifacts_with_dir_and_threshold(&test_staging, 24 * 60 * 60);

        assert!(
            fresh_orphan.exists(),
            "Fresh orphan (<24h) must NOT be deleted by standard cleanup threshold"
        );

        // Clean up test files
        let _ = fs::remove_file(&active_artifact);
        let _ = fs::remove_file(&fresh_orphan);
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_session_restore_preserves_concurrent_writes_in_quarantine_file() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_wal_preserve_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("doc.txt");
        fs::write(&orig_file, b"Original V1").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        let hist_mgr = crate::history::get_history_manager();

        // Simulate state right after swap + crash:
        // orig_file now has reflected content
        fs::write(&orig_file, b"Reflected Content V2").unwrap();
        fs::write(&staged.staged_path, b"Reflected Content V2").unwrap();

        // But external FD wrote to the quarantined original file before the crash!
        let quarantine_file = test_dir.join(format!(
            ".cuddle_reflect_{}_{}.tmp",
            staged.stage_id,
            Uuid::new_v4()
        ));
        let external_write_payload = b"Original V1 + Concurrent write from external process!";
        fs::write(&quarantine_file, external_write_payload).unwrap();

        // DB still in Applying
        let mut item = staged.clone();
        item.status = StageStatus::Applying;
        hist_mgr.save_session_item(&item).unwrap();

        // Run session restore
        let restored = mgr.restore_session().unwrap();
        let restored_item = restored
            .iter()
            .find(|i| i.stage_id == staged.stage_id)
            .unwrap();

        assert_eq!(
            restored_item.status,
            StageStatus::Reflected,
            "Item status should be Reflected because orig_file has reflected content"
        );

        // P0 CRITICAL VERIFICATION:
        // The quarantined file must NOT have been unlinked! It must be safely moved to a .concurrent_write backup!
        assert!(
            !quarantine_file.exists(),
            "Temporary quarantine filename should be renamed away"
        );

        let entries: Vec<_> = fs::read_dir(&test_dir)
            .unwrap()
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .contains(".concurrent_write")
            })
            .collect();

        assert_eq!(
            entries.len(),
            1,
            "Exactly one .concurrent_write backup must be preserved"
        );
        let preserved_bytes = fs::read(entries[0].path()).unwrap();
        assert_eq!(
            preserved_bytes, external_write_payload,
            "Preserved backup must contain the exact external write content without loss"
        );

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_session_restore_quarantine_preserve_failure_retains_file_and_marks_conflict()
    {
        let _guard = lock_test_mutex();
        use std::os::unix::fs::PermissionsExt;

        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_wal_preserve_fail_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("doc_fail.txt");
        fs::write(&orig_file, b"Original V1").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        let hist_mgr = crate::history::get_history_manager();

        fs::write(&orig_file, b"Reflected Content V2").unwrap();
        fs::write(&staged.staged_path, b"Reflected Content V2").unwrap();

        let quarantine_file = test_dir.join(format!(
            ".cuddle_reflect_{}_{}.tmp",
            staged.stage_id,
            Uuid::new_v4()
        ));
        let external_write_payload = b"Original V1 with unpreserved external write";
        fs::write(&quarantine_file, external_write_payload).unwrap();

        let mut item = staged.clone();
        item.status = StageStatus::Applying;
        hist_mgr.save_session_item(&item).unwrap();

        // Restrict directory permissions so rename to backup fails
        let _ = fs::set_permissions(&test_dir, fs::Permissions::from_mode(0o555));

        let restored = mgr.restore_session().unwrap();

        // Restore permissions for assertions and cleanup
        let _ = fs::set_permissions(&test_dir, fs::Permissions::from_mode(0o755));

        let restored_item = restored
            .iter()
            .find(|i| i.stage_id == staged.stage_id)
            .unwrap();

        // P0 CRITICAL DEFENSE:
        // Must mark Conflict and NEVER remove quarantine file when backup preservation fails!
        assert_eq!(
            restored_item.status,
            StageStatus::Conflict,
            "Must fall back to Conflict if concurrent write cannot be safely renamed"
        );
        assert!(
            quarantine_file.exists(),
            "CRITICAL: Quarantined original file must NEVER be deleted if rename fails!"
        );
        let preserved_bytes = fs::read(&quarantine_file).unwrap();
        assert_eq!(preserved_bytes, external_write_payload);

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_stage_file_db_failure_cleans_up_staged_file_and_memory() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_stage_db_fail_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("db_fail_test.txt");
        fs::write(&orig_file, b"Content for DB failure test").unwrap();

        let mgr = WorkspaceManager::new();
        test_hooks::set_fail_stage_db(true);
        let res = mgr.stage_file(&orig_file);
        test_hooks::set_fail_stage_db(false);

        assert!(
            res.is_err(),
            "stage_file must return Err on DB persistence failure"
        );
        assert!(
            mgr.list_staged().is_empty(),
            "Memory map must have 0 items on DB failure"
        );

        // Verify no ghost history records were recorded in history table
        let hist_mgr = crate::history::get_history_manager();
        let history = hist_mgr.list_history(50).unwrap_or_default();
        let has_ghost_history = history.iter().any(|h| h.display_name == "db_fail_test.txt");
        assert!(
            !has_ghost_history,
            "No ghost history record must exist for failed stage"
        );

        let staging_dir = resolve_staging_directory(&orig_file);
        if staging_dir.exists() {
            let count = fs::read_dir(&staging_dir)
                .unwrap()
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().contains("db_fail_test.txt"))
                .count();
            assert_eq!(count, 0, "No orphan staged copy must remain on disk");
        }

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_session_restore_quarantine_hash_failure_preserves_file() {
        let _guard = lock_test_mutex();
        use std::os::unix::fs::PermissionsExt;

        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_wal_unreadable_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("unreadable.txt");
        fs::write(&orig_file, b"Original V1").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();
        let hist_mgr = crate::history::get_history_manager();

        fs::write(&orig_file, b"Reflected Content V2").unwrap();
        fs::write(&staged.staged_path, b"Reflected Content V2").unwrap();

        let quarantine_file = test_dir.join(format!(
            ".cuddle_reflect_{}_{}.tmp",
            staged.stage_id,
            Uuid::new_v4()
        ));
        fs::write(&quarantine_file, b"Unreadable original data").unwrap();

        // Make quarantine file unreadable so SHA-256 computation fails (returns None)
        let _ = fs::set_permissions(&quarantine_file, fs::Permissions::from_mode(0o000));

        let mut item = staged.clone();
        item.status = StageStatus::Applying;
        hist_mgr.save_session_item(&item).unwrap();

        let _restored = mgr.restore_session().unwrap();

        // Restore permissions for assertion and cleanup
        let _ = fs::set_permissions(&quarantine_file, fs::Permissions::from_mode(0o644));

        // When SHA-256 cannot be verified, it must NEVER be deleted!
        // It must either be preserved as .concurrent_write or kept as quarantine file with Conflict.
        let has_quarantine_or_backup = quarantine_file.exists()
            || fs::read_dir(&test_dir).unwrap().flatten().any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .contains(".concurrent_write")
            });
        assert!(
            has_quarantine_or_backup,
            "Unverifiable quarantine file must NEVER be unconditionally deleted"
        );

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_session_restore_concurrent_write_path_collision_avoids_overwrite() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_wal_collision_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("collision.txt");
        fs::write(&orig_file, b"Original V1").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();
        let hist_mgr = crate::history::get_history_manager();

        fs::write(&orig_file, b"Reflected Content V2").unwrap();
        fs::write(&staged.staged_path, b"Reflected Content V2").unwrap();

        // Pre-create the first expected .concurrent_write file to force collision resolution
        let expected_backup_path = generate_lost_write_backup_path(&orig_file);
        fs::write(
            &expected_backup_path,
            b"Pre-existing backup from earlier run",
        )
        .unwrap();

        let quarantine_file = test_dir.join(format!(
            ".cuddle_reflect_{}_{}.tmp",
            staged.stage_id,
            Uuid::new_v4()
        ));
        let concurrent_write_content = b"New concurrent write during crash";
        fs::write(&quarantine_file, concurrent_write_content).unwrap();

        let mut item = staged.clone();
        item.status = StageStatus::Applying;
        hist_mgr.save_session_item(&item).unwrap();

        let _restored = mgr.restore_session().unwrap();

        // Pre-existing backup must NOT be overwritten!
        assert_eq!(
            fs::read(&expected_backup_path).unwrap(),
            b"Pre-existing backup from earlier run",
            "Pre-existing backup must not be overwritten"
        );

        // A second backup with numbered suffix must have been created
        let backup_files: Vec<_> = fs::read_dir(&test_dir)
            .unwrap()
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .contains(".concurrent_write")
            })
            .collect();
        assert_eq!(
            backup_files.len(),
            2,
            "Both pre-existing and newly preserved backups must exist"
        );

        let has_new_content = backup_files.iter().any(|e| {
            fs::read(e.path())
                .map(|b| b == concurrent_write_content)
                .unwrap_or(false)
        });
        assert!(
            has_new_content,
            "New concurrent write content must be preserved under collision-free name"
        );

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_session_restore_db_failure_retains_quarantine_file() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_wal_db_fail_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("db_fail.txt");
        fs::write(&orig_file, b"Original V1").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();
        let hist_mgr = crate::history::get_history_manager();

        fs::write(&orig_file, b"Reflected Content V2").unwrap();
        fs::write(&staged.staged_path, b"Reflected Content V2").unwrap();

        let quarantine_file = test_dir.join(format!(
            ".cuddle_reflect_{}_{}.tmp",
            staged.stage_id,
            Uuid::new_v4()
        ));
        fs::write(&quarantine_file, b"Quarantine content").unwrap();

        let mut item = staged.clone();
        item.status = StageStatus::Applying;
        hist_mgr.save_session_item(&item).unwrap();

        // Simulate SQLite failure during restore
        test_hooks::set_fail_restore_db(true);
        let _restored = mgr.restore_session();
        test_hooks::set_fail_restore_db(false);

        // When DB save fails, quarantine file must be completely untouched and retained for next restart!
        assert!(
            quarantine_file.exists(),
            "Quarantine file must be preserved when DB persistence fails during recovery"
        );

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_session_restore_multiple_quarantine_files_safely_processed() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_wal_multi_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("multi.txt");
        fs::write(&orig_file, b"Original V1").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();
        let hist_mgr = crate::history::get_history_manager();

        fs::write(&orig_file, b"Reflected Content V2").unwrap();
        fs::write(&staged.staged_path, b"Reflected Content V2").unwrap();

        // File 1: identical to pre-reflection snapshot (clean - safe to remove)
        let q_clean = test_dir.join(format!(
            ".cuddle_reflect_{}_{}.tmp",
            staged.stage_id,
            Uuid::new_v4()
        ));
        fs::write(&q_clean, b"Original V1").unwrap();

        // File 2: concurrent write A (must preserve)
        let q_mod_a = test_dir.join(format!(
            ".cuddle_reflect_{}_{}.tmp",
            staged.stage_id,
            Uuid::new_v4()
        ));
        fs::write(&q_mod_a, b"External write payload A").unwrap();

        // File 3: concurrent write B (must preserve)
        let q_mod_b = test_dir.join(format!(
            ".cuddle_reflect_{}_{}.tmp",
            staged.stage_id,
            Uuid::new_v4()
        ));
        fs::write(&q_mod_b, b"External write payload B").unwrap();

        let mut item = staged.clone();
        item.status = StageStatus::Applying;
        hist_mgr.save_session_item(&item).unwrap();

        let restored = mgr.restore_session().unwrap();
        let restored_item = restored
            .iter()
            .find(|i| i.stage_id == staged.stage_id)
            .unwrap();
        // Section 8 Test 3: Multiple quarantines must be treated as Conflict fail-safe
        assert_eq!(restored_item.status, StageStatus::Conflict);

        // All quarantine files must be preserved - none unlinked!
        assert!(
            q_clean.exists(),
            "Clean quarantine file must NOT be unlinked"
        );
        assert!(q_mod_a.exists(), "Quarantine file A must NOT be unlinked");
        assert!(q_mod_b.exists(), "Quarantine file B must NOT be unlinked");

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_session_restore_recovers_tmp_stage_crash_window() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_tmp_stage_rec_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("crash_window.txt");
        let initial_payload = b"Original content before stage crash";
        fs::write(&orig_file, initial_payload).unwrap();

        let orig_item = FileItem::from_path(&orig_file).unwrap();
        let snapshot = OperationPolicy::create_snapshot(&orig_item, true).unwrap();

        let staging_dir = resolve_staging_directory(&orig_file);
        let stage_id = Uuid::new_v4().to_string();
        let staged_filename = format!("{}_{}", stage_id, orig_item.display_name);
        let formal_staged_path = staging_dir.join(&staged_filename);

        // Simulate crash right after SQLite commit and BEFORE rename:
        // Formal staged path does NOT exist yet!
        assert!(!formal_staged_path.exists());

        // Pre-commit temporary stage copy exists with identical valid content
        let tmp_stage_filename = format!(".tmp_stage_{}_{}", stage_id, orig_item.display_name);
        let tmp_stage_path = staging_dir.join(&tmp_stage_filename);
        fs::write(&tmp_stage_path, initial_payload).unwrap();

        // Session registered in SQLite DB with formal staged path
        let hist_mgr = crate::history::get_history_manager();
        let staged_dto = StagedItemDto {
            stage_id: stage_id.clone(),
            original_item_id: orig_item.item_id.clone(),
            raw_base64: orig_item.raw_base64.clone(),
            display_name: orig_item.display_name.clone(),
            staged_raw_base64: encode_path_to_base64(&formal_staged_path),
            staged_path: formal_staged_path.to_string_lossy().to_string(),
            size: snapshot.size,
            clone_method: CloneMethod::StandardCopy,
            status: StageStatus::Staged,
            staged_at_ms: 1000,
            original_snapshot: snapshot,
        };
        hist_mgr.save_session_item(&staged_dto).unwrap();

        // Execute session restore on restart
        let mgr = WorkspaceManager::new();
        let restored = mgr.restore_session().unwrap();
        let item = restored
            .iter()
            .find(|i| i.stage_id == stage_id)
            .expect("Stage must be restored");

        // P1 CRITICAL VERIFICATION:
        // Temporary copy must have been rolled forward to formal staged path and marked Staged!
        assert_eq!(
            item.status,
            StageStatus::Staged,
            "Must roll-forward to Staged status"
        );
        assert!(
            formal_staged_path.exists(),
            "Formal staged file must exist after roll-forward"
        );
        assert!(
            !tmp_stage_path.exists(),
            "Temporary file must have been renamed away"
        );
        assert_eq!(fs::read(&formal_staged_path).unwrap(), initial_payload);

        let _ = mgr.unstage_file(&stage_id);
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_session_restore_corrupted_tmp_stage_retained_as_conflict() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_corrupt_tmp_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("corrupt_test.txt");
        fs::write(&orig_file, b"Original snapshot data").unwrap();

        let orig_item = FileItem::from_path(&orig_file).unwrap();
        let snapshot = OperationPolicy::create_snapshot(&orig_item, true).unwrap();

        let staging_dir = resolve_staging_directory(&orig_file);
        let stage_id = Uuid::new_v4().to_string();
        let formal_staged_path =
            staging_dir.join(format!("{}_{}", stage_id, orig_item.display_name));

        // Temporary stage copy exists but is corrupted (content mismatch)
        let tmp_stage_path = staging_dir.join(format!(
            ".tmp_stage_{}_{}",
            stage_id, orig_item.display_name
        ));
        fs::write(&tmp_stage_path, b"Corrupted content from torn write!").unwrap();

        let hist_mgr = crate::history::get_history_manager();
        let staged_dto = StagedItemDto {
            stage_id: stage_id.clone(),
            original_item_id: orig_item.item_id.clone(),
            raw_base64: orig_item.raw_base64.clone(),
            display_name: orig_item.display_name.clone(),
            staged_raw_base64: encode_path_to_base64(&formal_staged_path),
            staged_path: formal_staged_path.to_string_lossy().to_string(),
            size: snapshot.size,
            clone_method: CloneMethod::StandardCopy,
            status: StageStatus::Staged,
            staged_at_ms: 1000,
            original_snapshot: snapshot,
        };
        hist_mgr.save_session_item(&staged_dto).unwrap();

        let mgr = WorkspaceManager::new();
        let restored = mgr.restore_session().unwrap();
        let item = restored
            .iter()
            .find(|i| i.stage_id == stage_id)
            .expect("Stage must be restored");

        // Integrity verification must fail -> DO NOT roll-forward; mark Conflict and retain temporary copy
        assert_eq!(
            item.status,
            StageStatus::Conflict,
            "Corrupted temporary stage must be retained as Conflict"
        );
        assert!(!formal_staged_path.exists());
        assert!(
            tmp_stage_path.exists(),
            "Corrupted temporary file must NEVER be deleted"
        );

        let _ = hist_mgr.remove_session_item(&stage_id);
        let _ = fs::remove_file(&tmp_stage_path);
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_cleanup_queue_security_guardrails() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_queue_guard_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let staging_dir = resolve_staging_directory(test_dir.join("dummy.txt"));
        fs::create_dir_all(&staging_dir).unwrap();

        let hist_mgr = crate::history::get_history_manager();

        // 1. Outside directory attack (path outside managed staging directory)
        let outside_file = test_dir.join(".tmp_stage_outside.txt");
        fs::write(&outside_file, b"Do not delete me!").unwrap();
        hist_mgr
            .queue_cleanup_path(&outside_file.to_string_lossy(), None)
            .unwrap();

        // 2. Disallowed filename pattern (inside staging dir, but not an allowed artifact pattern)
        let disallowed_file = staging_dir.join("important_user_file.txt");
        fs::write(&disallowed_file, b"User file inside staging dir").unwrap();
        hist_mgr
            .queue_cleanup_path(&disallowed_file.to_string_lossy(), None)
            .unwrap();

        // 3. Valid orphan artifact (inside staging dir, allowed pattern, not in active session)
        let orphan_file = staging_dir.join(format!(".tmp_stage_{}_valid.txt", Uuid::new_v4()));
        fs::write(&orphan_file, b"Safe to clean up").unwrap();
        hist_mgr
            .queue_cleanup_path(&orphan_file.to_string_lossy(), None)
            .unwrap();

        // Run cleanup with threshold 0
        cleanup_stale_staging_artifacts_with_threshold(0);

        // Security Guardrail 1: outside file must NOT be deleted
        assert!(
            outside_file.exists(),
            "File outside managed staging directory must not be deleted"
        );

        // Security Guardrail 2: disallowed filename pattern must NOT be deleted
        assert!(
            disallowed_file.exists(),
            "File without temporary pattern must not be deleted"
        );

        // Valid orphan file MUST be cleaned up
        assert!(
            !orphan_file.exists(),
            "Valid orphan artifact must be cleaned up"
        );

        // Queue check: invalid items must be retained with security error recorded
        let queue = hist_mgr.list_cleanup_queue().unwrap();
        let has_outside = queue.iter().any(|q| q.1 == outside_file.to_string_lossy());
        let has_disallowed = queue
            .iter()
            .any(|q| q.1 == disallowed_file.to_string_lossy());
        assert!(has_outside);
        assert!(has_disallowed);

        // Cleanup test files
        for q in queue {
            let _ = hist_mgr.remove_cleanup_queue_item(q.0);
        }
        let _ = fs::remove_file(&outside_file);
        let _ = fs::remove_file(&disallowed_file);
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_is_cuddle_staging_artifact_name_strict_uuid() {
        let u1 = Uuid::new_v4().to_string();
        let u2 = Uuid::new_v4().to_string();

        // 1. .tmp_stage_{UUID}_{display_name}
        assert!(is_cuddle_staging_artifact_name(&format!(
            ".tmp_stage_{}_file.txt",
            u1
        )));
        assert!(!is_cuddle_staging_artifact_name(
            ".tmp_stage_not-a-uuid_file.txt"
        ));
        assert!(!is_cuddle_staging_artifact_name(&format!(
            ".tmp_stage_{}_",
            u1
        )));
        assert!(!is_cuddle_staging_artifact_name(".tmp_stage_"));

        // 2. .cuddle_reflect_{UUID}_{UUID}.tmp
        assert!(is_cuddle_staging_artifact_name(&format!(
            ".cuddle_reflect_{}_{}.tmp",
            u1, u2
        )));
        assert!(!is_cuddle_staging_artifact_name(&format!(
            ".cuddle_reflect_{}.tmp",
            u1
        )));
        assert!(!is_cuddle_staging_artifact_name(&format!(
            ".cuddle_reflect_{}_{}_{}.tmp",
            u1, u2, u1
        )));
        assert!(!is_cuddle_staging_artifact_name(
            ".cuddle_reflect_not-a-uuid.tmp"
        ));
        assert!(!is_cuddle_staging_artifact_name(".cuddle_notes.txt"));
        assert!(!is_cuddle_staging_artifact_name(".cuddle_"));

        // 3. {prefix}.unstage_trash_{UUID}_{UUID} or unstage_trash_{UUID}_{UUID}
        assert!(is_cuddle_staging_artifact_name(&format!(
            "prefix.unstage_trash_{}_{}",
            u1, u2
        )));
        assert!(is_cuddle_staging_artifact_name(&format!(
            "unstage_trash_{}_{}",
            u1, u2
        )));
        assert!(!is_cuddle_staging_artifact_name(&format!(
            "unstage_trash_{}",
            u1
        )));
        assert!(!is_cuddle_staging_artifact_name(&format!(
            "unstage_trash_{}_{}_{}",
            u1, u2, u1
        )));
        assert!(!is_cuddle_staging_artifact_name("unstage_trash_not-a-uuid"));
        assert!(!is_cuddle_staging_artifact_name(
            "random_unstage_trash_file.txt"
        ));

        // 4. .probe_{UUID}
        assert!(is_cuddle_staging_artifact_name(&format!(".probe_{}", u1)));
        assert!(!is_cuddle_staging_artifact_name(".probe_test"));
        assert!(!is_cuddle_staging_artifact_name(".probe_"));

        // 5. .cuddle_edit_{UUID}.tmp & .cuddle_copy_{UUID}.tmp
        assert!(is_cuddle_staging_artifact_name(&format!(
            ".cuddle_edit_{}.tmp",
            u1
        )));
        assert!(is_cuddle_staging_artifact_name(&format!(
            ".cuddle_copy_{}.tmp",
            u1
        )));
        assert!(!is_cuddle_staging_artifact_name(".cuddle_edit_bad.tmp"));
        assert!(!is_cuddle_staging_artifact_name(".cuddle_copy_bad.tmp"));

        // Non-matching normal files
        assert!(!is_cuddle_staging_artifact_name("normal_document.txt"));
        assert!(!is_cuddle_staging_artifact_name(".bashrc"));
    }

    #[test]
    fn test_stage_file_commit_rename_failure_rolls_back_history() {
        let _guard = lock_test_mutex();
        use std::os::unix::fs::PermissionsExt;

        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_rename_fail_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("rename_fail.txt");
        fs::write(&orig_file, b"Content for rename failure test").unwrap();

        let staging_dir = resolve_staging_directory(&orig_file);
        fs::create_dir_all(&staging_dir).unwrap();

        // When stage_file creates the temporary copy, restrict permissions on staging_dir to 0o555,
        // so that the final rename to formal staged path fails with EACCES.
        let staging_dir_clone = staging_dir.clone();
        test_hooks::set_post_stage_clone_hook(move |_path| {
            let _ = fs::set_permissions(&staging_dir_clone, fs::Permissions::from_mode(0o555));
        });

        let mgr = WorkspaceManager::new();
        let stage_res = mgr.stage_file(&orig_file);
        test_hooks::clear_post_stage_clone_hook();

        // Restore permissions for inspection and cleanup
        let _ = fs::set_permissions(&staging_dir, fs::Permissions::from_mode(0o755));

        assert!(stage_res.is_err(), "stage_file must fail when rename fails");

        // Verify session item is deleted
        let hist_mgr = crate::history::get_history_manager();
        let session_items = hist_mgr.load_all_session_items().unwrap_or_default();
        let has_session = session_items
            .iter()
            .any(|item| item.display_name == "rename_fail.txt");
        assert!(!has_session, "Session item must be rolled back and deleted");

        // Verify history record is rolled back to Failed status with can_undo = 0
        let history = hist_mgr.list_history(50).unwrap_or_default();
        let history_entry = history.iter().find(|h| h.display_name == "rename_fail.txt");
        assert!(history_entry.is_some(), "History entry must exist");
        let entry = history_entry.unwrap();
        assert_eq!(
            entry.status, "Failed",
            "History entry status must be rolled back to Failed"
        );
        assert!(
            !entry.can_undo,
            "Rolled back history entry must not be undoable"
        );
        assert!(entry.details.is_some(), "Error details must be recorded");

        // Verify cleanup queue received the temporary file that failed to remove
        let queue = hist_mgr.list_cleanup_queue().unwrap_or_default();
        let has_queued = queue
            .iter()
            .any(|q| q.1.contains(".tmp_stage_") && q.1.contains("rename_fail.txt"));
        assert!(
            has_queued,
            "Temporary stage copy must be scheduled in cleanup queue"
        );

        // Cleanup
        let _ = fs::remove_dir_all(&test_dir);
        let _ = fs::remove_dir_all(&staging_dir);
    }

    #[test]
    fn test_write_staged_text_db_failure_marks_conflict_and_preserves_file() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_edit_fail_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("editable.txt");
        fs::write(&orig_file, b"Original text before edit").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        // Simulate SQLite failure during write_staged_text
        test_hooks::set_fail_stage_db(true);
        let edited_text = "Newly edited text that must be preserved on disk";
        let edit_res = mgr.write_staged_text(&staged.stage_id, edited_text);
        test_hooks::set_fail_stage_db(false);

        assert!(
            edit_res.is_err(),
            "write_staged_text must return error when DB fails"
        );

        // 1. Data Protection: Staged file on disk MUST have the edited content!
        let disk_content = fs::read_to_string(&staged.staged_path).unwrap();
        assert_eq!(
            disk_content, edited_text,
            "Physical file on disk must retain edits even when DB save fails"
        );

        // 2. Memory State: Staged item MUST be marked Conflict (NOT successfully Modified!)
        let staged_in_mem = mgr
            .list_staged()
            .into_iter()
            .find(|i| i.stage_id == staged.stage_id)
            .unwrap();
        assert_eq!(
            staged_in_mem.status,
            StageStatus::Conflict,
            "In-memory staged item must be marked Conflict on DB failure"
        );

        // 3. Auto-recovery on restart: restore_session detects unpersisted edits and recovers as Modified
        let restored = mgr.restore_session().unwrap();
        let restored_item = restored
            .iter()
            .find(|i| i.stage_id == staged.stage_id)
            .unwrap();
        assert_eq!(
            restored_item.status,
            StageStatus::Modified,
            "Session restore must auto-recover unpersisted edits on disk as Modified"
        );
        assert_eq!(restored_item.size, edited_text.len() as u64);

        let _ = mgr.unstage_file(&staged.stage_id);
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_reflect_staged_item_as_copy_db_failure_marks_conflict() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_copy_fail_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("copy_target.txt");
        fs::write(&orig_file, b"Original file data").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();

        // Simulate SQLite failure during reflect_staged_item_as_copy
        test_hooks::set_fail_stage_db(true);
        let reflect_res = mgr.reflect_staged_item_as_copy(&staged.stage_id, None);
        test_hooks::set_fail_stage_db(false);

        assert!(
            reflect_res.is_err(),
            "reflect_staged_item_as_copy must return Err on DB failure"
        );

        // Staged item in memory must be marked Conflict (NOT Reflected)
        let staged_in_mem = mgr
            .list_staged()
            .into_iter()
            .find(|i| i.stage_id == staged.stage_id)
            .unwrap();
        assert_eq!(
            staged_in_mem.status,
            StageStatus::Conflict,
            "In-memory item must be marked Conflict on DB failure"
        );

        // Alternate copy file exists on disk (user data preserved)
        let alternate_copy = test_dir.join("copy_target.edit.txt");
        assert!(
            alternate_copy.exists(),
            "Created alternate copy must not be deleted even on DB failure"
        );

        let _ = mgr.unstage_file(&staged.stage_id);
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_session_restore_quarantine_dir_read_failure_marks_conflict() {
        let _guard = lock_test_mutex();
        use std::os::unix::fs::PermissionsExt;

        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_dir_unreadable_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("dir_unreadable.txt");
        fs::write(&orig_file, b"Original V1").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();
        let hist_mgr = crate::history::get_history_manager();

        fs::write(&orig_file, b"Reflected Content V2").unwrap();
        fs::write(&staged.staged_path, b"Reflected Content V2").unwrap();

        let mut item = staged.clone();
        item.status = StageStatus::Applying;
        hist_mgr.save_session_item(&item).unwrap();

        // Restrict parent directory permissions to 0o000 so fs::read_dir(parent) fails with EACCES
        let _ = fs::set_permissions(&test_dir, fs::Permissions::from_mode(0o000));

        let restored = mgr.restore_session().unwrap();

        // Restore permissions for assertions and cleanup
        let _ = fs::set_permissions(&test_dir, fs::Permissions::from_mode(0o755));

        let restored_item = restored
            .iter()
            .find(|i| i.stage_id == staged.stage_id)
            .unwrap();

        // When fs::read_dir fails, quarantine check cannot be verified -> must fall back to Conflict!
        assert_eq!(
            restored_item.status,
            StageStatus::Conflict,
            "Failed parent dir read during WAL recovery must result in Conflict"
        );

        let _ = mgr.unstage_file(&staged.stage_id);
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_cleanup_queue_collects_local_staging_dir_when_session_items_empty() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_local_queue_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let local_staging = test_dir.join(".cuddle-temp");
        fs::create_dir_all(&local_staging).unwrap();

        let hist_mgr = crate::history::get_history_manager();

        // Ensure session_items is completely empty for this test
        let _ = hist_mgr.clear_session_items();

        // 1. Create a valid local staging orphan file (e.g. from DB save failure during stage_file)
        let orphan_id = Uuid::new_v4().to_string();
        let orphan_file = local_staging.join(format!(".tmp_stage_{}_orphan.txt", orphan_id));
        fs::write(&orphan_file, b"Orphaned temporary stage data").unwrap();

        // Register in cleanup_queue
        hist_mgr
            .queue_cleanup_path(&orphan_file.to_string_lossy(), None)
            .unwrap();

        // 2. Also register an unauthorized attack path in cleanup_queue (outside staging)
        let outside_file =
            test_dir.join(".tmp_stage_00000000-0000-0000-0000-000000000000_attack.txt");
        fs::write(&outside_file, b"Do not delete me outside!").unwrap();
        hist_mgr
            .queue_cleanup_path(&outside_file.to_string_lossy(), None)
            .unwrap();

        // 3. Verify get_known_staging_directories collects local_staging from cleanup_queue
        let known_dirs = hist_mgr.get_known_staging_directories().unwrap();
        let has_local_staging = known_dirs.iter().any(|d| d == &local_staging);
        assert!(
            has_local_staging,
            "Validated local .cuddle-temp from cleanup_queue must be collected even when session_items is empty"
        );

        // 4. Run cleanup
        cleanup_stale_staging_artifacts_with_threshold(0);

        // P2 CRITICAL DEFENSE & RECOVERY VERIFICATION:
        // The orphaned local staging file MUST be successfully deleted!
        assert!(
            !orphan_file.exists(),
            "Orphaned local staging file in cleanup_queue must be recovered and removed"
        );

        // Outside attack file MUST NOT be deleted!
        assert!(
            outside_file.exists(),
            "File outside managed staging directory must not be deleted"
        );

        // Cleanup
        let _ = fs::remove_file(&outside_file);
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_wal_recovery_test1_quarantine_sha256_match_preserves_quarantine_and_state() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_wal_test1_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("test1_doc.txt");
        let orig_initial_content = b"Original content pristine V1";
        fs::write(&orig_file, orig_initial_content).unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();
        let hist_mgr = crate::history::get_history_manager();

        // Simulate state where swap happened, orig_file matches staged copy
        let reflected_content = b"Reflected Content V2 from Staging";
        fs::write(&orig_file, reflected_content).unwrap();
        fs::write(&staged.staged_path, reflected_content).unwrap();

        // Quarantine file has exact SHA-256 matching original snapshot
        let quarantine_file = test_dir.join(format!(
            ".cuddle_reflect_{}_{}.tmp",
            staged.stage_id,
            Uuid::new_v4()
        ));
        fs::write(&quarantine_file, orig_initial_content).unwrap();

        let mut item = staged.clone();
        item.status = StageStatus::Applying;
        hist_mgr.save_session_item(&item).unwrap();

        // Execute recovery
        let restored = mgr.restore_session().unwrap();
        let restored_item = restored
            .iter()
            .find(|i| i.stage_id == staged.stage_id)
            .expect("Stage must be restored");

        // Test 1 Assertions:
        // 1. Quarantine file must NOT be deleted!
        assert!(
            quarantine_file.exists(),
            "Quarantine file must NOT be deleted even when SHA-256 matches pre-reflection snapshot"
        );
        // 2. Recovery state properly saved as Conflict for safety (or appropriate preserved state)
        assert_eq!(
            restored_item.status,
            StageStatus::Conflict,
            "Must be marked Conflict because external write to open FD cannot be ruled out"
        );
        // 3. Data is preserved
        assert_eq!(
            fs::read(&quarantine_file).unwrap(),
            orig_initial_content,
            "Quarantine data must be completely preserved without alteration"
        );

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_wal_recovery_test2_quarantine_sha256_mismatch_preserves_data_and_state() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_wal_test2_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("test2_doc.txt");
        fs::write(&orig_file, b"Original V1").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();
        let hist_mgr = crate::history::get_history_manager();

        // orig_file matches staged copy
        fs::write(&orig_file, b"Reflected Content V2").unwrap();
        fs::write(&staged.staged_path, b"Reflected Content V2").unwrap();

        // Quarantine file has content that DIFFERS from original snapshot (concurrent external write)
        let quarantine_file = test_dir.join(format!(
            ".cuddle_reflect_{}_{}.tmp",
            staged.stage_id,
            Uuid::new_v4()
        ));
        let external_write_payload = b"Original V1 with concurrent write modification!";
        fs::write(&quarantine_file, external_write_payload).unwrap();

        let mut item = staged.clone();
        item.status = StageStatus::Applying;
        hist_mgr.save_session_item(&item).unwrap();

        let restored = mgr.restore_session().unwrap();
        let restored_item = restored
            .iter()
            .find(|i| i.stage_id == staged.stage_id)
            .expect("Stage must be restored");

        // Test 2 Assertions:
        // 1. Data must NOT be deleted! Either kept as quarantine or preserved to .concurrent_write
        let has_quarantine = quarantine_file.exists();
        let concurrent_backups: Vec<_> = fs::read_dir(&test_dir)
            .unwrap()
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .contains(".concurrent_write")
            })
            .collect();

        assert!(
            has_quarantine || !concurrent_backups.is_empty(),
            "Quarantine or concurrent_write backup must be preserved"
        );

        let preserved_data = if has_quarantine {
            fs::read(&quarantine_file).unwrap()
        } else {
            fs::read(concurrent_backups[0].path()).unwrap()
        };
        assert_eq!(
            preserved_data, external_write_payload,
            "External write content must be preserved without loss"
        );

        // 2. Conflict or appropriate preserved state
        assert!(
            restored_item.status == StageStatus::Conflict
                || restored_item.status == StageStatus::Reflected,
            "Recovery state must be Conflict or appropriate preserved state"
        );

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_wal_recovery_test3_multiple_quarantines_retains_all_and_marks_conflict() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_wal_test3_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("test3_doc.txt");
        fs::write(&orig_file, b"Original V1").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();
        let hist_mgr = crate::history::get_history_manager();

        fs::write(&orig_file, b"Reflected Content V2").unwrap();
        fs::write(&staged.staged_path, b"Reflected Content V2").unwrap();

        // 3 separate quarantine files for same stage_id
        let q1 = test_dir.join(format!(
            ".cuddle_reflect_{}_{}.tmp",
            staged.stage_id,
            Uuid::new_v4()
        ));
        fs::write(&q1, b"Payload Q1").unwrap();

        let q2 = test_dir.join(format!(
            ".cuddle_reflect_{}_{}.tmp",
            staged.stage_id,
            Uuid::new_v4()
        ));
        fs::write(&q2, b"Payload Q2").unwrap();

        let q3 = test_dir.join(format!(
            ".cuddle_reflect_{}_{}.tmp",
            staged.stage_id,
            Uuid::new_v4()
        ));
        fs::write(&q3, b"Payload Q3").unwrap();

        let mut item = staged.clone();
        item.status = StageStatus::Applying;
        hist_mgr.save_session_item(&item).unwrap();

        let restored = mgr.restore_session().unwrap();
        let restored_item = restored
            .iter()
            .find(|i| i.stage_id == staged.stage_id)
            .expect("Stage must be restored");

        // Test 3 Assertions:
        // 1. None of the files deleted
        assert!(q1.exists(), "Q1 must not be deleted");
        assert!(q2.exists(), "Q2 must not be deleted");
        assert!(q3.exists(), "Q3 must not be deleted");

        // 2. None overwritten
        assert_eq!(fs::read(&q1).unwrap(), b"Payload Q1");
        assert_eq!(fs::read(&q2).unwrap(), b"Payload Q2");
        assert_eq!(fs::read(&q3).unwrap(), b"Payload Q3");

        // 3. Ambiguous state treated as Conflict
        assert_eq!(restored_item.status, StageStatus::Conflict);

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_wal_recovery_test4_sqlite_failure_retains_quarantine_and_allows_retry() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_wal_test4_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("test4_doc.txt");
        fs::write(&orig_file, b"Original V1").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();
        let hist_mgr = crate::history::get_history_manager();

        fs::write(&orig_file, b"Reflected Content V2").unwrap();
        fs::write(&staged.staged_path, b"Reflected Content V2").unwrap();

        let quarantine_file = test_dir.join(format!(
            ".cuddle_reflect_{}_{}.tmp",
            staged.stage_id,
            Uuid::new_v4()
        ));
        fs::write(&quarantine_file, b"Quarantine content V1").unwrap();

        let mut item = staged.clone();
        item.status = StageStatus::Applying;
        hist_mgr.save_session_item(&item).unwrap();

        // 1. Simulate SQLite persistence failure during restore
        test_hooks::set_fail_restore_db(true);
        let _ = mgr.restore_session();
        test_hooks::set_fail_restore_db(false);

        // Quarantine must NOT be deleted
        assert!(
            quarantine_file.exists(),
            "Quarantine file must be preserved on SQLite save failure"
        );

        // SQLite DB must still record Applying (not completed)
        let loaded = hist_mgr.load_all_session_items().unwrap();
        let db_item = loaded
            .iter()
            .find(|i| i.stage_id == staged.stage_id)
            .expect("Item must still exist in DB");
        assert_eq!(
            db_item.status,
            StageStatus::Applying,
            "DB item status must remain Applying so retry is possible"
        );

        // 2. Retry restore with DB operational -> succeeds
        let restored_retry = mgr.restore_session().unwrap();
        let retry_item = restored_retry
            .iter()
            .find(|i| i.stage_id == staged.stage_id)
            .expect("Item must be restored on retry");
        assert!(
            retry_item.status == StageStatus::Conflict
                || retry_item.status == StageStatus::Reflected,
            "Retry item status must be Conflict or Reflected"
        );
        let has_quarantine_or_backup = quarantine_file.exists()
            || fs::read_dir(&test_dir).unwrap().flatten().any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .contains(".concurrent_write")
            });
        assert!(
            has_quarantine_or_backup,
            "Quarantine data must be preserved on retry"
        );

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_wal_recovery_test5_cleanup_queue_conflict_retains_quarantine() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_wal_test5_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let local_staging = test_dir.join(".cuddle-temp");
        fs::create_dir_all(&local_staging).unwrap();

        let orig_file = test_dir.join("test5_doc.txt");
        fs::write(&orig_file, b"Original V1").unwrap();

        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();
        let hist_mgr = crate::history::get_history_manager();

        // Quarantine file located in local staging
        let quarantine_file = local_staging.join(format!(
            ".cuddle_reflect_{}_{}.tmp",
            staged.stage_id,
            Uuid::new_v4()
        ));
        fs::write(&quarantine_file, b"Quarantine data needing protection").unwrap();

        // Item is registered in Conflict status
        let mut item = staged.clone();
        item.status = StageStatus::Conflict;
        hist_mgr.save_session_item(&item).unwrap();

        // Register quarantine path in cleanup_queue
        hist_mgr
            .queue_cleanup_path(&quarantine_file.to_string_lossy(), None)
            .unwrap();

        // Execute cleanup with 0 threshold (immediate stale cleanup)
        cleanup_stale_staging_artifacts_with_threshold(0);

        // Guardrail Assertions:
        // Quarantine file must NOT be deleted because it is associated with a Conflict session item!
        assert!(
            quarantine_file.exists(),
            "Quarantine file must NOT be deleted by cleanup_queue when associated with Conflict session item"
        );

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_wal_recovery_test6_db_read_failure_during_cleanup_aborts_and_protects_files() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_wal_test6_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let local_staging = test_dir.join(".cuddle-temp");
        fs::create_dir_all(&local_staging).unwrap();

        // Create a staging artifact in local staging
        let stage_artifact = local_staging.join(format!(
            ".cuddle_reflect_{}_{}.tmp",
            Uuid::new_v4(),
            Uuid::new_v4()
        ));
        fs::write(&stage_artifact, b"Artifact data to protect").unwrap();

        // Simulate DB read failure in load_all_session_items()
        test_hooks::set_fail_load_session_db(true);

        cleanup_stale_staging_artifacts_with_threshold(0);
        cleanup_stale_staging_artifacts_with_dir_and_threshold(&local_staging, 0);

        test_hooks::set_fail_load_session_db(false);

        // Cleanup must abort and NOT delete staging artifact!
        assert!(
            stage_artifact.exists(),
            "Staging artifact must NOT be deleted when database read fails during cleanup maintenance"
        );

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_wal_recovery_test7_crash_injection_at_critical_points() {
        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_wal_test7_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_payload = b"Original content at crash injection start";
        let staged_payload = b"Staged candidate content for crash injection";

        // Crash Point 1: Immediately after Applying saved in SQLite (before physical exchange)
        {
            let orig_file1 = test_dir.join("crash_point1.txt");
            fs::write(&orig_file1, orig_payload).unwrap();

            let mgr = WorkspaceManager::new();
            let staged1 = mgr.stage_file(&orig_file1).unwrap();
            fs::write(&staged1.staged_path, staged_payload).unwrap();

            // Candidate tmp file in orig directory exists (pre-swap copy)
            let tmp_candidate = test_dir.join(format!(
                ".cuddle_reflect_{}_{}.tmp",
                staged1.stage_id,
                Uuid::new_v4()
            ));
            fs::write(&tmp_candidate, staged_payload).unwrap();

            let mut item1 = staged1.clone();
            item1.status = StageStatus::Applying;
            let hist_mgr = crate::history::get_history_manager();
            hist_mgr.save_session_item(&item1).unwrap();

            // Restore after Point 1 crash
            let restored = mgr.restore_session().unwrap();
            let rec_item1 = restored
                .iter()
                .find(|i| i.stage_id == staged1.stage_id)
                .unwrap();
            // CASE B with quarantine file: safety cannot be proven -> Conflict, all files retained!
            assert_eq!(rec_item1.status, StageStatus::Conflict);
            assert!(tmp_candidate.exists(), "Tmp file must not be deleted");
            assert_eq!(fs::read(&orig_file1).unwrap(), orig_payload);
        }

        // Crash Point 2: Immediately after RENAME_EXCHANGE (swap succeeded, before inspection)
        {
            let orig_file2 = test_dir.join("crash_point2.txt");
            fs::write(&orig_file2, orig_payload).unwrap();

            let mgr = WorkspaceManager::new();
            let staged2 = mgr.stage_file(&orig_file2).unwrap();
            fs::write(&staged2.staged_path, staged_payload).unwrap();

            // Orig file now has staged_payload, quarantine file has orig_payload
            fs::write(&orig_file2, staged_payload).unwrap();
            let quarantine2 = test_dir.join(format!(
                ".cuddle_reflect_{}_{}.tmp",
                staged2.stage_id,
                Uuid::new_v4()
            ));
            fs::write(&quarantine2, orig_payload).unwrap();

            let mut item2 = staged2.clone();
            item2.status = StageStatus::Applying;
            let hist_mgr = crate::history::get_history_manager();
            hist_mgr.save_session_item(&item2).unwrap();

            // Restore after Point 2 crash
            let restored = mgr.restore_session().unwrap();
            let rec_item2 = restored
                .iter()
                .find(|i| i.stage_id == staged2.stage_id)
                .unwrap();
            assert_eq!(rec_item2.status, StageStatus::Conflict);
            assert!(quarantine2.exists(), "Quarantine file must not be deleted");
            assert_eq!(fs::read(&quarantine2).unwrap(), orig_payload);
        }

        // Crash Point 3: Immediately after quarantine SHA-256 calculation
        {
            let orig_file3 = test_dir.join("crash_point3.txt");
            fs::write(&orig_file3, orig_payload).unwrap();

            let mgr = WorkspaceManager::new();
            let staged3 = mgr.stage_file(&orig_file3).unwrap();
            fs::write(&staged3.staged_path, staged_payload).unwrap();

            fs::write(&orig_file3, staged_payload).unwrap();
            let quarantine3 = test_dir.join(format!(
                ".cuddle_reflect_{}_{}.tmp",
                staged3.stage_id,
                Uuid::new_v4()
            ));
            fs::write(&quarantine3, orig_payload).unwrap();

            let mut item3 = staged3.clone();
            item3.status = StageStatus::Applying;
            let hist_mgr = crate::history::get_history_manager();
            hist_mgr.save_session_item(&item3).unwrap();

            let restored = mgr.restore_session().unwrap();
            let rec_item3 = restored
                .iter()
                .find(|i| i.stage_id == staged3.stage_id)
                .unwrap();
            assert_eq!(rec_item3.status, StageStatus::Conflict);
            assert!(quarantine3.exists());
        }

        // Crash Point 4: Immediately after SQLite recovery status update to Conflict
        {
            let orig_file4 = test_dir.join("crash_point4.txt");
            fs::write(&orig_file4, orig_payload).unwrap();

            let mgr = WorkspaceManager::new();
            let staged4 = mgr.stage_file(&orig_file4).unwrap();
            fs::write(&staged4.staged_path, staged_payload).unwrap();

            fs::write(&orig_file4, staged_payload).unwrap();
            let quarantine4 = test_dir.join(format!(
                ".cuddle_reflect_{}_{}.tmp",
                staged4.stage_id,
                Uuid::new_v4()
            ));
            fs::write(&quarantine4, orig_payload).unwrap();

            let mut item4 = staged4.clone();
            item4.status = StageStatus::Conflict;
            let hist_mgr = crate::history::get_history_manager();
            hist_mgr.save_session_item(&item4).unwrap();

            let restored = mgr.restore_session().unwrap();
            let rec_item4 = restored
                .iter()
                .find(|i| i.stage_id == staged4.stage_id)
                .unwrap();
            assert_eq!(rec_item4.status, StageStatus::Conflict);
            assert!(quarantine4.exists());
        }

        // Crash Point 5: Immediately after quarantine preservation
        {
            let orig_file5 = test_dir.join("crash_point5.txt");
            fs::write(&orig_file5, orig_payload).unwrap();

            let mgr = WorkspaceManager::new();
            let staged5 = mgr.stage_file(&orig_file5).unwrap();
            fs::write(&staged5.staged_path, staged_payload).unwrap();

            fs::write(&orig_file5, staged_payload).unwrap();
            let quarantine5 = test_dir.join(format!(
                ".cuddle_reflect_{}_{}.tmp",
                staged5.stage_id,
                Uuid::new_v4()
            ));
            fs::write(&quarantine5, orig_payload).unwrap();

            let mut item5 = staged5.clone();
            item5.status = StageStatus::Conflict;
            let hist_mgr = crate::history::get_history_manager();
            hist_mgr.save_session_item(&item5).unwrap();

            let restored = mgr.restore_session().unwrap();
            let rec_item5 = restored
                .iter()
                .find(|i| i.stage_id == staged5.stage_id)
                .unwrap();
            assert_eq!(rec_item5.status, StageStatus::Conflict);
            assert!(quarantine5.exists());
        }

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_wal_recovery_test8_external_open_fd_write_protection_linux() {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;

        let _guard = lock_test_mutex();
        let home = dirs::home_dir().expect("Home must exist");
        let test_dir = home.join(format!(".cuddle_test_wal_test8_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();

        let orig_file = test_dir.join("open_fd_test.txt");
        let initial_data = b"Initial original data before external process opens FD";
        fs::write(&orig_file, initial_data).unwrap();

        // 1. External application opens original file and holds open file descriptor
        let mut open_fd = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&orig_file)
            .expect("Failed to open file descriptor for external process");

        // 2. Stage the file
        let mgr = WorkspaceManager::new();
        let staged = mgr.stage_file(&orig_file).unwrap();
        let hist_mgr = crate::history::get_history_manager();

        // 3. Cuddle simulates atomic_exchange_files:
        // Staged content moves to orig_file.
        // Original inode moves to quarantine_file.
        let reflected_content = b"New reflected content from Cuddle staging candidate";
        let quarantine_file = test_dir.join(format!(
            ".cuddle_reflect_{}_{}.tmp",
            staged.stage_id,
            Uuid::new_v4()
        ));

        // Create quarantine file with identical content first, then atomic exchange
        fs::write(&quarantine_file, reflected_content).unwrap();
        atomic_exchange_files(&quarantine_file, &orig_file).expect("Atomic swap failed");

        // Now orig_file has reflected_content!
        // quarantine_file now holds the original inode that open_fd is pointing to!
        fs::write(&staged.staged_path, reflected_content).unwrap();

        // 4. External process writes to its open file descriptor!
        // Under POSIX semantics, this writes directly to the quarantined inode at quarantine_file!
        let concurrent_external_write = b"CRITICAL CONCURRENT WRITE FROM OPEN UNIX FD AFTER SWAP!";
        open_fd
            .write_all(concurrent_external_write)
            .expect("Writing to open FD must succeed");
        open_fd.flush().expect("Flushing open FD must succeed");

        // 5. Cuddle simulates crash in Applying WAL state
        let mut item = staged.clone();
        item.status = StageStatus::Applying;
        hist_mgr.save_session_item(&item).unwrap();

        // 6. Execute session restore (WAL recovery)
        let restored = mgr.restore_session().unwrap();
        let restored_item = restored
            .iter()
            .find(|i| i.stage_id == staged.stage_id)
            .expect("Stage must be restored");

        // Close external FD now that recovery has executed
        drop(open_fd);

        // Test 8 Assertions:
        // 1. External FD writes must NOT be lost!
        let has_quarantine = quarantine_file.exists();
        let backups: Vec<_> = fs::read_dir(&test_dir)
            .unwrap()
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .contains(".concurrent_write")
            })
            .collect();

        assert!(
            has_quarantine || !backups.is_empty(),
            "Quarantine file or preserved concurrent write must exist"
        );

        let preserved_bytes = if has_quarantine {
            fs::read(&quarantine_file).unwrap()
        } else {
            fs::read(backups[0].path()).unwrap()
        };

        assert!(
            preserved_bytes
                .windows(concurrent_external_write.len())
                .any(|w| w == concurrent_external_write),
            "Preserved file must contain the exact data written via open Unix file descriptor"
        );

        // 2. Recovery state must be safely preserved as Conflict (or Reflected with preserved backup)
        assert!(
            restored_item.status == StageStatus::Conflict
                || restored_item.status == StageStatus::Reflected,
            "Item status must be Conflict or Reflected with preserved backup"
        );

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_export_typescript_bindings_workspace() {
        let _ = fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        StagedItemDto::export(&cfg).expect("Failed to export StagedItemDto");
        CloneMethod::export(&cfg).expect("Failed to export CloneMethod");
        StageStatus::export(&cfg).expect("Failed to export StageStatus");
        ReflectionMode::export(&cfg).expect("Failed to export ReflectionMode");
    }
}
