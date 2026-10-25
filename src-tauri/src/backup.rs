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

/// Generates a timestamped default backup path: `$XDG_DATA_HOME/Cuddle/backups/backup_YYYYMMDD_HHMMSS.tar.gz`
pub fn get_default_backup_path() -> Result<String, String> {
    let base_dir = dirs::data_dir()
        .map(|p| p.join("Cuddle").join("backups"))
        .unwrap_or_else(|| PathBuf::from("./backups"));

    let now = chrono::Local::now();
    let filename = format!("backup_{}.tar.gz", now.format("%Y%m%d_%H%M%S"));
    let full_path = base_dir.join(filename);
    Ok(full_path.to_string_lossy().to_string())
}

/// Creates a tar.gz backup archive of all active staged working copies and includes session manifest.json
pub fn create_workspace_backup(
    destination: Option<String>,
    staged_items: &[StagedItemDto],
) -> Result<BackupResultDto, String> {
    if staged_items.is_empty() {
        return Err("No staged working copies to backup".to_string());
    }

    let archive_path_buf = match destination {
        Some(custom) if !custom.trim().is_empty() => PathBuf::from(custom),
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

    Ok(result)
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
            &[sample_item],
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
    fn test_export_typescript_bindings_backup() {
        let _ = std::fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        BackupResultDto::export(&cfg).expect("Failed to export BackupResultDto");
    }
}
