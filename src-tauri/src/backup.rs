use crate::workspace::StagedItemDto;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Read};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tar::{Builder, Header};
use ts_rs::TS;
use uuid::Uuid;

/// Safety limits for workspace backup archive processing
pub const MAX_BACKUP_ENTRIES: usize = 10_000;
pub const MAX_MANIFEST_BYTES: u64 = 10 * 1024 * 1024; // 10 MiB
pub const MAX_SINGLE_FILE_BYTES: u64 = 1024 * 1024 * 1024; // 1 GiB
pub const MAX_TOTAL_EXTRACTED_BYTES: u64 = 5 * 1024 * 1024 * 1024; // 5 GiB
pub const MAX_COMPRESSION_RATIO: u64 = 100; // 100x decompression bomb threshold

/// DTO representing the result of a workspace backup operation
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct BackupResultDto {
    pub archive_path: String,
    pub file_count: usize,
    pub total_bytes: u64,
    pub timestamp_ms: i64,
}

/// Information about a single workspace backup archive
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct BackupArchiveInfoDto {
    pub filename: String,
    pub full_path: String,
    pub created_at_ms: i64,
    pub size_bytes: u64,
    pub file_count: Option<usize>,
}

/// Returns the persistent backups storage directory: `$XDG_DATA_HOME/Cuddle/backups`
pub fn get_backups_dir() -> PathBuf {
    dirs::data_dir()
        .map(|p| p.join("Cuddle").join("backups"))
        .unwrap_or_else(|| PathBuf::from("./backups"))
}

/// Generates a timestamped default backup path with millisecond precision:
/// `$XDG_DATA_HOME/Cuddle/backups/backup_YYYYMMDD_HHMMSS_mmm.tar.gz`
pub fn get_default_backup_path() -> Result<String, String> {
    let base_dir = get_backups_dir();

    let now = chrono::Local::now();
    let filename = format!(
        "backup_{}_{:03}.tar.gz",
        now.format("%Y%m%d_%H%M%S"),
        now.timestamp_subsec_millis()
    );
    let full_path = base_dir.join(filename);
    Ok(full_path.to_string_lossy().to_string())
}

/// Lists all workspace backup archives in $XDG_DATA_HOME/Cuddle/backups/
/// Sorted by creation timestamp descending (newest first).
pub fn list_backups() -> Result<Vec<BackupArchiveInfoDto>, String> {
    let backups_dir = get_backups_dir();
    if !backups_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&backups_dir).map_err(|e| {
        format!(
            "Failed to read backups directory {}: {}",
            backups_dir.display(),
            e
        )
    })?;

    let mut list = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
            if filename.starts_with("backup_")
                && (filename.ends_with(".tar.gz") || filename.ends_with(".tar"))
            {
                if let Ok(meta) = path.metadata() {
                    let mtime_ms = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_millis() as i64)
                        .unwrap_or(0);

                    let file_count = count_files_in_archive(&path);

                    list.push(BackupArchiveInfoDto {
                        filename: filename.to_string(),
                        full_path: path.to_string_lossy().to_string(),
                        created_at_ms: mtime_ms,
                        size_bytes: meta.len(),
                        file_count,
                    });
                }
            }
        }
    }

    // Sort descending by created_at_ms
    list.sort_by_key(|a| std::cmp::Reverse(a.created_at_ms));
    Ok(list)
}

fn count_files_in_archive(path: &Path) -> Option<usize> {
    let file = File::open(path).ok()?;
    if path.to_string_lossy().ends_with(".tar.gz") {
        let gz = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(gz);
        let mut count = 0;
        if let Ok(entries) = archive.entries() {
            for entry in entries.flatten() {
                if let Ok(p) = entry.path() {
                    let p_str = p.to_string_lossy();
                    if p_str.starts_with("files/") {
                        count += 1;
                    }
                }
            }
        }
        Some(count)
    } else {
        None
    }
}

/// Deletes a specific backup file safely (prevents directory traversal attacks).
pub fn delete_backup(filename: &str) -> Result<(), String> {
    let clean = filename.trim();
    if clean.is_empty() || clean.contains('/') || clean.contains('\\') || clean.contains("..") {
        return Err("Invalid backup filename".to_string());
    }
    if !clean.starts_with("backup_") || (!clean.ends_with(".tar.gz") && !clean.ends_with(".tar")) {
        return Err("Invalid backup file extension".to_string());
    }

    let file_path = get_backups_dir().join(clean);
    if !file_path.exists() {
        return Err(format!("Backup archive '{}' does not exist", clean));
    }

    fs::remove_file(&file_path)
        .map_err(|e| format!("Failed to delete backup '{}': {}", clean, e))?;
    Ok(())
}

/// Prunes old backups keeping only the most recent `keep_count` archives.
/// Returns a list of filenames that were deleted.
pub fn prune_old_backups(keep_count: usize) -> Result<Vec<String>, String> {
    let backups = list_backups()?;
    let mut deleted = Vec::new();

    if backups.len() > keep_count {
        for old in &backups[keep_count..] {
            if let Ok(()) = delete_backup(&old.filename) {
                deleted.push(old.filename.clone());
            }
        }
    }

    Ok(deleted)
}

/// Validates a single manifest item for path traversal, UUID format, and security constraints.
pub fn validate_manifest_item(
    item: &StagedItemDto,
    global_staging: &Path,
) -> Result<PathBuf, String> {
    // 1. stage_id must be a valid UUID v4
    Uuid::parse_str(&item.stage_id).map_err(|_| {
        format!(
            "Invalid stage_id in manifest (must be valid UUID format): '{}'",
            item.stage_id
        )
    })?;

    // 2. display_name must be a pure filename without path separators or traversal elements
    let name = item.display_name.trim();
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
        || name.contains("..")
    {
        return Err(format!(
            "Invalid or unsafe display_name in manifest: '{}'",
            item.display_name
        ));
    }

    let file_name_check = Path::new(name).file_name().and_then(|f| f.to_str());
    if file_name_check != Some(name) {
        return Err(format!(
            "display_name contains path separator components: '{}'",
            item.display_name
        ));
    }

    // 3. Destination filename and path
    let dest_filename = format!("{}_{}", item.stage_id, name);
    let dest_path = global_staging.join(&dest_filename);

    // 4. Ensure destination is strictly inside global_staging directory
    if !dest_path.starts_with(global_staging) || dest_path.parent() != Some(global_staging) {
        return Err(format!(
            "Path traversal attempt detected in manifest for item '{}': '{}'",
            item.display_name,
            dest_path.display()
        ));
    }

    Ok(dest_path)
}

/// Creates a tar.gz backup archive of all active staged working copies and includes session manifest.json.
/// If saving to default directory, prunes archives exceeding `auto_prune_limit`.
pub fn create_workspace_backup(
    destination: Option<String>,
    staged_items: &[StagedItemDto],
    auto_prune_limit: Option<usize>,
) -> Result<BackupResultDto, String> {
    if staged_items.is_empty() {
        return Err("No staged working copies to backup".to_string());
    }

    let archive_path_buf = match destination {
        Some(ref custom) if !custom.trim().is_empty() => PathBuf::from(custom),
        _ => PathBuf::from(get_default_backup_path()?),
    };

    if archive_path_buf.exists() {
        return Err(format!(
            "Target backup archive already exists: {}",
            archive_path_buf.display()
        ));
    }

    if let Some(parent) = archive_path_buf.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create backup directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    // Write to a temporary file first to prevent partial/corrupted archives on crash
    let tmp_archive_path =
        archive_path_buf.with_extension(format!("tmp_{}.tar.gz", Uuid::new_v4()));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp_archive_path)
        .map_err(|e| {
            format!(
                "Failed to create temporary backup file {}: {}",
                tmp_archive_path.display(),
                e
            )
        })?;

    let enc = GzEncoder::new(BufWriter::new(file), Compression::default());
    let mut builder = Builder::new(enc);

    let mut archived_count = 0;

    // 1. Append staged files to archive under `files/<stage_id>` to prevent name collisions
    for item in staged_items {
        let staged_path = Path::new(&item.staged_path);
        if staged_path.exists() {
            let entry_name = format!("files/{}", item.stage_id);
            let mut file_to_archive = File::open(staged_path).map_err(|e| {
                let _ = fs::remove_file(&tmp_archive_path);
                format!(
                    "Failed to open staged file {}: {}",
                    staged_path.display(),
                    e
                )
            })?;
            builder
                .append_file(&entry_name, &mut file_to_archive)
                .map_err(|e| {
                    let _ = fs::remove_file(&tmp_archive_path);
                    format!("Failed to append {} to tar.gz: {}", entry_name, e)
                })?;
            archived_count += 1;
        }
    }

    // 2. Append manifest.json metadata to root of archive
    let manifest_bytes = serde_json::to_vec_pretty(staged_items).map_err(|e| {
        let _ = fs::remove_file(&tmp_archive_path);
        format!("Failed to serialize session manifest: {}", e)
    })?;
    let mut header = Header::new_gnu();
    header.set_size(manifest_bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "manifest.json", &manifest_bytes[..])
        .map_err(|e| {
            let _ = fs::remove_file(&tmp_archive_path);
            format!("Failed to append manifest.json: {}", e)
        })?;

    // 3. Finalize tar and gzip writer
    let encoder = builder.into_inner().map_err(|e| {
        let _ = fs::remove_file(&tmp_archive_path);
        format!("Failed to finalize tar archive: {}", e)
    })?;
    encoder.finish().map_err(|e| {
        let _ = fs::remove_file(&tmp_archive_path);
        format!("Failed to finalize gzip compression: {}", e)
    })?;

    // Rename temporary file to target path atomically
    if let Err(e) = fs::rename(&tmp_archive_path, &archive_path_buf) {
        let _ = fs::remove_file(&tmp_archive_path);
        return Err(format!(
            "Failed to finalize backup archive rename from {} to {}: {}",
            tmp_archive_path.display(),
            archive_path_buf.display(),
            e
        ));
    }

    let meta = fs::metadata(&archive_path_buf)
        .map_err(|e| format!("Failed to read created archive metadata: {}", e))?;
    let total_bytes = meta.len();

    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let result = BackupResultDto {
        archive_path: archive_path_buf.to_string_lossy().to_string(),
        file_count: archived_count,
        total_bytes,
        timestamp_ms: now_ms,
    };

    // 4. Record operation in SQLite persistent history
    let hist_mgr = crate::history::get_history_manager();
    let _ = hist_mgr.record_operation(&crate::history::NewHistoryEntry {
        operation_id: uuid::Uuid::new_v4().to_string(),
        operation_type: "BackupWorkspace".to_string(),
        display_name: format!("{} files archived", archived_count),
        original_path: None,
        staged_path: Some(result.archive_path.clone()),
        inode: None,
        status: "Success".to_string(),
        can_undo: false,
        details: Some(format!("tar.gz backup ({} bytes)", total_bytes)),
    });

    // 5. Automatic retention pruning if saved to default directory
    if destination.is_none() {
        if let Some(limit) = auto_prune_limit {
            let _ = prune_old_backups(limit);
        }
    }

    Ok(result)
}

/// Restores staged working copies and session metadata from a tar.gz backup archive.
/// If `archive_path` is None, restores from the newest available backup in get_backups_dir().
/// Implements full streaming extraction, ZipBomb defense, path traversal validation,
/// atomic commits, and transactional rollback on any error.
pub fn restore_workspace_backup(
    archive_path: Option<String>,
) -> Result<Vec<StagedItemDto>, String> {
    let target_path = match archive_path {
        Some(ref p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => {
            let backups = list_backups()?;
            if backups.is_empty() {
                return Err("No backup archives found to restore".to_string());
            }
            PathBuf::from(&backups[0].full_path)
        }
    };

    if !target_path.exists() {
        return Err(format!(
            "Backup archive '{}' does not exist",
            target_path.display()
        ));
    }

    let archive_meta = fs::metadata(&target_path).map_err(|e| {
        format!(
            "Failed to read archive metadata {}: {}",
            target_path.display(),
            e
        )
    })?;
    let archive_size = archive_meta.len();

    let file = File::open(&target_path).map_err(|e| {
        format!(
            "Failed to open backup archive {}: {}",
            target_path.display(),
            e
        )
    })?;

    let global_staging = crate::workspace::get_global_staging_directory();
    fs::create_dir_all(&global_staging).map_err(|e| {
        format!(
            "Failed to create staging directory {}: {}",
            global_staging.display(),
            e
        )
    })?;

    // Create an isolated temporary staging directory for streaming extraction
    let temp_restore_dir = global_staging.join(format!(".tmp_restore_{}", Uuid::new_v4()));
    fs::create_dir_all(&temp_restore_dir).map_err(|e| {
        format!(
            "Failed to create temporary restore directory {}: {}",
            temp_restore_dir.display(),
            e
        )
    })?;

    // Helper closure to ensure cleanup of temporary restore directory on failure
    let cleanup_temp = |dir: &Path| {
        let _ = fs::remove_dir_all(dir);
    };

    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);

    let mut manifest_data: Option<Vec<StagedItemDto>> = None;
    let mut extracted_file_keys: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    let mut entry_count: usize = 0;
    let mut total_extracted_bytes: u64 = 0;

    let entries = match archive.entries() {
        Ok(e) => e,
        Err(e) => {
            cleanup_temp(&temp_restore_dir);
            return Err(format!("Failed to read archive entries: {}", e));
        }
    };

    for entry in entries {
        entry_count += 1;
        if entry_count > MAX_BACKUP_ENTRIES {
            cleanup_temp(&temp_restore_dir);
            return Err(format!(
                "Archive exceeds maximum allowed entry count ({} entries)",
                MAX_BACKUP_ENTRIES
            ));
        }

        let mut entry = match entry {
            Ok(e) => e,
            Err(e) => {
                cleanup_temp(&temp_restore_dir);
                return Err(format!("Failed to parse archive entry: {}", e));
            }
        };

        let entry_path = match entry.path() {
            Ok(p) => p.to_path_buf(),
            Err(e) => {
                cleanup_temp(&temp_restore_dir);
                return Err(format!("Invalid entry path in archive: {}", e));
            }
        };
        let path_str = entry_path.to_string_lossy().to_string();

        if path_str.contains("..") || path_str.starts_with('/') || path_str.starts_with('\\') {
            cleanup_temp(&temp_restore_dir);
            return Err(format!(
                "Path traversal element detected in entry path: '{}'",
                path_str
            ));
        }

        if path_str == "manifest.json" {
            let entry_size = entry.size();
            if entry_size > MAX_MANIFEST_BYTES {
                cleanup_temp(&temp_restore_dir);
                return Err(format!(
                    "manifest.json size ({} bytes) exceeds limit of {} bytes",
                    entry_size, MAX_MANIFEST_BYTES
                ));
            }

            let mut buf = Vec::new();
            if let Err(e) = entry.read_to_end(&mut buf) {
                cleanup_temp(&temp_restore_dir);
                return Err(format!("Failed to read manifest.json: {}", e));
            }

            total_extracted_bytes += buf.len() as u64;
            let items: Vec<StagedItemDto> = match serde_json::from_slice(&buf) {
                Ok(it) => it,
                Err(e) => {
                    cleanup_temp(&temp_restore_dir);
                    return Err(format!("Failed to deserialize manifest.json: {}", e));
                }
            };

            // Pre-validate all items in manifest before proceeding
            for item in &items {
                if let Err(e) = validate_manifest_item(item, &global_staging) {
                    cleanup_temp(&temp_restore_dir);
                    return Err(format!("Manifest validation failed: {}", e));
                }
            }

            manifest_data = Some(items);
        } else if path_str.starts_with("files/") {
            let relative_name = path_str.trim_start_matches("files/");
            if relative_name.is_empty()
                || relative_name.contains('/')
                || relative_name.contains('\\')
                || relative_name.contains("..")
            {
                cleanup_temp(&temp_restore_dir);
                return Err(format!(
                    "Invalid file entry name in archive: '{}'",
                    relative_name
                ));
            }

            let entry_size = entry.size();
            if entry_size > MAX_SINGLE_FILE_BYTES {
                cleanup_temp(&temp_restore_dir);
                return Err(format!(
                    "File '{}' exceeds single file size limit ({} bytes)",
                    relative_name, MAX_SINGLE_FILE_BYTES
                ));
            }

            // Stream extract directly to temporary restore directory
            let temp_dest = temp_restore_dir.join(relative_name);
            let mut out_file = match File::create(&temp_dest) {
                Ok(f) => f,
                Err(e) => {
                    cleanup_temp(&temp_restore_dir);
                    return Err(format!(
                        "Failed to create temporary extracted file {}: {}",
                        temp_dest.display(),
                        e
                    ));
                }
            };

            let written = match std::io::copy(&mut entry, &mut out_file) {
                Ok(w) => w,
                Err(e) => {
                    cleanup_temp(&temp_restore_dir);
                    return Err(format!("Failed to extract file {}: {}", relative_name, e));
                }
            };

            total_extracted_bytes += written;
            if total_extracted_bytes > MAX_TOTAL_EXTRACTED_BYTES {
                cleanup_temp(&temp_restore_dir);
                return Err(format!(
                    "Archive total uncompressed bytes exceed safety limit of {} bytes",
                    MAX_TOTAL_EXTRACTED_BYTES
                ));
            }

            if archive_size > 1024 * 1024
                && total_extracted_bytes > archive_size * MAX_COMPRESSION_RATIO
            {
                cleanup_temp(&temp_restore_dir);
                return Err(format!(
                    "Decompression bomb detected: uncompressed ratio exceeds {}x",
                    MAX_COMPRESSION_RATIO
                ));
            }

            extracted_file_keys.insert(relative_name.to_string());
        }
    }

    let items = match manifest_data {
        Some(it) => it,
        None => {
            cleanup_temp(&temp_restore_dir);
            return Err("Backup archive does not contain session manifest.json".to_string());
        }
    };

    // Verify that every manifest item has a corresponding extracted file in the archive
    let mut planned_restorations = Vec::new();
    for item in &items {
        let dest_path = match validate_manifest_item(item, &global_staging) {
            Ok(p) => p,
            Err(e) => {
                cleanup_temp(&temp_restore_dir);
                return Err(e);
            }
        };

        // Locate source file in temp_restore_dir: by stage_id (new) or display_name (legacy)
        let src_temp_path = if extracted_file_keys.contains(&item.stage_id) {
            temp_restore_dir.join(&item.stage_id)
        } else if extracted_file_keys.contains(&item.display_name) {
            temp_restore_dir.join(&item.display_name)
        } else {
            cleanup_temp(&temp_restore_dir);
            return Err(format!(
                "Archive is missing payload file for manifest item '{}' (stage_id: {})",
                item.display_name, item.stage_id
            ));
        };

        if !src_temp_path.exists() {
            cleanup_temp(&temp_restore_dir);
            return Err(format!(
                "Extracted file payload '{}' is missing on disk",
                src_temp_path.display()
            ));
        }

        // Prevent overwriting active staging files unexpectedly
        if dest_path.exists() {
            cleanup_temp(&temp_restore_dir);
            return Err(format!(
                "Destination staged file already exists (refusing to overwrite): {}",
                dest_path.display()
            ));
        }

        planned_restorations.push((src_temp_path, dest_path, item.clone()));
    }

    // Atomic Move Phase: move extracted files from temp_restore_dir to destination
    let mut committed_paths: Vec<PathBuf> = Vec::new();
    for (src, dest, _) in &planned_restorations {
        if let Err(e) = fs::rename(src, dest) {
            // Rollback all committed files
            for p in &committed_paths {
                let _ = fs::remove_file(p);
            }
            cleanup_temp(&temp_restore_dir);
            return Err(format!(
                "Failed to commit restored file {} -> {}: {}",
                src.display(),
                dest.display(),
                e
            ));
        }
        committed_paths.push(dest.clone());
    }

    // Clean up temporary restore directory
    cleanup_temp(&temp_restore_dir);

    // Register into SQLite session_items and WorkspaceManager
    let hist_mgr = crate::history::get_history_manager();
    let ws_mgr = crate::workspace::get_workspace_manager();
    let mut restored_items = Vec::new();

    for (_, dest_path, mut item) in planned_restorations {
        item.staged_path = dest_path.to_string_lossy().to_string();
        item.staged_raw_base64 = crate::models::encode_path_to_base64(&dest_path);

        // Conflict check against original file
        let orig_path = Path::new(&item.original_snapshot.canonical_path);
        if !orig_path.exists() {
            item.status = crate::workspace::StageStatus::Conflict;
        } else if let Err(crate::policy::PolicyError::ConflictDetected(_)) =
            crate::policy::OperationPolicy::verify_conflict(&item.original_snapshot, true)
        {
            item.status = crate::workspace::StageStatus::Conflict;
        } else {
            item.status = crate::workspace::StageStatus::Staged;
        }

        // Save into SQLite session items
        if let Err(e) = hist_mgr.save_session_item(&item) {
            eprintln!("Warning: Failed to save restored session item to DB: {}", e);
        }
        ws_mgr.insert_staged_item(item.clone());
        restored_items.push(item);
    }

    // Record into operation history
    let _ = hist_mgr.record_operation(&crate::history::NewHistoryEntry {
        operation_id: uuid::Uuid::new_v4().to_string(),
        operation_type: "RestoreBackup".to_string(),
        display_name: format!("{} files restored", restored_items.len()),
        original_path: None,
        staged_path: Some(target_path.to_string_lossy().to_string()),
        inode: None,
        status: "Success".to_string(),
        can_undo: false,
        details: Some(format!(
            "Restored from backup archive '{}'",
            target_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("")
        )),
    });

    Ok(restored_items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::OriginalSnapshotDto;
    use crate::workspace::{CloneMethod, StageStatus};
    use flate2::read::GzDecoder;
    use std::io::BufReader;
    use tar::Archive;
    use uuid::Uuid;

    #[test]
    fn test_create_workspace_backup_and_verify_archive() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle_backup_test_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let staged_file = temp_dir.join("sample.txt");
        fs::write(&staged_file, b"Staged content for backup test").unwrap();

        let stage_uuid = Uuid::new_v4().to_string();
        let sample_item = StagedItemDto {
            stage_id: stage_uuid.clone(),
            original_item_id: "orig-1".to_string(),
            raw_base64: "L2hvbWUvc3VzaWUvc2FtcGxlLnR4dA==".to_string(),
            display_name: "sample.txt".to_string(),
            staged_raw_base64: "L3RtcC9zdGFnZWQ=".to_string(),
            staged_path: staged_file.to_string_lossy().to_string(),
            size: 30,
            clone_method: CloneMethod::Reflink,
            status: StageStatus::Modified,
            staged_at_ms: 1729750000000,
            original_snapshot: OriginalSnapshotDto {
                item_id: "orig-1".to_string(),
                raw_base64: "L2hvbWUvc3VzaWUvc2FtcGxlLnR4dA==".to_string(),
                display_name: "sample.txt".to_string(),
                canonical_path: "/home/susie/sample.txt".to_string(),
                inode: 12345,
                dev: 66305,
                size: 30,
                mtime_ms: 1729750000000,
                sha256: Some("sha256sample".to_string()),
            },
        };

        let archive_target = temp_dir.join("test_backup.tar.gz");
        let result = create_workspace_backup(
            Some(archive_target.to_string_lossy().to_string()),
            std::slice::from_ref(&sample_item),
            None,
        )
        .unwrap();

        assert_eq!(result.file_count, 1);
        assert!(result.total_bytes > 0);
        assert!(archive_target.exists());

        // Verify content inside tar.gz archive
        let tar_file = File::open(&archive_target).unwrap();
        let gz = GzDecoder::new(BufReader::new(tar_file));
        let mut archive = Archive::new(gz);

        let entries: Vec<String> = archive
            .entries()
            .unwrap()
            .map(|e| e.unwrap().path().unwrap().to_string_lossy().to_string())
            .collect();

        // Archived with stage_id to prevent name collisions
        let expected_file_entry = format!("files/{}", stage_uuid);
        assert!(entries.contains(&expected_file_entry));
        assert!(entries.contains(&"manifest.json".to_string()));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_backup_list_delete_and_prune() {
        let bkp_dir = get_backups_dir();
        fs::create_dir_all(&bkp_dir).unwrap();

        // Create 3 dummy backup archives
        let b1 = bkp_dir.join("backup_20261020_010000.tar.gz");
        let b2 = bkp_dir.join("backup_20261021_020000.tar.gz");
        let b3 = bkp_dir.join("backup_20261022_030000.tar.gz");

        fs::write(&b1, b"dummy backup 1").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(15));
        fs::write(&b2, b"dummy backup 2").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(15));
        fs::write(&b3, b"dummy backup 3").unwrap();

        let list = list_backups().unwrap();
        assert!(list.len() >= 3);
        // Newest should be first
        assert_eq!(list[0].filename, "backup_20261022_030000.tar.gz");

        // Prune to keep 2
        let pruned = prune_old_backups(2).unwrap();
        assert!(pruned.contains(&"backup_20261020_010000.tar.gz".to_string()));
        assert!(!b1.exists());
        assert!(b2.exists());
        assert!(b3.exists());

        // Delete b2
        delete_backup("backup_20261021_020000.tar.gz").unwrap();
        assert!(!b2.exists());

        // Clean up remaining
        let _ = delete_backup("backup_20261022_030000.tar.gz");
    }

    #[test]
    fn test_restore_workspace_backup() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle_restore_test_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let orig_file = temp_dir.join("orig.txt");
        fs::write(&orig_file, b"Original file content").unwrap();

        let staged_file = temp_dir.join("staged_orig.txt");
        fs::write(&staged_file, b"Modified in staging").unwrap();

        let orig_snapshot = OriginalSnapshotDto {
            item_id: "orig-item-1".to_string(),
            raw_base64: crate::models::encode_path_to_base64(&orig_file),
            display_name: "orig.txt".to_string(),
            canonical_path: orig_file.to_string_lossy().to_string(),
            inode: 12345,
            dev: 66305,
            size: 21,
            mtime_ms: 1729750000000,
            sha256: Some("dummy-sha256".to_string()),
        };

        let stage_uuid = Uuid::new_v4().to_string();
        let item = StagedItemDto {
            stage_id: stage_uuid.clone(),
            original_item_id: "orig-item-1".to_string(),
            raw_base64: crate::models::encode_path_to_base64(&orig_file),
            display_name: "orig.txt".to_string(),
            staged_raw_base64: crate::models::encode_path_to_base64(&staged_file),
            staged_path: staged_file.to_string_lossy().to_string(),
            size: 19,
            clone_method: CloneMethod::Reflink,
            status: StageStatus::Modified,
            staged_at_ms: 1729750000000,
            original_snapshot: orig_snapshot,
        };

        let backup_dest = temp_dir.join("test_backup.tar.gz");
        let res = create_workspace_backup(
            Some(backup_dest.to_string_lossy().to_string()),
            &[item],
            None,
        )
        .unwrap();
        assert_eq!(res.file_count, 1);
        assert!(backup_dest.exists());

        // Restore from backup
        let restored =
            restore_workspace_backup(Some(backup_dest.to_string_lossy().to_string())).unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].display_name, "orig.txt");
        assert_eq!(restored[0].stage_id, stage_uuid);
        assert!(Path::new(&restored[0].staged_path).exists());

        let restored_bytes = fs::read(&restored[0].staged_path).unwrap();
        assert_eq!(restored_bytes, b"Modified in staging");

        // Clean up restored staged file
        let _ = fs::remove_file(&restored[0].staged_path);
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_backup_restore_duplicate_filenames_different_directories() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle_dup_test_{}", Uuid::new_v4()));
        let dir_a = temp_dir.join("dir_a");
        let dir_b = temp_dir.join("dir_b");
        fs::create_dir_all(&dir_a).unwrap();
        fs::create_dir_all(&dir_b).unwrap();

        let file_a = dir_a.join("image.png");
        let file_b = dir_b.join("image.png");
        fs::write(&file_a, b"Binary content of Image A").unwrap();
        fs::write(&file_b, b"Different binary content of Image B").unwrap();

        let uuid_a = Uuid::new_v4().to_string();
        let uuid_b = Uuid::new_v4().to_string();

        let item_a = StagedItemDto {
            stage_id: uuid_a.clone(),
            original_item_id: "orig-a".to_string(),
            raw_base64: crate::models::encode_path_to_base64(&file_a),
            display_name: "image.png".to_string(),
            staged_raw_base64: crate::models::encode_path_to_base64(&file_a),
            staged_path: file_a.to_string_lossy().to_string(),
            size: 25,
            clone_method: CloneMethod::StandardCopy,
            status: StageStatus::Modified,
            staged_at_ms: 1729750000000,
            original_snapshot: OriginalSnapshotDto {
                item_id: "orig-a".to_string(),
                raw_base64: crate::models::encode_path_to_base64(&file_a),
                display_name: "image.png".to_string(),
                canonical_path: file_a.to_string_lossy().to_string(),
                inode: 1111,
                dev: 2222,
                size: 25,
                mtime_ms: 1729750000000,
                sha256: Some("sha-a".to_string()),
            },
        };

        let item_b = StagedItemDto {
            stage_id: uuid_b.clone(),
            original_item_id: "orig-b".to_string(),
            raw_base64: crate::models::encode_path_to_base64(&file_b),
            display_name: "image.png".to_string(), // SAME display_name!
            staged_raw_base64: crate::models::encode_path_to_base64(&file_b),
            staged_path: file_b.to_string_lossy().to_string(),
            size: 35,
            clone_method: CloneMethod::StandardCopy,
            status: StageStatus::Modified,
            staged_at_ms: 1729750000000,
            original_snapshot: OriginalSnapshotDto {
                item_id: "orig-b".to_string(),
                raw_base64: crate::models::encode_path_to_base64(&file_b),
                display_name: "image.png".to_string(),
                canonical_path: file_b.to_string_lossy().to_string(),
                inode: 3333,
                dev: 2222,
                size: 35,
                mtime_ms: 1729750000000,
                sha256: Some("sha-b".to_string()),
            },
        };

        let backup_archive = temp_dir.join("dual_images.tar.gz");
        create_workspace_backup(
            Some(backup_archive.to_string_lossy().to_string()),
            &[item_a, item_b],
            None,
        )
        .unwrap();

        // Restore and verify both are preserved distinctly without collision
        let restored =
            restore_workspace_backup(Some(backup_archive.to_string_lossy().to_string())).unwrap();
        assert_eq!(restored.len(), 2);

        let restored_a = restored.iter().find(|i| i.stage_id == uuid_a).unwrap();
        let restored_b = restored.iter().find(|i| i.stage_id == uuid_b).unwrap();

        let content_a = fs::read(&restored_a.staged_path).unwrap();
        let content_b = fs::read(&restored_b.staged_path).unwrap();

        assert_eq!(content_a, b"Binary content of Image A");
        assert_eq!(content_b, b"Different binary content of Image B");

        // Clean up restored files
        let _ = fs::remove_file(&restored_a.staged_path);
        let _ = fs::remove_file(&restored_b.staged_path);
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_restore_rejects_path_traversal_in_manifest() {
        let global_staging = crate::workspace::get_global_staging_directory();
        let valid_uuid = Uuid::new_v4().to_string();

        let evil_item = StagedItemDto {
            stage_id: valid_uuid,
            original_item_id: "orig-evil".to_string(),
            raw_base64: "dGVzdA==".to_string(),
            display_name: "../../../../etc/passwd".to_string(), // Traversal!
            staged_raw_base64: "dGVzdA==".to_string(),
            staged_path: "/tmp/evil".to_string(),
            size: 10,
            clone_method: CloneMethod::StandardCopy,
            status: StageStatus::Modified,
            staged_at_ms: 1729750000000,
            original_snapshot: OriginalSnapshotDto {
                item_id: "orig-evil".to_string(),
                raw_base64: "dGVzdA==".to_string(),
                display_name: "passwd".to_string(),
                canonical_path: "/etc/passwd".to_string(),
                inode: 1,
                dev: 1,
                size: 10,
                mtime_ms: 1729750000000,
                sha256: None,
            },
        };

        let result = validate_manifest_item(&evil_item, &global_staging);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Invalid or unsafe display_name"));
    }

    #[test]
    fn test_restore_rejects_invalid_uuid_stage_id() {
        let global_staging = crate::workspace::get_global_staging_directory();
        let bad_uuid_item = StagedItemDto {
            stage_id: "not-a-valid-uuid-123".to_string(),
            original_item_id: "orig".to_string(),
            raw_base64: "dGVzdA==".to_string(),
            display_name: "safe.txt".to_string(),
            staged_raw_base64: "dGVzdA==".to_string(),
            staged_path: "/tmp/safe.txt".to_string(),
            size: 10,
            clone_method: CloneMethod::StandardCopy,
            status: StageStatus::Modified,
            staged_at_ms: 1729750000000,
            original_snapshot: OriginalSnapshotDto {
                item_id: "orig".to_string(),
                raw_base64: "dGVzdA==".to_string(),
                display_name: "safe.txt".to_string(),
                canonical_path: "/home/safe.txt".to_string(),
                inode: 1,
                dev: 1,
                size: 10,
                mtime_ms: 1729750000000,
                sha256: None,
            },
        };

        let result = validate_manifest_item(&bad_uuid_item, &global_staging);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be valid UUID format"));
    }

    #[test]
    fn test_export_typescript_bindings_backup() {
        let _ = std::fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        BackupResultDto::export(&cfg).expect("Failed to export BackupResultDto");
        BackupArchiveInfoDto::export(&cfg).expect("Failed to export BackupArchiveInfoDto");
    }
}
