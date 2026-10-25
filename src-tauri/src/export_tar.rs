// src-tauri/src/export_tar.rs
// Cuddle - Phase 9.3: Workspace Tar & Tar.gz Export Feature

use std::fs::File;
use std::path::{Path, PathBuf};

use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use tar::Builder;
use ts_rs::TS;

use crate::history::{HistoryManager, NewHistoryEntry};
use crate::workspace::{StageStatus, StagedItemDto};

/// Compression format for tar export
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub enum TarExportFormat {
    /// Uncompressed POSIX tar archive (.tar)
    Tar,
    /// Gzip compressed tar archive (.tar.gz)
    TarGz,
}

/// Scope of items included in tar export
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub enum TarExportScope {
    /// Only modified files in workspace-temp
    ModifiedOnly,
    /// All staged files currently in workspace-temp
    AllStaged,
    /// Only explicitly selected items by ID
    SelectedOnly,
}

/// Options DTO for tar export
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct TarExportOptionsDto {
    pub destination_path: String,
    pub format: TarExportFormat,
    pub scope: TarExportScope,
    pub selected_ids: Vec<String>,
}

/// Result DTO after tar export
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct TarExportResultDto {
    pub success: bool,
    pub archive_path: String,
    pub file_count: usize,
    pub total_uncompressed_bytes: u64,
    pub archive_bytes: u64,
    pub error_message: Option<String>,
}

/// Creates a tar or tar.gz archive from workspace items according to options
pub fn export_workspace_to_tar(
    staged_items: &[StagedItemDto],
    options: &TarExportOptionsDto,
    history_mgr: Option<&HistoryManager>,
) -> Result<TarExportResultDto, String> {
    let dest_path = PathBuf::from(&options.destination_path);

    // Ensure parent directory exists
    if let Some(parent) = dest_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create destination directory: {e}"))?;
        }
    }

    // Filter target items based on scope
    let target_items: Vec<&StagedItemDto> = staged_items
        .iter()
        .filter(|item| match options.scope {
            TarExportScope::ModifiedOnly => item.status == StageStatus::Modified,
            TarExportScope::AllStaged => true,
            TarExportScope::SelectedOnly => options.selected_ids.contains(&item.stage_id),
        })
        .collect();

    if target_items.is_empty() {
        return Err("No files matched the selected export scope".to_string());
    }

    let file =
        File::create(&dest_path).map_err(|e| format!("Failed to create destination file: {e}"))?;

    let mut total_uncompressed_bytes: u64 = 0;
    let mut file_count: usize = 0;

    match options.format {
        TarExportFormat::Tar => {
            let mut builder = Builder::new(file);
            for item in &target_items {
                let staged_path = Path::new(&item.staged_path);
                if staged_path.exists() {
                    let entry_name = &item.display_name;
                    builder
                        .append_path_with_name(staged_path, entry_name)
                        .map_err(|e| {
                            format!("Failed to append {} to tar archive: {e}", item.display_name)
                        })?;
                    total_uncompressed_bytes += item.size;
                    file_count += 1;
                }
            }
            builder
                .finish()
                .map_err(|e| format!("Failed to finalize tar archive: {e}"))?;
        }
        TarExportFormat::TarGz => {
            let encoder = GzEncoder::new(file, Compression::default());
            let mut builder = Builder::new(encoder);
            for item in &target_items {
                let staged_path = Path::new(&item.staged_path);
                if staged_path.exists() {
                    let entry_name = &item.display_name;
                    builder
                        .append_path_with_name(staged_path, entry_name)
                        .map_err(|e| {
                            format!(
                                "Failed to append {} to tar.gz archive: {e}",
                                item.display_name
                            )
                        })?;
                    total_uncompressed_bytes += item.size;
                    file_count += 1;
                }
            }
            let encoder = builder
                .into_inner()
                .map_err(|e| format!("Failed to finalize tar.gz archive stream: {e}"))?;
            encoder
                .finish()
                .map_err(|e| format!("Failed to finish gzip compression: {e}"))?;
        }
    }

    // Get final archive file size
    let archive_bytes = std::fs::metadata(&dest_path).map(|m| m.len()).unwrap_or(0);

    // Record in SQLite WAL history
    if let Some(hist) = history_mgr {
        let _ = hist.record_operation(&NewHistoryEntry {
            operation_id: uuid::Uuid::new_v4().to_string(),
            operation_type: "export_tar".to_string(),
            display_name: format!(
                "Export: {}",
                dest_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("archive")
            ),
            original_path: None,
            staged_path: Some(dest_path.to_string_lossy().to_string()),
            inode: None,
            status: "Success".to_string(),
            can_undo: false,
            details: Some(format!(
                "Exported {} files to {} ({} bytes)",
                file_count,
                dest_path.display(),
                archive_bytes
            )),
        });
    }

    Ok(TarExportResultDto {
        success: true,
        archive_path: dest_path.to_string_lossy().to_string(),
        file_count,
        total_uncompressed_bytes,
        archive_bytes,
        error_message: None,
    })
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_export_tar_uncompressed() {
        let temp_dir =
            std::env::temp_dir().join(format!("cuddle_test_tar_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let src_file = temp_dir.join("sample.txt");
        let mut f = File::create(&src_file).unwrap();
        writeln!(f, "Hello Cuddle Tar Export").unwrap();

        let staged_item = StagedItemDto {
            stage_id: "stage-1".to_string(),
            original_item_id: "orig-1".to_string(),
            raw_base64: "c2FtcGxlLnR4dA==".to_string(),
            staged_raw_base64: "c2FtcGxlLnR4dA==".to_string(),
            display_name: "sample.txt".to_string(),
            staged_path: src_file.to_str().unwrap().to_string(),
            size: 24,
            clone_method: crate::workspace::CloneMethod::Reflink,
            status: StageStatus::Modified,
            staged_at_ms: 1700000000000,
            original_snapshot: crate::policy::OriginalSnapshotDto {
                item_id: "orig-1".to_string(),
                raw_base64: "c2FtcGxlLnR4dA==".to_string(),
                display_name: "sample.txt".to_string(),
                canonical_path: "/dummy/sample.txt".to_string(),
                size: 24,
                mtime_ms: 1700000000000,
                sha256: Some("hash".to_string()),
                inode: 12345,
                dev: 66305,
            },
        };

        let tar_out = temp_dir.join("output.tar");
        let options = TarExportOptionsDto {
            destination_path: tar_out.to_str().unwrap().to_string(),
            format: TarExportFormat::Tar,
            scope: TarExportScope::ModifiedOnly,
            selected_ids: vec![],
        };

        let res = export_workspace_to_tar(&[staged_item], &options, None).unwrap();
        assert!(res.success);
        assert_eq!(res.file_count, 1);
        assert!(res.archive_bytes > 0);
        assert!(tar_out.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_export_tar_gz_compressed() {
        let temp_dir =
            std::env::temp_dir().join(format!("cuddle_test_targz_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let src_file = temp_dir.join("doc.md");
        let mut f = File::create(&src_file).unwrap();
        writeln!(f, "# Markdown document for Cuddle").unwrap();

        let staged_item = StagedItemDto {
            stage_id: "stage-2".to_string(),
            original_item_id: "orig-2".to_string(),
            raw_base64: "ZG9jLm1k".to_string(),
            staged_raw_base64: "ZG9jLm1k".to_string(),
            display_name: "doc.md".to_string(),
            staged_path: src_file.to_str().unwrap().to_string(),
            size: 32,
            clone_method: crate::workspace::CloneMethod::Reflink,
            status: StageStatus::Staged,
            staged_at_ms: 1700000000000,
            original_snapshot: crate::policy::OriginalSnapshotDto {
                item_id: "orig-2".to_string(),
                raw_base64: "ZG9jLm1k".to_string(),
                display_name: "doc.md".to_string(),
                canonical_path: "/dummy/doc.md".to_string(),
                size: 32,
                mtime_ms: 1700000000000,
                sha256: Some("hash".to_string()),
                inode: 12346,
                dev: 66305,
            },
        };

        let targz_out = temp_dir.join("output.tar.gz");
        let options = TarExportOptionsDto {
            destination_path: targz_out.to_str().unwrap().to_string(),
            format: TarExportFormat::TarGz,
            scope: TarExportScope::AllStaged,
            selected_ids: vec![],
        };

        let res = export_workspace_to_tar(&[staged_item], &options, None).unwrap();
        assert!(res.success);
        assert_eq!(res.file_count, 1);
        assert!(res.archive_bytes > 0);
        assert!(targz_out.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_export_typescript_bindings_tar() {
        let _ = std::fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();

        TarExportFormat::export(&cfg).unwrap();
        TarExportScope::export(&cfg).unwrap();
        TarExportOptionsDto::export(&cfg).unwrap();
        TarExportResultDto::export(&cfg).unwrap();
    }
}
