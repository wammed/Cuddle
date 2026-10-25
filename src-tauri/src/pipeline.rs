use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use regex::Regex;
use sha2::{Digest, Sha256};
use crate::models::AppErrorDto;
use crate::sandbox::LandlockSandbox;

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
        cmd.arg(arg)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

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
        let isolated_tmp = options.working_dir.as_ref().map(PathBuf::from).unwrap_or_else(std::env::temp_dir);

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
            AppErrorDto::new("ERR_PROCESS_SPAWN_FAILED", format!("Failed to spawn {}: {}", executable, e))
        })?;

        // Monitor process with timeout
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    let output = child.wait_with_output().map_err(|e| {
                        AppErrorDto::new("ERR_PROCESS_IO", format!("Failed to read process output: {}", e))
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
            stderr: format!("Process timed out after {} seconds and was killed", timeout_secs),
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
        let re_token = Regex::new(r"(?i)\b(?:bearer\s+|api[_-]?key\s*[:=]\s*)[a-zA-Z0-9_\-\.]{16,}").unwrap();

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
        let val: serde_json::Value = serde_json::from_str(text)
            .map_err(|e| format!("Invalid JSON syntax: {}", e))?;
        let minified = serde_json::to_string(&val)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
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
            if trimmed.is_empty() || trimmed.starts_with("--") || trimmed.starts_with("/*") && trimmed.ends_with("*/") {
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
        let time_re = Regex::new(r"(\d{2}):(\d{2}):(\d{2}),(\d{3})\s*-->\s*(\d{2}):(\d{2}):(\d{2}),(\d{3})").unwrap();
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
    }

    #[test]
    fn test_tool_detector_probes_common_binaries() {
        let tools = ToolDetector::probe_all();
        assert!(!tools.is_empty(), "Should probe known tools list");
        println!("Probed tools count: {}", tools.len());
        for t in &tools {
            println!("Tool {}: available={}, version={:?}", t.tool_id, t.is_available, t.version);
        }
    }

    #[test]
    fn test_sandboxed_executor_runs_safe_binary() {
        let echo_bin = ToolDetector::find_binary_in_path("echo")
            .unwrap_or_else(|| PathBuf::from("/bin/echo"));

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
        assert_eq!(minified, r#"{"app":"Cuddle","items":["one","two"],"version":1}"#);
    }

    #[test]
    fn test_builtin_convert_tsv_csv() {
        let tsv = "User Name\tUser Age\tUser Country\nAlice\t20\tJapan\nBob\t25\tUSA\n";
        let res = BuiltinPipelineEngine::convert_tsv_csv(tsv, true);
        assert!(res.is_ok());
        let (csv, rows) = res.unwrap();
        assert_eq!(rows, 3);
        assert_eq!(csv, "user_name,user_age,user_country\nAlice,20,Japan\nBob,25,USA\n");
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
}
