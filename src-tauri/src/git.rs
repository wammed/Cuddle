use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use ts_rs::TS;

use crate::models::AppErrorDto;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct GitFileItemDto {
    pub path: String,
    pub status_code: String,
    pub status_label: String,
    pub original_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct GitStatusDto {
    pub is_repo: bool,
    pub repo_root: Option<String>,
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub latest_commit_hash: Option<String>,
    pub latest_commit_message: Option<String>,
    pub latest_commit_author: Option<String>,
    pub latest_commit_date: Option<String>,
    pub staged_files: Vec<GitFileItemDto>,
    pub unstaged_files: Vec<GitFileItemDto>,
    pub untracked_files: Vec<GitFileItemDto>,
    pub conflict_files: Vec<GitFileItemDto>,
    pub is_clean: bool,
    pub error_message: Option<String>,
}

impl Default for GitStatusDto {
    fn default() -> Self {
        Self {
            is_repo: false,
            repo_root: None,
            branch: None,
            upstream: None,
            ahead: 0,
            behind: 0,
            latest_commit_hash: None,
            latest_commit_message: None,
            latest_commit_author: None,
            latest_commit_date: None,
            staged_files: Vec::new(),
            unstaged_files: Vec::new(),
            untracked_files: Vec::new(),
            conflict_files: Vec::new(),
            is_clean: true,
            error_message: None,
        }
    }
}

/// Decode Base64 or treat as raw UTF-8 path string
fn resolve_target_dir(path_str: &str) -> PathBuf {
    use base64::Engine;
    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(path_str) {
        #[cfg(unix)]
        {
            use std::ffi::OsStr;
            use std::os::unix::ffi::OsStrExt;
            let os_str = OsStr::from_bytes(&bytes);
            return PathBuf::from(os_str);
        }
        #[cfg(not(unix))]
        {
            if let Ok(utf8) = String::from_utf8(bytes) {
                return PathBuf::from(utf8);
            }
        }
    }
    PathBuf::from(path_str)
}

/// Retrieve structured git status for the given path
#[tauri::command]
pub fn get_git_status(path_base64: String) -> Result<GitStatusDto, AppErrorDto> {
    let target_dir = resolve_target_dir(&path_base64);

    // If path is a file, use its parent directory
    let dir = if target_dir.is_file() {
        target_dir.parent().unwrap_or(&target_dir).to_path_buf()
    } else {
        target_dir
    };

    if !dir.exists() {
        return Ok(GitStatusDto {
            error_message: Some(format!("Directory does not exist: {}", dir.display())),
            is_clean: false,
            ..Default::default()
        });
    }

    // 1. Check if git repo exists (git rev-parse --show-toplevel)
    let rev_parse_output = Command::new("git")
        .arg("-C")
        .arg(&dir)
        .args(["rev-parse", "--show-toplevel"])
        .output();

    let repo_root = match rev_parse_output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Ok(_) => {
            // Not a git repository
            return Ok(GitStatusDto {
                is_repo: false,
                is_clean: false,
                ..Default::default()
            });
        }
        Err(err) => {
            return Ok(GitStatusDto {
                error_message: Some(format!("Failed to invoke git: {err}")),
                is_clean: false,
                ..Default::default()
            });
        }
    };

    let mut status_dto = GitStatusDto {
        is_repo: true,
        repo_root: Some(repo_root.clone()),
        ..GitStatusDto::default()
    };

    // 2. Query git status using porcelain v2 with branch information and NUL termination (-z)
    let status_output = Command::new("git")
        .arg("-C")
        .arg(&repo_root)
        .args(["status", "--porcelain=v2", "-z", "--branch"])
        .output();

    match status_output {
        Ok(out) => {
            if out.status.success() {
                let output_str = String::from_utf8_lossy(&out.stdout);
                parse_porcelain_v2(&output_str, &mut status_dto);
            } else {
                let stderr_msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
                status_dto.error_message = Some(if stderr_msg.is_empty() {
                    format!("git status failed with exit code {:?}", out.status.code())
                } else {
                    stderr_msg
                });
            }
        }
        Err(err) => {
            status_dto.error_message = Some(format!("Failed to invoke git status: {err}"));
        }
    }

    // 3. Query latest commit info (git log -1)
    let log_output = Command::new("git")
        .arg("-C")
        .arg(&repo_root)
        .args(["log", "-1", "--pretty=format:%h%x09%s%x09%an%x09%ar"])
        .output();

    if let Ok(out) = log_output {
        if out.status.success() {
            let log_str = String::from_utf8_lossy(&out.stdout);
            let parts: Vec<&str> = log_str.split('\t').collect();
            if parts.len() >= 4 {
                status_dto.latest_commit_hash = Some(parts[0].to_string());
                status_dto.latest_commit_message = Some(parts[1].to_string());
                status_dto.latest_commit_author = Some(parts[2].to_string());
                status_dto.latest_commit_date = Some(parts[3].to_string());
            }
        }
    }

    // is_clean is ONLY true if there were no errors AND all change lists are empty
    status_dto.is_clean = status_dto.error_message.is_none()
        && status_dto.staged_files.is_empty()
        && status_dto.unstaged_files.is_empty()
        && status_dto.untracked_files.is_empty()
        && status_dto.conflict_files.is_empty();

    Ok(status_dto)
}

fn map_status_label(code: char) -> &'static str {
    match code {
        'M' => "Modified",
        'A' => "Added",
        'D' => "Deleted",
        'R' => "Renamed",
        'C' => "Copied",
        'U' => "Updated Unmerged",
        '?' => "Untracked",
        '!' => "Ignored",
        _ => "Changed",
    }
}

/// Parse `git status --porcelain=v2 -z --branch` output.
/// Supports both NUL-separated (-z) streams and newline-separated fallback.
/// Note: Preserves leading and trailing whitespace in file paths.
pub fn parse_porcelain_v2(output: &str, status: &mut GitStatusDto) {
    let is_nul_separated = output.contains('\0');
    let records: Vec<&str> = if is_nul_separated {
        output.split('\0').collect()
    } else {
        output.lines().collect()
    };

    let mut it = records.into_iter().peekable();
    while let Some(raw_record) = it.next() {
        // Strip trailing CR (\r) ONLY for newline-separated fallback to handle Windows line endings.
        // For NUL-separated (-z) streams, trailing \r in filenames is legitimate and must NOT be stripped!
        let line = if is_nul_separated {
            raw_record
        } else {
            raw_record.trim_end_matches('\r')
        };
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("# branch.head ") {
            status.branch = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("# branch.upstream ") {
            status.upstream = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            for part in rest.split_whitespace() {
                if let Some(ahead_str) = part.strip_prefix('+') {
                    status.ahead = ahead_str.parse().unwrap_or(0);
                } else if let Some(behind_str) = part.strip_prefix('-') {
                    status.behind = behind_str.parse().unwrap_or(0);
                }
            }
        } else if let Some(rest) = line.strip_prefix("1 ") {
            // Ordinary change entry: 1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>
            // 8 fields before path: splitn(8, ' ') leaves exactly the raw <path> in index 7
            let parts: Vec<&str> = rest.splitn(8, ' ').collect();
            if parts.len() == 8 {
                let xy = parts[0];
                let path = parts[7].to_string();
                let x_staged = xy.chars().next().unwrap_or('.');
                let y_unstaged = xy.chars().nth(1).unwrap_or('.');

                if x_staged != '.' {
                    status.staged_files.push(GitFileItemDto {
                        path: path.clone(),
                        status_code: x_staged.to_string(),
                        status_label: map_status_label(x_staged).to_string(),
                        original_path: None,
                    });
                }
                if y_unstaged != '.' {
                    status.unstaged_files.push(GitFileItemDto {
                        path,
                        status_code: y_unstaged.to_string(),
                        status_label: map_status_label(y_unstaged).to_string(),
                        original_path: None,
                    });
                }
            }
        } else if let Some(rest) = line.strip_prefix("2 ") {
            // Renamed/copied entry:
            // 2 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <Xscore> <path>
            // 9 fields before path: splitn(9, ' ') leaves <path> in index 8
            // With -z, next entry is <origPath>. With tab, path\torigPath
            let parts: Vec<&str> = rest.splitn(9, ' ').collect();
            if parts.len() == 9 {
                let xy = parts[0];
                let path_field = parts[8];
                let (path, orig_path) = if let Some((p, o)) = path_field.split_once('\t') {
                    (p.to_string(), Some(o.to_string()))
                } else if let Some(orig) = it.next() {
                    let clean_orig = if is_nul_separated {
                        orig.to_string()
                    } else {
                        orig.trim_end_matches('\r').to_string()
                    };
                    (path_field.to_string(), Some(clean_orig))
                } else {
                    (path_field.to_string(), None)
                };

                let x_staged = xy.chars().next().unwrap_or('.');
                let y_unstaged = xy.chars().nth(1).unwrap_or('.');

                if x_staged != '.' {
                    status.staged_files.push(GitFileItemDto {
                        path: path.clone(),
                        status_code: x_staged.to_string(),
                        status_label: map_status_label(x_staged).to_string(),
                        original_path: orig_path.clone(),
                    });
                }
                if y_unstaged != '.' {
                    status.unstaged_files.push(GitFileItemDto {
                        path,
                        status_code: y_unstaged.to_string(),
                        status_label: map_status_label(y_unstaged).to_string(),
                        original_path: orig_path,
                    });
                }
            }
        } else if let Some(rest) = line.strip_prefix("u ") {
            // Unmerged / conflict entry: u <XY> ... <path>
            // 10 fields before path: splitn(10, ' ') leaves <path> in index 9
            let parts: Vec<&str> = rest.splitn(10, ' ').collect();
            if parts.len() == 10 {
                let path = parts[9].to_string();
                status.conflict_files.push(GitFileItemDto {
                    path,
                    status_code: "U".to_string(),
                    status_label: "Conflict / Unmerged".to_string(),
                    original_path: None,
                });
            }
        } else if let Some(path) = line.strip_prefix("? ") {
            // Untracked entry: ? <path>
            status.untracked_files.push(GitFileItemDto {
                path: path.to_string(),
                status_code: "?".to_string(),
                status_label: "Untracked".to_string(),
                original_path: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_porcelain_v2_clean_repo() {
        let output = r#"# branch.oid 1234567890abcdef
# branch.head main
# branch.upstream origin/main
# branch.ab +0 -0
"#;
        let mut status = GitStatusDto::default();
        parse_porcelain_v2(output, &mut status);

        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(status.upstream.as_deref(), Some("origin/main"));
        assert_eq!(status.ahead, 0);
        assert_eq!(status.behind, 0);
        assert!(status.staged_files.is_empty());
        assert!(status.unstaged_files.is_empty());
        assert!(status.untracked_files.is_empty());
    }

    #[test]
    fn test_parse_porcelain_v2_modified_and_untracked() {
        let output = r#"# branch.oid a1b2c3d4
# branch.head feature/ui-tabs
# branch.upstream origin/feature/ui-tabs
# branch.ab +2 -1
1 M. N... 100644 100644 100644 1111 2222 src/App.tsx
1 .M N... 100644 100644 100644 3333 4444 src/components/Header.tsx
? new_file.txt
u UU N... 100644 100644 100644 100644 5555 6666 7777 conflict.txt
"#;
        let mut status = GitStatusDto::default();
        parse_porcelain_v2(output, &mut status);

        assert_eq!(status.branch.as_deref(), Some("feature/ui-tabs"));
        assert_eq!(status.ahead, 2);
        assert_eq!(status.behind, 1);

        assert_eq!(status.staged_files.len(), 1);
        assert_eq!(status.staged_files[0].path, "src/App.tsx");
        assert_eq!(status.staged_files[0].status_label, "Modified");

        assert_eq!(status.unstaged_files.len(), 1);
        assert_eq!(status.unstaged_files[0].path, "src/components/Header.tsx");

        assert_eq!(status.untracked_files.len(), 1);
        assert_eq!(status.untracked_files[0].path, "new_file.txt");

        assert_eq!(status.conflict_files.len(), 1);
        assert_eq!(status.conflict_files[0].path, "conflict.txt");
    }

    #[test]
    fn test_parse_porcelain_v2_with_nul_separation_and_rename_with_spaces_and_japanese() {
        // Construct simulated `git status --porcelain=v2 -z --branch` output
        let mut output = String::new();
        output.push_str("# branch.head main\0");
        output.push_str("# branch.upstream origin/main\0");
        output.push_str("# branch.ab +1 -0\0");
        // Ordinary change with spaces and Japanese filename
        output.push_str("1 M. N... 100644 100644 100644 1111 2222 資料/重要な 仕様書 2026.docx\0");
        // Renamed file with spaces: 2 <XY> ... <new_path>\0<old_path>\0
        output.push_str("2 R. N... 100644 100644 100644 3333 4444 R100 新しい ドキュメント.pdf\0古い ドキュメント.pdf\0");
        // Untracked with tab/space
        output.push_str("? memo draft.txt\0");
        // Test paths with leading and trailing spaces
        output.push_str("1 .M N... 100644 100644 100644 5555 6666   spaced filename.txt  \0");
        output.push_str("2 R. N... 100644 100644 100644 7777 8888 R100  new leading space.txt \0 old trailing space.txt \0");
        // Test paths with trailing carriage return (\r) preserved in -z mode
        output.push_str("1 M. N... 100644 100644 100644 9999 0000 file_with_trailing_cr.txt\r\0");
        output
            .push_str("2 R. N... 100644 100644 100644 aaaa bbbb R100 new_cr.txt\r\0old_cr.txt\r\0");

        let mut status = GitStatusDto::default();
        parse_porcelain_v2(&output, &mut status);

        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(status.ahead, 1);
        assert_eq!(status.behind, 0);

        assert_eq!(status.staged_files.len(), 5);
        assert_eq!(status.staged_files[0].path, "資料/重要な 仕様書 2026.docx");
        assert_eq!(status.staged_files[0].status_label, "Modified");

        // Verify rename target and original path
        assert_eq!(status.staged_files[1].path, "新しい ドキュメント.pdf");
        assert_eq!(
            status.staged_files[1].original_path.as_deref(),
            Some("古い ドキュメント.pdf")
        );
        assert_eq!(status.staged_files[1].status_label, "Renamed");

        // Verify whitespace preservation in rename
        assert_eq!(status.staged_files[2].path, " new leading space.txt ");
        assert_eq!(
            status.staged_files[2].original_path.as_deref(),
            Some(" old trailing space.txt ")
        );

        // Verify trailing CR preservation in -z mode
        assert_eq!(status.staged_files[3].path, "file_with_trailing_cr.txt\r");
        assert_eq!(status.staged_files[4].path, "new_cr.txt\r");
        assert_eq!(
            status.staged_files[4].original_path.as_deref(),
            Some("old_cr.txt\r")
        );

        assert_eq!(status.unstaged_files.len(), 1);
        assert_eq!(status.unstaged_files[0].path, "  spaced filename.txt  ");

        assert_eq!(status.untracked_files.len(), 1);
        assert_eq!(status.untracked_files[0].path, "memo draft.txt");
    }

    #[test]
    fn test_git_status_invalid_path_not_reported_as_clean() {
        let res = get_git_status("/path/that/does/not/exist/ever_12345".to_string()).unwrap();
        assert!(!res.is_clean, "Invalid directory must NOT be clean");
        assert!(res.error_message.is_some());
    }

    #[test]
    fn test_get_git_status_on_current_repo() {
        // Run get_git_status on current crate directory
        let res =
            get_git_status(".".to_string()).expect("Should succeed querying current git repo");
        assert!(
            res.is_repo,
            "Current workspace should be recognized as git repo"
        );
        assert!(res.branch.is_some(), "Current branch should be detected");
    }

    #[test]
    fn test_export_typescript_bindings_git() {
        use ts_rs::TS;
        let cfg = ts_rs::Config::default();
        let _ = std::fs::create_dir_all("../src/types");
        GitStatusDto::export(&cfg).expect("Failed to export GitStatusDto");
        GitFileItemDto::export(&cfg).expect("Failed to export GitFileItemDto");
    }
}
