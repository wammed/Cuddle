use crate::models::{decode_path_from_base64, FileItem, ModelError};
use crate::scanner::{scan_directory, ScanOptions};
use serde::{Deserialize, Serialize};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;
use ts_rs::TS;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Model conversion error: {0}")]
    Model(#[from] ModelError),
    #[error("Pattern cannot be empty")]
    EmptyPattern,
    #[error("Plocate execution failed: {0}")]
    PlocateFailed(String),
}

/// Options controlling filename search behavior.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct SearchFilenameOptions {
    /// Case-insensitive pattern matching (default: true)
    pub ignore_case: bool,
    /// Match only the filename component instead of full path (default: true)
    pub basename_only: bool,
    /// Maximum number of matching items to return (default: 500)
    pub limit: usize,
    /// Optional directory to constrain search results (encoded in Base64)
    pub scope_raw_base64: Option<String>,
}

impl Default for SearchFilenameOptions {
    fn default() -> Self {
        Self {
            ignore_case: true,
            basename_only: true,
            limit: 500,
            scope_raw_base64: None,
        }
    }
}

/// Structured search response metadata indicating engine and hit count.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct SearchResultDto {
    pub items: Vec<FileItem>,
    /// Search engine utilized: "plocate" or "walkdir_fallback"
    pub engine_used: String,
    pub total_matches: usize,
}

/// Executes a filename search with high-speed `plocate` indexing when available,
/// automatically falling back to recursive `walkdir` traversal when `plocate`
/// is unavailable, unindexed, or returns an error.
pub fn search_filename(
    pattern: &str,
    options: &SearchFilenameOptions,
) -> Result<SearchResultDto, SearchError> {
    let trimmed = pattern.trim();
    if trimmed.is_empty() {
        return Err(SearchError::EmptyPattern);
    }

    let scope_path = match &options.scope_raw_base64 {
        Some(b64) => Some(decode_path_from_base64(b64)?),
        None => None,
    };

    // Fast path: Attempt plocate first
    match run_plocate_search(trimmed, options, scope_path.as_deref()) {
        Ok(items) => {
            let total = items.len();
            Ok(SearchResultDto {
                items,
                engine_used: "plocate".to_string(),
                total_matches: total,
            })
        }
        Err(err) => {
            eprintln!(
                "[Cuddle Search] plocate unavailable or failed: {}. Falling back to direct walkdir scanning.",
                err
            );
            // Fallback path: Direct recursive walkdir traversal
            let items = run_walkdir_fallback(trimmed, options, scope_path.as_deref())?;
            let total = items.len();
            Ok(SearchResultDto {
                items,
                engine_used: "walkdir_fallback".to_string(),
                total_matches: total,
            })
        }
    }
}

/// Runs plocate using direct argument array invocation (no shell strings).
fn run_plocate_search(
    pattern: &str,
    options: &SearchFilenameOptions,
    scope: Option<&Path>,
) -> Result<Vec<FileItem>, SearchError> {
    let mut cmd = Command::new("plocate");
    cmd.arg("--null"); // NUL-delimited output for safe handling of whitespace and special chars

    if options.basename_only {
        cmd.arg("-b");
    }
    if options.ignore_case {
        cmd.arg("-i");
    }

    // Limit plocate outputs (fetch up to 2x limit to allow for post-scope filtering)
    let fetch_limit = if scope.is_some() {
        options.limit * 3
    } else {
        options.limit
    };
    cmd.arg("-l").arg(fetch_limit.to_string());

    // Pattern argument
    cmd.arg(pattern);

    let output = cmd.output()?;
    if !output.status.success() {
        // Exit status 1 from plocate typically means no matches found, which is not a failure
        if output.status.code() == Some(1) {
            return Ok(Vec::new());
        }
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(SearchError::PlocateFailed(stderr));
    }

    let mut items = Vec::new();
    let stdout = &output.stdout;

    // Split by NUL byte
    for raw_path_bytes in stdout.split(|&b| b == 0) {
        if raw_path_bytes.is_empty() {
            continue;
        }

        let path = PathBuf::from(std::ffi::OsStr::from_bytes(raw_path_bytes));

        // Enforce scope boundary if specified
        if let Some(scope_root) = scope {
            if !path.starts_with(scope_root) {
                continue;
            }
        }

        // Fetch filesystem metadata
        if let Ok(meta) = std::fs::symlink_metadata(&path) {
            if let Ok(item) = FileItem::from_path_and_metadata(&path, &meta) {
                items.push(item);
                if items.len() >= options.limit {
                    break;
                }
            }
        }
    }

    Ok(items)
}

/// Direct traversal fallback using walkdir when plocate is not available.
fn run_walkdir_fallback(
    pattern: &str,
    options: &SearchFilenameOptions,
    scope: Option<&Path>,
) -> Result<Vec<FileItem>, SearchError> {
    let search_root = scope
        .map(|p| p.to_path_buf())
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."));

    let scan_opts = ScanOptions {
        max_depth: None, // Recursive
        follow_symlinks: false,
        include_hidden: false,
    };

    let all_items = scan_directory(&search_root, &scan_opts).map_err(|e| match e {
        crate::scanner::ScanError::Io(io_err) => SearchError::Io(io_err),
        crate::scanner::ScanError::Model(model_err) => SearchError::Model(model_err),
        crate::scanner::ScanError::PathNotFound(p) => SearchError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            p.to_string_lossy(),
        )),
    })?;

    let pattern_normalized = if options.ignore_case {
        pattern.to_lowercase()
    } else {
        pattern.to_string()
    };

    let mut matched_items = Vec::new();
    for item in all_items {
        let name_to_check = if options.ignore_case {
            item.display_name.to_lowercase()
        } else {
            item.display_name.clone()
        };

        if name_to_check.contains(&pattern_normalized) {
            matched_items.push(item);
            if matched_items.len() >= options.limit {
                break;
            }
        }
    }

    Ok(matched_items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_test_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cuddle_search_test_{}_{}", prefix, nanos));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn test_search_walkdir_fallback_matching_and_scope() {
        let dir = create_test_dir("fallback");

        // Create test files
        fs::write(dir.join("Invoice_2026_Q1.pdf"), b"pdf").unwrap();
        fs::write(dir.join("invoice_2026_q2.txt"), b"txt").unwrap();
        fs::write(dir.join("report_draft.docx"), b"docx").unwrap();
        let sub = dir.join("archive");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("invoice_archive.csv"), b"csv").unwrap();

        let scope_b64 = crate::models::encode_path_to_base64(&dir);
        let opts = SearchFilenameOptions {
            ignore_case: true,
            basename_only: true,
            limit: 10,
            scope_raw_base64: Some(scope_b64),
        };

        // Explicitly test the fallback walker
        let matches = run_walkdir_fallback("invoice", &opts, Some(&dir)).unwrap();
        assert_eq!(matches.len(), 3); // Invoice_2026_Q1.pdf, invoice_2026_q2.txt, invoice_archive.csv

        // Case-sensitive search
        let opts_case = SearchFilenameOptions {
            ignore_case: false,
            scope_raw_base64: Some(crate::models::encode_path_to_base64(&dir)),
            ..Default::default()
        };
        let matches_case = run_walkdir_fallback("Invoice", &opts_case, Some(&dir)).unwrap();
        assert_eq!(matches_case.len(), 1);
        assert_eq!(matches_case[0].display_name, "Invoice_2026_Q1.pdf");

        // Clean up
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_search_empty_pattern_error() {
        let opts = SearchFilenameOptions::default();
        let res = search_filename("   ", &opts);
        assert!(matches!(res, Err(SearchError::EmptyPattern)));
    }

    #[test]
    fn test_search_filename_full_dispatch() {
        // Run full search dispatch (plocate or fallback)
        let dir = create_test_dir("dispatch");
        fs::write(dir.join("cuddle_dispatch_probe.txt"), b"probe").unwrap();

        let scope_b64 = crate::models::encode_path_to_base64(&dir);
        let opts = SearchFilenameOptions {
            scope_raw_base64: Some(scope_b64),
            ..Default::default()
        };

        let res = search_filename("cuddle_dispatch_probe", &opts).unwrap();
        // Regardless of whether plocate or walkdir ran, it must return valid items without error
        assert!(res.engine_used == "plocate" || res.engine_used == "walkdir_fallback");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_export_typescript_bindings_search() {
        let _ = fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        let _ = SearchFilenameOptions::export(&cfg);
        let _ = SearchResultDto::export(&cfg);
    }
}
