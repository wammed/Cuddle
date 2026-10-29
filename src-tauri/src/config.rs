use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use ts_rs::TS;

const APP_DIR_NAME: &str = "Cuddle";
const CONFIG_FILE_NAME: &str = "config.toml";
pub const DEFAULT_MIN_FREE_SPACE_MULTIPLIER: f64 = 1.5;
pub const DEFAULT_BACKUP_RETENTION_COUNT: usize = 5;
pub const MAX_BACKUP_RETENTION_COUNT: usize = 10;
pub const MIN_BACKUP_RETENTION_COUNT: usize = 1;

fn default_backup_retention() -> usize {
    DEFAULT_BACKUP_RETENTION_COUNT
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("Config directory could not be resolved")]
    PathResolutionFailed,
}

/// Root configuration structure for Cuddle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct AppConfig {
    pub app: AppSection,
    pub workspace: WorkspaceSection,
    pub security: SecuritySection,
    #[serde(default = "get_default_custom_pipeline_templates")]
    pub custom_pipelines: Vec<CustomPipelineDefDto>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app: AppSection::default(),
            workspace: WorkspaceSection::default(),
            security: SecuritySection::default(),
            custom_pipelines: get_default_custom_pipeline_templates(),
        }
    }
}

/// Execution input mode for pipeline tasks (Design Section 8.1, Task 4-1).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
#[serde(rename_all = "snake_case")]
pub enum ExecutionInputMode {
    PerFile,
    BatchList,
    StandardInput,
}

impl Default for ExecutionInputMode {
    fn default() -> Self {
        Self::PerFile
    }
}

/// User-defined pipeline definition following Design Section 8.3 mandatory safe flow.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct CustomPipelineDefDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub executable: String,
    pub args: Vec<String>,
    pub supported_extensions: Vec<String>,
    pub output_extension: Option<String>,
    pub default_suffix: String,
    /// Additional read-only paths allowed under Landlock LSM (Task 4-2)
    #[serde(default)]
    pub allowed_read_paths: Vec<String>,
    /// Input execution mode: per_file, batch_list, standard_input (Task 4-1)
    #[serde(default)]
    pub input_mode: ExecutionInputMode,
}

pub fn get_default_custom_pipeline_templates() -> Vec<CustomPipelineDefDto> {
    vec![
        CustomPipelineDefDto {
            id: "custom-webp-lossless".to_string(),
            name: "WebP 可逆圧縮カスタムコンバータ".to_string(),
            description: "cwebp を使用して PNG/JPEG を WebP に高品質変換".to_string(),
            executable: "cwebp".to_string(),
            args: vec![
                "-q".to_string(),
                "85".to_string(),
                "{input}".to_string(),
                "-o".to_string(),
                "{output}".to_string(),
            ],
            supported_extensions: vec!["png".to_string(), "jpg".to_string(), "jpeg".to_string()],
            output_extension: Some("webp".to_string()),
            default_suffix: "_web".to_string(),
            allowed_read_paths: Vec::new(),
            input_mode: ExecutionInputMode::PerFile,
        },
        CustomPipelineDefDto {
            id: "custom-strip-metadata".to_string(),
            name: "ExifTool メタデータ完全消去".to_string(),
            description: "exiftool を用いて全 Exif/GPS メタデータを消去".to_string(),
            executable: "exiftool".to_string(),
            args: vec![
                "-all=".to_string(),
                "-overwrite_original".to_string(),
                "{input}".to_string(),
            ],
            supported_extensions: vec![
                "jpg".to_string(),
                "jpeg".to_string(),
                "png".to_string(),
                "tiff".to_string(),
                "webp".to_string(),
            ],
            output_extension: None,
            default_suffix: "_clean".to_string(),
            allowed_read_paths: Vec::new(),
            input_mode: ExecutionInputMode::PerFile,
        },
    ]
}

/// Safety confirmation level for operations and original reflections (Task 3-1).
/// Note: "brief" (skip confirmation) is intentionally omitted to uphold Zero Data Loss.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationLevel {
    /// Standard confirmation dialogs with summary validation
    Standard,
    /// Strict confirmation with full SHA-256 pre-verification and mandatory review
    Strict,
}

impl Default for ConfirmationLevel {
    fn default() -> Self {
        Self::Standard
    }
}

/// General application preferences.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct AppSection {
    /// Language code: "auto", "ja", "en"
    pub language: String,
    /// Theme preference: "dark", "light", "system", or 30 theme keys
    pub theme: String,
    /// Information density preference: "compact", "normal", "spacious"
    #[serde(default)]
    pub density: Option<String>,
}

impl Default for AppSection {
    fn default() -> Self {
        Self {
            language: "auto".to_string(),
            theme: "dark".to_string(),
            density: Some("normal".to_string()),
        }
    }
}

/// Workspace and original protection settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct WorkspaceSection {
    /// Custom temporary workspace directory (None uses same-volume preferred path)
    pub temp_directory: Option<String>,
    /// Minimum disk free space required relative to original file size (default: 1.5x)
    pub min_free_space_multiplier: f64,
    /// Number of recent workspace backup archives to retain (default: 5, min: 1, max: 10)
    #[serde(default = "default_backup_retention")]
    pub backup_retention_count: usize,
}

impl Default for WorkspaceSection {
    fn default() -> Self {
        Self {
            temp_directory: None,
            min_free_space_multiplier: DEFAULT_MIN_FREE_SPACE_MULTIPLIER,
            backup_retention_count: DEFAULT_BACKUP_RETENTION_COUNT,
        }
    }
}

/// Mandatory security settings and Landlock LSM enforcement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct SecuritySection {
    /// Enforce Landlock LSM kernel isolation.
    /// Safety Guardrail: Even if edited to false in TOML, Cuddle will sanitize to true.
    pub enforce_landlock: bool,
    /// Confirmation level: standard | strict (Task 3-1)
    #[serde(default)]
    pub confirmation_level: ConfirmationLevel,
}

impl Default for SecuritySection {
    fn default() -> Self {
        Self {
            enforce_landlock: true,
            confirmation_level: ConfirmationLevel::Standard,
        }
    }
}

/// Returns the standard Linux XDG config file path:
/// `$XDG_CONFIG_HOME/Cuddle/config.toml` (or `~/.config/Cuddle/config.toml`).
pub fn get_config_path() -> Result<PathBuf, ConfigError> {
    let base = dirs::config_dir().ok_or(ConfigError::PathResolutionFailed)?;
    Ok(base.join(APP_DIR_NAME).join(CONFIG_FILE_NAME))
}

/// Validates and applies mandatory safety guardrails to AppConfig.
/// - enforce_landlock is strictly forced to `true`.
/// - min_free_space_multiplier cannot be lower than 1.5 (falls back to 1.5).
pub fn validate_and_sanitize(mut config: AppConfig) -> AppConfig {
    if !config.security.enforce_landlock {
        eprintln!("[Cuddle Guardrail] Landlock enforcement cannot be disabled. Automatically restored to true.");
        config.security.enforce_landlock = true;
    }

    if config.workspace.min_free_space_multiplier < DEFAULT_MIN_FREE_SPACE_MULTIPLIER {
        eprintln!(
            "[Cuddle Guardrail] min_free_space_multiplier ({:.2}) is below safe threshold. Automatically restored to {:.2}.",
            config.workspace.min_free_space_multiplier,
            DEFAULT_MIN_FREE_SPACE_MULTIPLIER
        );
        config.workspace.min_free_space_multiplier = DEFAULT_MIN_FREE_SPACE_MULTIPLIER;
    }

    config.workspace.backup_retention_count = config
        .workspace
        .backup_retention_count
        .clamp(MIN_BACKUP_RETENTION_COUNT, MAX_BACKUP_RETENTION_COUNT);

    config
}

/// Loads the application configuration or creates a default one if absent or corrupted.
pub fn load_or_create_config() -> AppConfig {
    match get_config_path() {
        Ok(path) => load_or_create_config_from_path(&path),
        Err(err) => {
            eprintln!(
                "[Cuddle Config] Failed to resolve config path: {}. Using defaults.",
                err
            );
            AppConfig::default()
        }
    }
}

/// Loads config from a specific path. Handles missing files, corruption self-healing (.bak),
/// and mandatory safety guardrail sanitation.
pub fn load_or_create_config_from_path(path: &Path) -> AppConfig {
    if !path.exists() {
        let default_config = AppConfig::default();
        if let Err(e) = save_config_to_path(path, &default_config) {
            eprintln!(
                "[Cuddle Config] Could not save default config to {:?}: {}",
                path, e
            );
        }
        return default_config;
    }

    match fs::read_to_string(path) {
        Ok(contents) => match toml::from_str::<AppConfig>(&contents) {
            Ok(loaded) => validate_and_sanitize(loaded),
            Err(parse_err) => {
                eprintln!(
                    "[Cuddle Config] Corrupted TOML at {:?}: {}. Creating .bak and restoring defaults.",
                    path, parse_err
                );
                backup_and_recreate_default(path)
            }
        },
        Err(read_err) => {
            eprintln!(
                "[Cuddle Config] Could not read {:?}: {}. Using defaults.",
                path, read_err
            );
            AppConfig::default()
        }
    }
}

/// Saves the given config to the standard configuration path.
pub fn save_config(config: &AppConfig) -> Result<(), ConfigError> {
    let path = get_config_path()?;
    save_config_to_path(&path, config)
}

/// Saves the given config to the specified path after applying safety guardrails.
pub fn save_config_to_path(path: &Path, config: &AppConfig) -> Result<(), ConfigError> {
    let sanitized = validate_and_sanitize(config.clone());
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    let toml_string = toml::to_string_pretty(&sanitized)?;
    fs::write(path, toml_string)?;
    Ok(())
}

fn backup_and_recreate_default(path: &Path) -> AppConfig {
    let bak_path = path.with_extension("toml.bak");
    let _ = fs::copy(path, &bak_path);
    let default_config = AppConfig::default();
    let _ = save_config_to_path(path, &default_config);
    default_config
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_test_path(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("cuddle_config_test_{}_{}", prefix, nanos))
            .join("config.toml")
    }

    #[test]
    fn test_default_config_creation_and_roundtrip() {
        let path = temp_test_path("roundtrip");
        let loaded = load_or_create_config_from_path(&path);

        assert_eq!(loaded, AppConfig::default());
        assert!(path.exists());

        // Read file contents directly and verify valid TOML
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("[app]"));
        assert!(raw.contains("theme = \"dark\""));
        assert!(raw.contains("enforce_landlock = true"));

        // Clean up
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn test_corrupted_toml_self_healing_and_backup() {
        let path = temp_test_path("corrupt");
        let parent = path.parent().unwrap();
        fs::create_dir_all(parent).unwrap();

        // Write corrupt TOML
        fs::write(&path, "[[broken toml\nkey = ==").unwrap();

        let loaded = load_or_create_config_from_path(&path);
        assert_eq!(loaded, AppConfig::default());

        // Verify .bak file was created
        let bak_path = path.with_extension("toml.bak");
        assert!(bak_path.exists());
        let bak_content = fs::read_to_string(&bak_path).unwrap();
        assert!(bak_content.contains("[[broken toml"));

        // Clean up
        let _ = fs::remove_dir_all(parent);
    }

    #[test]
    fn test_safety_guardrails_on_landlock_and_disk_multiplier() {
        let path = temp_test_path("guardrail");
        let parent = path.parent().unwrap();
        fs::create_dir_all(parent).unwrap();

        // Deliberately dangerous TOML: disable Landlock, set multiplier to 0.1, retention to 999
        let dangerous_toml = r#"
[app]
language = "en"
theme = "light"

[workspace]
min_free_space_multiplier = 0.1
backup_retention_count = 999

[security]
enforce_landlock = false
"#;
        fs::write(&path, dangerous_toml).unwrap();

        let loaded = load_or_create_config_from_path(&path);

        // Verify safety guardrails intervened
        assert_eq!(loaded.app.language, "en");
        assert_eq!(loaded.app.theme, "light");
        assert!(
            loaded.security.enforce_landlock,
            "Landlock must be forced to true!"
        );
        assert_eq!(
            loaded.workspace.min_free_space_multiplier, DEFAULT_MIN_FREE_SPACE_MULTIPLIER,
            "Multiplier must be forced to safe default!"
        );
        assert_eq!(
            loaded.workspace.backup_retention_count, MAX_BACKUP_RETENTION_COUNT,
            "Retention count must be clamped to max 10!"
        );
        assert_eq!(
            loaded.security.confirmation_level,
            ConfirmationLevel::Standard,
            "Default confirmation level must be standard!"
        );

        // Clean up
        let _ = fs::remove_dir_all(parent);
    }

    #[test]
    fn test_custom_pipeline_def_dto_phase12_fields() {
        // Test default deserialization when fields are omitted in TOML
        let toml_str = r#"
id = "custom-test"
name = "Test Pipeline"
description = "Test Description"
executable = "echo"
args = ["{input}"]
supported_extensions = ["txt"]
default_suffix = "_tested"
"#;
        let def: CustomPipelineDefDto = toml::from_str(toml_str).unwrap();
        assert_eq!(def.allowed_read_paths, Vec::<String>::new());
        assert_eq!(def.input_mode, ExecutionInputMode::PerFile);

        // Test explicit fields
        let toml_explicit = r#"
id = "custom-batch"
name = "Batch Pipeline"
description = "Batch Description"
executable = "tar"
args = ["-cf", "archive.tar", "{InputList}"]
supported_extensions = ["*"]
default_suffix = ""
allowed_read_paths = ["/etc/hosts", "/opt/data"]
input_mode = "batch_list"
"#;
        let def_explicit: CustomPipelineDefDto = toml::from_str(toml_explicit).unwrap();
        assert_eq!(
            def_explicit.allowed_read_paths,
            vec!["/etc/hosts", "/opt/data"]
        );
        assert_eq!(def_explicit.input_mode, ExecutionInputMode::BatchList);
    }

    #[test]
    fn test_export_typescript_bindings_config() {
        let _ = fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        let _ = AppConfig::export(&cfg);
        let _ = AppSection::export(&cfg);
        let _ = WorkspaceSection::export(&cfg);
        let _ = SecuritySection::export(&cfg);
        let _ = ConfirmationLevel::export(&cfg);
        let _ = CustomPipelineDefDto::export(&cfg);
        let _ = ExecutionInputMode::export(&cfg);
    }
}
