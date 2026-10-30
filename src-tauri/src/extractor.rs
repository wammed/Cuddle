use crate::models::{decode_path_from_base64, FileItem};
use crate::sandbox::LandlockSandbox;
use crate::scanner::{self, ScanOptions};
use crate::search_fulltext;
use flate2::read::DeflateDecoder;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
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

/// Operation mode for on-demand explicit index creation/maintenance.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub enum IndexOperationMode {
    /// 差分更新 (Incremental Update): 変更・追加されたファイルのみ検出してインデックスを高速更新
    IncrementalUpdate,
    /// フォルダ追加 (Index Folder): 指定フォルダ内の対応ファイルを走査してインデックスに追加
    IndexDirectory,
    /// 全再構築 (Full Rebuild): 既存インデックスを全消去し、フォルダ全体をゼロから再構築
    FullRebuild,
    /// リセット・消去 (Reset & Clear): インデックスを全消去して空にする
    ResetClear,
}

/// Options controlling explicit index batch execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct RunIndexOptions {
    /// Base64-encoded target directory path. If None, falls back to current dir or user home.
    pub target_raw_base64: Option<String>,
    /// Operation mode: IncrementalUpdate | IndexDirectory | FullRebuild | ResetClear
    pub mode: IndexOperationMode,
    /// Maximum recursive traversal depth (None for unlimited)
    pub max_depth: Option<usize>,
    /// Whether to include hidden files (default: false)
    pub include_hidden: bool,
    /// Whether excluded directory contents (like .git, node_modules) should be indexed as metadata-only (empty body) (Task 1-5)
    #[serde(default)]
    pub include_metadata_only_excluded: Option<bool>,
}

/// Structured summary returned after an index operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct IndexOperationResultDto {
    pub mode: IndexOperationMode,
    pub target_path: String,
    pub indexed_count: usize,
    pub updated_count: usize,
    pub unchanged_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub total_documents: usize,
    pub elapsed_ms: u64,
    pub message: String,
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
            | "cfg"
            | "service"
            | "lua"
            | "vim"
            | "fish"
            | "rc"
            | "properties"
            | "theme"
            | "rules"
            | "ninja"
            | "vue"
            | "svelte"
    )
}

/// Determines if a path component belongs to heavy cache/build/VCS directories that should be skipped during full-text index crawling.
pub fn is_index_excluded_path(path: &Path) -> bool {
    for comp in path.components() {
        if let std::path::Component::Normal(c) = comp {
            if let Some(s) = c.to_str() {
                if matches!(
                    s,
                    ".git"
                        | ".cache"
                        | "node_modules"
                        | ".cargo"
                        | ".rustup"
                        | ".npm"
                        | ".yarn"
                        | ".pnpm-store"
                        | ".Trash"
                ) {
                    return true;
                }
            }
        }
    }
    false
}

/// Checks whether a given path is a PDF document requiring Tier 2 isolation.
pub fn is_tier2_pdf_extension(ext: &str) -> bool {
    ext.eq_ignore_ascii_case("pdf")
}

/// Checks whether a given path is an Office OpenXML document (.docx, .xlsx) for Tier 3 extraction.
pub fn is_tier3_office_extension(ext: &str) -> bool {
    ext.eq_ignore_ascii_case("docx") || ext.eq_ignore_ascii_case("xlsx")
}

/// Checks whether an extension is Markdown.
pub fn is_tier1_markdown_extension(ext: &str) -> bool {
    ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown")
}

/// Checks whether an extension is HTML.
pub fn is_tier1_html_extension(ext: &str) -> bool {
    ext.eq_ignore_ascii_case("html")
        || ext.eq_ignore_ascii_case("htm")
        || ext.eq_ignore_ascii_case("xhtml")
}

/// Policy governing how a path should be indexed into the search engine (Task 1-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub enum IndexingPolicy {
    /// Full extraction and indexing of both metadata and body text
    Full,
    /// Store filename, path, mtime and size in index, but keep body empty (for heavy directories like node_modules, .git)
    MetadataOnly,
    /// Skip indexing entirely
    Exclude,
}

/// Resolves the indexing policy for a given path based on exclusion lists and crawl options.
pub fn get_path_indexing_policy(path: &Path, include_metadata_only: bool) -> IndexingPolicy {
    if is_index_excluded_path(path) {
        if include_metadata_only {
            IndexingPolicy::MetadataOnly
        } else {
            IndexingPolicy::Exclude
        }
    } else {
        IndexingPolicy::Full
    }
}

/// Extracts clean human-readable prose text from Markdown, stripping decorative syntax (Task 1-4).
pub fn extract_markdown_text(markdown_input: &str) -> String {
    use pulldown_cmark::{Event, Parser, TagEnd};

    let parser = Parser::new(markdown_input);
    let mut out = String::with_capacity(markdown_input.len());
    let mut needs_space = false;

    for event in parser {
        match event {
            Event::Text(text) => {
                if needs_space && !out.ends_with(' ') && !out.ends_with('\n') {
                    out.push(' ');
                }
                out.push_str(&text);
                needs_space = false;
            }
            Event::Code(code) => {
                if needs_space && !out.ends_with(' ') && !out.ends_with('\n') {
                    out.push(' ');
                }
                out.push_str(&code);
                needs_space = true;
            }
            Event::End(TagEnd::Paragraph | TagEnd::Heading(_)) => {
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                needs_space = false;
            }
            Event::End(TagEnd::Item) => {
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                needs_space = false;
            }
            Event::SoftBreak | Event::HardBreak => {
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                needs_space = false;
            }
            _ => {}
        }
    }
    out.trim().to_string()
}

/// Extracts prose text from HTML for search indexing purposes (Task 1-4).
///
/// NOTE: This function uses a fast heuristic regex pipeline designed specifically for
/// indexing and preview extraction. It safely removes `<script>`, `<style>`, `<noscript>`,
/// `<svg>`, comments (`<!-- ... -->`), and generic tags while decoding common HTML entities.
/// It is NOT a full DOM parser and does not guarantee strict HTML syntax validation. If deep
/// DOM structural manipulation is required in the future, integrate a dedicated HTML parser
/// such as `scraper` or `html5ever`.
pub fn extract_html_text(html_input: &str) -> String {
    // 1. Strip comments, scripts, styles, noscript, and SVG blocks
    let re_blocks = regex::Regex::new(r"(?is)<!--.*?-->|<script[^>]*>.*?</script>|<style[^>]*>.*?</style>|<noscript[^>]*>.*?</noscript>|<svg[^>]*>.*?</svg>").unwrap();
    let stripped_blocks = re_blocks.replace_all(html_input, " ");

    // 2. Strip standard HTML tags
    let re_tags = regex::Regex::new(r"<[^>]+>").unwrap();
    let text_with_entities = re_tags.replace_all(&stripped_blocks, " ");

    // 3. Decode common HTML entities
    let decoded = text_with_entities
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'");

    let re_whitespace = regex::Regex::new(r"[ \t]+").unwrap();
    let single_spaced = re_whitespace.replace_all(&decoded, " ");
    let re_lines = regex::Regex::new(r"\n\s*\n+").unwrap();
    let normalized = re_lines.replace_all(&single_spaced, "\n");

    normalized.trim().to_string()
}

/// Reads a specific uncompressed or deflated entry from a standard ZIP container (like DOCX / XLSX).
/// Bounded by max_uncompressed_bytes to strictly prevent ZipBomb decompression attacks (Task 1-3).
pub fn read_zip_file_entry(
    path: &Path,
    target_filename: &str,
    max_uncompressed_bytes: usize,
) -> Result<Option<Vec<u8>>, ExtractorError> {
    let mut file = File::open(path)?;
    let file_len = file.metadata()?.len();
    if file_len < 22 {
        return Ok(None);
    }

    let search_len = (file_len as usize).min(65535 + 22);
    file.seek(SeekFrom::End(-(search_len as i64)))?;
    let mut tail_buf = vec![0u8; search_len];
    file.read_exact(&mut tail_buf)?;

    let mut eocd_pos = None;
    for i in (0..=search_len.saturating_sub(22)).rev() {
        if tail_buf[i..].starts_with(b"PK") {
            eocd_pos = Some(file_len - (search_len - i) as u64);
            break;
        }
    }

    if let Some(eocd_offset) = eocd_pos {
        file.seek(SeekFrom::Start(eocd_offset))?;
        let mut eocd_hdr = [0u8; 22];
        file.read_exact(&mut eocd_hdr)?;

        let total_entries = u16::from_le_bytes([eocd_hdr[10], eocd_hdr[11]]) as usize;
        let cd_offset =
            u32::from_le_bytes([eocd_hdr[16], eocd_hdr[17], eocd_hdr[18], eocd_hdr[19]]) as u64;

        file.seek(SeekFrom::Start(cd_offset))?;
        for _ in 0..total_entries {
            let mut cd_hdr = [0u8; 46];
            if file.read_exact(&mut cd_hdr).is_err() {
                break;
            }
            if !cd_hdr.starts_with(b"PK") {
                break;
            }

            let method = u16::from_le_bytes([cd_hdr[10], cd_hdr[11]]);
            let comp_size =
                u32::from_le_bytes([cd_hdr[20], cd_hdr[21], cd_hdr[22], cd_hdr[23]]) as usize;
            let uncomp_size =
                u32::from_le_bytes([cd_hdr[24], cd_hdr[25], cd_hdr[26], cd_hdr[27]]) as usize;
            let fn_len = u16::from_le_bytes([cd_hdr[28], cd_hdr[29]]) as usize;
            let extra_len = u16::from_le_bytes([cd_hdr[30], cd_hdr[31]]) as usize;
            let comment_len = u16::from_le_bytes([cd_hdr[32], cd_hdr[33]]) as usize;
            let local_offset =
                u32::from_le_bytes([cd_hdr[42], cd_hdr[43], cd_hdr[44], cd_hdr[45]]) as u64;

            let mut fn_buf = vec![0u8; fn_len];
            file.read_exact(&mut fn_buf)?;
            let current_name = String::from_utf8_lossy(&fn_buf);

            file.seek(SeekFrom::Current((extra_len + comment_len) as i64))?;

            if current_name == target_filename {
                if uncomp_size > max_uncompressed_bytes {
                    return Err(ExtractorError::ExecutionFailed(format!(
                        "ZipBomb guard: file {} uncompressed size ({}) exceeds limit ({})",
                        target_filename, uncomp_size, max_uncompressed_bytes
                    )));
                }

                file.seek(SeekFrom::Start(local_offset))?;

                let mut local_hdr = [0u8; 30];
                file.read_exact(&mut local_hdr)?;
                if !local_hdr.starts_with(b"PK") {
                    return Err(ExtractorError::ExecutionFailed(
                        "Invalid Local Header in ZIP".into(),
                    ));
                }
                let loc_fn_len = u16::from_le_bytes([local_hdr[26], local_hdr[27]]) as i64;
                let loc_extra_len = u16::from_le_bytes([local_hdr[28], local_hdr[29]]) as i64;
                file.seek(SeekFrom::Current(loc_fn_len + loc_extra_len))?;

                let mut comp_buf = vec![0u8; comp_size];
                file.read_exact(&mut comp_buf)?;

                let decompressed = match method {
                    0 => comp_buf,
                    8 => {
                        let decoder = DeflateDecoder::new(&comp_buf[..]);
                        let mut out = Vec::with_capacity(uncomp_size.min(max_uncompressed_bytes));
                        decoder
                            .take(max_uncompressed_bytes as u64)
                            .read_to_end(&mut out)?;
                        out
                    }
                    m => {
                        return Err(ExtractorError::ExecutionFailed(format!(
                            "Unsupported ZIP compression method: {}",
                            m
                        )))
                    }
                };

                return Ok(Some(decompressed));
            }
        }
    }

    Ok(None)
}

fn parse_docx_xml(xml_bytes: &[u8]) -> Result<String, quick_xml::Error> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_reader(xml_bytes);
    reader.config_mut().trim_text(true);

    let mut out = String::new();
    let mut in_text = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.name();
                if name.as_ref() == b"w:t" {
                    in_text = true;
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                if name.as_ref() == b"w:t" {
                    in_text = false;
                } else if name.as_ref() == b"w:p" && !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            Ok(Event::Empty(e)) => {
                let name = e.name();
                if name.as_ref() == b"w:tab" {
                    out.push('\t');
                } else if name.as_ref() == b"w:br" {
                    out.push('\n');
                }
            }
            Ok(Event::Text(e)) => {
                if in_text {
                    let raw_str = std::str::from_utf8(e.as_ref()).unwrap_or_default();
                    if let Ok(text) = quick_xml::escape::unescape(raw_str) {
                        out.push_str(&text);
                    } else {
                        out.push_str(raw_str);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e),
            _ => {}
        }
    }

    Ok(out.trim().to_string())
}

fn parse_xlsx_shared_strings(xml_bytes: &[u8]) -> Result<String, quick_xml::Error> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_reader(xml_bytes);
    reader.config_mut().trim_text(true);

    let mut out = String::new();
    let mut in_text = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.name();
                if name.as_ref() == b"t" {
                    in_text = true;
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                if name.as_ref() == b"t" {
                    in_text = false;
                    out.push(' ');
                } else if name.as_ref() == b"si" && !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            Ok(Event::Text(e)) => {
                if in_text {
                    let raw_str = std::str::from_utf8(e.as_ref()).unwrap_or_default();
                    if let Ok(text) = quick_xml::escape::unescape(raw_str) {
                        out.push_str(&text);
                    } else {
                        out.push_str(raw_str);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e),
            _ => {}
        }
    }

    Ok(out.trim().to_string())
}

fn extract_tier3_office(
    path: &Path,
    item: &FileItem,
    options: &ExtractTextOptions,
) -> Result<ExtractedTextDto, ExtractorError> {
    let max_chars = options.max_chars.unwrap_or(DEFAULT_MAX_CHARS);
    let max_bytes = options.max_bytes.unwrap_or(DEFAULT_MAX_BYTES);

    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let is_docx = ext.eq_ignore_ascii_case("docx");
    let is_xlsx = ext.eq_ignore_ascii_case("xlsx");

    let target_entry = if is_docx {
        "word/document.xml"
    } else if is_xlsx {
        "xl/sharedStrings.xml"
    } else {
        return Ok(ExtractedTextDto {
            item_id: item.item_id.clone(),
            raw_base64: item.raw_base64.clone(),
            display_name: item.display_name.clone(),
            text: String::new(),
            extracted_chars: 0,
            tier_used: "skipped_binary".to_string(),
            encoding_used: "N/A".to_string(),
            truncated: false,
            warning: Some(format!("Unsupported office format .{}", ext)),
        });
    };

    match read_zip_file_entry(path, target_entry, max_bytes) {
        Ok(Some(xml_bytes)) => {
            let parsed_text_res = if is_docx {
                parse_docx_xml(&xml_bytes)
            } else {
                parse_xlsx_shared_strings(&xml_bytes)
            };

            match parsed_text_res {
                Ok(mut text) => {
                    let mut truncated = false;
                    if text.chars().count() > max_chars {
                        text = text.chars().take(max_chars).collect();
                        truncated = true;
                    }
                    let extracted_chars = text.chars().count();
                    Ok(ExtractedTextDto {
                        item_id: item.item_id.clone(),
                        raw_base64: item.raw_base64.clone(),
                        display_name: item.display_name.clone(),
                        text,
                        extracted_chars,
                        tier_used: "tier3_office".to_string(),
                        encoding_used: "UTF-8".to_string(),
                        truncated,
                        warning: None,
                    })
                }
                Err(err) => Ok(ExtractedTextDto {
                    item_id: item.item_id.clone(),
                    raw_base64: item.raw_base64.clone(),
                    display_name: item.display_name.clone(),
                    text: String::new(),
                    extracted_chars: 0,
                    tier_used: "tier3_office".to_string(),
                    encoding_used: "N/A".to_string(),
                    truncated: false,
                    warning: Some(format!("XML parsing failed for {}: {}", target_entry, err)),
                }),
            }
        }
        Ok(None) => Ok(ExtractedTextDto {
            item_id: item.item_id.clone(),
            raw_base64: item.raw_base64.clone(),
            display_name: item.display_name.clone(),
            text: String::new(),
            extracted_chars: 0,
            tier_used: "tier3_office".to_string(),
            encoding_used: "N/A".to_string(),
            truncated: false,
            warning: Some(format!(
                "Target entry {} not found in archive",
                target_entry
            )),
        }),
        Err(err) => Ok(ExtractedTextDto {
            item_id: item.item_id.clone(),
            raw_base64: item.raw_base64.clone(),
            display_name: item.display_name.clone(),
            text: String::new(),
            extracted_chars: 0,
            tier_used: "tier3_office".to_string(),
            encoding_used: "N/A".to_string(),
            truncated: false,
            warning: Some(format!("Failed to read Office archive: {}", err)),
        }),
    }
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

    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    if is_tier1_markdown_extension(ext) {
        decoded_text = extract_markdown_text(&decoded_text);
    } else if is_tier1_html_extension(ext) {
        decoded_text = extract_html_text(&decoded_text);
    }

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
            warning: Some(
                "pdftotext (poppler-utils) is not installed. PDF extraction skipped safely."
                    .to_string(),
            ),
        });
    }

    let max_chars = options.max_chars.unwrap_or(DEFAULT_MAX_CHARS);
    let mem_limit = options
        .memory_limit_bytes
        .unwrap_or(DEFAULT_MEMORY_LIMIT_BYTES);
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
            warning: Some(format!(
                "pdftotext exited with non-zero status: {:?}",
                output.status.code()
            )),
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

    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    if is_tier2_pdf_extension(ext) {
        extract_tier2_pdf(path, &item, &opts)
    } else if is_tier3_office_extension(ext) {
        extract_tier3_office(path, &item, &opts)
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

/// Executes an explicit index creation or maintenance operation (Incremental, Add, Full Rebuild, or Reset)
/// across the specified target directory.
pub fn run_index_operation(
    options: RunIndexOptions,
) -> Result<IndexOperationResultDto, ExtractorError> {
    let start_time = std::time::Instant::now();
    let engine = search_fulltext::get_or_init_engine()?;

    let target_path = match &options.target_raw_base64 {
        Some(b64) => decode_path_from_base64(b64)?,
        None => dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")),
    };

    if options.mode == IndexOperationMode::ResetClear {
        engine.clear_all_documents()?;
        let elapsed_ms = start_time.elapsed().as_millis() as u64;
        return Ok(IndexOperationResultDto {
            mode: options.mode,
            target_path: target_path.to_string_lossy().to_string(),
            indexed_count: 0,
            updated_count: 0,
            unchanged_count: 0,
            skipped_count: 0,
            failed_count: 0,
            total_documents: 0,
            elapsed_ms,
            message: "Fulltext search index cleared successfully.".to_string(),
        });
    }

    if options.mode == IndexOperationMode::FullRebuild {
        engine.clear_all_documents()?;
    }

    let scan_opts = ScanOptions {
        max_depth: options.max_depth,
        follow_symlinks: false,
        include_hidden: options.include_hidden,
    };

    let items = scanner::scan_directory(&target_path, &scan_opts).map_err(|e| match e {
        scanner::ScanError::Io(io_err) => ExtractorError::Io(io_err),
        scanner::ScanError::Model(model_err) => ExtractorError::Model(model_err),
        scanner::ScanError::PathNotFound(p) => ExtractorError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            p.to_string_lossy(),
        )),
    })?;

    let mut indexed_count = 0;
    let mut updated_count = 0;
    let mut unchanged_count = 0;
    let mut skipped_count = 0;
    let mut failed_count = 0;

    for item in &items {
        if item.is_dir || item.is_symlink {
            continue;
        }

        let item_path = match decode_path_from_base64(&item.raw_base64) {
            Ok(p) => p,
            Err(_) => {
                failed_count += 1;
                continue;
            }
        };

        let policy = get_path_indexing_policy(
            &item_path,
            options.include_metadata_only_excluded.unwrap_or(false),
        );

        if policy == IndexingPolicy::Exclude {
            skipped_count += 1;
            continue;
        }

        let ext = item_path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if !is_tier1_text_extension(ext)
            && !is_tier2_pdf_extension(ext)
            && !is_tier3_office_extension(ext)
            && !ext.is_empty()
        {
            skipped_count += 1;
            continue;
        }

        let existing_mtime = engine.get_indexed_mtime(&item.raw_base64);

        if options.mode == IndexOperationMode::IncrementalUpdate {
            if let Some(mtime) = existing_mtime {
                if mtime == item.mtime_ms {
                    unchanged_count += 1;
                    continue;
                }
            }
        }

        if policy == IndexingPolicy::MetadataOnly {
            if let Err(e) = engine.index_document(item, "") {
                eprintln!(
                    "[Cuddle Indexer] Failed to index metadata-only document {:?}: {}",
                    item_path, e
                );
                search_fulltext::enqueue_repair_on_failure(
                    &item.raw_base64,
                    "index",
                    &e.to_string(),
                );
                failed_count += 1;
            } else if existing_mtime.is_some() {
                updated_count += 1;
            } else {
                indexed_count += 1;
            }
            continue;
        }

        let existing_mtime = engine.get_indexed_mtime(&item.raw_base64);

        if options.mode == IndexOperationMode::IncrementalUpdate {
            if let Some(mtime) = existing_mtime {
                if mtime == item.mtime_ms {
                    unchanged_count += 1;
                    continue;
                }
            }
        }

        match extract_file_text(&item_path, None) {
            Ok(extracted) => {
                if extracted.text.trim().is_empty() {
                    skipped_count += 1;
                } else {
                    if let Err(e) = engine.index_document(item, &extracted.text) {
                        eprintln!(
                            "[Cuddle Indexer] Failed to index document {:?}: {}",
                            item_path, e
                        );
                        failed_count += 1;
                    } else if existing_mtime.is_some() {
                        updated_count += 1;
                    } else {
                        indexed_count += 1;
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "[Cuddle Indexer] Extraction error on {:?}: {}",
                    item_path, e
                );
                failed_count += 1;
            }
        }
    }

    engine.commit()?;

    let _ = search_fulltext::process_index_repair_queue();

    let elapsed_ms = start_time.elapsed().as_millis() as u64;
    let total_documents = engine.get_total_documents();
    let message = format!(
        "Index operation completed: {} indexed, {} updated, {} unchanged, {} skipped, {} failed (total: {} docs)",
        indexed_count, updated_count, unchanged_count, skipped_count, failed_count, total_documents
    );

    Ok(IndexOperationResultDto {
        mode: options.mode,
        target_path: target_path.to_string_lossy().to_string(),
        indexed_count,
        updated_count,
        unchanged_count,
        skipped_count,
        failed_count,
        total_documents,
        elapsed_ms,
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_tier1_utf8_text_extraction() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("sample.txt");

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
        let hits = engine
            .search(
                "Landlock",
                &search_fulltext::FulltextSearchOptions::default(),
            )
            .unwrap();
        assert!(
            !hits.is_empty(),
            "Fulltext search must find the newly indexed document"
        );
        assert!(hits.iter().any(|h| h.display_name == "cuddle_manual.md"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_run_index_operations_workflow() {
        use crate::models::encode_path_to_base64;
        let temp_dir = std::env::temp_dir().join(format!("cuddle-index-ops-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let file1 = temp_dir.join("note1.txt");
        fs::write(&file1, "北海道の札幌で美味しい味噌ラーメンを食べました。").unwrap();

        let file2 = temp_dir.join("note2.md");
        fs::write(&file2, "沖縄の那覇で青い海と美ら海水族館を観光しました。").unwrap();

        let b64 = encode_path_to_base64(&temp_dir);

        // 1. Initial Index Directory
        let res_initial = run_index_operation(RunIndexOptions {
            target_raw_base64: Some(b64.clone()),
            mode: IndexOperationMode::IndexDirectory,
            max_depth: None,
            include_hidden: false,
            include_metadata_only_excluded: None,
        })
        .unwrap();
        assert_eq!(res_initial.indexed_count, 2);
        assert_eq!(res_initial.unchanged_count, 0);

        // 2. Incremental Update without modification -> both unchanged
        let res_inc1 = run_index_operation(RunIndexOptions {
            target_raw_base64: Some(b64.clone()),
            mode: IndexOperationMode::IncrementalUpdate,
            max_depth: None,
            include_hidden: false,
            include_metadata_only_excluded: None,
        })
        .unwrap();
        assert_eq!(res_inc1.indexed_count, 0);
        assert_eq!(res_inc1.unchanged_count, 2);

        // 3. Modify note1 and re-run incremental -> 1 updated, 1 unchanged
        std::thread::sleep(std::time::Duration::from_millis(50));
        fs::write(&file1, "北海道の旭川で美味しい醤油ラーメンを食べました。").unwrap();
        let res_inc2 = run_index_operation(RunIndexOptions {
            target_raw_base64: Some(b64.clone()),
            mode: IndexOperationMode::IncrementalUpdate,
            max_depth: None,
            include_hidden: false,
            include_metadata_only_excluded: None,
        })
        .unwrap();
        assert_eq!(res_inc2.updated_count, 1);
        assert_eq!(res_inc2.unchanged_count, 1);

        // 4. Verify search matches updated content
        let engine = search_fulltext::get_or_init_engine().unwrap();
        let hits = engine
            .search("旭川", &search_fulltext::FulltextSearchOptions::default())
            .unwrap();
        assert!(!hits.is_empty());
        assert!(hits.iter().any(|h| h.display_name == "note1.txt"));

        // 5. Reset and Clear Index
        let res_clear = run_index_operation(RunIndexOptions {
            target_raw_base64: Some(b64.clone()),
            mode: IndexOperationMode::ResetClear,
            max_depth: None,
            include_hidden: false,
            include_metadata_only_excluded: None,
        })
        .unwrap();
        assert_eq!(res_clear.total_documents, 0);

        let hits_after = engine
            .search("旭川", &search_fulltext::FulltextSearchOptions::default())
            .unwrap();
        assert!(hits_after.is_empty());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_index_dot_directory_and_dot_files() {
        use crate::models::encode_path_to_base64;
        let temp_dir = std::env::temp_dir().join(format!("cuddle-dotfiles-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        // 1. Create a dotfile (.bashrc)
        let bashrc_path = temp_dir.join(".bashrc");
        fs::write(&bashrc_path, "export EDITOR=nvim\nalias ll='ls -la'").unwrap();

        // 2. Create a dot directory (.config/app/settings.toml)
        let config_dir = temp_dir.join(".config").join("app");
        fs::create_dir_all(&config_dir).unwrap();
        let settings_path = config_dir.join("settings.toml");
        fs::write(
            &settings_path,
            "[server]\nport = 8080\nhost = \"127.0.0.1\"",
        )
        .unwrap();

        // 3. Create an excluded .git directory (.git/HEAD)
        let git_dir = temp_dir.join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        let git_head = git_dir.join("HEAD");
        fs::write(&git_head, "ref: refs/heads/main").unwrap();

        // 4. Run index operation with include_hidden: true
        let b64 = encode_path_to_base64(&temp_dir);
        let res = run_index_operation(RunIndexOptions {
            target_raw_base64: Some(b64.clone()),
            mode: IndexOperationMode::IndexDirectory,
            max_depth: None,
            include_hidden: true,
            include_metadata_only_excluded: None,
        })
        .unwrap();

        // .bashrc and settings.toml must be indexed (2 files), .git/HEAD must be skipped
        assert_eq!(res.indexed_count, 2);
        assert!(res.skipped_count >= 1);

        let engine = search_fulltext::get_or_init_engine().unwrap();
        let opts = search_fulltext::FulltextSearchOptions::default();

        // Search for .bashrc content
        let hits_bash = engine.search("EDITOR", &opts).unwrap();
        assert!(hits_bash.iter().any(|h| h.display_name == ".bashrc"));

        // Search for .config/app/settings.toml content
        let hits_toml = engine.search("settings", &opts).unwrap();
        assert!(hits_toml.iter().any(|h| h.display_name == "settings.toml"));

        // Verify .git was excluded
        let hits_git = engine.search("refs/heads/main", &opts).unwrap();
        assert!(!hits_git.iter().any(|h| h.display_name == "HEAD"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_markdown_and_html_prose_extraction() {
        let md = "# Cuddle タイトル\n\n本日は**快晴**です。\n\n```rust\nfn main() {}\n```\n\n* アイテム1\n* アイテム2";
        let md_clean = extract_markdown_text(md);
        assert!(md_clean.contains("Cuddle タイトル"));
        assert!(md_clean.contains("本日は快晴です"));
        assert!(md_clean.contains("fn main()"));
        assert!(!md_clean.contains("**快晴**"));
        assert!(!md_clean.starts_with('#'));

        let html = r#"
            <!DOCTYPE html>
            <html>
                <head>
                    <title>Test Page</title>
                    <style>body { color: red; }</style>
                    <script>alert("ignore me");</script>
                </head>
                <body>
                    <h1>見出し &amp; タイトル</h1>
                    <p>これは&nbsp;HTML&lt;本文&gt;のテストです。</p>
                </body>
            </html>
        "#;
        let html_clean = extract_html_text(html);
        assert!(html_clean.contains("見出し & タイトル"));
        assert!(html_clean.contains("HTML<本文>のテストです"));
        assert!(!html_clean.contains("alert"));
        assert!(!html_clean.contains("color: red"));
        assert!(!html_clean.contains("<p>"));
    }

    #[test]
    fn test_tier3_office_docx_extraction() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle-office-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let docx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
                <w:body>
                    <w:p><w:r><w:t>Cuddle Office Document Test</w:t></w:r></w:p>
                    <w:p><w:r><w:t>原本保護型ファイルワークベンチのWord抽出です。</w:t></w:r></w:p>
                </w:body>
            </w:document>"#
            .as_bytes();

        let docx_bytes = create_test_zip_stored("word/document.xml", docx_xml);
        let docx_path = temp_dir.join("sample.docx");
        fs::write(&docx_path, docx_bytes).unwrap();

        let extracted = extract_file_text(&docx_path, None).unwrap();
        assert_eq!(extracted.tier_used, "tier3_office");
        assert!(extracted.text.contains("Cuddle Office Document Test"));
        assert!(extracted.text.contains("Word抽出です"));
        assert!(extracted.warning.is_none());

        let corrupted_path = temp_dir.join("corrupted.docx");
        fs::write(&corrupted_path, b"not a zip file content").unwrap();
        let extracted_corrupt = extract_file_text(&corrupted_path, None).unwrap();
        assert_eq!(extracted_corrupt.tier_used, "tier3_office");
        assert!(extracted_corrupt.text.is_empty());
        assert!(extracted_corrupt.warning.is_some());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_metadata_only_indexing_for_excluded_directories() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle-meta-only-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let node_modules_dir = temp_dir.join("node_modules").join("heavy-lib");
        fs::create_dir_all(&node_modules_dir).unwrap();
        let package_json = node_modules_dir.join("package.json");
        fs::write(
            &package_json,
            r#"{"name": "heavy-lib", "secret": "shh-heavy-body-token"}"#,
        )
        .unwrap();

        assert_eq!(
            get_path_indexing_policy(&package_json, true),
            IndexingPolicy::MetadataOnly
        );
        assert_eq!(
            get_path_indexing_policy(&package_json, false),
            IndexingPolicy::Exclude
        );

        let engine = search_fulltext::CuddleFulltextEngine::new_in_ram().unwrap();
        let item = FileItem::from_path(&package_json).unwrap();
        engine.index_document(&item, "").unwrap();
        engine.commit().unwrap();

        let search_opts = search_fulltext::FulltextSearchOptions::default();
        let hits_name = engine.search("package.json", &search_opts).unwrap();
        assert_eq!(hits_name.len(), 1);
        assert_eq!(hits_name[0].display_name, "package.json");

        let hits_secret = engine.search("shh-heavy-body-token", &search_opts).unwrap();
        assert!(hits_secret.is_empty());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    fn create_test_zip_stored(filename: &str, content: &[u8]) -> Vec<u8> {
        let mut zip = Vec::new();
        let fn_bytes = filename.as_bytes();
        let fn_len = fn_bytes.len() as u16;
        let content_len = content.len() as u32;
        let local_offset = 0u32;

        zip.extend_from_slice(b"PK\x03\x04");
        zip.extend_from_slice(&[20, 0]);
        zip.extend_from_slice(&[0, 0]);
        zip.extend_from_slice(&[0, 0]);
        zip.extend_from_slice(&[0, 0, 0, 0]);
        zip.extend_from_slice(&[0, 0, 0, 0]);
        zip.extend_from_slice(&content_len.to_le_bytes());
        zip.extend_from_slice(&content_len.to_le_bytes());
        zip.extend_from_slice(&fn_len.to_le_bytes());
        zip.extend_from_slice(&[0, 0]);
        zip.extend_from_slice(fn_bytes);
        zip.extend_from_slice(content);

        let cd_offset = zip.len() as u32;

        zip.extend_from_slice(b"PK\x01\x02");
        zip.extend_from_slice(&[20, 0]);
        zip.extend_from_slice(&[20, 0]);
        zip.extend_from_slice(&[0, 0]);
        zip.extend_from_slice(&[0, 0]);
        zip.extend_from_slice(&[0, 0, 0, 0]);
        zip.extend_from_slice(&[0, 0, 0, 0]);
        zip.extend_from_slice(&content_len.to_le_bytes());
        zip.extend_from_slice(&content_len.to_le_bytes());
        zip.extend_from_slice(&fn_len.to_le_bytes());
        zip.extend_from_slice(&[0, 0]);
        zip.extend_from_slice(&[0, 0]);
        zip.extend_from_slice(&[0, 0]);
        zip.extend_from_slice(&[0, 0]);
        zip.extend_from_slice(&[0, 0, 0, 0]);
        zip.extend_from_slice(&local_offset.to_le_bytes());
        zip.extend_from_slice(fn_bytes);

        let cd_size = (zip.len() as u32) - cd_offset;

        zip.extend_from_slice(b"PK\x05\x06");
        zip.extend_from_slice(&[0, 0]);
        zip.extend_from_slice(&[0, 0]);
        zip.extend_from_slice(&[1, 0]);
        zip.extend_from_slice(&[1, 0]);
        zip.extend_from_slice(&cd_size.to_le_bytes());
        zip.extend_from_slice(&cd_offset.to_le_bytes());
        zip.extend_from_slice(&[0, 0]);

        zip
    }

    #[test]
    fn test_export_typescript_bindings_extractor() {
        let _ = fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        ExtractedTextDto::export(&cfg)
            .expect("Failed to export ExtractedTextDto TypeScript binding");
        ExtractTextOptions::export(&cfg)
            .expect("Failed to export ExtractTextOptions TypeScript binding");
        IndexOperationMode::export(&cfg)
            .expect("Failed to export IndexOperationMode TypeScript binding");
        RunIndexOptions::export(&cfg).expect("Failed to export RunIndexOptions TypeScript binding");
        IndexOperationResultDto::export(&cfg)
            .expect("Failed to export IndexOperationResultDto TypeScript binding");
        IndexingPolicy::export(&cfg).expect("Failed to export IndexingPolicy TypeScript binding");
    }
}
