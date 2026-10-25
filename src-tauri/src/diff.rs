use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use thiserror::Error;
use ts_rs::TS;

#[derive(Error, Debug)]
pub enum DiffError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("File not found: {0}")]
    NotFound(String),
}

/// The nature of a change in a line of code or text.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub enum DiffChangeType {
    Equal,
    Delete,
    Insert,
}

/// A single line in a unified diff comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct DiffLineDto {
    pub change_type: DiffChangeType,
    pub old_line_number: Option<usize>,
    pub new_line_number: Option<usize>,
    pub content: String,
}

/// Detailed result of comparing the original file and its staged working copy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct FileDiffResultDto {
    pub stage_id: String,
    pub is_binary: bool,
    pub has_changes: bool,
    pub additions: usize,
    pub deletions: usize,
    pub lines: Vec<DiffLineDto>,
}

/// Detects if a file appears to be binary by checking the first 512 bytes for null characters.
pub fn is_binary_content(bytes: &[u8]) -> bool {
    let check_len = bytes.len().min(512);
    bytes[..check_len].contains(&0)
}

/// Reads a file into a byte vector, returning an error if unreadable.
fn read_file_bytes<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, DiffError> {
    let p = path.as_ref();
    if !p.exists() {
        return Err(DiffError::NotFound(p.to_string_lossy().to_string()));
    }
    let mut file = File::open(p)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

/// Computes unified line-by-line diff between the original file and staged file.
pub fn compute_file_diff<P: AsRef<Path>, Q: AsRef<Path>>(
    stage_id: &str,
    original_path: P,
    staged_path: Q,
) -> Result<FileDiffResultDto, DiffError> {
    let orig_bytes = read_file_bytes(original_path)?;
    let staged_bytes = read_file_bytes(staged_path)?;

    // 1. Binary check
    if is_binary_content(&orig_bytes) || is_binary_content(&staged_bytes) {
        let has_changes = orig_bytes != staged_bytes;
        return Ok(FileDiffResultDto {
            stage_id: stage_id.to_string(),
            is_binary: true,
            has_changes,
            additions: 0,
            deletions: 0,
            lines: Vec::new(),
        });
    }

    // 2. Decode text safely (lossy UTF-8)
    let orig_text = String::from_utf8_lossy(&orig_bytes);
    let staged_text = String::from_utf8_lossy(&staged_bytes);

    let text_diff = TextDiff::from_lines(&orig_text, &staged_text);

    let mut lines = Vec::new();
    let mut additions = 0;
    let mut deletions = 0;

    for change in text_diff.iter_all_changes() {
        let change_type = match change.tag() {
            ChangeTag::Equal => DiffChangeType::Equal,
            ChangeTag::Delete => {
                deletions += 1;
                DiffChangeType::Delete
            }
            ChangeTag::Insert => {
                additions += 1;
                DiffChangeType::Insert
            }
        };

        // Trim trailing newline for cleaner UI rendering
        let content = change.value().trim_end_matches(['\r', '\n']).to_string();

        lines.push(DiffLineDto {
            change_type,
            old_line_number: change.old_index().map(|i| i + 1),
            new_line_number: change.new_index().map(|i| i + 1),
            content,
        });
    }

    let has_changes = additions > 0 || deletions > 0;

    Ok(FileDiffResultDto {
        stage_id: stage_id.to_string(),
        is_binary: false,
        has_changes,
        additions,
        deletions,
        lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn test_compute_diff_text_changes() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle_diff_test_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let orig = temp_dir.join("orig.txt");
        let staged = temp_dir.join("staged.txt");

        fs::write(&orig, "line1\nline2\nline3\n").unwrap();
        fs::write(&staged, "line1\nline2 modified\nline3\nline4 added\n").unwrap();

        let diff = compute_file_diff("test-stage", &orig, &staged).unwrap();

        assert!(!diff.is_binary);
        assert!(diff.has_changes);
        assert_eq!(diff.deletions, 1); // line2 deleted
        assert_eq!(diff.additions, 2); // line2 modified + line4 added

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_compute_diff_identical_files() {
        let temp_dir =
            std::env::temp_dir().join(format!("cuddle_diff_identical_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let orig = temp_dir.join("orig.txt");
        let staged = temp_dir.join("staged.txt");

        fs::write(&orig, "identical content\n").unwrap();
        fs::write(&staged, "identical content\n").unwrap();

        let diff = compute_file_diff("test-stage", &orig, &staged).unwrap();

        assert!(!diff.is_binary);
        assert!(!diff.has_changes);
        assert_eq!(diff.additions, 0);
        assert_eq!(diff.deletions, 0);
        assert_eq!(diff.lines.len(), 1);
        assert_eq!(diff.lines[0].change_type, DiffChangeType::Equal);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_compute_diff_binary_detection() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle_diff_bin_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let orig = temp_dir.join("bin1.dat");
        let staged = temp_dir.join("bin2.dat");

        fs::write(&orig, &[0x00, 0x01, 0x02, 0x03]).unwrap();
        fs::write(&staged, &[0x00, 0x01, 0x02, 0x99]).unwrap();

        let diff = compute_file_diff("test-stage", &orig, &staged).unwrap();

        assert!(diff.is_binary);
        assert!(diff.has_changes);
        assert!(diff.lines.is_empty());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_export_typescript_bindings_diff() {
        let _ = fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        DiffChangeType::export(&cfg).expect("Failed to export DiffChangeType");
        DiffLineDto::export(&cfg).expect("Failed to export DiffLineDto");
        FileDiffResultDto::export(&cfg).expect("Failed to export FileDiffResultDto");
    }
}
