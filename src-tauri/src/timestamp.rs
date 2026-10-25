// src-tauri/src/timestamp.rs
// Cuddle - Phase 9.3: Batch Timestamp Change (touch / relative shift / EXIF sync)

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use filetime::{set_file_mtime, FileTime};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::history::{HistoryManager, NewHistoryEntry};

/// Timestamp mode specification (Touch / Absolute / Relative / EXIF)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "value")]
#[ts(export, export_to = "../../src/types/")]
pub enum TimestampModeSpec {
    /// Update to current time (touch equivalent)
    CurrentTime,
    /// Set to an explicit absolute datetime string (e.g. "2026-10-25 15:30:00")
    Absolute(String),
    /// Relative offset in seconds (e.g. +3600 for +1h, -1800 for -30m)
    RelativeOffset(i64),
    /// Synchronize with camera EXIF DateTimeOriginal if available
    ExifSync,
}

/// Target item input for timestamp operations
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct TimestampTargetInputDto {
    pub item_id: String,
    pub path_or_base64: String,
    pub display_name: String,
}

/// Single item within a timestamp plan
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct PlannedTimestampItemDto {
    pub item_id: String,
    pub path: String,
    pub display_name: String,
    pub is_dir: bool,
    pub current_mtime_ms: i64,
    pub current_mtime_formatted: String,
    pub target_mtime_ms: i64,
    pub target_mtime_formatted: String,
    pub is_changed: bool,
    pub warning: Option<String>,
}

/// Complete plan DTO for bulk timestamp changes with dry-run results
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct TimestampPlanDto {
    pub plan_id: String,
    pub items: Vec<PlannedTimestampItemDto>,
    pub total_count: usize,
    pub changed_count: usize,
    pub has_warnings: bool,
    pub is_executable: bool,
    pub error_message: Option<String>,
}

/// Result DTO after executing bulk timestamp changes
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct TimestampExecutionResultDto {
    pub plan_id: String,
    pub success: bool,
    pub total_count: usize,
    pub succeeded_count: usize,
    pub failed_count: usize,
    pub is_rolled_back: bool,
    pub error_message: Option<String>,
}

/// Resolves path from standard string or Base64
fn resolve_target_path(path_or_base64: &str) -> PathBuf {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    if let Ok(decoded_bytes) = STANDARD.decode(path_or_base64) {
        #[cfg(unix)]
        {
            use std::ffi::OsStr;
            use std::os::unix::ffi::OsStrExt;
            PathBuf::from(OsStr::from_bytes(&decoded_bytes))
        }
        #[cfg(not(unix))]
        {
            if let Ok(s) = String::from_utf8(decoded_bytes) {
                PathBuf::from(s)
            } else {
                PathBuf::from(path_or_base64)
            }
        }
    } else {
        PathBuf::from(path_or_base64)
    }
}

/// Format epoch milliseconds to local YYYY-MM-DD HH:MM:SS
pub fn format_mtime_ms(ms: i64) -> String {
    let secs = ms / 1000;
    let nsecs = ((ms % 1000).abs() * 1_000_000) as u32;
    if let Some(dt) = Local.timestamp_opt(secs, nsecs).single() {
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        "1970-01-01 00:00:00".to_string()
    }
}

/// Parse date string in common formats
pub fn parse_datetime_string(s: &str) -> Result<i64, String> {
    let trimmed = s.trim();

    // Try standard formats
    let formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d",
    ];

    for fmt in &formats {
        if let Ok(naive) = NaiveDateTime::parse_from_str(trimmed, fmt) {
            if let Some(local_dt) = Local.from_local_datetime(&naive).single() {
                return Ok(local_dt.timestamp_millis());
            }
        }
        // Try naive date only with midnight
        if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(trimmed, fmt) {
            if let Some(naive_dt) = naive_date.and_hms_opt(0, 0, 0) {
                if let Some(local_dt) = Local.from_local_datetime(&naive_dt).single() {
                    return Ok(local_dt.timestamp_millis());
                }
            }
        }
    }

    // Try RFC3339 / ISO8601 with timezone
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(dt.timestamp_millis());
    }

    Err(format!(
        "Invalid datetime format: '{}'. Expected 'YYYY-MM-DD HH:MM:SS' or ISO 8601",
        s
    ))
}

/// Extract EXIF DateTimeOriginal from image files (JPEG, TIFF, WebP, etc.)
pub fn extract_exif_timestamp(path: &Path) -> Result<i64, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open file: {e}"))?;
    let mut reader = BufReader::new(file);

    let exifreader = exif::Reader::new();
    let exif = exifreader
        .read_from_container(&mut reader)
        .map_err(|e| format!("EXIF read failed: {e}"))?;

    // Try DateTimeOriginal first, then DateTimeDigitized, then DateTime
    let target_tags = [
        exif::Tag::DateTimeOriginal,
        exif::Tag::DateTimeDigitized,
        exif::Tag::DateTime,
    ];

    for tag in &target_tags {
        if let Some(field) = exif.get_field(*tag, exif::In::PRIMARY) {
            let val_str = field.display_value().to_string();
            // EXIF datetime format is usually "YYYY:MM:DD HH:MM:SS"
            let normalized = val_str.replace('/', "-").replacen(':', "-", 2);
            if let Ok(ms) = parse_datetime_string(&normalized) {
                return Ok(ms);
            }
        }
    }

    Err("No DateTimeOriginal or DateTime tag found in EXIF metadata".to_string())
}

/// Calculate target timestamp given current mtime and specification
pub fn compute_target_timestamp(
    current_mtime_ms: i64,
    mode: &TimestampModeSpec,
    path: &Path,
) -> (i64, Option<String>) {
    match mode {
        TimestampModeSpec::CurrentTime => {
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(current_mtime_ms);
            (now_ms, None)
        }
        TimestampModeSpec::Absolute(dt_str) => match parse_datetime_string(dt_str) {
            Ok(target_ms) => (target_ms, None),
            Err(e) => (current_mtime_ms, Some(format!("Invalid date: {e}"))),
        },
        TimestampModeSpec::RelativeOffset(offset_secs) => {
            let offset_ms = offset_secs.saturating_mul(1000);
            let target_ms = current_mtime_ms.saturating_add(offset_ms);
            (target_ms, None)
        }
        TimestampModeSpec::ExifSync => match extract_exif_timestamp(path) {
            Ok(exif_ms) => (exif_ms, None),
            Err(e) => (
                current_mtime_ms,
                Some(format!("EXIF metadata not found: {e}")),
            ),
        },
    }
}

/// Generates dry-run plan for bulk timestamp changes
pub fn generate_timestamp_plan(
    targets: Vec<TimestampTargetInputDto>,
    mode: TimestampModeSpec,
) -> TimestampPlanDto {
    let plan_id = format!("tsplan_{}", Uuid::new_v4());
    let mut planned_items = Vec::new();
    let mut changed_count = 0;
    let mut has_warnings = false;

    for target in targets {
        let resolved = resolve_target_path(&target.path_or_base64);
        let metadata_res = std::fs::symlink_metadata(&resolved);

        match metadata_res {
            Ok(meta) => {
                let is_dir = meta.is_dir();
                let current_mtime_ms = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);

                let (target_mtime_ms, warning) =
                    compute_target_timestamp(current_mtime_ms, &mode, &resolved);

                if warning.is_some() {
                    has_warnings = true;
                }

                // Consider changed if difference is 1 second or more (due to filesystem granularity)
                let is_changed = (target_mtime_ms / 1000) != (current_mtime_ms / 1000);
                if is_changed {
                    changed_count += 1;
                }

                planned_items.push(PlannedTimestampItemDto {
                    item_id: target.item_id,
                    path: resolved.to_string_lossy().to_string(),
                    display_name: target.display_name,
                    is_dir,
                    current_mtime_ms,
                    current_mtime_formatted: format_mtime_ms(current_mtime_ms),
                    target_mtime_ms,
                    target_mtime_formatted: format_mtime_ms(target_mtime_ms),
                    is_changed,
                    warning,
                });
            }
            Err(e) => {
                has_warnings = true;
                planned_items.push(PlannedTimestampItemDto {
                    item_id: target.item_id,
                    path: resolved.to_string_lossy().to_string(),
                    display_name: target.display_name,
                    is_dir: false,
                    current_mtime_ms: 0,
                    current_mtime_formatted: "N/A".to_string(),
                    target_mtime_ms: 0,
                    target_mtime_formatted: "N/A".to_string(),
                    is_changed: false,
                    warning: Some(format!("Cannot read metadata: {e}")),
                });
            }
        }
    }

    let total_count = planned_items.len();
    let is_executable = !planned_items.is_empty();

    TimestampPlanDto {
        plan_id,
        items: planned_items,
        total_count,
        changed_count,
        has_warnings,
        is_executable,
        error_message: None,
    }
}

/// Executes timestamp plan with rollback on error
pub fn execute_timestamp_plan(
    plan: TimestampPlanDto,
    history_mgr: Option<&HistoryManager>,
) -> TimestampExecutionResultDto {
    let mut applied_items: Vec<(PathBuf, FileTime)> = Vec::new();
    let mut succeeded_count = 0;
    let mut failed_count = 0;
    let mut error_msg = None;

    for item in &plan.items {
        if !item.is_changed {
            continue;
        }

        let path = PathBuf::from(&item.path);

        // Record current mtime for rollback
        let current_ft = FileTime::from_unix_time(
            item.current_mtime_ms / 1000,
            ((item.current_mtime_ms % 1000).abs() * 1_000_000) as u32,
        );

        let target_ft = FileTime::from_unix_time(
            item.target_mtime_ms / 1000,
            ((item.target_mtime_ms % 1000).abs() * 1_000_000) as u32,
        );

        match set_file_mtime(&path, target_ft) {
            Ok(_) => {
                applied_items.push((path, current_ft));
                succeeded_count += 1;
            }
            Err(e) => {
                failed_count += 1;
                error_msg = Some(format!("Failed to set mtime on {}: {e}", item.display_name));
                break;
            }
        }
    }

    if failed_count > 0 {
        // Reverse-order rollback
        for (p, prev_ft) in applied_items.into_iter().rev() {
            let _ = set_file_mtime(&p, prev_ft);
        }

        TimestampExecutionResultDto {
            plan_id: plan.plan_id,
            success: false,
            total_count: plan.items.len(),
            succeeded_count: 0,
            failed_count,
            is_rolled_back: true,
            error_message: error_msg,
        }
    } else {
        if let Some(hist) = history_mgr {
            for item in &plan.items {
                if !item.is_changed {
                    continue;
                }
                let _ = hist.record_operation(&NewHistoryEntry {
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    operation_type: "touch_mtime".to_string(),
                    display_name: item.display_name.clone(),
                    original_path: Some(item.path.clone()),
                    staged_path: Some(item.path.clone()),
                    inode: None,
                    status: "Success".to_string(),
                    can_undo: true,
                    details: Some(format!(
                        "Mtime changed: {} -> {}",
                        item.current_mtime_formatted, item.target_mtime_formatted
                    )),
                });
            }
        }

        TimestampExecutionResultDto {
            plan_id: plan.plan_id,
            success: true,
            total_count: plan.items.len(),
            succeeded_count,
            failed_count: 0,
            is_rolled_back: false,
            error_message: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_parse_datetime_string_formats() {
        let ts1 = parse_datetime_string("2026-10-25 15:30:00").unwrap();
        assert!(ts1 > 0);

        let ts2 = parse_datetime_string("2026/10/25 15:30:00").unwrap();
        assert_eq!(ts1, ts2);

        let ts_date_only = parse_datetime_string("2026-10-25").unwrap();
        assert!(ts_date_only > 0);

        let err = parse_datetime_string("not-a-date");
        assert!(err.is_err());
    }

    #[test]
    fn test_relative_offset_computation() {
        let base_ms = 1_700_000_000_000i64;
        let mode_plus = TimestampModeSpec::RelativeOffset(3600);
        let dummy_path = Path::new("/dummy");

        let (target_ms, warning) = compute_target_timestamp(base_ms, &mode_plus, dummy_path);
        assert_eq!(target_ms, base_ms + 3_600_000);
        assert!(warning.is_none());

        let mode_minus = TimestampModeSpec::RelativeOffset(-1800);
        let (target_minus_ms, _) = compute_target_timestamp(base_ms, &mode_minus, dummy_path);
        assert_eq!(target_minus_ms, base_ms - 1_800_000);
    }

    #[test]
    fn test_generate_and_execute_timestamp_plan() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle_test_ts_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("test_touch.txt");
        File::create(&file_path).unwrap();

        let target_input = TimestampTargetInputDto {
            item_id: "test-item-1".to_string(),
            path_or_base64: file_path.to_str().unwrap().to_string(),
            display_name: "test_touch.txt".to_string(),
        };

        // Plan: set to 2026-01-01 12:00:00
        let mode = TimestampModeSpec::Absolute("2026-01-01 12:00:00".to_string());
        let plan = generate_timestamp_plan(vec![target_input], mode);

        assert_eq!(plan.items.len(), 1);
        assert!(plan.is_executable);
        assert!(plan.items[0].is_changed);

        // Execute plan
        let result = execute_timestamp_plan(plan, None);
        assert!(result.success);
        assert_eq!(result.succeeded_count, 1);

        // Verify filesystem mtime
        let meta = std::fs::metadata(&file_path).unwrap();
        let mtime = meta.modified().unwrap();
        let mtime_ms = mtime.duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let expected_ms = parse_datetime_string("2026-01-01 12:00:00").unwrap();
        assert_eq!(mtime_ms / 1000, expected_ms / 1000);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_export_typescript_bindings_timestamp() {
        let _ = std::fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();

        TimestampModeSpec::export(&cfg).unwrap();
        TimestampTargetInputDto::export(&cfg).unwrap();
        PlannedTimestampItemDto::export(&cfg).unwrap();
        TimestampPlanDto::export(&cfg).unwrap();
        TimestampExecutionResultDto::export(&cfg).unwrap();
    }
}
