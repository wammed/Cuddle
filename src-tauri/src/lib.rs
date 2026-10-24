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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, get_landlock_status])
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

