pub mod backup;
pub mod batch;
pub mod chmod;
pub mod config;
pub mod diff;
pub mod export_tar;
pub mod extractor;
pub mod git;
pub mod history;
pub mod image_preview;
pub mod models;
pub mod notify;
pub mod periodic_index;
pub mod pipeline;
pub mod policy;
pub mod sandbox;
pub mod scanner;
pub mod search_filename;
pub mod search_fulltext;
pub mod timestamp;
pub mod workspace;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct ReflectExecutionResultDto {
    pub staged_item: workspace::StagedItemDto,
    pub backup_path: Option<String>,
    pub mode: workspace::ReflectionMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct ReflectAsCopyResultDto {
    pub staged_item: workspace::StagedItemDto,
    pub copy_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct UndoHistoryResultDto {
    pub operation_id: String,
    pub operation_type: String,
    pub success: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LandlockStatusInfo {
    pub supported: bool,
    pub abi_version: Option<String>,
}

#[tauri::command]
fn get_landlock_status() -> LandlockStatusInfo {
    match sandbox::probe_landlock_abi() {
        Ok(abi) => LandlockStatusInfo {
            supported: true,
            abi_version: Some(format!("{:?}", abi)),
        },
        Err(_) => LandlockStatusInfo {
            supported: false,
            abi_version: None,
        },
    }
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_demo_items() -> Vec<models::FileItem> {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::path::PathBuf;

    // Normal UTF-8 item
    let normal_path = PathBuf::from("/home/susie/documents/sample_project.md");
    let normal_item = models::FileItem {
        item_id: "00000000-0000-0000-0000-000000000001".to_string(),
        raw_base64: models::encode_path_to_base64(&normal_path),
        display_name: "sample_project.md".to_string(),
        parent_raw_base64: Some(models::encode_path_to_base64(PathBuf::from(
            "/home/susie/documents",
        ))),
        is_dir: false,
        is_symlink: false,
        size: 2048,
        mtime_ms: 1729750000000,
        readonly: false,
        inode: 12345678,
        dev: 66305,
    };

    // Non-UTF-8 path with 0xFF, 0xFE bytes
    let invalid_bytes = b"/home/susie/legacy_data/\xff\xfe_non_utf8_archive.tar";
    let invalid_os = OsString::from_vec(invalid_bytes.to_vec());
    let invalid_path = PathBuf::from(invalid_os);
    let non_utf8_item = models::FileItem {
        item_id: "00000000-0000-0000-0000-000000000002".to_string(),
        raw_base64: models::encode_path_to_base64(&invalid_path),
        display_name: invalid_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string(),
        parent_raw_base64: Some(models::encode_path_to_base64(PathBuf::from(
            "/home/susie/legacy_data",
        ))),
        is_dir: false,
        is_symlink: false,
        size: 1048576,
        mtime_ms: 1729750000000,
        readonly: false,
        inode: 87654321,
        dev: 66305,
    };

    vec![normal_item, non_utf8_item]
}

#[tauri::command]
fn get_app_config() -> config::AppConfig {
    config::load_or_create_config()
}

#[tauri::command]
fn save_app_config(config: config::AppConfig) -> Result<(), String> {
    config::save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_system_paths() -> models::SystemPathsDto {
    models::get_system_paths()
}

#[tauri::command]
fn resolve_path(input: String) -> Result<models::ResolvedPathDto, String> {
    models::resolve_path_str_or_b64(&input).map_err(|e| e.to_string())
}

#[tauri::command]
fn scan_dir(
    path_base64: String,
    options: Option<scanner::ScanOptions>,
) -> Result<Vec<models::FileItem>, String> {
    let resolved = models::resolve_path_str_or_b64(&path_base64).map_err(|e| e.to_string())?;
    let path = std::path::PathBuf::from(&resolved.path_str);
    let opts = options.unwrap_or_default();
    scanner::scan_directory(&path, &opts).map_err(|e| e.to_string())
}

#[tauri::command]
fn search_files_by_name(
    pattern: String,
    options: Option<search_filename::SearchFilenameOptions>,
) -> Result<search_filename::SearchResultDto, String> {
    let opts = options.unwrap_or_default();
    search_filename::search_filename(&pattern, &opts).map_err(|e| e.to_string())
}

#[tauri::command]
fn search_fulltext(
    query: String,
    options: Option<search_fulltext::FulltextSearchOptions>,
) -> Result<Vec<search_fulltext::FulltextHitDto>, String> {
    let engine = search_fulltext::get_or_init_engine().map_err(|e| e.to_string())?;
    let opts = options.unwrap_or_default();
    engine.search(&query, &opts).map_err(|e| e.to_string())
}

#[tauri::command]
fn extract_file_text(
    path_base64: String,
    options: Option<extractor::ExtractTextOptions>,
) -> Result<extractor::ExtractedTextDto, String> {
    let path = models::decode_path_from_base64(&path_base64).map_err(|e| e.to_string())?;
    extractor::extract_file_text(&path, options).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_image_preview(
    path_base64: String,
    max_bytes: Option<usize>,
) -> Result<image_preview::ImagePreviewDto, String> {
    image_preview::load_image_preview_from_base64(&path_base64, max_bytes)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn index_file_with_text(
    path_base64: String,
    options: Option<extractor::ExtractTextOptions>,
) -> Result<extractor::ExtractedTextDto, String> {
    extractor::index_file_with_text(&path_base64, options).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_index_stats() -> Result<search_fulltext::IndexStatsDto, String> {
    let engine = search_fulltext::get_or_init_engine().map_err(|e| e.to_string())?;
    let path = search_fulltext::get_default_index_dir().map_err(|e| e.to_string())?;
    Ok(search_fulltext::IndexStatsDto {
        total_documents: engine.get_total_documents(),
        index_path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
async fn run_index_operation(
    options: extractor::RunIndexOptions,
) -> Result<extractor::IndexOperationResultDto, String> {
    tokio::task::spawn_blocking(move || extractor::run_index_operation(options))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn process_index_repair_queue() -> Result<history::IndexRepairResultDto, String> {
    tokio::task::spawn_blocking(search_fulltext::process_index_repair_queue)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_index_repair_queue_count() -> Result<usize, String> {
    history::get_history_manager().get_index_repair_queue_count()
}

#[tauri::command]
fn get_systemd_timer_status() -> Result<periodic_index::SystemdTimerStatusDto, String> {
    periodic_index::get_systemd_timer_status()
}

#[tauri::command]
fn install_systemd_timer() -> Result<periodic_index::SystemdTimerStatusDto, String> {
    periodic_index::install_systemd_timer()
}

#[tauri::command]
fn uninstall_systemd_timer() -> Result<periodic_index::SystemdTimerStatusDto, String> {
    periodic_index::uninstall_systemd_timer()
}

#[tauri::command]
fn check_operation_policy(
    path_base64: String,
    for_reflection: bool,
) -> Result<policy::PolicyCheckResultDto, String> {
    let path = models::decode_path_from_base64(&path_base64).map_err(|e| e.to_string())?;
    Ok(policy::OperationPolicy::check_policy(&path, for_reflection))
}

#[tauri::command]
fn create_original_snapshot(
    path_base64: String,
    compute_sha256: bool,
) -> Result<policy::OriginalSnapshotDto, String> {
    let path = models::decode_path_from_base64(&path_base64).map_err(|e| e.to_string())?;
    let item = models::FileItem::from_path(&path).map_err(|e| e.to_string())?;
    policy::OperationPolicy::create_snapshot(&item, compute_sha256).map_err(|e| e.to_string())
}

#[tauri::command]
fn verify_original_conflict(
    snapshot: policy::OriginalSnapshotDto,
    verify_sha256: bool,
) -> Result<policy::PolicyCheckResultDto, String> {
    match policy::OperationPolicy::verify_conflict(&snapshot, verify_sha256) {
        Ok(()) => Ok(policy::PolicyCheckResultDto {
            allowed: true,
            is_in_home: true,
            is_external_or_mount: false,
            can_reflect_original: true,
            conflict_detected: false,
            error_code: None,
            error_message: None,
        }),
        Err(e) => Ok(policy::PolicyCheckResultDto {
            allowed: false,
            is_in_home: true,
            is_external_or_mount: false,
            can_reflect_original: false,
            conflict_detected: true,
            error_code: Some(e.error_code().to_string()),
            error_message: Some(e.to_string()),
        }),
    }
}

#[tauri::command]
fn stage_file(path_base64: String) -> Result<workspace::StagedItemDto, String> {
    let path = models::decode_path_from_base64(&path_base64).map_err(|e| e.to_string())?;
    let mgr = workspace::get_workspace_manager();
    mgr.stage_file(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn unstage_file(stage_id: String) -> Result<(), String> {
    let mgr = workspace::get_workspace_manager();
    mgr.unstage_file(&stage_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_staged_items() -> Result<Vec<workspace::StagedItemDto>, String> {
    let mgr = workspace::get_workspace_manager();
    Ok(mgr.list_staged())
}

#[tauri::command]
fn clear_staged_items() -> Result<(), String> {
    let mgr = workspace::get_workspace_manager();
    mgr.clear_all().map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_workspace_session() -> Result<Vec<workspace::StagedItemDto>, String> {
    let mgr = workspace::get_workspace_manager();
    mgr.restore_session().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_file_diff(stage_id: String) -> Result<diff::FileDiffResultDto, String> {
    let mgr = workspace::get_workspace_manager();
    mgr.get_diff(&stage_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn reset_staged_file(stage_id: String) -> Result<workspace::StagedItemDto, String> {
    let mgr = workspace::get_workspace_manager();
    mgr.reset_staged_file(&stage_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn reflect_staged_file(
    stage_id: String,
    mode: Option<workspace::ReflectionMode>,
) -> Result<ReflectExecutionResultDto, String> {
    let mgr = workspace::get_workspace_manager();
    let target_mode = mode.unwrap_or(workspace::ReflectionMode::Overwrite);
    let (staged_item, backup_path) = mgr
        .reflect_staged_file_with_mode(&stage_id, target_mode)
        .map_err(|e| e.to_string())?;

    Ok(ReflectExecutionResultDto {
        staged_item,
        backup_path: backup_path.map(|p| p.to_string_lossy().to_string()),
        mode: target_mode,
    })
}

#[tauri::command]
fn get_origin_backup_preview(path: String) -> Result<String, String> {
    let p = std::path::Path::new(&path);
    let preview = workspace::generate_origin_backup_path(p);
    Ok(preview.to_string_lossy().to_string())
}

#[tauri::command]
fn reflect_staged_file_as_copy(
    stage_id: String,
    suffix: Option<String>,
) -> Result<ReflectAsCopyResultDto, String> {
    let mgr = workspace::get_workspace_manager();
    let (staged_item, copy_path) = mgr
        .reflect_staged_item_as_copy(&stage_id, suffix)
        .map_err(|e| e.to_string())?;

    Ok(ReflectAsCopyResultDto {
        staged_item,
        copy_path: copy_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
fn get_alternate_copy_preview(path: String, suffix: Option<String>) -> Result<String, String> {
    let p = std::path::Path::new(&path);
    let preview = workspace::generate_alternate_copy_path(p, suffix.as_deref());
    Ok(preview.to_string_lossy().to_string())
}

#[tauri::command]
fn read_staged_file_text(stage_id: String) -> Result<String, String> {
    let mgr = workspace::get_workspace_manager();
    mgr.read_staged_text(&stage_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_staged_file_text(
    stage_id: String,
    content: String,
) -> Result<workspace::StagedItemDto, String> {
    let mgr = workspace::get_workspace_manager();
    mgr.write_staged_text(&stage_id, &content)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn undo_history_operation(operation_id: String) -> Result<UndoHistoryResultDto, String> {
    let hist_mgr = history::get_history_manager();
    let op = hist_mgr
        .get_operation(&operation_id)?
        .ok_or_else(|| format!("Operation not found: {}", operation_id))?;

    if !op.can_undo {
        return Err("This operation cannot be undone or has already been undone.".to_string());
    }

    match op.operation_type.as_str() {
        "BulkRename" => {
            let res = batch::undo_batch_by_id(&operation_id)?;
            Ok(UndoHistoryResultDto {
                operation_id: operation_id.clone(),
                operation_type: op.operation_type,
                success: true,
                message: format!(
                    "Successfully undid bulk rename ({} files restored)",
                    res.succeeded_count
                ),
            })
        }
        "chmod" => {
            let path_str = op
                .original_path
                .as_deref()
                .ok_or_else(|| "Missing target path in chmod history".to_string())?;
            let path = std::path::Path::new(path_str);
            if !path.exists() {
                return Err(format!(
                    "Cannot undo chmod: file no longer exists at {}",
                    path_str
                ));
            }

            let details = op.details.as_deref().unwrap_or("");
            let octal_str = if let Some(pos) = details.find("[orig_octal:") {
                let rest = &details[pos + 12..];
                rest.trim_end_matches(']').trim()
            } else if let Some(caps) = details.strip_prefix("Mode changed: ") {
                caps.split_whitespace().next().unwrap_or("")
            } else {
                ""
            };

            let mode = chmod::parse_octal_mode(octal_str)
                .map_err(|e| format!("Failed to parse original mode '{}': {}", octal_str, e))?;

            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(mode);
            std::fs::set_permissions(path, permissions)
                .map_err(|e| format!("Failed to restore permissions to {:04o}: {}", mode, e))?;

            let _ = hist_mgr.mark_as_undone(&operation_id);
            let undo_id = format!("undo_{}", operation_id);
            let _ = hist_mgr.record_operation(&history::NewHistoryEntry {
                operation_id: undo_id,
                operation_type: "UndoChmod".to_string(),
                display_name: op.display_name.clone(),
                original_path: Some(path_str.to_string()),
                staged_path: Some(path_str.to_string()),
                inode: None,
                status: "Success".to_string(),
                can_undo: false,
                details: Some(format!(
                    "Restored permissions to {:04o} on {}",
                    mode, path_str
                )),
            });

            Ok(UndoHistoryResultDto {
                operation_id,
                operation_type: op.operation_type,
                success: true,
                message: format!(
                    "Restored permissions of '{}' to {:04o}",
                    op.display_name, mode
                ),
            })
        }
        "touch_mtime" => {
            let path_str = op
                .original_path
                .as_deref()
                .ok_or_else(|| "Missing target path in timestamp history".to_string())?;
            let path = std::path::Path::new(path_str);
            if !path.exists() {
                return Err(format!(
                    "Cannot undo timestamp: file no longer exists at {}",
                    path_str
                ));
            }

            let details = op.details.as_deref().unwrap_or("");
            let mtime_ms: i64 = if let Some(pos) = details.find("[orig_ms:") {
                let rest = &details[pos + 9..];
                let ms_str = rest.trim_end_matches(']').trim();
                ms_str
                    .parse::<i64>()
                    .map_err(|e| format!("Failed to parse mtime ms: {}", e))?
            } else {
                return Err("Missing original timestamp data in operation details".to_string());
            };

            let file_time = filetime::FileTime::from_unix_time(
                mtime_ms / 1000,
                ((mtime_ms % 1000) * 1_000_000) as u32,
            );
            filetime::set_file_mtime(path, file_time)
                .map_err(|e| format!("Failed to restore mtime on {}: {}", path_str, e))?;

            let _ = hist_mgr.mark_as_undone(&operation_id);
            let undo_id = format!("undo_{}", operation_id);
            let _ = hist_mgr.record_operation(&history::NewHistoryEntry {
                operation_id: undo_id,
                operation_type: "UndoTouchMtime".to_string(),
                display_name: op.display_name.clone(),
                original_path: Some(path_str.to_string()),
                staged_path: Some(path_str.to_string()),
                inode: None,
                status: "Success".to_string(),
                can_undo: false,
                details: Some(format!("Restored mtime to ms:{} on {}", mtime_ms, path_str)),
            });

            Ok(UndoHistoryResultDto {
                operation_id,
                operation_type: op.operation_type,
                success: true,
                message: format!("Restored modification timestamp of '{}'", op.display_name),
            })
        }
        other => Err(format!(
            "Operation type '{}' does not support selective undo",
            other
        )),
    }
}

#[tauri::command]
fn generate_bulk_rename_plan(
    options: batch::BulkRenameOptions,
) -> Result<batch::BatchPlanDto, String> {
    batch::generate_plan_from_options(options)
}

#[tauri::command]
fn execute_bulk_rename(
    plan: batch::BatchPlanDto,
) -> Result<batch::BatchExecutionResultDto, String> {
    batch::execute_batch_rename(plan)
}

#[tauri::command]
fn can_undo_bulk_rename() -> bool {
    batch::can_undo_batch()
}

#[tauri::command]
fn undo_last_bulk_rename() -> Result<batch::BatchExecutionResultDto, String> {
    batch::undo_last_batch()
}

#[tauri::command]
fn generate_vidir_text(target_items: Vec<batch::RenameTargetInputDto>) -> Result<String, String> {
    batch::generate_vidir_text(target_items)
}

#[tauri::command]
fn parse_vidir_text(
    target_items: Vec<batch::RenameTargetInputDto>,
    content: String,
) -> Result<batch::BatchPlanDto, String> {
    batch::parse_vidir_text(target_items, content)
}

#[tauri::command]
fn launch_external_editor_vidir(
    target_items: Vec<batch::RenameTargetInputDto>,
    custom_editor: Option<String>,
) -> Result<batch::BatchPlanDto, String> {
    batch::launch_external_editor(target_items, custom_editor)
}

#[tauri::command]
fn generate_chmod_plan(
    target_items: Vec<chmod::ChmodTargetInputDto>,
    spec: chmod::ChmodModeSpec,
) -> Result<chmod::ChmodPlanDto, String> {
    chmod::generate_chmod_plan(target_items, spec)
}

#[tauri::command]
fn execute_chmod_plan(plan: chmod::ChmodPlanDto) -> Result<chmod::ChmodExecutionResultDto, String> {
    chmod::execute_chmod_plan(plan)
}

#[tauri::command]
fn generate_timestamp_plan(
    target_items: Vec<timestamp::TimestampTargetInputDto>,
    mode: timestamp::TimestampModeSpec,
) -> Result<timestamp::TimestampPlanDto, String> {
    Ok(timestamp::generate_timestamp_plan(target_items, mode))
}

#[tauri::command]
fn execute_timestamp_plan(
    plan: timestamp::TimestampPlanDto,
) -> Result<timestamp::TimestampExecutionResultDto, String> {
    let hist_mgr = history::get_history_manager();
    Ok(timestamp::execute_timestamp_plan(plan, Some(hist_mgr)))
}

#[tauri::command]
fn export_workspace_tar(
    options: export_tar::TarExportOptionsDto,
) -> Result<export_tar::TarExportResultDto, String> {
    let mgr = workspace::get_workspace_manager();
    let staged = mgr.list_staged();
    let hist_mgr = history::get_history_manager();
    export_tar::export_workspace_to_tar(&staged, &options, Some(hist_mgr))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_operation_history(
    limit: Option<usize>,
) -> Result<Vec<history::OperationHistoryDto>, String> {
    let mgr = history::get_history_manager();
    mgr.list_history(limit.unwrap_or(50))
}

#[tauri::command]
fn clear_operation_history() -> Result<(), String> {
    let mgr = history::get_history_manager();
    mgr.clear_history()
}

#[tauri::command]
fn get_default_backup_path() -> Result<String, String> {
    backup::get_default_backup_path()
}

#[tauri::command]
fn create_workspace_backup(
    destination_path: Option<String>,
    auto_prune_limit: Option<usize>,
) -> Result<backup::BackupResultDto, String> {
    let mgr = workspace::get_workspace_manager();
    let staged = mgr.list_staged();
    let cfg = config::load_or_create_config();
    let limit = auto_prune_limit.unwrap_or(cfg.workspace.backup_retention_count);
    backup::create_workspace_backup(destination_path, &staged, Some(limit))
}

#[tauri::command]
fn list_workspace_backups() -> Result<Vec<backup::BackupArchiveInfoDto>, String> {
    backup::list_backups()
}

#[tauri::command]
fn delete_workspace_backup(filename: String) -> Result<(), String> {
    backup::delete_backup(&filename)
}

#[tauri::command]
fn prune_workspace_backups(keep_count: usize) -> Result<Vec<String>, String> {
    backup::prune_old_backups(keep_count)
}

#[tauri::command]
fn restore_workspace_backup(
    archive_path: Option<String>,
) -> Result<Vec<workspace::StagedItemDto>, String> {
    backup::restore_workspace_backup(archive_path)
}

#[tauri::command]
fn has_unreflected_staged_items() -> bool {
    let mgr = workspace::get_workspace_manager();
    let staged = mgr.list_staged();
    staged
        .iter()
        .any(|item| item.status != workspace::StageStatus::Reflected)
}

#[tauri::command]
fn probe_external_tools() -> Vec<pipeline::ExternalToolStatusDto> {
    pipeline::ToolDetector::probe_all()
}

#[tauri::command]
fn execute_sandboxed_command(
    executable: String,
    args: Vec<String>,
    options: Option<pipeline::CommandExecutionOptionsDto>,
) -> Result<pipeline::CommandExecutionResultDto, models::AppErrorDto> {
    let opts = options.unwrap_or_default();
    pipeline::SandboxedExecutor::execute(&executable, &args, &opts)
}

#[tauri::command]
fn run_builtin_text_transform(
    transform_id: String,
    input_text: String,
    offset_param: Option<i64>,
) -> Result<pipeline::BuiltinTransformResultDto, models::AppErrorDto> {
    match transform_id.as_str() {
        "mask_secrets" => {
            let (out, count) = pipeline::BuiltinPipelineEngine::mask_log_secrets(&input_text);
            Ok(pipeline::BuiltinTransformResultDto {
                success: true,
                output_text: out,
                items_processed: count,
                error_message: None,
            })
        }
        "clean_csv" => match pipeline::BuiltinPipelineEngine::clean_csv(input_text.as_bytes()) {
            Ok((out, rows)) => Ok(pipeline::BuiltinTransformResultDto {
                success: true,
                output_text: out,
                items_processed: rows,
                error_message: None,
            }),
            Err(e) => Err(models::AppErrorDto::new("ERR_CLEAN_CSV_FAILED", e)),
        },
        "minify_json" => match pipeline::BuiltinPipelineEngine::minify_json(&input_text) {
            Ok((out, count)) => Ok(pipeline::BuiltinTransformResultDto {
                success: true,
                output_text: out,
                items_processed: count,
                error_message: None,
            }),
            Err(e) => Err(models::AppErrorDto::new("ERR_MINIFY_JSON_FAILED", e)),
        },
        "tsv_to_csv" => match pipeline::BuiltinPipelineEngine::convert_tsv_csv(&input_text, true) {
            Ok((out, rows)) => Ok(pipeline::BuiltinTransformResultDto {
                success: true,
                output_text: out,
                items_processed: rows,
                error_message: None,
            }),
            Err(e) => Err(models::AppErrorDto::new("ERR_CONVERT_TSV_CSV_FAILED", e)),
        },
        "csv_to_tsv" => {
            match pipeline::BuiltinPipelineEngine::convert_tsv_csv(&input_text, false) {
                Ok((out, rows)) => Ok(pipeline::BuiltinTransformResultDto {
                    success: true,
                    output_text: out,
                    items_processed: rows,
                    error_message: None,
                }),
                Err(e) => Err(models::AppErrorDto::new("ERR_CONVERT_CSV_TSV_FAILED", e)),
            }
        }
        "minify_sql" => {
            let (out, count) = pipeline::BuiltinPipelineEngine::minify_sql_dump(&input_text);
            Ok(pipeline::BuiltinTransformResultDto {
                success: true,
                output_text: out,
                items_processed: count,
                error_message: None,
            })
        }
        "shift_srt" => {
            let offset = offset_param.unwrap_or(1000);
            match pipeline::BuiltinPipelineEngine::shift_srt_timecodes(&input_text, offset) {
                Ok((out, count)) => Ok(pipeline::BuiltinTransformResultDto {
                    success: true,
                    output_text: out,
                    items_processed: count,
                    error_message: None,
                }),
                Err(e) => Err(models::AppErrorDto::new("ERR_SHIFT_SRT_FAILED", e)),
            }
        }
        _ => Err(models::AppErrorDto::new(
            "ERR_UNKNOWN_TRANSFORM",
            format!("Unknown builtin transform: {}", transform_id),
        )),
    }
}

#[tauri::command]
fn get_pipeline_presets() -> Vec<pipeline::PipelinePresetDto> {
    pipeline::PipelineRegistry::get_all_presets()
}

#[tauri::command]
fn cancel_pipeline(job_id: String) -> Result<bool, models::AppErrorDto> {
    let cancel_mgr = pipeline::get_cancellation_manager();
    let cancelled = cancel_mgr.cancel_job(&job_id);
    Ok(cancelled)
}

#[tauri::command]
async fn execute_pipeline_preset(
    app: tauri::AppHandle,
    request: pipeline::ExecutePipelineRequestDto,
) -> Result<pipeline::PipelineExecutionResultDto, models::AppErrorDto> {
    use tauri::Emitter;

    let mgr = workspace::get_workspace_manager();
    let app_handle = app.clone();

    // Run blocking workload on Tokio threadpool so the command thread is not blocked, allowing cancellation requests
    let res = tokio::task::spawn_blocking(move || {
        let callback = Box::new(move |event: pipeline::PipelineProgressEvent| {
            let _ = app_handle.emit("pipeline-progress", event);
        });
        pipeline::PipelineRunner::execute_with_progress(&request, &mgr, Some(callback))
    })
    .await
    .map_err(|e| models::AppErrorDto::new("TaskJoinError", e.to_string()))??;

    // Record into SQLite persistent history
    let hist_mgr = history::get_history_manager();
    let _ = hist_mgr.record_operation(&history::NewHistoryEntry {
        operation_id: uuid::Uuid::new_v4().to_string(),
        operation_type: "Pipeline".to_string(),
        display_name: format!("Preset: {}", res.preset_id),
        original_path: None,
        staged_path: None,
        inode: None,
        status: if res.is_cancelled {
            "Cancelled".to_string()
        } else if res.failure_count == 0 {
            "Success".to_string()
        } else {
            "Failed".to_string()
        },
        can_undo: false,
        details: Some(format!(
            "Executed on {} items ({} success, {} failed, cancelled: {})",
            res.total_count, res.success_count, res.failure_count, res.is_cancelled
        )),
    });

    Ok(res)
}

#[tauri::command]
fn validate_custom_pipeline_dry_run(
    pipeline: config::CustomPipelineDefDto,
    stage_ids: Vec<String>,
) -> Result<pipeline::CustomPipelineDryRunResultDto, models::AppErrorDto> {
    let mgr = workspace::get_workspace_manager();
    pipeline::PipelineRunner::dry_run_custom_pipeline(&pipeline, &stage_ids, &mgr)
}

#[tauri::command]
async fn execute_custom_pipeline(
    app: tauri::AppHandle,
    pipeline: config::CustomPipelineDefDto,
    stage_ids: Vec<String>,
    job_id: Option<String>,
) -> Result<pipeline::PipelineExecutionResultDto, models::AppErrorDto> {
    use tauri::Emitter;

    let mgr = workspace::get_workspace_manager();
    let app_handle = app.clone();
    let pipeline_clone = pipeline.clone();

    let res = tokio::task::spawn_blocking(move || {
        let callback = Box::new(move |event: pipeline::PipelineProgressEvent| {
            let _ = app_handle.emit("pipeline-progress", event);
        });
        pipeline::PipelineRunner::execute_custom_pipeline_with_progress(
            &pipeline_clone,
            &stage_ids,
            &mgr,
            job_id,
            Some(callback),
        )
    })
    .await
    .map_err(|e| models::AppErrorDto::new("TaskJoinError", e.to_string()))??;

    // Record into SQLite persistent history
    let hist_mgr = history::get_history_manager();
    let _ = hist_mgr.record_operation(&history::NewHistoryEntry {
        operation_id: uuid::Uuid::new_v4().to_string(),
        operation_type: "CustomPipeline".to_string(),
        display_name: format!("Custom: {}", pipeline.name),
        original_path: None,
        staged_path: None,
        inode: None,
        status: if res.is_cancelled {
            "Cancelled".to_string()
        } else if res.failure_count == 0 {
            "Success".to_string()
        } else {
            "Failed".to_string()
        },
        can_undo: false,
        details: Some(format!(
            "Executed custom pipeline '{}' on {} items ({} success, {} failed)",
            pipeline.name, res.total_count, res.success_count, res.failure_count
        )),
    });

    Ok(res)
}

#[tauri::command]
fn exit_cuddle_app(app_handle: tauri::AppHandle) {
    app_handle.exit(0);
}

static GUI_LOCK: std::sync::Mutex<Option<periodic_index::AppLock>> = std::sync::Mutex::new(None);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(res) = periodic_index::handle_cli_args(&args) {
        match res {
            Ok(()) => std::process::exit(0),
            Err(e) => {
                eprintln!("[Cuddle Error] {}", e);
                std::process::exit(1);
            }
        }
    }

    // Acquire GUI lock so background periodic indexers skip when GUI is active
    if let Ok(Some(lock)) = periodic_index::try_acquire_lock() {
        *GUI_LOCK.lock().unwrap() = Some(lock);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            exit_cuddle_app,
            greet,
            get_landlock_status,
            get_demo_items,
            get_app_config,
            save_app_config,
            get_system_paths,
            resolve_path,
            scan_dir,
            search_files_by_name,
            search_fulltext,
            extract_file_text,
            load_image_preview,
            index_file_with_text,
            get_index_stats,
            run_index_operation,
            process_index_repair_queue,
            get_index_repair_queue_count,
            get_systemd_timer_status,
            install_systemd_timer,
            uninstall_systemd_timer,
            check_operation_policy,
            create_original_snapshot,
            verify_original_conflict,
            stage_file,
            unstage_file,
            list_staged_items,
            clear_staged_items,
            restore_workspace_session,
            get_file_diff,
            reset_staged_file,
            reflect_staged_file,
            reflect_staged_file_as_copy,
            read_staged_file_text,
            write_staged_file_text,
            get_alternate_copy_preview,
            get_origin_backup_preview,
            generate_bulk_rename_plan,
            execute_bulk_rename,
            can_undo_bulk_rename,
            undo_last_bulk_rename,
            undo_history_operation,
            generate_vidir_text,
            parse_vidir_text,
            launch_external_editor_vidir,
            generate_chmod_plan,
            execute_chmod_plan,
            generate_timestamp_plan,
            execute_timestamp_plan,
            export_workspace_tar,
            get_operation_history,
            clear_operation_history,
            get_default_backup_path,
            create_workspace_backup,
            list_workspace_backups,
            delete_workspace_backup,
            prune_workspace_backups,
            restore_workspace_backup,
            has_unreflected_staged_items,
            probe_external_tools,
            execute_sandboxed_command,
            run_builtin_text_transform,
            get_pipeline_presets,
            cancel_pipeline,
            execute_pipeline_preset,
            validate_custom_pipeline_dry_run,
            execute_custom_pipeline,
            notify::notify_user,
            git::get_git_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        assert_eq!(
            greet("World"),
            "Hello, World! You've been greeted from Rust!"
        );
    }

    #[test]
    fn test_export_typescript_bindings_lib() {
        let _ = std::fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        ReflectExecutionResultDto::export(&cfg)
            .expect("Failed to export ReflectExecutionResultDto");
        ReflectAsCopyResultDto::export(&cfg).expect("Failed to export ReflectAsCopyResultDto");
        UndoHistoryResultDto::export(&cfg).expect("Failed to export UndoHistoryResultDto");
        notify::NotificationLevel::export(&cfg).expect("Failed to export NotificationLevel");
    }

    #[test]
    fn test_undo_history_chmod_and_touch() {
        let temp_dir =
            std::env::temp_dir().join(format!("cuddle_undo_lib_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let file_path = temp_dir.join("chmod_test.txt");
        std::fs::write(&file_path, "hello chmod").unwrap();

        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o644);
        std::fs::set_permissions(&file_path, perms).unwrap();

        // 1. Chmod to 0755
        let plan = chmod::generate_chmod_plan(
            vec![chmod::ChmodTargetInputDto {
                item_id: "chmod-1".to_string(),
                path_or_base64: file_path.to_string_lossy().to_string(),
                display_name: "chmod_test.txt".to_string(),
            }],
            chmod::ChmodModeSpec::Octal("0755".to_string()),
        )
        .unwrap();
        let res = chmod::execute_chmod_plan(plan).unwrap();
        assert!(res.success);

        // Verify disk has 0755
        let meta = std::fs::metadata(&file_path).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o755);

        // Find the operation in history
        let hist_mgr = history::get_history_manager();
        let list = hist_mgr.list_history(5).unwrap();
        let chmod_op = list
            .iter()
            .find(|op| op.operation_type == "chmod" && op.can_undo)
            .expect("Chmod operation must be present and undoable");

        // Undo chmod operation
        let undo_res = undo_history_operation(chmod_op.operation_id.clone()).unwrap();
        assert!(undo_res.success);

        // Verify disk is restored to 0644
        let meta_restored = std::fs::metadata(&file_path).unwrap();
        assert_eq!(meta_restored.permissions().mode() & 0o777, 0o644);

        // Cannot undo again
        assert!(undo_history_operation(chmod_op.operation_id.clone()).is_err());

        // 2. Touch mtime
        let past_time_str = "2020-01-01 12:00:00";
        let time_plan = timestamp::generate_timestamp_plan(
            vec![timestamp::TimestampTargetInputDto {
                item_id: "time-1".to_string(),
                path_or_base64: file_path.to_string_lossy().to_string(),
                display_name: "chmod_test.txt".to_string(),
            }],
            timestamp::TimestampModeSpec::Absolute(past_time_str.to_string()),
        );
        let time_res = timestamp::execute_timestamp_plan(time_plan, Some(hist_mgr));
        assert!(time_res.success);

        let list_time = hist_mgr.list_history(5).unwrap();
        let time_op = list_time
            .iter()
            .find(|op| op.operation_type == "touch_mtime" && op.can_undo)
            .expect("Touch mtime operation must be present and undoable");

        let undo_time_res = undo_history_operation(time_op.operation_id.clone()).unwrap();
        assert!(undo_time_res.success);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
