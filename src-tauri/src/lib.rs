pub mod config;
pub mod models;
pub mod sandbox;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
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
        parent_raw_base64: Some(models::encode_path_to_base64(PathBuf::from("/home/susie/documents"))),
        is_dir: false,
        is_symlink: false,
        size: 2048,
        mtime_ms: 1729750000000,
        readonly: false,
    };

    // Non-UTF-8 path with 0xFF, 0xFE bytes
    let invalid_bytes = b"/home/susie/legacy_data/\xff\xfe_non_utf8_archive.tar";
    let invalid_os = OsString::from_vec(invalid_bytes.to_vec());
    let invalid_path = PathBuf::from(invalid_os);
    let non_utf8_item = models::FileItem {
        item_id: "00000000-0000-0000-0000-000000000002".to_string(),
        raw_base64: models::encode_path_to_base64(&invalid_path),
        display_name: invalid_path.file_name().unwrap().to_string_lossy().to_string(),
        parent_raw_base64: Some(models::encode_path_to_base64(PathBuf::from("/home/susie/legacy_data"))),
        is_dir: false,
        is_symlink: false,
        size: 1048576,
        mtime_ms: 1729750000000,
        readonly: false,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_landlock_status,
            get_demo_items,
            get_app_config,
            save_app_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        assert_eq!(greet("World"), "Hello, World! You've been greeted from Rust!");
    }
}

