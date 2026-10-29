use crate::workspace::StagedItemDto;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tar::{Builder, Header};
use ts_rs::TS;

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

/// Generates a timestamped default backup path: `$XDG_DATA_HOME/Cuddle/backups/backup_YYYYMMDD_HHMMSS.tar.gz`
pub fn get_default_backup_path() -> Result<String, String> {
    let base_dir = get_backups_dir();

    let now = chrono::Local::now();
    let filename = format!("backup_{}.tar.gz", now.format("%Y%m%d_%H%M%S"));
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

    if let Some(parent) = archive_path_buf.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create backup directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    let file = File::create(&archive_path_buf).map_err(|e| {
        format!(
            "Failed to create backup file {}: {}",
            archive_path_buf.display(),
            e
        )
    })?;
    let enc = GzEncoder::new(BufWriter::new(file), Compression::default());
    let mut builder = Builder::new(enc);

    let mut archived_count = 0;

    // 1. Append staged files to archive under `files/<display_name>`
    for item in staged_items {
        let staged_path = Path::new(&item.staged_path);
        if staged_path.exists() {
            let entry_name = format!("files/{}", item.display_name);
            let mut file_to_archive = File::open(staged_path).map_err(|e| {
                format!(
                    "Failed to open staged file {}: {}",
                    staged_path.display(),
                    e
                )
            })?;
            builder
                .append_file(&entry_name, &mut file_to_archive)
                .map_err(|e| format!("Failed to append {} to tar.gz: {}", entry_name, e))?;
            archived_count += 1;
        }
    }

    // 2. Append manifest.json metadata to root of archive
    let manifest_bytes = serde_json::to_vec_pretty(staged_items)
        .map_err(|e| format!("Failed to serialize session manifest: {}", e))?;
    let mut header = Header::new_gnu();
    header.set_size(manifest_bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "manifest.json", &manifest_bytes[..])
        .map_err(|e| format!("Failed to append manifest.json: {}", e))?;

    // 3. Finalize tar and gzip writer
    let encoder = builder
        .into_inner()
        .map_err(|e| format!("Failed to finalize tar archive: {}", e))?;
    encoder
        .finish()
        .map_err(|e| format!("Failed to finalize gzip compression: {}", e))?;

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

    let file = File::open(&target_path).map_err(|e| {
        format!(
            "Failed to open backup archive {}: {}",
            target_path.display(),
            e
        )
    })?;

    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);

    let mut manifest_data: Option<Vec<StagedItemDto>> = None;
    let mut extracted_files: std::collections::HashMap<String, Vec<u8>> =
        std::collections::HashMap::new();

    let entries = archive
        .entries()
        .map_err(|e| format!("Failed to read archive entries: {}", e))?;

    for entry in entries {
        let mut entry = entry.map_err(|e| format!("Failed to parse archive entry: {}", e))?;
        let entry_path = entry
            .path()
            .map_err(|e| format!("Invalid entry path: {}", e))?
            .to_path_buf();
        let path_str = entry_path.to_string_lossy().to_string();

        if path_str == "manifest.json" {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut buf)
                .map_err(|e| format!("Failed to read manifest.json: {}", e))?;
            let items: Vec<StagedItemDto> = serde_json::from_slice(&buf)
                .map_err(|e| format!("Failed to deserialize manifest.json: {}", e))?;
            manifest_data = Some(items);
        } else if path_str.starts_with("files/") {
            let relative_name = path_str.trim_start_matches("files/");
            if !relative_name.contains("..") && !relative_name.is_empty() {
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut entry, &mut buf)
                    .map_err(|e| format!("Failed to extract file {}: {}", relative_name, e))?;
                extracted_files.insert(relative_name.to_string(), buf);
            }
        }
    }

    let items = manifest_data
        .ok_or_else(|| "Backup archive does not contain session manifest.json".to_string())?;

    let hist_mgr = crate::history::get_history_manager();
    let ws_mgr = crate::workspace::get_workspace_manager();
    let global_staging = crate::workspace::get_global_staging_directory();
    fs::create_dir_all(&global_staging).map_err(|e| {
        format!(
            "Failed to create staging directory {}: {}",
            global_staging.display(),
            e
        )
    })?;

    let mut restored_items = Vec::new();

    for mut item in items {
        let file_bytes = extracted_files.get(&item.display_name);
        let dest_filename = format!("{}_{}", item.stage_id, item.display_name);
        let dest_path = global_staging.join(&dest_filename);

        if let Some(bytes) = file_bytes {
            fs::write(&dest_path, bytes).map_err(|e| {
                format!(
                    "Failed to restore staged file {}: {}",
                    dest_path.display(),
                    e
                )
            })?;
        }
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
        let _ = hist_mgr.save_session_item(&item);
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

        let sample_item = StagedItemDto {
            stage_id: "test-stage-1".to_string(),
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
            &[sample_item.clone()],
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

        assert!(entries.contains(&"files/sample.txt".to_string()));
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

        let item = StagedItemDto {
            stage_id: "test-restore-stage-1".to_string(),
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
        assert_eq!(restored[0].stage_id, "test-restore-stage-1");
        assert!(Path::new(&restored[0].staged_path).exists());

        let restored_bytes = fs::read(&restored[0].staged_path).unwrap();
        assert_eq!(restored_bytes, b"Modified in staging");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_export_typescript_bindings_backup() {
        let _ = std::fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        BackupResultDto::export(&cfg).expect("Failed to export BackupResultDto");
        BackupArchiveInfoDto::export(&cfg).expect("Failed to export BackupArchiveInfoDto");
    }
}
