use crate::models::{decode_path_from_base64, FileItem};
use crate::sandbox::LandlockSandbox;
use crate::search_fulltext;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

const DEFAULT_MAX_BYTES: usize = 10 * 1024 * 1024; // 10 MiB size limit
const DEFAULT_MAX_CHARS: usize = 200_000; // 200k characters text limit
const DEFAULT_MEMORY_LIMIT_BYTES: u64 = 256 * 1024 * 1024; // 256 MiB RLIMIT_AS
const DEFAULT_TIMEOUT_SECS: u64 = 30; // 30 seconds

#[derive(Error, Debug)]
pub enum ExtractorError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Model error: {0}")]
    Model(#[from] crate::models::ModelError),
    #[error("Fulltext engine error: {0}")]
    Fulltext(#[from] search_fulltext::FulltextError),
    #[error("Extraction timeout after {0} seconds")]
    Timeout(u64),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

/// Options controlling the text extraction behavior.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct ExtractTextOptions {
    /// Maximum bytes to read from disk (default: 10 MiB)
    pub max_bytes: Option<usize>,
    /// Maximum character length of returned text (default: 200,000)
    pub max_chars: Option<usize>,
    /// Memory limit in bytes for isolated Tier 2 child processes (default: 256 MiB)
    pub memory_limit_bytes: Option<u64>,
    /// Timeout in seconds for isolated Tier 2 child processes (default: 30s)
    pub timeout_secs: Option<u64>,
}

impl Default for ExtractTextOptions {
    fn default() -> Self {
        Self {
            max_bytes: Some(DEFAULT_MAX_BYTES),
            max_chars: Some(DEFAULT_MAX_CHARS),
            memory_limit_bytes: Some(DEFAULT_MEMORY_LIMIT_BYTES),
            timeout_secs: Some(DEFAULT_TIMEOUT_SECS),
        }
    }
}

/// Result DTO returned after extracting text from a file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct ExtractedTextDto {
    pub item_id: String,
    pub raw_base64: String,
    pub display_name: String,
    pub text: String,
    pub extracted_chars: usize,
    /// Tier mechanism used: "tier1_direct" | "tier2_isolated_pdf" | "skipped_binary"
    pub tier_used: String,
    /// Detected encoding: "UTF-8" | "Shift_JIS" | "EUC-JP" | "N/A"
    pub encoding_used: String,
    pub truncated: bool,
    pub warning: Option<String>,
}

/// Checks whether a given path is a known plain-text or source-code extension.
pub fn is_tier1_text_extension(ext: &str) -> bool {
    let lower = ext.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "txt"
            | "md"
            | "markdown"
            | "json"
            | "csv"
            | "tsv"
            | "toml"
            | "yaml"
            | "yml"
            | "rs"
            | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "py"
            | "sh"
            | "bash"
            | "zsh"
            | "c"
            | "cpp"
            | "cc"
            | "h"
            | "hpp"
            | "go"
            | "html"
            | "htm"
            | "css"
            | "scss"
            | "xml"
            | "log"
            | "ini"
            | "conf"
            | "sql"
            | "env"
            | "diff"
            | "patch"
            | "gitignore"
            | "desktop"
    )
}

/// Checks whether a given path is a PDF document requiring Tier 2 isolation.
pub fn is_tier2_pdf_extension(ext: &str) -> bool {
    ext.eq_ignore_ascii_case("pdf")
}

/// Checks whether a file is likely binary by scanning the first buffer for NULL bytes.
pub fn is_binary_buffer(buffer: &[u8]) -> bool {
    // If there is a null byte (\0) in the first 8 KB, treat as binary
    let check_len = buffer.len().min(8192);
    buffer[..check_len].contains(&0)
}

/// Decodes raw bytes using encoding detection (UTF-8, Shift_JIS, EUC-JP fallback).
pub fn decode_text_bytes(bytes: &[u8]) -> (String, &'static str) {
    // 1. Check for UTF-8 BOM
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        let content = String::from_utf8_lossy(&bytes[3..]).into_owned();
        return (content, "UTF-8");
    }

    // 2. Try standard valid UTF-8
    if let Ok(utf8_str) = std::str::from_utf8(bytes) {
        return (utf8_str.to_string(), "UTF-8");
    }

    // 3. Use Mozilla chardetng for statistical Japanese encoding detection
    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(bytes, true);
    let encoding = detector.guess(None, true);
    let (cow, _, _) = encoding.decode(bytes);

    let name = match encoding.name() {
        "Shift_JIS" | "windows-31j" => "Shift_JIS",
        "EUC-JP" => "EUC-JP",
        "UTF-8" => "UTF-8",
        _ => encoding.name(),
    };

    (cow.into_owned(), name)
}

/// Tier 1: Fast direct in-process extraction for text/code files.
fn extract_tier1_direct(
    path: &Path,
    item: &FileItem,
    options: &ExtractTextOptions,
) -> Result<ExtractedTextDto, ExtractorError> {
    let max_bytes = options.max_bytes.unwrap_or(DEFAULT_MAX_BYTES);
    let max_chars = options.max_chars.unwrap_or(DEFAULT_MAX_CHARS);

    let mut file = File::open(path)?;
    let file_len = file.metadata()?.len() as usize;

    let read_limit = file_len.min(max_bytes);
    let mut buffer = vec![0u8; read_limit];
    file.read_exact(&mut buffer)?;

    let mut truncated = file_len > max_bytes;

    // Fast binary probe for extensionless or ambiguous files
    if is_binary_buffer(&buffer) {
        return Ok(ExtractedTextDto {
            item_id: item.item_id.clone(),
            raw_base64: item.raw_base64.clone(),
            display_name: item.display_name.clone(),
            text: String::new(),
            extracted_chars: 0,
            tier_used: "skipped_binary".to_string(),
            encoding_used: "N/A".to_string(),
            truncated: false,
            warning: Some("File contains binary data".to_string()),
        });
    }

    let (mut decoded_text, encoding_used) = decode_text_bytes(&buffer);

    if decoded_text.chars().count() > max_chars {
        decoded_text = decoded_text.chars().take(max_chars).collect();
        truncated = true;
    }

    let extracted_chars = decoded_text.chars().count();

    Ok(ExtractedTextDto {
        item_id: item.item_id.clone(),
        raw_base64: item.raw_base64.clone(),
        display_name: item.display_name.clone(),
        text: decoded_text,
        extracted_chars,
        tier_used: "tier1_direct".to_string(),
        encoding_used: encoding_used.to_string(),
        truncated,
        warning: None,
    })
}

/// Checks whether `pdftotext` is installed on the host system.
fn is_pdftotext_available() -> bool {
    Command::new("pdftotext")
        .arg("-v")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success() || s.code() == Some(99)) // poppler's pdftotext -v exits with 99 on some versions
        .unwrap_or(false)
}

/// Tier 2: Isolated process extraction with Landlock LSM read-only original protection,
/// bounded memory quota (RLIMIT_AS), and timeout.
fn extract_tier2_pdf(
    path: &Path,
    item: &FileItem,
    options: &ExtractTextOptions,
) -> Result<ExtractedTextDto, ExtractorError> {
    if !is_pdftotext_available() {
        return Ok(ExtractedTextDto {
            item_id: item.item_id.clone(),
            raw_base64: item.raw_base64.clone(),
            display_name: item.display_name.clone(),
            text: String::new(),
            extracted_chars: 0,
            tier_used: "tier2_isolated_pdf".to_string(),
            encoding_used: "N/A".to_string(),
            truncated: false,
            warning: Some("pdftotext (poppler-utils) is not installed. PDF extraction skipped safely.".to_string()),
        });
    }

    let max_chars = options.max_chars.unwrap_or(DEFAULT_MAX_CHARS);
    let mem_limit = options.memory_limit_bytes.unwrap_or(DEFAULT_MEMORY_LIMIT_BYTES);
    let timeout_secs = options.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS);

    // Setup isolated tempdir for Landlock execution
    let temp_sub = std::env::temp_dir().join(format!("cuddle-pdf-{}", Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&temp_sub);

    // Landlock sandbox: system defaults + allow read of the PDF's parent directory
    let parent_dir = path.parent().unwrap_or(path);
    let sandbox = LandlockSandbox::new()
        .add_system_defaults()
        .allow_read(parent_dir);

    let mut cmd = Command::new("pdftotext");
    cmd.arg("-layout")
        .arg(path)
        .arg("-") // Output to stdout
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    sandbox.configure_command_with_limits(&mut cmd, &temp_sub, Some(mem_limit));

    let child = cmd.spawn()?;

    // Handle timeout with bounded polling
    let start_time = std::time::Instant::now();
    let poll_interval = Duration::from_millis(50);
    let timeout_dur = Duration::from_secs(timeout_secs);

    let mut child = child;
    let output = loop {
        match child.try_wait()? {
            Some(_status) => {
                break child.wait_with_output()?;
            }
            None => {
                if start_time.elapsed() >= timeout_dur {
                    let _ = child.kill();
                    let _ = std::fs::remove_dir_all(&temp_sub);
                    return Err(ExtractorError::Timeout(timeout_secs));
                }
                std::thread::sleep(poll_interval);
            }
        }
    };

    // Clean up isolated temporary directory
    let _ = std::fs::remove_dir_all(&temp_sub);

    if !output.status.success() {
        return Ok(ExtractedTextDto {
            item_id: item.item_id.clone(),
            raw_base64: item.raw_base64.clone(),
            display_name: item.display_name.clone(),
            text: String::new(),
            extracted_chars: 0,
            tier_used: "tier2_isolated_pdf".to_string(),
            encoding_used: "N/A".to_string(),
            truncated: false,
            warning: Some(format!("pdftotext exited with non-zero status: {:?}", output.status.code())),
        });
    }

    let (mut decoded_text, encoding_used) = decode_text_bytes(&output.stdout);
    let mut truncated = false;

    if decoded_text.chars().count() > max_chars {
        decoded_text = decoded_text.chars().take(max_chars).collect();
        truncated = true;
    }

    let extracted_chars = decoded_text.chars().count();

    Ok(ExtractedTextDto {
        item_id: item.item_id.clone(),
        raw_base64: item.raw_base64.clone(),
        display_name: item.display_name.clone(),
        text: decoded_text,
        extracted_chars,
        tier_used: "tier2_isolated_pdf".to_string(),
        encoding_used: encoding_used.to_string(),
        truncated,
        warning: None,
    })
}

/// Unified entry point to extract text from a file safely.
pub fn extract_file_text(
    path: &Path,
    options: Option<ExtractTextOptions>,
) -> Result<ExtractedTextDto, ExtractorError> {
    let opts = options.unwrap_or_default();
    let item = FileItem::from_path(path)?;

    if item.is_dir {
        return Ok(ExtractedTextDto {
            item_id: item.item_id.clone(),
            raw_base64: item.raw_base64.clone(),
            display_name: item.display_name.clone(),
            text: String::new(),
            extracted_chars: 0,
            tier_used: "skipped_binary".to_string(),
            encoding_used: "N/A".to_string(),
            truncated: false,
            warning: Some("Target is a directory".to_string()),
        });
    }

    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    if is_tier2_pdf_extension(ext) {
        extract_tier2_pdf(path, &item, &opts)
    } else if is_tier1_text_extension(ext) || ext.is_empty() {
        extract_tier1_direct(path, &item, &opts)
    } else {
        // Binary or unsupported media format
        Ok(ExtractedTextDto {
            item_id: item.item_id.clone(),
            raw_base64: item.raw_base64.clone(),
            display_name: item.display_name.clone(),
            text: String::new(),
            extracted_chars: 0,
            tier_used: "skipped_binary".to_string(),
            encoding_used: "N/A".to_string(),
            truncated: false,
            warning: Some(format!("Unsupported binary extension: .{}", ext)),
        })
    }
}

/// Extracts text from a Base64-encoded path and immediately indexes it into the Tantivy full-text index.
pub fn index_file_with_text(
    path_base64: &str,
    options: Option<ExtractTextOptions>,
) -> Result<ExtractedTextDto, ExtractorError> {
    let path = decode_path_from_base64(path_base64)?;
    let item = FileItem::from_path(&path)?;
    let extracted = extract_file_text(&path, options)?;

    if !extracted.text.is_empty() {
        let engine = search_fulltext::get_or_init_engine()?;
        engine.index_document(&item, &extracted.text)?;
        engine.commit()?;
    }

    Ok(extracted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_tier1_utf8_text_extraction() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("sample.md");

        let sample_content = "# Cuddle ドキュメント\n原本保護型ファイルワークベンチ";
        fs::write(&file_path, sample_content).unwrap();

        let result = extract_file_text(&file_path, None).unwrap();
        assert_eq!(result.tier_used, "tier1_direct");
        assert_eq!(result.encoding_used, "UTF-8");
        assert_eq!(result.text, sample_content);
        assert_eq!(result.extracted_chars, sample_content.chars().count());
        assert!(!result.truncated);
        assert!(result.warning.is_none());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_tier1_shift_jis_text_extraction() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("sjis_file.txt");

        let original_text = "日本語Shift_JISテストデータ原本保護";
        let (sjis_bytes, _, _) = encoding_rs::SHIFT_JIS.encode(original_text);
        fs::write(&file_path, sjis_bytes).unwrap();

        let result = extract_file_text(&file_path, None).unwrap();
        assert_eq!(result.tier_used, "tier1_direct");
        assert_eq!(result.encoding_used, "Shift_JIS");
        assert_eq!(result.text, original_text);
        assert!(!result.truncated);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_tier1_euc_jp_text_extraction() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("euc_file.txt");

        let original_text = "EUC-JPテキストテスト";
        let (euc_bytes, _, _) = encoding_rs::EUC_JP.encode(original_text);
        fs::write(&file_path, euc_bytes).unwrap();

        let result = extract_file_text(&file_path, None).unwrap();
        assert_eq!(result.tier_used, "tier1_direct");
        assert_eq!(result.encoding_used, "EUC-JP");
        assert_eq!(result.text, original_text);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_tier1_max_bytes_truncation() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("large_text.txt");

        // Write 1000 bytes
        let sample = "A".repeat(1000);
        fs::write(&file_path, sample).unwrap();

        // Limit to 200 bytes
        let opts = ExtractTextOptions {
            max_bytes: Some(200),
            max_chars: Some(500),
            memory_limit_bytes: None,
            timeout_secs: None,
        };

        let result = extract_file_text(&file_path, Some(opts)).unwrap();
        assert_eq!(result.tier_used, "tier1_direct");
        assert!(result.truncated);
        assert_eq!(result.text.len(), 200);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_binary_file_skipped() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("archive.zip");

        // Write dummy binary data
        let binary_data = vec![0x50, 0x4b, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00];
        fs::write(&file_path, binary_data).unwrap();

        let result = extract_file_text(&file_path, None).unwrap();
        assert_eq!(result.tier_used, "skipped_binary");
        assert_eq!(result.text, "");
        assert_eq!(result.extracted_chars, 0);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_pdf_safety_when_tool_missing_or_isolated() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("test.pdf");

        fs::write(&file_path, b"%PDF-1.4 dummy pdf header").unwrap();

        let result = extract_file_text(&file_path, None).unwrap();
        assert_eq!(result.tier_used, "tier2_isolated_pdf");
        // pdftotext might not be installed or might fail on dummy PDF - in either case must NOT crash!
        if result.warning.is_some() {
            println!("PDF note: {:?}", result.warning);
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_extract_and_index_pipeline_japanese() {
        use crate::models::encode_path_to_base64;
        let temp_dir = std::env::temp_dir().join(format!("cuddle-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("cuddle_manual.md");

        let content = "# Cuddle マニュアル\n原本ファイルをLandlockカーネル隔離で安全に保護します。";
        fs::write(&file_path, content).unwrap();

        let b64 = encode_path_to_base64(&file_path);
        let extracted = index_file_with_text(&b64, None).unwrap();
        assert_eq!(extracted.tier_used, "tier1_direct");
        assert_eq!(extracted.encoding_used, "UTF-8");

        let engine = search_fulltext::get_or_init_engine().unwrap();
        let hits = engine.search("Landlock", &search_fulltext::FulltextSearchOptions::default()).unwrap();
        assert!(!hits.is_empty(), "Fulltext search must find the newly indexed document");
        assert!(hits.iter().any(|h| h.display_name == "cuddle_manual.md"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_export_typescript_bindings_extractor() {
        let _ = fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        ExtractedTextDto::export(&cfg).expect("Failed to export ExtractedTextDto TypeScript binding");
        ExtractTextOptions::export(&cfg).expect("Failed to export ExtractTextOptions TypeScript binding");
    }
}
