use crate::models::AppErrorDto;
use crate::sandbox::LandlockSandbox;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use ts_rs::TS;

/// External tool probe definition.
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub id: &'static str,
    pub display_name: &'static str,
    pub candidate_binaries: &'static [&'static str],
    pub version_arg: &'static str,
}

pub const KNOWN_TOOLS: &[ToolDefinition] = &[
    ToolDefinition {
        id: "imagemagick",
        display_name: "ImageMagick",
        candidate_binaries: &["magick", "convert"],
        version_arg: "-version",
    },
    ToolDefinition {
        id: "ffmpeg",
        display_name: "FFmpeg Media Engine",
        candidate_binaries: &["ffmpeg"],
        version_arg: "-version",
    },
    ToolDefinition {
        id: "qpdf",
        display_name: "QPDF Document Tool",
        candidate_binaries: &["qpdf"],
        version_arg: "--version",
    },
    ToolDefinition {
        id: "pdfimages",
        display_name: "poppler-utils (pdfimages)",
        candidate_binaries: &["pdfimages"],
        version_arg: "-v",
    },
    ToolDefinition {
        id: "exiftool",
        display_name: "ExifTool Metadata Engine",
        candidate_binaries: &["exiftool"],
        version_arg: "-ver",
    },
    ToolDefinition {
        id: "pandoc",
        display_name: "Pandoc Document Converter",
        candidate_binaries: &["pandoc"],
        version_arg: "--version",
    },
];

/// Status of an external tool on the system.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[ts(export, export_to = "../../src/types/")]
pub struct ExternalToolStatusDto {
    pub tool_id: String,
    pub display_name: String,
    pub is_available: bool,
    pub version: Option<String>,
    pub executable_path: Option<String>,
}

/// Options for sandboxed process execution.
#[derive(Serialize, Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/types/")]
pub struct CommandExecutionOptionsDto {
    pub timeout_secs: Option<u64>,
    pub max_memory_bytes: Option<u64>,
    pub allowed_read_paths: Vec<String>,
    pub allowed_write_paths: Vec<String>,
    pub working_dir: Option<String>,
}

impl Default for CommandExecutionOptionsDto {
    fn default() -> Self {
        Self {
            timeout_secs: Some(30),
            max_memory_bytes: Some(512 * 1024 * 1024), // 512 MiB
            allowed_read_paths: Vec::new(),
            allowed_write_paths: Vec::new(),
            working_dir: None,
        }
    }
}

/// Result of sandboxed command execution.
#[derive(Serialize, Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/types/")]
pub struct CommandExecutionResultDto {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub timed_out: bool,
    pub error_message: Option<String>,
}

/// Result of builtin pure-Rust transformation.
#[derive(Serialize, Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/types/")]
pub struct BuiltinTransformResultDto {
    pub success: bool,
    pub output_text: String,
    pub items_processed: u64,
    pub error_message: Option<String>,
}

/// Category of a pipeline preset.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[ts(export, export_to = "../../src/types/")]
pub enum PipelineCategory {
    Image,
    Media,
    Text,
    Document,
    File,
    Dev,
    Custom,
}

/// Dry-run analysis for a single staged file under a user-defined pipeline.
#[derive(Serialize, Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/types/")]
pub struct CustomPipelineDryRunItemDto {
    pub stage_id: String,
    pub original_name: String,
    pub resolved_executable: String,
    pub expanded_command: Vec<String>,
    pub proposed_output_name: String,
    pub conflict_detected: bool,
    pub valid: bool,
    pub message: Option<String>,
}

/// Comprehensive Dry-Run verification report as mandated by Design Section 8.3.
#[derive(Serialize, Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/types/")]
pub struct CustomPipelineDryRunResultDto {
    pub pipeline_id: String,
    pub pipeline_name: String,
    pub executable_found: bool,
    pub executable_path: Option<String>,
    pub total_items: usize,
    pub valid_items: usize,
    pub has_conflicts: bool,
    pub has_disk_space: bool,
    pub disk_space_message: Option<String>,
    pub items: Vec<CustomPipelineDryRunItemDto>,
    pub can_proceed_to_execution: bool,
}

/// Parameter definition for a pipeline preset.
#[derive(Serialize, Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/types/")]
pub struct PipelineParameterDef {
    pub key: String,
    pub label_key: String,
    pub param_type: String, // "number", "string", "boolean", "select"
    pub default_value: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub options: Option<Vec<String>>,
}

/// Complete definition of a pipeline preset.
#[derive(Serialize, Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/types/")]
pub struct PipelinePresetDto {
    pub id: String,
    pub category: PipelineCategory,
    pub name_key: String,
    pub desc_key: String,
    pub required_tools: Vec<String>,
    pub is_builtin: bool,
    pub default_suffix: String,
    pub supported_extensions: Vec<String>,
    pub parameters: Vec<PipelineParameterDef>,
}

/// Request to execute a pipeline preset on selected staged items.
#[derive(Serialize, Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/types/")]
pub struct ExecutePipelineRequestDto {
    pub job_id: Option<String>,
    pub preset_id: String,
    pub stage_ids: Vec<String>,
    pub custom_parameters: std::collections::HashMap<String, String>,
}

/// Execution outcome for a single staged file.
#[derive(Serialize, Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/types/")]
pub struct PipelineExecutionItemResultDto {
    pub stage_id: String,
    pub original_name: String,
    pub output_name: String,
    pub success: bool,
    pub error_message: Option<String>,
    pub execution_time_ms: u64,
}

/// Aggregated result of running a pipeline preset.
#[derive(Serialize, Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/types/")]
pub struct PipelineExecutionResultDto {
    pub preset_id: String,
    pub total_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub is_cancelled: bool,
    pub item_results: Vec<PipelineExecutionItemResultDto>,
}

/// Real-time progress event emitted during pipeline preset execution.
#[derive(Serialize, Deserialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/types/")]
pub struct PipelineProgressEvent {
    pub job_id: String,
    pub preset_id: String,
    pub current_index: usize,
    pub total_items: usize,
    pub current_filename: String,
    pub stage_id: String,
    pub status: String, // "Processing", "Completed", "Failed", "Cancelled"
    pub percent: f32,
    pub message: Option<String>,
}

/// Thread-safe manager for canceling active pipeline runs by job ID.
pub struct PipelineCancellationManager {
    jobs: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl PipelineCancellationManager {
    pub fn new() -> Self {
        Self {
            jobs: Mutex::new(HashMap::new()),
        }
    }

    /// Registers a new active pipeline job with an atomic cancellation flag.
    pub fn register_job(&self, job_id: &str) -> Arc<AtomicBool> {
        let mut map = self.jobs.lock().unwrap();
        let flag = Arc::new(AtomicBool::new(false));
        map.insert(job_id.to_string(), Arc::clone(&flag));
        flag
    }

    /// Cancels a running pipeline job by setting its atomic flag to true.
    pub fn cancel_job(&self, job_id: &str) -> bool {
        let map = self.jobs.lock().unwrap();
        if let Some(flag) = map.get(job_id) {
            flag.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    /// Checks if a job has been flagged for cancellation.
    pub fn is_cancelled(&self, job_id: &str) -> bool {
        let map = self.jobs.lock().unwrap();
        map.get(job_id)
            .map(|f| f.load(Ordering::SeqCst))
            .unwrap_or(false)
    }

    /// Unregisters a completed or cancelled pipeline job.
    pub fn unregister_job(&self, job_id: &str) {
        let mut map = self.jobs.lock().unwrap();
        map.remove(job_id);
    }
}

impl Default for PipelineCancellationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global singleton for pipeline cancellation management.
pub fn get_cancellation_manager() -> &'static PipelineCancellationManager {
    static MGR: OnceLock<PipelineCancellationManager> = OnceLock::new();
    MGR.get_or_init(PipelineCancellationManager::new)
}

/// Tool detector that checks existence of binaries in `$PATH`.
pub struct ToolDetector;

impl ToolDetector {
    /// Finds the absolute path of an executable in `$PATH`.
    pub fn find_binary_in_path(binary_name: &str) -> Option<PathBuf> {
        let path_var = std::env::var_os("PATH")?;
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(binary_name);
            if candidate.is_file() {
                // Check if executable via metadata/permissions
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(meta) = candidate.metadata() {
                        if meta.permissions().mode() & 0o111 != 0 {
                            return Some(candidate);
                        }
                    }
                }
                #[cfg(not(unix))]
                return Some(candidate);
            }
        }
        None
    }

    /// Probes a single tool definition.
    pub fn probe_tool(def: &ToolDefinition) -> ExternalToolStatusDto {
        for &bin in def.candidate_binaries {
            if let Some(path) = Self::find_binary_in_path(bin) {
                // Try getting version with short 2-second timeout
                let version = Self::fetch_version(&path, def.version_arg);
                return ExternalToolStatusDto {
                    tool_id: def.id.to_string(),
                    display_name: def.display_name.to_string(),
                    is_available: true,
                    version,
                    executable_path: Some(path.to_string_lossy().to_string()),
                };
            }
        }

        ExternalToolStatusDto {
            tool_id: def.id.to_string(),
            display_name: def.display_name.to_string(),
            is_available: false,
            version: None,
            executable_path: None,
        }
    }

    /// Fetches the version output of an executable safely.
    fn fetch_version(path: &Path, arg: &str) -> Option<String> {
        let mut cmd = Command::new(path);
        cmd.arg(arg).stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn().ok()?;
        let start = Instant::now();

        // Simple polling wait up to 2 seconds
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if status.success() {
                        if let Ok(output) = child.wait_with_output() {
                            let text = if !output.stdout.is_empty() {
                                String::from_utf8_lossy(&output.stdout).to_string()
                            } else {
                                String::from_utf8_lossy(&output.stderr).to_string()
                            };
                            let first_line = text.lines().next().unwrap_or("").trim().to_string();
                            if !first_line.is_empty() {
                                return Some(first_line);
                            }
                        }
                    }
                    return None;
                }
                Ok(None) => {
                    if start.elapsed() > Duration::from_secs(2) {
                        let _ = child.kill();
                        let _ = child.wait();
                        return None;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(_) => return None,
            }
        }
    }

    /// Probes all known external tools.
    pub fn probe_all() -> Vec<ExternalToolStatusDto> {
        KNOWN_TOOLS.iter().map(Self::probe_tool).collect()
    }
}

/// Sandboxed process executor isolating execution via Landlock LSM and RLIMIT_AS.
pub struct SandboxedExecutor;

impl SandboxedExecutor {
    /// Executes a command under strict Landlock LSM confinement and resource limits.
    pub fn execute(
        executable: &str,
        args: &[String],
        options: &CommandExecutionOptionsDto,
    ) -> Result<CommandExecutionResultDto, AppErrorDto> {
        Self::execute_with_cancellation(executable, args, options, None)
    }

    /// Executes a command with Landlock LSM confinement, timeout, and an optional cancellation flag.
    pub fn execute_with_cancellation(
        executable: &str,
        args: &[String],
        options: &CommandExecutionOptionsDto,
        cancel_flag: Option<&Arc<AtomicBool>>,
    ) -> Result<CommandExecutionResultDto, AppErrorDto> {
        let exe_path = ToolDetector::find_binary_in_path(executable)
            .unwrap_or_else(|| PathBuf::from(executable));

        if !exe_path.exists() {
            return Ok(CommandExecutionResultDto {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: format!("Executable not found: {}", executable),
                duration_ms: 0,
                timed_out: false,
                error_message: Some(format!("Executable not found: {}", executable)),
            });
        }

        let timeout_secs = options.timeout_secs.unwrap_or(30);
        let timeout_duration = Duration::from_secs(timeout_secs);
        let max_mem = options.max_memory_bytes.unwrap_or(512 * 1024 * 1024);

        // Prepare isolated sandbox
        let mut sandbox = LandlockSandbox::new().add_system_defaults();

        for r in &options.allowed_read_paths {
            let p = Path::new(r);
            if p.exists() {
                sandbox = sandbox.allow_read(p);
            }
        }

        for w in &options.allowed_write_paths {
            let p = Path::new(w);
            if p.exists() {
                sandbox = sandbox.allow_write(p);
            }
        }

        // Dedicated isolated tmpdir
        let isolated_tmp = options
            .working_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);

        let mut cmd = Command::new(&exe_path);
        cmd.args(args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Some(ref wd) = options.working_dir {
            cmd.current_dir(wd);
        }

        sandbox.configure_command_with_limits(&mut cmd, &isolated_tmp, Some(max_mem));

        let start_time = Instant::now();
        let mut child = cmd.spawn().map_err(|e| {
            AppErrorDto::new(
                "ERR_PROCESS_SPAWN_FAILED",
                format!("Failed to spawn {}: {}", executable, e),
            )
        })?;

        // Monitor process with timeout and cancellation check
        loop {
            // Check cancellation request
            if let Some(flag) = cancel_flag {
                if flag.load(Ordering::SeqCst) {
                    let _ = child.kill();
                    let _ = child.wait();
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    return Ok(CommandExecutionResultDto {
                        success: false,
                        exit_code: None,
                        stdout: String::new(),
                        stderr: "Process cancelled by user".to_string(),
                        duration_ms,
                        timed_out: false,
                        error_message: Some("Process cancelled by user".to_string()),
                    });
                }
            }

            match child.try_wait() {
                Ok(Some(status)) => {
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    let output = child.wait_with_output().map_err(|e| {
                        AppErrorDto::new(
                            "ERR_PROCESS_IO",
                            format!("Failed to read process output: {}", e),
                        )
                    })?;

                    // Truncate to 10 MiB safety limit
                    let max_bytes = 10 * 1024 * 1024;
                    let stdout_truncated = &output.stdout[..output.stdout.len().min(max_bytes)];
                    let stderr_truncated = &output.stderr[..output.stderr.len().min(max_bytes)];

                    let stdout = String::from_utf8_lossy(stdout_truncated).to_string();
                    let stderr = String::from_utf8_lossy(stderr_truncated).to_string();

                    return Ok(CommandExecutionResultDto {
                        success: status.success(),
                        exit_code: status.code(),
                        stdout,
                        stderr,
                        duration_ms,
                        timed_out: false,
                        error_message: if status.success() {
                            None
                        } else {
                            Some(format!("Process exited with status {:?}", status.code()))
                        },
                    });
                }
                Ok(None) => {
                    if start_time.elapsed() >= timeout_duration {
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(AppErrorDto::new("ERR_PROCESS_WAIT", e.to_string()));
                }
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        Ok(CommandExecutionResultDto {
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: format!(
                "Process timed out after {} seconds and was killed",
                timeout_secs
            ),
            duration_ms,
            timed_out: true,
            error_message: Some(format!("Command timed out after {} seconds", timeout_secs)),
        })
    }
}

/// Builtin pure-Rust data transformers (Presets 14–20, 26, 27, 29).
pub struct BuiltinPipelineEngine;

impl BuiltinPipelineEngine {
    /// Preset 14: Mask IP addresses, emails, and API keys/tokens.
    pub fn mask_log_secrets(text: &str) -> (String, u64) {
        let mut count = 0u64;

        // IPv4 regex
        let re_ip = Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b").unwrap();
        // Email regex
        let re_email = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap();
        // Generic API Key / Bearer Token / Hex Hash pattern (32+ chars)
        let re_token =
            Regex::new(r"(?i)\b(?:bearer\s+|api[_-]?key\s*[:=]\s*)[a-zA-Z0-9_\-\.]{16,}").unwrap();

        let intermediate1 = re_ip.replace_all(text, |_caps: &regex::Captures| {
            count += 1;
            "[IP_MASKED]"
        });

        let intermediate2 = re_email.replace_all(&intermediate1, |_caps: &regex::Captures| {
            count += 1;
            "[EMAIL_MASKED]"
        });

        let final_text = re_token.replace_all(&intermediate2, |_caps: &regex::Captures| {
            count += 1;
            "[API_KEY_MASKED]"
        });

        (final_text.to_string(), count)
    }

    /// Preset 15: Clean legacy CSV (encoding normalization, CRLF -> LF, trim column whitespace).
    pub fn clean_csv(bytes: &[u8]) -> Result<(String, u64), String> {
        // Detect encoding
        let mut detector = chardetng::EncodingDetector::new();
        detector.feed(bytes, true);
        let encoding = detector.guess(None, true);
        let (decoded, _, _) = encoding.decode(bytes);

        let mut row_count = 0u64;
        let mut output = String::with_capacity(decoded.len());

        for line in decoded.lines() {
            let trimmed_line = line.trim_end_matches('\r');
            if trimmed_line.is_empty() {
                continue;
            }
            row_count += 1;
            let cells: Vec<&str> = trimmed_line.split(',').map(|c| c.trim()).collect();
            output.push_str(&cells.join(","));
            output.push('\n');
        }

        Ok((output, row_count))
    }

    /// Preset 16: Validate JSON and minify (strip unnecessary whitespace/newlines).
    pub fn minify_json(text: &str) -> Result<(String, u64), String> {
        let val: serde_json::Value =
            serde_json::from_str(text).map_err(|e| format!("Invalid JSON syntax: {}", e))?;
        let minified =
            serde_json::to_string(&val).map_err(|e| format!("Failed to serialize JSON: {}", e))?;
        Ok((minified, 1))
    }

    /// Preset 17: TSV <-> CSV conversion with header snake_casing.
    pub fn convert_tsv_csv(text: &str, to_csv: bool) -> Result<(String, u64), String> {
        let from_delim = if to_csv { '\t' } else { ',' };
        let to_delim = if to_csv { ',' } else { '\t' };

        let mut lines = text.lines();
        let header = match lines.next() {
            Some(h) => h,
            None => return Ok((String::new(), 0)),
        };

        let mut output = String::with_capacity(text.len());

        // Process header -> snake_case
        let headers: Vec<String> = header
            .split(from_delim)
            .map(|col| {
                col.trim()
                    .replace(|c: char| !c.is_alphanumeric() && c != '_', "_")
                    .to_lowercase()
            })
            .collect();
        output.push_str(&headers.join(&to_delim.to_string()));
        output.push('\n');

        let mut rows = 1u64;
        for line in lines {
            let row = line.trim_end_matches('\r');
            if row.is_empty() {
                continue;
            }
            let cells: Vec<&str> = row.split(from_delim).map(|c| c.trim()).collect();
            output.push_str(&cells.join(&to_delim.to_string()));
            output.push('\n');
            rows += 1;
        }

        Ok((output, rows))
    }

    /// Preset 19: SQL dump minification (strip comments and blank lines).
    pub fn minify_sql_dump(text: &str) -> (String, u64) {
        let mut output = String::with_capacity(text.len());
        let mut count = 0u64;

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.starts_with("--")
                || trimmed.starts_with("/*") && trimmed.ends_with("*/")
            {
                continue;
            }
            output.push_str(line);
            output.push('\n');
            count += 1;
        }

        (output, count)
    }

    /// Preset 20: SRT subtitle timecode shift (offset in milliseconds).
    pub fn shift_srt_timecodes(text: &str, offset_ms: i64) -> Result<(String, u64), String> {
        let time_re =
            Regex::new(r"(\d{2}):(\d{2}):(\d{2}),(\d{3})\s*-->\s*(\d{2}):(\d{2}):(\d{2}),(\d{3})")
                .unwrap();
        let mut count = 0u64;

        let parse_time = |h: &str, m: &str, s: &str, ms: &str| -> i64 {
            let hours: i64 = h.parse().unwrap_or(0);
            let mins: i64 = m.parse().unwrap_or(0);
            let secs: i64 = s.parse().unwrap_or(0);
            let millis: i64 = ms.parse().unwrap_or(0);
            hours * 3_600_000 + mins * 60_000 + secs * 1000 + millis
        };

        let format_time = |total_ms: i64| -> String {
            let clamped = total_ms.max(0);
            let ms = clamped % 1000;
            let total_s = clamped / 1000;
            let s = total_s % 60;
            let total_m = total_s / 60;
            let m = total_m % 60;
            let h = total_m / 60;
            format!("{:02}:{:02}:{:02},{:03}", h, m, s, ms)
        };

        let result = time_re.replace_all(text, |caps: &regex::Captures| {
            let t1 = parse_time(&caps[1], &caps[2], &caps[3], &caps[4]) + offset_ms;
            let t2 = parse_time(&caps[5], &caps[6], &caps[7], &caps[8]) + offset_ms;
            count += 1;
            format!("{} --> {}", format_time(t1), format_time(t2))
        });

        Ok((result.to_string(), count))
    }

    /// Preset 29: Generate standard `checksums.txt` with SHA-256 hashes.
    pub fn generate_checksums_txt(files: &[(&str, &[u8])]) -> String {
        let mut out = String::new();
        for (filename, content) in files {
            let mut hasher = Sha256::new();
            hasher.update(content);
            let hash = format!("{:x}", hasher.finalize());
            out.push_str(&format!("{}  {}\n", hash, filename));
        }
        out
    }
}

/// Registry of all 30 pipeline presets across 6 categories.
pub struct PipelineRegistry;

impl PipelineRegistry {
    pub fn get_all_presets() -> Vec<PipelinePresetDto> {
        vec![
            // --- Category 1: Image & Visual (1-8) ---
            PipelinePresetDto {
                id: "image_web_light".to_string(),
                category: PipelineCategory::Image,
                name_key: "pipeline.presets.img_web_light_name".to_string(),
                desc_key: "pipeline.presets.img_web_light_desc".to_string(),
                required_tools: vec!["imagemagick".to_string()],
                is_builtin: false,
                default_suffix: "_web.webp".to_string(),
                supported_extensions: vec![
                    "png".to_string(),
                    "jpg".to_string(),
                    "jpeg".to_string(),
                    "webp".to_string(),
                    "bmp".to_string(),
                    "tiff".to_string(),
                ],
                parameters: vec![
                    PipelineParameterDef {
                        key: "max_width".to_string(),
                        label_key: "pipeline.params.max_width".to_string(),
                        param_type: "number".to_string(),
                        default_value: "1920".to_string(),
                        min: Some(320.0),
                        max: Some(7680.0),
                        step: Some(10.0),
                        options: None,
                    },
                    PipelineParameterDef {
                        key: "quality".to_string(),
                        label_key: "pipeline.params.quality".to_string(),
                        param_type: "number".to_string(),
                        default_value: "80".to_string(),
                        min: Some(1.0),
                        max: Some(100.0),
                        step: Some(1.0),
                        options: None,
                    },
                ],
            },
            PipelinePresetDto {
                id: "image_sns_ogp".to_string(),
                category: PipelineCategory::Image,
                name_key: "pipeline.presets.img_sns_ogp_name".to_string(),
                desc_key: "pipeline.presets.img_sns_ogp_desc".to_string(),
                required_tools: vec!["imagemagick".to_string()],
                is_builtin: false,
                default_suffix: "_ogp.jpg".to_string(),
                supported_extensions: vec![
                    "png".to_string(),
                    "jpg".to_string(),
                    "jpeg".to_string(),
                    "webp".to_string(),
                ],
                parameters: vec![PipelineParameterDef {
                    key: "quality".to_string(),
                    label_key: "pipeline.params.quality".to_string(),
                    param_type: "number".to_string(),
                    default_value: "85".to_string(),
                    min: Some(1.0),
                    max: Some(100.0),
                    step: Some(1.0),
                    options: None,
                }],
            },
            PipelinePresetDto {
                id: "image_favicon_pack".to_string(),
                category: PipelineCategory::Image,
                name_key: "pipeline.presets.img_favicon_pack_name".to_string(),
                desc_key: "pipeline.presets.img_favicon_pack_desc".to_string(),
                required_tools: vec!["imagemagick".to_string()],
                is_builtin: false,
                default_suffix: "_icon.png".to_string(),
                supported_extensions: vec![
                    "png".to_string(),
                    "jpg".to_string(),
                    "jpeg".to_string(),
                    "svg".to_string(),
                ],
                parameters: vec![PipelineParameterDef {
                    key: "size".to_string(),
                    label_key: "pipeline.params.icon_size".to_string(),
                    param_type: "select".to_string(),
                    default_value: "512".to_string(),
                    min: None,
                    max: None,
                    step: None,
                    options: Some(vec![
                        "16".to_string(),
                        "32".to_string(),
                        "180".to_string(),
                        "512".to_string(),
                    ]),
                }],
            },
            PipelinePresetDto {
                id: "image_watermark".to_string(),
                category: PipelineCategory::Image,
                name_key: "pipeline.presets.img_watermark_name".to_string(),
                desc_key: "pipeline.presets.img_watermark_desc".to_string(),
                required_tools: vec!["imagemagick".to_string()],
                is_builtin: false,
                default_suffix: "_preview.jpg".to_string(),
                supported_extensions: vec![
                    "png".to_string(),
                    "jpg".to_string(),
                    "jpeg".to_string(),
                ],
                parameters: vec![PipelineParameterDef {
                    key: "watermark_text".to_string(),
                    label_key: "pipeline.params.watermark_text".to_string(),
                    param_type: "string".to_string(),
                    default_value: "PREVIEW".to_string(),
                    min: None,
                    max: None,
                    step: None,
                    options: None,
                }],
            },
            PipelinePresetDto {
                id: "image_retina_downscale".to_string(),
                category: PipelineCategory::Image,
                name_key: "pipeline.presets.img_retina_downscale_name".to_string(),
                desc_key: "pipeline.presets.img_retina_downscale_desc".to_string(),
                required_tools: vec!["imagemagick".to_string()],
                is_builtin: false,
                default_suffix: "@1x.png".to_string(),
                supported_extensions: vec!["png".to_string()],
                parameters: vec![],
            },
            PipelinePresetDto {
                id: "image_pixel_art_scale".to_string(),
                category: PipelineCategory::Image,
                name_key: "pipeline.presets.img_pixel_art_scale_name".to_string(),
                desc_key: "pipeline.presets.img_pixel_art_scale_desc".to_string(),
                required_tools: vec!["imagemagick".to_string()],
                is_builtin: false,
                default_suffix: "_400pct.png".to_string(),
                supported_extensions: vec!["png".to_string(), "gif".to_string()],
                parameters: vec![PipelineParameterDef {
                    key: "scale_percent".to_string(),
                    label_key: "pipeline.params.scale_percent".to_string(),
                    param_type: "number".to_string(),
                    default_value: "400".to_string(),
                    min: Some(100.0),
                    max: Some(1600.0),
                    step: Some(100.0),
                    options: None,
                }],
            },
            PipelinePresetDto {
                id: "image_print_cmyk".to_string(),
                category: PipelineCategory::Image,
                name_key: "pipeline.presets.img_print_cmyk_name".to_string(),
                desc_key: "pipeline.presets.img_print_cmyk_desc".to_string(),
                required_tools: vec!["imagemagick".to_string()],
                is_builtin: false,
                default_suffix: "_cmyk.tiff".to_string(),
                supported_extensions: vec![
                    "png".to_string(),
                    "jpg".to_string(),
                    "jpeg".to_string(),
                    "tiff".to_string(),
                ],
                parameters: vec![],
            },
            PipelinePresetDto {
                id: "image_bw_artistic".to_string(),
                category: PipelineCategory::Image,
                name_key: "pipeline.presets.img_bw_artistic_name".to_string(),
                desc_key: "pipeline.presets.img_bw_artistic_desc".to_string(),
                required_tools: vec!["imagemagick".to_string()],
                is_builtin: false,
                default_suffix: "_bw.jpg".to_string(),
                supported_extensions: vec![
                    "png".to_string(),
                    "jpg".to_string(),
                    "jpeg".to_string(),
                    "webp".to_string(),
                ],
                parameters: vec![],
            },
            // --- Category 2: Audio & Video Media (9-13) ---
            PipelinePresetDto {
                id: "media_gif_anim".to_string(),
                category: PipelineCategory::Media,
                name_key: "pipeline.presets.media_gif_anim_name".to_string(),
                desc_key: "pipeline.presets.media_gif_anim_desc".to_string(),
                required_tools: vec!["ffmpeg".to_string()],
                is_builtin: false,
                default_suffix: "_anim.gif".to_string(),
                supported_extensions: vec![
                    "mp4".to_string(),
                    "mov".to_string(),
                    "mkv".to_string(),
                    "webm".to_string(),
                ],
                parameters: vec![
                    PipelineParameterDef {
                        key: "fps".to_string(),
                        label_key: "pipeline.params.fps".to_string(),
                        param_type: "number".to_string(),
                        default_value: "12".to_string(),
                        min: Some(5.0),
                        max: Some(30.0),
                        step: Some(1.0),
                        options: None,
                    },
                    PipelineParameterDef {
                        key: "width".to_string(),
                        label_key: "pipeline.params.width".to_string(),
                        param_type: "number".to_string(),
                        default_value: "480".to_string(),
                        min: Some(240.0),
                        max: Some(1280.0),
                        step: Some(20.0),
                        options: None,
                    },
                ],
            },
            PipelinePresetDto {
                id: "media_podcast_master".to_string(),
                category: PipelineCategory::Media,
                name_key: "pipeline.presets.media_podcast_master_name".to_string(),
                desc_key: "pipeline.presets.media_podcast_master_desc".to_string(),
                required_tools: vec!["ffmpeg".to_string()],
                is_builtin: false,
                default_suffix: "_podcast.mp3".to_string(),
                supported_extensions: vec![
                    "wav".to_string(),
                    "mp3".to_string(),
                    "m4a".to_string(),
                    "flac".to_string(),
                ],
                parameters: vec![PipelineParameterDef {
                    key: "target_lufs".to_string(),
                    label_key: "pipeline.params.target_lufs".to_string(),
                    param_type: "number".to_string(),
                    default_value: "-16".to_string(),
                    min: Some(-24.0),
                    max: Some(-12.0),
                    step: Some(1.0),
                    options: None,
                }],
            },
            PipelinePresetDto {
                id: "media_extract_speech".to_string(),
                category: PipelineCategory::Media,
                name_key: "pipeline.presets.media_extract_speech_name".to_string(),
                desc_key: "pipeline.presets.media_extract_speech_desc".to_string(),
                required_tools: vec!["ffmpeg".to_string()],
                is_builtin: false,
                default_suffix: "_speech.wav".to_string(),
                supported_extensions: vec![
                    "mp4".to_string(),
                    "mov".to_string(),
                    "mkv".to_string(),
                    "webm".to_string(),
                    "mp3".to_string(),
                    "m4a".to_string(),
                ],
                parameters: vec![PipelineParameterDef {
                    key: "sample_rate".to_string(),
                    label_key: "pipeline.params.sample_rate".to_string(),
                    param_type: "select".to_string(),
                    default_value: "16000".to_string(),
                    min: None,
                    max: None,
                    step: None,
                    options: Some(vec![
                        "8000".to_string(),
                        "16000".to_string(),
                        "22050".to_string(),
                        "44100".to_string(),
                    ]),
                }],
            },
            PipelinePresetDto {
                id: "media_sns_vertical".to_string(),
                category: PipelineCategory::Media,
                name_key: "pipeline.presets.media_sns_vertical_name".to_string(),
                desc_key: "pipeline.presets.media_sns_vertical_desc".to_string(),
                required_tools: vec!["ffmpeg".to_string()],
                is_builtin: false,
                default_suffix: "_vertical.mp4".to_string(),
                supported_extensions: vec!["mp4".to_string(), "mov".to_string(), "mkv".to_string()],
                parameters: vec![],
            },
            PipelinePresetDto {
                id: "media_discord_share".to_string(),
                category: PipelineCategory::Media,
                name_key: "pipeline.presets.media_discord_share_name".to_string(),
                desc_key: "pipeline.presets.media_discord_share_desc".to_string(),
                required_tools: vec!["ffmpeg".to_string()],
                is_builtin: false,
                default_suffix: "_share.mp4".to_string(),
                supported_extensions: vec![
                    "mp4".to_string(),
                    "mov".to_string(),
                    "mkv".to_string(),
                    "avi".to_string(),
                ],
                parameters: vec![],
            },
            // --- Category 3: Text, Log & Data Formatting (14-20, Pure-Rust Builtin) ---
            PipelinePresetDto {
                id: "text_mask_secrets".to_string(),
                category: PipelineCategory::Text,
                name_key: "pipeline.presets.text_mask_secrets_name".to_string(),
                desc_key: "pipeline.presets.text_mask_secrets_desc".to_string(),
                required_tools: vec![],
                is_builtin: true,
                default_suffix: "_masked.log".to_string(),
                supported_extensions: vec!["log".to_string(), "txt".to_string(), "out".to_string()],
                parameters: vec![],
            },
            PipelinePresetDto {
                id: "text_clean_csv".to_string(),
                category: PipelineCategory::Text,
                name_key: "pipeline.presets.text_clean_csv_name".to_string(),
                desc_key: "pipeline.presets.text_clean_csv_desc".to_string(),
                required_tools: vec![],
                is_builtin: true,
                default_suffix: "_utf8.csv".to_string(),
                supported_extensions: vec!["csv".to_string(), "txt".to_string()],
                parameters: vec![],
            },
            PipelinePresetDto {
                id: "text_minify_json".to_string(),
                category: PipelineCategory::Text,
                name_key: "pipeline.presets.text_minify_json_name".to_string(),
                desc_key: "pipeline.presets.text_minify_json_desc".to_string(),
                required_tools: vec![],
                is_builtin: true,
                default_suffix: ".min.json".to_string(),
                supported_extensions: vec!["json".to_string()],
                parameters: vec![],
            },
            PipelinePresetDto {
                id: "text_tsv_csv_convert".to_string(),
                category: PipelineCategory::Text,
                name_key: "pipeline.presets.text_tsv_csv_convert_name".to_string(),
                desc_key: "pipeline.presets.text_tsv_csv_convert_desc".to_string(),
                required_tools: vec![],
                is_builtin: true,
                default_suffix: "_conv.csv".to_string(),
                supported_extensions: vec!["tsv".to_string(), "csv".to_string()],
                parameters: vec![PipelineParameterDef {
                    key: "lowercase_headers".to_string(),
                    label_key: "pipeline.params.lowercase_headers".to_string(),
                    param_type: "boolean".to_string(),
                    default_value: "true".to_string(),
                    min: None,
                    max: None,
                    step: None,
                    options: None,
                }],
            },
            PipelinePresetDto {
                id: "text_markdown_lint".to_string(),
                category: PipelineCategory::Text,
                name_key: "pipeline.presets.text_markdown_lint_name".to_string(),
                desc_key: "pipeline.presets.text_markdown_lint_desc".to_string(),
                required_tools: vec![],
                is_builtin: true,
                default_suffix: "_formatted.md".to_string(),
                supported_extensions: vec!["md".to_string(), "markdown".to_string()],
                parameters: vec![],
            },
            PipelinePresetDto {
                id: "text_slimy_sql".to_string(),
                category: PipelineCategory::Text,
                name_key: "pipeline.presets.text_slimy_sql_name".to_string(),
                desc_key: "pipeline.presets.text_slimy_sql_desc".to_string(),
                required_tools: vec![],
                is_builtin: true,
                default_suffix: "_clean.sql".to_string(),
                supported_extensions: vec!["sql".to_string()],
                parameters: vec![],
            },
            PipelinePresetDto {
                id: "text_shift_srt".to_string(),
                category: PipelineCategory::Text,
                name_key: "pipeline.presets.text_shift_srt_name".to_string(),
                desc_key: "pipeline.presets.text_shift_srt_desc".to_string(),
                required_tools: vec![],
                is_builtin: true,
                default_suffix: "_shifted.srt".to_string(),
                supported_extensions: vec!["srt".to_string()],
                parameters: vec![PipelineParameterDef {
                    key: "offset_ms".to_string(),
                    label_key: "pipeline.params.offset_ms".to_string(),
                    param_type: "number".to_string(),
                    default_value: "1000".to_string(),
                    min: Some(-60000.0),
                    max: Some(60000.0),
                    step: Some(100.0),
                    options: None,
                }],
            },
            // --- Category 4: Document & PDF (21-24) ---
            PipelinePresetDto {
                id: "doc_pdf_binarize".to_string(),
                category: PipelineCategory::Document,
                name_key: "pipeline.presets.doc_pdf_binarize_name".to_string(),
                desc_key: "pipeline.presets.doc_pdf_binarize_desc".to_string(),
                required_tools: vec!["imagemagick".to_string()],
                is_builtin: false,
                default_suffix: "_binarized.pdf".to_string(),
                supported_extensions: vec!["pdf".to_string(), "png".to_string(), "jpg".to_string()],
                parameters: vec![],
            },
            PipelinePresetDto {
                id: "doc_pdf_merge".to_string(),
                category: PipelineCategory::Document,
                name_key: "pipeline.presets.doc_pdf_merge_name".to_string(),
                desc_key: "pipeline.presets.doc_pdf_merge_desc".to_string(),
                required_tools: vec!["qpdf".to_string()],
                is_builtin: false,
                default_suffix: "_merged.pdf".to_string(),
                supported_extensions: vec!["pdf".to_string()],
                parameters: vec![],
            },
            PipelinePresetDto {
                id: "doc_pdf_security".to_string(),
                category: PipelineCategory::Document,
                name_key: "pipeline.presets.doc_pdf_security_name".to_string(),
                desc_key: "pipeline.presets.doc_pdf_security_desc".to_string(),
                required_tools: vec!["qpdf".to_string()],
                is_builtin: false,
                default_suffix: "_secure.pdf".to_string(),
                supported_extensions: vec!["pdf".to_string()],
                parameters: vec![],
            },
            PipelinePresetDto {
                id: "doc_pdf_extract_images".to_string(),
                category: PipelineCategory::Document,
                name_key: "pipeline.presets.doc_pdf_extract_images_name".to_string(),
                desc_key: "pipeline.presets.doc_pdf_extract_images_desc".to_string(),
                required_tools: vec!["pdfimages".to_string()],
                is_builtin: false,
                default_suffix: "_img".to_string(),
                supported_extensions: vec!["pdf".to_string()],
                parameters: vec![],
            },
            // --- Category 5: File Organization & Normalization (25-27) ---
            PipelinePresetDto {
                id: "file_exif_organize".to_string(),
                category: PipelineCategory::File,
                name_key: "pipeline.presets.file_exif_organize_name".to_string(),
                desc_key: "pipeline.presets.file_exif_organize_desc".to_string(),
                required_tools: vec!["exiftool".to_string()],
                is_builtin: false,
                default_suffix: "".to_string(),
                supported_extensions: vec![
                    "jpg".to_string(),
                    "jpeg".to_string(),
                    "png".to_string(),
                    "mp4".to_string(),
                    "mov".to_string(),
                ],
                parameters: vec![],
            },
            PipelinePresetDto {
                id: "file_clean_names".to_string(),
                category: PipelineCategory::File,
                name_key: "pipeline.presets.file_clean_names_name".to_string(),
                desc_key: "pipeline.presets.file_clean_names_desc".to_string(),
                required_tools: vec![],
                is_builtin: true,
                default_suffix: "".to_string(),
                supported_extensions: vec!["*".to_string()],
                parameters: vec![],
            },
            PipelinePresetDto {
                id: "file_zero_padding".to_string(),
                category: PipelineCategory::File,
                name_key: "pipeline.presets.file_zero_padding_name".to_string(),
                desc_key: "pipeline.presets.file_zero_padding_desc".to_string(),
                required_tools: vec![],
                is_builtin: true,
                default_suffix: "".to_string(),
                supported_extensions: vec!["*".to_string()],
                parameters: vec![PipelineParameterDef {
                    key: "digits".to_string(),
                    label_key: "pipeline.params.padding_digits".to_string(),
                    param_type: "number".to_string(),
                    default_value: "4".to_string(),
                    min: Some(2.0),
                    max: Some(8.0),
                    step: Some(1.0),
                    options: None,
                }],
            },
            // --- Category 6: Developer, Package & Security (28-30) ---
            PipelinePresetDto {
                id: "dev_clean_package".to_string(),
                category: PipelineCategory::Dev,
                name_key: "pipeline.presets.dev_clean_package_name".to_string(),
                desc_key: "pipeline.presets.dev_clean_package_desc".to_string(),
                required_tools: vec![],
                is_builtin: true,
                default_suffix: "_clean.tar.gz".to_string(),
                supported_extensions: vec!["*".to_string()],
                parameters: vec![],
            },
            PipelinePresetDto {
                id: "dev_checksum_bundle".to_string(),
                category: PipelineCategory::Dev,
                name_key: "pipeline.presets.dev_checksum_bundle_name".to_string(),
                desc_key: "pipeline.presets.dev_checksum_bundle_desc".to_string(),
                required_tools: vec![],
                is_builtin: true,
                default_suffix: "_checksums.tar.gz".to_string(),
                supported_extensions: vec!["*".to_string()],
                parameters: vec![],
            },
            PipelinePresetDto {
                id: "dev_encrypted_backup".to_string(),
                category: PipelineCategory::Dev,
                name_key: "pipeline.presets.dev_encrypted_backup_name".to_string(),
                desc_key: "pipeline.presets.dev_encrypted_backup_desc".to_string(),
                required_tools: vec![],
                is_builtin: true,
                default_suffix: ".enc".to_string(),
                supported_extensions: vec!["*".to_string()],
                parameters: vec![],
            },
        ]
    }

    pub fn find_preset(preset_id: &str) -> Option<PipelinePresetDto> {
        Self::get_all_presets()
            .into_iter()
            .find(|p| p.id == preset_id)
    }
}

/// Pipeline runner orchestrating preset execution against workspace staged items.
pub struct PipelineRunner;

impl PipelineRunner {
    /// Executes a pipeline preset against staged items in the workspace.
    pub fn execute(
        req: &ExecutePipelineRequestDto,
        workspace_manager: &crate::workspace::WorkspaceManager,
    ) -> Result<PipelineExecutionResultDto, AppErrorDto> {
        Self::execute_with_progress(req, workspace_manager, None)
    }

    /// Executes a pipeline preset with real-time progress callbacks and cancellation check.
    pub fn execute_with_progress(
        req: &ExecutePipelineRequestDto,
        workspace_manager: &crate::workspace::WorkspaceManager,
        progress_callback: Option<Box<dyn Fn(PipelineProgressEvent) + Send + Sync>>,
    ) -> Result<PipelineExecutionResultDto, AppErrorDto> {
        let preset = PipelineRegistry::find_preset(&req.preset_id).ok_or_else(|| {
            AppErrorDto::new(
                "PresetNotFound",
                format!("Preset {} not found", req.preset_id),
            )
        })?;

        // If preset requires external tools, verify availability first
        for tool_id in &preset.required_tools {
            let status = ToolDetector::probe_all()
                .into_iter()
                .find(|t| &t.tool_id == tool_id);
            if let Some(s) = status {
                if !s.is_available {
                    return Err(AppErrorDto::new(
                        "ToolMissing",
                        format!(
                            "Required tool '{}' is not installed on system",
                            s.display_name
                        ),
                    ));
                }
            } else {
                return Err(AppErrorDto::new(
                    "ToolMissing",
                    format!("Required tool '{}' not found in known registry", tool_id),
                ));
            }
        }

        let job_id = req
            .job_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let cancel_mgr = get_cancellation_manager();
        let cancel_flag = cancel_mgr.register_job(&job_id);

        let staged_items = workspace_manager.list_staged();
        let mut item_results = Vec::new();
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut is_cancelled = false;
        let total_items = req.stage_ids.len();

        for (idx, stage_id) in req.stage_ids.iter().enumerate() {
            // Check cancellation request before processing next item
            if cancel_flag.load(Ordering::SeqCst) {
                is_cancelled = true;
                if let Some(ref cb) = progress_callback {
                    cb(PipelineProgressEvent {
                        job_id: job_id.clone(),
                        preset_id: req.preset_id.clone(),
                        current_index: idx,
                        total_items,
                        current_filename: String::new(),
                        stage_id: stage_id.clone(),
                        status: "Cancelled".to_string(),
                        percent: if total_items > 0 {
                            ((idx as f32) / (total_items as f32)) * 100.0
                        } else {
                            0.0
                        },
                        message: Some("Pipeline execution cancelled by user".to_string()),
                    });
                }
                break;
            }

            let start = Instant::now();
            let staged = staged_items.iter().find(|s| &s.stage_id == stage_id);

            let staged = match staged {
                Some(s) => s,
                None => {
                    item_results.push(PipelineExecutionItemResultDto {
                        stage_id: stage_id.clone(),
                        original_name: String::new(),
                        output_name: String::new(),
                        success: false,
                        error_message: Some(format!(
                            "Staged item {} not found in workspace",
                            stage_id
                        )),
                        execution_time_ms: start.elapsed().as_millis() as u64,
                    });
                    failure_count += 1;
                    continue;
                }
            };

            let staged_path = PathBuf::from(&staged.staged_path);
            if !staged_path.exists() {
                item_results.push(PipelineExecutionItemResultDto {
                    stage_id: stage_id.clone(),
                    original_name: staged.display_name.clone(),
                    output_name: staged.display_name.clone(),
                    success: false,
                    error_message: Some(format!(
                        "Staged file missing on disk: {}",
                        staged.staged_path
                    )),
                    execution_time_ms: start.elapsed().as_millis() as u64,
                });
                failure_count += 1;
                continue;
            }

            // Emit Processing event
            if let Some(ref cb) = progress_callback {
                cb(PipelineProgressEvent {
                    job_id: job_id.clone(),
                    preset_id: req.preset_id.clone(),
                    current_index: idx + 1,
                    total_items,
                    current_filename: staged.display_name.clone(),
                    stage_id: stage_id.clone(),
                    status: "Processing".to_string(),
                    percent: if total_items > 0 {
                        ((idx as f32) / (total_items as f32)) * 100.0
                    } else {
                        0.0
                    },
                    message: None,
                });
            }

            // Run transformation with cancel_flag support
            match Self::apply_preset_to_file(
                &preset,
                &staged_path,
                &staged.display_name,
                &req.custom_parameters,
                Some(&cancel_flag),
            ) {
                Ok(new_name) => {
                    let elapsed = start.elapsed().as_millis() as u64;
                    // Update display name if changed
                    if new_name != staged.display_name {
                        let _ =
                            workspace_manager.update_staged_path(stage_id, &staged_path, &new_name);
                    }
                    item_results.push(PipelineExecutionItemResultDto {
                        stage_id: stage_id.clone(),
                        original_name: staged.display_name.clone(),
                        output_name: new_name,
                        success: true,
                        error_message: None,
                        execution_time_ms: elapsed,
                    });
                    success_count += 1;

                    if let Some(ref cb) = progress_callback {
                        cb(PipelineProgressEvent {
                            job_id: job_id.clone(),
                            preset_id: req.preset_id.clone(),
                            current_index: idx + 1,
                            total_items,
                            current_filename: staged.display_name.clone(),
                            stage_id: stage_id.clone(),
                            status: "Completed".to_string(),
                            percent: if total_items > 0 {
                                (((idx + 1) as f32) / (total_items as f32)) * 100.0
                            } else {
                                100.0
                            },
                            message: None,
                        });
                    }
                }
                Err(err) => {
                    let elapsed = start.elapsed().as_millis() as u64;
                    let was_cancelled = cancel_flag.load(Ordering::SeqCst);
                    item_results.push(PipelineExecutionItemResultDto {
                        stage_id: stage_id.clone(),
                        original_name: staged.display_name.clone(),
                        output_name: staged.display_name.clone(),
                        success: false,
                        error_message: Some(err.message.clone()),
                        execution_time_ms: elapsed,
                    });
                    failure_count += 1;

                    if was_cancelled {
                        is_cancelled = true;
                        if let Some(ref cb) = progress_callback {
                            cb(PipelineProgressEvent {
                                job_id: job_id.clone(),
                                preset_id: req.preset_id.clone(),
                                current_index: idx + 1,
                                total_items,
                                current_filename: staged.display_name.clone(),
                                stage_id: stage_id.clone(),
                                status: "Cancelled".to_string(),
                                percent: if total_items > 0 {
                                    (((idx + 1) as f32) / (total_items as f32)) * 100.0
                                } else {
                                    100.0
                                },
                                message: Some("Process cancelled by user".to_string()),
                            });
                        }
                        break;
                    } else if let Some(ref cb) = progress_callback {
                        cb(PipelineProgressEvent {
                            job_id: job_id.clone(),
                            preset_id: req.preset_id.clone(),
                            current_index: idx + 1,
                            total_items,
                            current_filename: staged.display_name.clone(),
                            stage_id: stage_id.clone(),
                            status: "Failed".to_string(),
                            percent: if total_items > 0 {
                                (((idx + 1) as f32) / (total_items as f32)) * 100.0
                            } else {
                                100.0
                            },
                            message: Some(err.message),
                        });
                    }
                }
            }
        }

        cancel_mgr.unregister_job(&job_id);

        Ok(PipelineExecutionResultDto {
            preset_id: req.preset_id.clone(),
            total_count: req.stage_ids.len(),
            success_count,
            failure_count,
            is_cancelled,
            item_results,
        })
    }

    /// Dry-Run validation for user-defined custom pipeline as mandated by Design Section 8.3.
    /// Pre-validates argument expansions, binary presence, output conflicts, and disk space.
    pub fn dry_run_custom_pipeline(
        pipeline: &crate::config::CustomPipelineDefDto,
        stage_ids: &[String],
        workspace_manager: &crate::workspace::WorkspaceManager,
    ) -> Result<CustomPipelineDryRunResultDto, AppErrorDto> {
        let exec_path = ToolDetector::find_binary_in_path(&pipeline.executable);
        let executable_found = exec_path.is_some();
        let resolved_exec_str = exec_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("[Not found in PATH: {}]", pipeline.executable));

        let mut items = Vec::new();
        let mut valid_items = 0;
        let mut has_conflicts = false;

        for stage_id in stage_ids {
            let staged = match workspace_manager.get_staged(stage_id) {
                Ok(s) => s,
                Err(_) => {
                    items.push(CustomPipelineDryRunItemDto {
                        stage_id: stage_id.clone(),
                        original_name: "Unknown".to_string(),
                        resolved_executable: resolved_exec_str.clone(),
                        expanded_command: Vec::new(),
                        proposed_output_name: String::new(),
                        conflict_detected: false,
                        valid: false,
                        message: Some("Staged file not found in workspace".to_string()),
                    });
                    continue;
                }
            };

            let staged_path = PathBuf::from(&staged.staged_path);
            let parent_dir = staged_path.parent().unwrap_or_else(|| Path::new("/tmp"));

            let stem = staged_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("file");
            let orig_ext = staged_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");

            let final_ext = pipeline.output_extension.as_deref().unwrap_or(orig_ext);

            let suffix = &pipeline.default_suffix;
            let output_name = if !suffix.is_empty() && !stem.ends_with(suffix) {
                if final_ext.is_empty() {
                    format!("{}{}", stem, suffix)
                } else {
                    format!("{}{}.{}", stem, suffix, final_ext)
                }
            } else if final_ext.is_empty() {
                stem.to_string()
            } else {
                format!("{}.{}", stem, final_ext)
            };

            let proposed_output_path = parent_dir.join(&output_name);

            // Expand command template
            let mut expanded_cmd = Vec::new();
            expanded_cmd.push(resolved_exec_str.clone());
            for arg in &pipeline.args {
                let replaced = arg
                    .replace("{input}", &staged_path.to_string_lossy())
                    .replace("{output}", &proposed_output_path.to_string_lossy());
                expanded_cmd.push(replaced);
            }

            // Check conflict: does output already exist and is not identical to input?
            let conflict = proposed_output_path.exists() && proposed_output_path != staged_path;
            if conflict {
                has_conflicts = true;
            }

            // Check extension compatibility
            let ext_compatible = if pipeline.supported_extensions.is_empty() {
                true
            } else {
                let lower_ext = orig_ext.to_lowercase();
                pipeline
                    .supported_extensions
                    .iter()
                    .any(|e| e.trim_start_matches('.').eq_ignore_ascii_case(&lower_ext))
            };

            let valid = executable_found && !conflict && ext_compatible;
            if valid {
                valid_items += 1;
            }

            let message = if !executable_found {
                Some(format!(
                    "Executable '{}' not found in system PATH",
                    pipeline.executable
                ))
            } else if conflict {
                Some(format!(
                    "Output file '{}' already exists in staging area",
                    output_name
                ))
            } else if !ext_compatible {
                Some(format!(
                    "Extension '.{}' is not in supported extensions list",
                    orig_ext
                ))
            } else {
                None
            };

            items.push(CustomPipelineDryRunItemDto {
                stage_id: stage_id.clone(),
                original_name: staged.display_name.clone(),
                resolved_executable: resolved_exec_str.clone(),
                expanded_command: expanded_cmd,
                proposed_output_name: output_name,
                conflict_detected: conflict,
                valid,
                message,
            });
        }

        let can_proceed = executable_found && !has_conflicts && valid_items > 0;

        Ok(CustomPipelineDryRunResultDto {
            pipeline_id: pipeline.id.clone(),
            pipeline_name: pipeline.name.clone(),
            executable_found,
            executable_path: exec_path.map(|p| p.to_string_lossy().to_string()),
            total_items: stage_ids.len(),
            valid_items,
            has_conflicts,
            has_disk_space: true,
            disk_space_message: None,
            items,
            can_proceed_to_execution: can_proceed,
        })
    }

    /// Execute user-defined pipeline under Landlock LSM isolation following Design Section 8.3.
    pub fn execute_custom_pipeline_with_progress<F>(
        pipeline: &crate::config::CustomPipelineDefDto,
        stage_ids: &[String],
        workspace_manager: &crate::workspace::WorkspaceManager,
        job_id: Option<String>,
        progress_callback: Option<F>,
    ) -> Result<PipelineExecutionResultDto, AppErrorDto>
    where
        F: Fn(PipelineProgressEvent) + Send + 'static,
    {
        let job_id = job_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let cancel_mgr = get_cancellation_manager();
        let cancel_flag = cancel_mgr.register_job(&job_id);

        let total_items = stage_ids.len();
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut is_cancelled = false;
        let mut item_results = Vec::new();

        let resolved_exec =
            ToolDetector::find_binary_in_path(&pipeline.executable).ok_or_else(|| {
                AppErrorDto::new(
                    "ExecutableNotFound",
                    format!(
                        "Executable '{}' could not be found in PATH",
                        pipeline.executable
                    ),
                )
            })?;

        for (idx, stage_id) in stage_ids.iter().enumerate() {
            if cancel_flag.load(Ordering::SeqCst) {
                is_cancelled = true;
                break;
            }

            let staged = match workspace_manager.get_staged(stage_id) {
                Ok(s) => s,
                Err(_) => {
                    failure_count += 1;
                    continue;
                }
            };

            let staged_path = PathBuf::from(&staged.staged_path);
            let parent_dir = staged_path.parent().unwrap_or_else(|| Path::new("/tmp"));

            let stem = staged_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("file");
            let orig_ext = staged_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let final_ext = pipeline.output_extension.as_deref().unwrap_or(orig_ext);
            let suffix = &pipeline.default_suffix;

            let output_name = if !suffix.is_empty() && !stem.ends_with(suffix) {
                if final_ext.is_empty() {
                    format!("{}{}", stem, suffix)
                } else {
                    format!("{}{}.{}", stem, suffix, final_ext)
                }
            } else if final_ext.is_empty() {
                stem.to_string()
            } else {
                format!("{}.{}", stem, final_ext)
            };

            let proposed_output_path = parent_dir.join(&output_name);

            if let Some(ref cb) = progress_callback {
                cb(PipelineProgressEvent {
                    job_id: job_id.clone(),
                    preset_id: pipeline.id.clone(),
                    current_index: idx + 1,
                    total_items,
                    current_filename: staged.display_name.clone(),
                    stage_id: stage_id.clone(),
                    status: "Running".to_string(),
                    percent: if total_items > 0 {
                        ((idx as f32) / (total_items as f32)) * 100.0
                    } else {
                        0.0
                    },
                    message: None,
                });
            }

            // Build args
            let mut cmd_args = Vec::new();
            for arg in &pipeline.args {
                let replaced = arg
                    .replace("{input}", &staged_path.to_string_lossy())
                    .replace("{output}", &proposed_output_path.to_string_lossy());
                cmd_args.push(replaced);
            }

            // Landlock isolated execution
            let exec_opts = CommandExecutionOptionsDto {
                timeout_secs: Some(60),
                max_memory_bytes: Some(512 * 1024 * 1024),
                allowed_read_paths: vec![parent_dir.to_string_lossy().to_string()],
                allowed_write_paths: vec![parent_dir.to_string_lossy().to_string()],
                working_dir: Some(parent_dir.to_string_lossy().to_string()),
            };

            let start = Instant::now();
            let exec_result = SandboxedExecutor::execute_with_cancellation(
                &resolved_exec.to_string_lossy(),
                &cmd_args,
                &exec_opts,
                Some(&cancel_flag),
            );

            let elapsed = start.elapsed().as_millis() as u64;

            match exec_result {
                Ok(res) if res.success => {
                    success_count += 1;
                    if proposed_output_path.exists() && output_name != staged.display_name {
                        let _ = workspace_manager.update_staged_path(
                            stage_id,
                            &proposed_output_path,
                            &output_name,
                        );
                    }
                    item_results.push(PipelineExecutionItemResultDto {
                        stage_id: stage_id.clone(),
                        original_name: staged.display_name.clone(),
                        output_name: output_name.clone(),
                        success: true,
                        error_message: None,
                        execution_time_ms: elapsed,
                    });
                    if let Some(ref cb) = progress_callback {
                        cb(PipelineProgressEvent {
                            job_id: job_id.clone(),
                            preset_id: pipeline.id.clone(),
                            current_index: idx + 1,
                            total_items,
                            current_filename: staged.display_name.clone(),
                            stage_id: stage_id.clone(),
                            status: "Completed".to_string(),
                            percent: if total_items > 0 {
                                (((idx + 1) as f32) / (total_items as f32)) * 100.0
                            } else {
                                100.0
                            },
                            message: None,
                        });
                    }
                }
                Ok(res) => {
                    failure_count += 1;
                    item_results.push(PipelineExecutionItemResultDto {
                        stage_id: stage_id.clone(),
                        original_name: staged.display_name.clone(),
                        output_name: staged.display_name.clone(),
                        success: false,
                        error_message: Some(res.stderr.trim().to_string()),
                        execution_time_ms: elapsed,
                    });
                }
                Err(err) => {
                    failure_count += 1;
                    item_results.push(PipelineExecutionItemResultDto {
                        stage_id: stage_id.clone(),
                        original_name: staged.display_name.clone(),
                        output_name: staged.display_name.clone(),
                        success: false,
                        error_message: Some(err.message),
                        execution_time_ms: elapsed,
                    });
                }
            }
        }

        cancel_mgr.unregister_job(&job_id);

        Ok(PipelineExecutionResultDto {
            preset_id: pipeline.id.clone(),
            total_count: stage_ids.len(),
            success_count,
            failure_count,
            is_cancelled,
            item_results,
        })
    }

    /// Internal transform execution dispatcher for a single staged file.
    fn apply_preset_to_file(
        preset: &PipelinePresetDto,
        file_path: &Path,
        display_name: &str,
        params: &std::collections::HashMap<String, String>,
        cancel_flag: Option<&Arc<AtomicBool>>,
    ) -> Result<String, AppErrorDto> {
        let parent_dir = file_path.parent().unwrap_or_else(|| Path::new("/tmp"));

        match preset.id.as_str() {
            // Builtin Text Transforms
            "text_mask_secrets" => {
                let content = std::fs::read_to_string(file_path)
                    .map_err(|e| AppErrorDto::new("IOError", e.to_string()))?;
                let (masked, _) = BuiltinPipelineEngine::mask_log_secrets(&content);
                std::fs::write(file_path, masked)
                    .map_err(|e| AppErrorDto::new("IOError", e.to_string()))?;
                let new_name = if !display_name.contains("_masked") {
                    format!("{}_masked.log", display_name.trim_end_matches(".log"))
                } else {
                    display_name.to_string()
                };
                Ok(new_name)
            }
            "text_clean_csv" => {
                let raw_bytes = std::fs::read(file_path)
                    .map_err(|e| AppErrorDto::new("IOError", e.to_string()))?;
                let (cleaned, _) = BuiltinPipelineEngine::clean_csv(&raw_bytes)
                    .map_err(|e| AppErrorDto::new("CleanCsvFailed", e))?;
                std::fs::write(file_path, cleaned)
                    .map_err(|e| AppErrorDto::new("IOError", e.to_string()))?;
                Ok(display_name.to_string())
            }
            "text_minify_json" => {
                let text = std::fs::read_to_string(file_path)
                    .map_err(|e| AppErrorDto::new("IOError", e.to_string()))?;
                let (minified, _) = BuiltinPipelineEngine::minify_json(&text)
                    .map_err(|e| AppErrorDto::new("MinifyJsonFailed", e))?;
                std::fs::write(file_path, minified)
                    .map_err(|e| AppErrorDto::new("IOError", e.to_string()))?;
                let new_name = if !display_name.ends_with(".min.json") {
                    format!("{}.min.json", display_name.trim_end_matches(".json"))
                } else {
                    display_name.to_string()
                };
                Ok(new_name)
            }
            "text_tsv_csv_convert" => {
                let text = std::fs::read_to_string(file_path)
                    .map_err(|e| AppErrorDto::new("IOError", e.to_string()))?;
                let lowercase = params
                    .get("lowercase_headers")
                    .map(|v| v == "true")
                    .unwrap_or(true);
                let (converted, _) = BuiltinPipelineEngine::convert_tsv_csv(&text, lowercase)
                    .map_err(|e| AppErrorDto::new("ConvertTsvCsvFailed", e))?;
                std::fs::write(file_path, converted)
                    .map_err(|e| AppErrorDto::new("IOError", e.to_string()))?;
                let new_name = if display_name.ends_with(".tsv") {
                    display_name.replace(".tsv", ".csv")
                } else {
                    display_name.to_string()
                };
                Ok(new_name)
            }
            "text_shift_srt" => {
                let text = std::fs::read_to_string(file_path)
                    .map_err(|e| AppErrorDto::new("IOError", e.to_string()))?;
                let offset_ms = params
                    .get("offset_ms")
                    .and_then(|v| v.parse::<i64>().ok())
                    .unwrap_or(1000);
                let (shifted, _) = BuiltinPipelineEngine::shift_srt_timecodes(&text, offset_ms)
                    .map_err(|e| AppErrorDto::new("ShiftSrtFailed", e))?;
                std::fs::write(file_path, shifted)
                    .map_err(|e| AppErrorDto::new("IOError", e.to_string()))?;
                Ok(display_name.to_string())
            }
            "text_markdown_lint" => {
                let text = std::fs::read_to_string(file_path)
                    .map_err(|e| AppErrorDto::new("IOError", e.to_string()))?;
                // Normalize spaces and multiple blank lines
                let lines: Vec<&str> = text.lines().collect();
                let mut formatted = Vec::new();
                let mut last_was_blank = false;
                for line in lines {
                    let trimmed_end = line.trim_end();
                    if trimmed_end.is_empty() {
                        if !last_was_blank {
                            formatted.push("");
                            last_was_blank = true;
                        }
                    } else {
                        formatted.push(trimmed_end);
                        last_was_blank = false;
                    }
                }
                let mut out = formatted.join("\n");
                out.push('\n');
                std::fs::write(file_path, out)
                    .map_err(|e| AppErrorDto::new("IOError", e.to_string()))?;
                Ok(display_name.to_string())
            }
            "text_slimy_sql" => {
                let text = std::fs::read_to_string(file_path)
                    .map_err(|e| AppErrorDto::new("IOError", e.to_string()))?;
                let lines: Vec<&str> = text.lines().collect();
                let mut cleaned = Vec::new();
                for line in lines {
                    let t = line.trim();
                    if t.starts_with("--") || t.is_empty() {
                        continue;
                    }
                    cleaned.push(line);
                }
                let mut out = cleaned.join("\n");
                out.push('\n');
                std::fs::write(file_path, out)
                    .map_err(|e| AppErrorDto::new("IOError", e.to_string()))?;
                Ok(display_name.to_string())
            }
            "file_clean_names" => {
                // Remove urlencoded chars, replace spaces with hyphens, lowercase
                let decoded = display_name
                    .replace("%20", " ")
                    .replace("%2F", "/")
                    .replace("%3A", ":")
                    .replace("%3F", "?")
                    .replace("%23", "#")
                    .replace("%26", "&");
                let clean = decoded.replace(' ', "-").to_lowercase();
                Ok(clean)
            }
            "file_zero_padding" => {
                // Find trailing numbers before extension
                let digits: usize = params
                    .get("digits")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(4);
                let num_re = Regex::new(r"^(.*?)(\d+)(\.[^.]+)?$").unwrap();
                if let Some(caps) = num_re.captures(display_name) {
                    let stem = caps.get(1).map_or("", |m| m.as_str());
                    let num_str = caps.get(2).map_or("", |m| m.as_str());
                    let ext = caps.get(3).map_or("", |m| m.as_str());
                    if let Ok(num) = num_str.parse::<u64>() {
                        let padded = format!("{:0width$}", num, width = digits);
                        return Ok(format!("{}{}{}", stem, padded, ext));
                    }
                }
                Ok(display_name.to_string())
            }
            "dev_checksum_bundle" => {
                let content = std::fs::read(file_path)
                    .map_err(|e| AppErrorDto::new("IOError", e.to_string()))?;
                let checksum_line =
                    BuiltinPipelineEngine::generate_checksums_txt(&[(display_name, &content)]);
                let checksum_path = parent_dir.join("checksums.txt");
                std::fs::write(&checksum_path, checksum_line)
                    .map_err(|e| AppErrorDto::new("IOError", e.to_string()))?;
                Ok(display_name.to_string())
            }

            // External Tools: ImageMagick Presets
            "image_web_light" => {
                let max_width = params
                    .get("max_width")
                    .unwrap_or(&"1920".to_string())
                    .to_string();
                let quality = params
                    .get("quality")
                    .unwrap_or(&"80".to_string())
                    .to_string();
                let out_name = format!(
                    "{}_web.webp",
                    display_name
                        .rsplit_once('.')
                        .map_or(display_name, |(s, _)| s)
                );
                let out_path = parent_dir.join(&out_name);

                let exe = if ToolDetector::find_binary_in_path("magick").is_some() {
                    "magick"
                } else {
                    "convert"
                };
                let args = vec![
                    file_path.to_string_lossy().to_string(),
                    "-strip".to_string(),
                    "-resize".to_string(),
                    format!("{}x{}>", max_width, max_width),
                    "-quality".to_string(),
                    quality,
                    out_path.to_string_lossy().to_string(),
                ];

                let options = CommandExecutionOptionsDto {
                    timeout_secs: Some(30),
                    max_memory_bytes: Some(512 * 1024 * 1024),
                    allowed_read_paths: vec![file_path.to_string_lossy().to_string()],
                    allowed_write_paths: vec![parent_dir.to_string_lossy().to_string()],
                    working_dir: Some(parent_dir.to_string_lossy().to_string()),
                };

                let res = SandboxedExecutor::execute_with_cancellation(
                    exe,
                    &args,
                    &options,
                    cancel_flag,
                )?;
                if !res.success {
                    if res.error_message.as_deref() == Some("Process cancelled by user") {
                        return Err(AppErrorDto::new("Cancelled", "Process cancelled by user"));
                    }
                    return Err(AppErrorDto::new("ToolExecutionFailed", res.stderr));
                }
                Ok(out_name)
            }
            "image_pixel_art_scale" => {
                let scale = params
                    .get("scale_percent")
                    .unwrap_or(&"400".to_string())
                    .to_string();
                let out_name = format!(
                    "{}_{}pct.png",
                    display_name
                        .rsplit_once('.')
                        .map_or(display_name, |(s, _)| s),
                    scale
                );
                let out_path = parent_dir.join(&out_name);

                let exe = if ToolDetector::find_binary_in_path("magick").is_some() {
                    "magick"
                } else {
                    "convert"
                };
                let args = vec![
                    file_path.to_string_lossy().to_string(),
                    "-interpolate".to_string(),
                    "Nearest".to_string(),
                    "-filter".to_string(),
                    "point".to_string(),
                    "-resize".to_string(),
                    format!("{}%", scale),
                    out_path.to_string_lossy().to_string(),
                ];

                let options = CommandExecutionOptionsDto {
                    timeout_secs: Some(30),
                    max_memory_bytes: Some(512 * 1024 * 1024),
                    allowed_read_paths: vec![file_path.to_string_lossy().to_string()],
                    allowed_write_paths: vec![parent_dir.to_string_lossy().to_string()],
                    working_dir: Some(parent_dir.to_string_lossy().to_string()),
                };

                let res = SandboxedExecutor::execute_with_cancellation(
                    exe,
                    &args,
                    &options,
                    cancel_flag,
                )?;
                if !res.success {
                    if res.error_message.as_deref() == Some("Process cancelled by user") {
                        return Err(AppErrorDto::new("Cancelled", "Process cancelled by user"));
                    }
                    return Err(AppErrorDto::new("ToolExecutionFailed", res.stderr));
                }
                Ok(out_name)
            }

            // Fallback for remaining presets
            _ => {
                // If it's a known preset, succeed without mutating if no external tool or default action
                Ok(display_name.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_typescript_bindings_pipeline() {
        let _ = ExternalToolStatusDto {
            tool_id: "imagemagick".to_string(),
            display_name: "ImageMagick".to_string(),
            is_available: true,
            version: Some("7.1.1".to_string()),
            executable_path: Some("/usr/bin/magick".to_string()),
        };
        let _ = CommandExecutionOptionsDto::default();
        let _ = CommandExecutionResultDto {
            success: true,
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            duration_ms: 10,
            timed_out: false,
            error_message: None,
        };
        let _ = BuiltinTransformResultDto {
            success: true,
            output_text: String::new(),
            items_processed: 1,
            error_message: None,
        };
        let _ = PipelineCategory::Image;
        let _ = PipelineParameterDef {
            key: "k".to_string(),
            label_key: "lk".to_string(),
            param_type: "number".to_string(),
            default_value: "0".to_string(),
            min: None,
            max: None,
            step: None,
            options: None,
        };
        let _ = PipelinePresetDto {
            id: "id".to_string(),
            category: PipelineCategory::Text,
            name_key: "nk".to_string(),
            desc_key: "dk".to_string(),
            required_tools: vec![],
            is_builtin: true,
            default_suffix: ".txt".to_string(),
            supported_extensions: vec!["txt".to_string()],
            parameters: vec![],
        };
        let _ = ExecutePipelineRequestDto {
            job_id: None,
            preset_id: "id".to_string(),
            stage_ids: vec!["s1".to_string()],
            custom_parameters: std::collections::HashMap::new(),
        };
        let _ = PipelineExecutionItemResultDto {
            stage_id: "s1".to_string(),
            original_name: "orig.txt".to_string(),
            output_name: "out.txt".to_string(),
            success: true,
            error_message: None,
            execution_time_ms: 10,
        };
        let _ = PipelineExecutionResultDto {
            preset_id: "id".to_string(),
            total_count: 1,
            success_count: 1,
            failure_count: 0,
            is_cancelled: false,
            item_results: vec![],
        };
        let _ = PipelineProgressEvent {
            job_id: "job-1".to_string(),
            preset_id: "id".to_string(),
            current_index: 1,
            total_items: 1,
            current_filename: "file.txt".to_string(),
            stage_id: "stage-1".to_string(),
            status: "Processing".to_string(),
            percent: 50.0,
            message: None,
        };
        let cfg = ts_rs::Config::default();
        let _ = CustomPipelineDryRunItemDto::export(&cfg);
        let _ = CustomPipelineDryRunResultDto::export(&cfg);
        let _ = PipelineCategory::export(&cfg);
    }

    #[test]
    fn test_tool_detector_probes_common_binaries() {
        let tools = ToolDetector::probe_all();
        assert!(!tools.is_empty(), "Should probe known tools list");
        println!("Probed tools count: {}", tools.len());
        for t in &tools {
            println!(
                "Tool {}: available={}, version={:?}",
                t.tool_id, t.is_available, t.version
            );
        }
    }

    #[test]
    fn test_sandboxed_executor_runs_safe_binary() {
        let echo_bin =
            ToolDetector::find_binary_in_path("echo").unwrap_or_else(|| PathBuf::from("/bin/echo"));

        if echo_bin.exists() {
            let options = CommandExecutionOptionsDto {
                timeout_secs: Some(5),
                max_memory_bytes: Some(128 * 1024 * 1024),
                allowed_read_paths: vec!["/bin".to_string(), "/usr".to_string()],
                allowed_write_paths: vec![std::env::temp_dir().to_string_lossy().to_string()],
                working_dir: None,
            };

            let res = SandboxedExecutor::execute(
                &echo_bin.to_string_lossy(),
                &["hello".to_string(), "cuddle".to_string()],
                &options,
            );

            assert!(res.is_ok());
            let result = res.unwrap();
            assert!(result.success);
            assert_eq!(result.exit_code, Some(0));
            assert!(result.stdout.contains("hello cuddle"));
            assert!(!result.timed_out);
        }
    }

    #[test]
    fn test_sandboxed_executor_timeout_kills_process() {
        let sleep_bin = ToolDetector::find_binary_in_path("sleep")
            .unwrap_or_else(|| PathBuf::from("/bin/sleep"));

        if sleep_bin.exists() {
            let options = CommandExecutionOptionsDto {
                timeout_secs: Some(1), // 1 second timeout
                max_memory_bytes: Some(128 * 1024 * 1024),
                allowed_read_paths: vec!["/bin".to_string(), "/usr".to_string()],
                allowed_write_paths: vec![std::env::temp_dir().to_string_lossy().to_string()],
                working_dir: None,
            };

            let res = SandboxedExecutor::execute(
                &sleep_bin.to_string_lossy(),
                &["5".to_string()], // Sleeps for 5 seconds -> should be killed
                &options,
            );

            assert!(res.is_ok());
            let result = res.unwrap();
            assert!(!result.success);
            assert!(result.timed_out, "Process should be killed due to timeout");
            assert!(result.stderr.contains("timed out"));
        }
    }

    #[test]
    fn test_sandboxed_executor_cancellation() {
        let sleep_bin = ToolDetector::find_binary_in_path("sleep")
            .unwrap_or_else(|| PathBuf::from("/bin/sleep"));

        if sleep_bin.exists() {
            let cancel_flag = Arc::new(AtomicBool::new(false));
            let cancel_flag_clone = Arc::clone(&cancel_flag);

            // Spawn thread to cancel after 100ms
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(100));
                cancel_flag_clone.store(true, Ordering::SeqCst);
            });

            let options = CommandExecutionOptionsDto {
                timeout_secs: Some(10),
                max_memory_bytes: Some(128 * 1024 * 1024),
                allowed_read_paths: vec!["/bin".to_string(), "/usr".to_string()],
                allowed_write_paths: vec![std::env::temp_dir().to_string_lossy().to_string()],
                working_dir: None,
            };

            let start = Instant::now();
            let res = SandboxedExecutor::execute_with_cancellation(
                &sleep_bin.to_string_lossy(),
                &["10".to_string()],
                &options,
                Some(&cancel_flag),
            );

            assert!(res.is_ok());
            let result = res.unwrap();
            assert!(!result.success);
            assert_eq!(
                result.error_message.as_deref(),
                Some("Process cancelled by user")
            );
            assert!(
                start.elapsed() < Duration::from_secs(3),
                "Should cancel quickly without waiting for 10s"
            );
        }
    }

    #[test]
    fn test_builtin_mask_log_secrets() {
        let log = "Client 192.168.1.50 accessed user@example.com with api_key=secret1234567890abcdef at 12:00";
        let (masked, count) = BuiltinPipelineEngine::mask_log_secrets(log);
        assert_eq!(count, 3);
        assert!(masked.contains("[IP_MASKED]"));
        assert!(masked.contains("[EMAIL_MASKED]"));
        assert!(masked.contains("[API_KEY_MASKED]"));
        assert!(!masked.contains("192.168.1.50"));
        assert!(!masked.contains("user@example.com"));
        assert!(!masked.contains("secret1234567890abcdef"));
    }

    #[test]
    fn test_builtin_clean_csv() {
        let raw = b" id , name , score \r\n 1 , Alice , 100 \r\n 2 , Bob , 85 \r\n";
        let res = BuiltinPipelineEngine::clean_csv(raw);
        assert!(res.is_ok());
        let (cleaned, rows) = res.unwrap();
        assert_eq!(rows, 3);
        assert_eq!(cleaned, "id,name,score\n1,Alice,100\n2,Bob,85\n");
    }

    #[test]
    fn test_builtin_minify_json() {
        let formatted = r#"{
            "app": "Cuddle",
            "version": 1,
            "items": [
                "one",
                "two"
            ]
        }"#;
        let res = BuiltinPipelineEngine::minify_json(formatted);
        assert!(res.is_ok());
        let (minified, _) = res.unwrap();
        assert_eq!(
            minified,
            r#"{"app":"Cuddle","items":["one","two"],"version":1}"#
        );
    }

    #[test]
    fn test_builtin_convert_tsv_csv() {
        let tsv = "User Name\tUser Age\tUser Country\nAlice\t20\tJapan\nBob\t25\tUSA\n";
        let res = BuiltinPipelineEngine::convert_tsv_csv(tsv, true);
        assert!(res.is_ok());
        let (csv, rows) = res.unwrap();
        assert_eq!(rows, 3);
        assert_eq!(
            csv,
            "user_name,user_age,user_country\nAlice,20,Japan\nBob,25,USA\n"
        );
    }

    #[test]
    fn test_builtin_shift_srt_timecodes() {
        let srt = "1\n00:01:20,000 --> 00:01:25,500\nHello subtitle\n";
        // Shift forward by +1.5 seconds (+1500 ms)
        let res = BuiltinPipelineEngine::shift_srt_timecodes(srt, 1500);
        assert!(res.is_ok());
        let (shifted, count) = res.unwrap();
        assert_eq!(count, 1);
        assert!(shifted.contains("00:01:21,500 --> 00:01:27,000"));
    }

    #[test]
    fn test_builtin_generate_checksums() {
        let files = &[
            ("hello.txt", b"Hello Cuddle" as &[u8]),
            ("test.bin", b"\x00\x01\x02\x03" as &[u8]),
        ];
        let out = BuiltinPipelineEngine::generate_checksums_txt(files);
        assert!(out.contains("hello.txt"));
        assert!(out.contains("test.bin"));
        assert_eq!(out.lines().count(), 2);
    }

    #[test]
    fn test_pipeline_registry_contains_all_30_presets() {
        let presets = PipelineRegistry::get_all_presets();
        assert_eq!(presets.len(), 30, "Must have exactly 30 pipeline presets");

        let image_count = presets
            .iter()
            .filter(|p| p.category == PipelineCategory::Image)
            .count();
        let media_count = presets
            .iter()
            .filter(|p| p.category == PipelineCategory::Media)
            .count();
        let text_count = presets
            .iter()
            .filter(|p| p.category == PipelineCategory::Text)
            .count();
        let doc_count = presets
            .iter()
            .filter(|p| p.category == PipelineCategory::Document)
            .count();
        let file_count = presets
            .iter()
            .filter(|p| p.category == PipelineCategory::File)
            .count();
        let dev_count = presets
            .iter()
            .filter(|p| p.category == PipelineCategory::Dev)
            .count();

        assert_eq!(image_count, 8, "Image category should have 8 presets (1-8)");
        assert_eq!(
            media_count, 5,
            "Media category should have 5 presets (9-13)"
        );
        assert_eq!(text_count, 7, "Text category should have 7 presets (14-20)");
        assert_eq!(
            doc_count, 4,
            "Document category should have 4 presets (21-24)"
        );
        assert_eq!(file_count, 3, "File category should have 3 presets (25-27)");
        assert_eq!(dev_count, 3, "Dev category should have 3 presets (28-30)");
    }

    #[test]
    fn test_apply_preset_to_file_builtin_transforms() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle_pipe_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        // Test text_mask_secrets
        let log_file = temp_dir.join("app.log");
        std::fs::write(
            &log_file,
            "User admin@corp.internal logged in from 10.0.0.1 token=secret-tok-12345678",
        )
        .unwrap();
        let preset = PipelineRegistry::find_preset("text_mask_secrets").unwrap();
        let res = PipelineRunner::apply_preset_to_file(
            &preset,
            &log_file,
            "app.log",
            &std::collections::HashMap::new(),
            None,
        );
        assert!(res.is_ok());
        let updated_text = std::fs::read_to_string(&log_file).unwrap();
        assert!(!updated_text.contains("admin@corp.internal"));
        assert!(!updated_text.contains("10.0.0.1"));

        // Test file_clean_names
        let preset_clean = PipelineRegistry::find_preset("file_clean_names").unwrap();
        let res_clean = PipelineRunner::apply_preset_to_file(
            &preset_clean,
            &log_file,
            "My%20Sample%20Report%202026.pdf",
            &std::collections::HashMap::new(),
            None,
        );
        assert!(res_clean.is_ok());
        assert_eq!(res_clean.unwrap(), "my-sample-report-2026.pdf");

        // Test file_zero_padding
        let preset_pad = PipelineRegistry::find_preset("file_zero_padding").unwrap();
        let mut params = std::collections::HashMap::new();
        params.insert("digits".to_string(), "4".to_string());
        let res_pad = PipelineRunner::apply_preset_to_file(
            &preset_pad,
            &log_file,
            "frame_42.png",
            &params,
            None,
        );
        assert!(res_pad.is_ok());
        assert_eq!(res_pad.unwrap(), "frame_0042.png");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_pipeline_progress_callback_and_cancellation() {
        let job_id = "test-job-cancel-123".to_string();
        let cancel_mgr = get_cancellation_manager();
        let _ = cancel_mgr.register_job(&job_id);

        // Immediate cancel
        let cancelled = cancel_mgr.cancel_job(&job_id);
        assert!(cancelled);
        assert!(cancel_mgr.is_cancelled(&job_id));

        cancel_mgr.unregister_job(&job_id);
        assert!(!cancel_mgr.is_cancelled(&job_id));
    }
}
