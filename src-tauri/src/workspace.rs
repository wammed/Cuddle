use crate::models::{decode_path_from_base64, encode_path_to_base64, FileItem, ModelError};
use crate::policy::{
    canonicalize_path, check_disk_space, is_external_or_mount_boundary, OperationPolicy,
    OriginalSnapshotDto, PolicyError,
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
}

impl WorkspaceError {
    pub fn error_code(&self) -> &'static str {
        match self {
            WorkspaceError::Policy(p) => p.error_code(),
            WorkspaceError::NotFound(_) => "ERR_STAGE_NOT_FOUND",
            WorkspaceError::StagingDirUnavailable => "ERR_STAGING_DIR_UNAVAILABLE",
            WorkspaceError::InsufficientSpace => "ERR_INSUFFICIENT_SPACE",
            WorkspaceError::RollbackFailed { .. } => "ERR_UNSTAGE_ROLLBACK_FAILED",
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
    use std::cell::RefCell;
    use std::path::Path;

    pub type HookFn = Box<dyn Fn(&Path)>;

    thread_local! {
        static POST_SWAP_HOOK: RefCell<Option<HookFn>> = const { RefCell::new(None) };
        static POST_VERIFY_HOOK: RefCell<Option<HookFn>> = const { RefCell::new(None) };
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

        // 7. Record operation in SQLite persistent history & save session
        let hist_mgr = crate::history::get_history_manager();
        hist_mgr
            .record_operation(&crate::history::NewHistoryEntry {
                operation_id: stage_id,
                operation_type: "Stage".to_string(),
                display_name: item.display_name.clone(),
                original_path: Some(canonical_src.to_string_lossy().to_string()),
                staged_path: Some(staged_dto.staged_path.clone()),
                inode: Some(staged_dto.original_snapshot.inode as i64),
                status: "Success".to_string(),
                can_undo: true,
                details: Some(format!("Cloned via {:?}", staged_dto.clone_method)),
            })
            .map_err(|e| WorkspaceError::Io(std::io::Error::other(e)))?;
        hist_mgr
            .save_session_item(&staged_dto)
            .map_err(|e| WorkspaceError::Io(std::io::Error::other(e)))?;

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

        // Temporary file in same directory for atomic POSIX rename
        let tmp_reflect_name = format!(".cuddle_reflect_{}.tmp", Uuid::new_v4());
        let tmp_reflect_path = orig_parent.join(tmp_reflect_name);

        // Copy staged content to temporary file in destination volume
        if let Err(e) = fs::copy(&staged_path, &tmp_reflect_path) {
            let _ = fs::remove_file(&tmp_reflect_path);
            return Err(WorkspaceError::Io(e));
        }

        // --- TOCTOU Prevention: Atomic Exchange & Quarantined Verification ---
        // Atomically swap candidate into place and quarantine the original file in tmp_reflect_path.
        // Then verify the quarantined original file in isolation against the snapshot.
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
                                let _ = atomic_exchange_files(&tmp_reflect_path, &orig_path);
                                let _ = fs::remove_file(&tmp_reflect_path);
                                return Err(WorkspaceError::Io(e));
                            }
                            backup_origin_path = Some(backup_target);
                        } else {
                            // Mode A (Overwrite):
                            // To protect against race conditions where an external process writes via an open FD
                            // immediately before or during unlink, trigger verification hook and perform final check:
                            #[cfg(test)]
                            test_hooks::trigger_post_verify_hook(&tmp_reflect_path);

                            let final_check = OperationPolicy::verify_conflict_on_path(
                                &tmp_reflect_path,
                                &staged.original_snapshot,
                                true,
                            );

                            if final_check.is_err() {
                                let lost_write_target = generate_lost_write_backup_path(&orig_path);
                                if let Err(e) = fs::rename(&tmp_reflect_path, &lost_write_target) {
                                    eprintln!(
                                        "[CRITICAL] Failed to preserve last-second external write '{}': {}",
                                        lost_write_target.display(),
                                        e
                                    );
                                    return Err(WorkspaceError::LostWritePreserveFailed {
                                        tmp_path: tmp_reflect_path.clone(),
                                        lost_write_path: lost_write_target.clone(),
                                        cause: format!("Failed to preserve last-second concurrent write before removal: {}", e),
                                    });
                                }
                                eprintln!(
                                    "[SECURITY] Concurrent external write detected right before removal and preserved to '{}'",
                                    lost_write_target.display()
                                );
                                concurrent_write_path = Some(lost_write_target);
                            } else {
                                // Safely remove the quarantined old original
                                let _ = fs::remove_file(&tmp_reflect_path);
                            }
                        }
                    }
                    Err(conflict_err) => {
                        // CONFLICT DETECTED!
                        // An external modification occurred prior to the swap.
                        // Rollback immediately: swap the original file back into place!
                        let _ = atomic_exchange_files(&tmp_reflect_path, &orig_path);
                        let _ = fs::remove_file(&tmp_reflect_path);
                        staged.status = StageStatus::Conflict;
                        let hist_mgr = crate::history::get_history_manager();
                        let _ = hist_mgr.save_session_item(staged);
                        return Err(WorkspaceError::Policy(conflict_err));
                    }
                }
            }
            Err(swap_err) => {
                // Remove temporary staging candidate
                let _ = fs::remove_file(&tmp_reflect_path);

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

        // 4. Update status to Reflected
        staged.status = StageStatus::Reflected;

        let details = format!(
            "Saved alternate copy: {} (original {} preserved untouched)",
            target_copy_path.display(),
            orig_path.display()
        );

        let hist_mgr = crate::history::get_history_manager();
        hist_mgr
            .save_session_item(staged)
            .map_err(|e| WorkspaceError::Io(std::io::Error::other(e)))?;
        hist_mgr
            .record_operation(&crate::history::NewHistoryEntry {
                operation_id: uuid::Uuid::new_v4().to_string(),
                operation_type: "ReflectAsCopy".to_string(),
                display_name: staged.display_name.clone(),
                original_path: Some(staged.original_snapshot.canonical_path.clone()),
                staged_path: Some(target_copy_path.to_string_lossy().to_string()),
                inode: None,
                status: "Success".to_string(),
                can_undo: false,
                details: Some(details),
            })
            .map_err(|e| WorkspaceError::Io(std::io::Error::other(e)))?;

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
        staged.size = metadata.len();
        staged.status = StageStatus::Modified;

        let details = format!(
            "Edited staged copy '{}' ({} bytes)",
            staged.display_name, staged.size
        );

        // 3. Persist to SQLite session & history
        let hist_mgr = crate::history::get_history_manager();
        hist_mgr
            .save_session_item(staged)
            .map_err(|e| WorkspaceError::Io(std::io::Error::other(e)))?;
        hist_mgr
            .record_operation(&crate::history::NewHistoryEntry {
                operation_id: Uuid::new_v4().to_string(),
                operation_type: "EditStaged".to_string(),
                display_name: staged.display_name.clone(),
                original_path: Some(staged.original_snapshot.canonical_path.clone()),
                staged_path: Some(staged.staged_path.clone()),
                inode: None,
                status: "Success".to_string(),
                can_undo: false,
                details: Some(details),
            })
            .map_err(|e| WorkspaceError::Io(std::io::Error::other(e)))?;

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
                // Check if an interrupted unstage rollback or quarantined trash file exists
                let mut recovered = false;
                if let Some(parent) = staged_path.parent() {
                    if let Ok(entries) = fs::read_dir(parent) {
                        let target_marker = format!("unstage_trash_{}", item.stage_id);
                        for entry in entries.flatten() {
                            let entry_name = entry.file_name().to_string_lossy().to_string();
                            if entry_name.contains(&target_marker)
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
                    let _ = hist_mgr.save_session_item(&item);
                    map.insert(item.stage_id.clone(), item.clone());
                    restored.push(item);
                    continue;
                }
            }

            // Verify original file and detect conflict
            let orig_path = Path::new(&item.original_snapshot.canonical_path);
            if !orig_path.exists() {
                item.status = StageStatus::Conflict;
                let _ = hist_mgr.save_session_item(&item);
            } else {
                match OperationPolicy::verify_conflict(&item.original_snapshot, true) {
                    Ok(_) => {}
                    Err(PolicyError::ConflictDetected(_)) => {
                        // External modification detected! Mark as Conflict per user decision
                        item.status = StageStatus::Conflict;
                        let _ = hist_mgr.save_session_item(&item);
                    }
                    Err(e) => {
                        // Permission denied or IO error: do not assume Clean! Mark as Conflict for safety
                        eprintln!(
                            "Warning: Error verifying conflict during session restore for '{}': {}",
                            orig_path.display(),
                            e
                        );
                        item.status = StageStatus::Conflict;
                        let _ = hist_mgr.save_session_item(&item);
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

/// Cleans up stale unstage trash and temporary files from the staging directory.
pub fn cleanup_stale_staging_artifacts() {
    let global_staging = get_global_staging_directory();
    if !global_staging.exists() {
        return;
    }

    if let Ok(entries) = fs::read_dir(&global_staging) {
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.contains("unstage_trash_") || fname.starts_with(".tmp_") {
                let _ = fs::remove_file(entry.path());
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
    fn test_workspace_session_restore_and_conflict_detection() {
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
        assert!(
            lost_bytes.contains(&b"Concurrent external write through open FD post-swap"[0]),
            "Preserved backup file must contain the concurrent external FD writes"
        );

        // Original file must have the staged content V2
        let orig_bytes = fs::read(&orig_file).unwrap();
        assert_eq!(orig_bytes, b"Staged reflected content V2");

        drop(external_fd);
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_reflect_concurrent_write_before_unlink_protection() {
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
        assert!(lost_bytes.contains(&b"Last-millisecond write before unlink"[0]));

        drop(external_fd);
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_workspace_unstage_recovery_preserves_extension_and_restores_by_stage_id() {
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

        // Hook simulates concurrent write and then unlinks tmp_reflect_path to force rename failure
        let fd_clone = Arc::clone(&external_fd);
        test_hooks::set_post_swap_hook(move |quarantined_path| {
            let mut fd = fd_clone.lock().unwrap();
            fd.write_all(b"Concurrent write").unwrap();
            fd.flush().unwrap();
            // Remove quarantined file behind the scenes to trigger rename error
            let _ = fs::remove_file(quarantined_path);
        });

        let reflect_res = mgr.reflect_staged_file(&staged.stage_id);
        test_hooks::clear_post_swap_hook();

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

        drop(external_fd);
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
