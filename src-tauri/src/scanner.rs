use crate::models::{FileItem, ModelError};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use thiserror::Error;
use ts_rs::TS;
use walkdir::WalkDir;

#[derive(Error, Debug)]
pub enum ScanError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path not found: {0:?}")]
    PathNotFound(PathBuf),
    #[error("Model conversion error: {0}")]
    Model(#[from] ModelError),
}

/// Options to control directory scanning behavior.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct ScanOptions {
    /// Maximum recursive traversal depth.
    /// `Some(1)` lists only direct children (non-recursive). `None` traverses fully.
    pub max_depth: Option<usize>,
    /// Whether to follow directory symlinks. Default: false.
    pub follow_symlinks: bool,
    /// Whether to include hidden files/directories (starting with '.'). Default: false.
    pub include_hidden: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_depth: Some(1),
            follow_symlinks: false,
            include_hidden: false,
        }
    }
}

/// Scans a directory and returns a list of `FileItem` representations.
/// Employs physical inode and device tracking `(dev, ino)` to prevent infinite recursion
/// on cyclic symlinks and safely handle non-UTF-8 paths.
pub fn scan_directory<P: AsRef<Path>>(
    root: P,
    options: &ScanOptions,
) -> Result<Vec<FileItem>, ScanError> {
    let root_path = root.as_ref();
    if !root_path.exists() {
        return Err(ScanError::PathNotFound(root_path.to_path_buf()));
    }

    let mut builder = WalkDir::new(root_path).min_depth(1);
    if let Some(depth) = options.max_depth {
        builder = builder.max_depth(depth);
    }
    builder = builder.follow_links(options.follow_symlinks);

    let mut items = Vec::new();
    let mut visited_inodes: HashSet<(u64, u64)> = HashSet::new();

    // Record root directory inode
    if let Ok(root_meta) = std::fs::symlink_metadata(root_path) {
        visited_inodes.insert((root_meta.dev(), root_meta.ino()));
    }

    for entry_res in builder {
        let entry = match entry_res {
            Ok(e) => e,
            Err(walk_err) => {
                // If a symlink loop is detected by walkdir or an unreadable path, skip safely
                eprintln!("[Cuddle Scanner] Warning during traversal: {}", walk_err);
                continue;
            }
        };

        let path = entry.path();
        let file_name_bytes = path.file_name().map(|n| n.as_bytes()).unwrap_or_default();

        // Filter hidden files/directories if not requested
        if !options.include_hidden && file_name_bytes.starts_with(b".") {
            continue;
        }

        let meta = match std::fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!(
                    "[Cuddle Scanner] Failed to get metadata for {:?}: {}",
                    path, e
                );
                continue;
            }
        };

        let dev = meta.dev();
        let ino = meta.ino();

        // Cyclic symlink prevention: if this is a directory/symlink already visited, skip
        if (meta.is_dir() || meta.file_type().is_symlink()) && !visited_inodes.insert((dev, ino)) {
            eprintln!("[Cuddle Scanner] Cyclic link or already visited inode detected for {:?} (dev={}, ino={}). Skipping.", path, dev, ino);
            continue;
        }

        match FileItem::from_path_and_metadata(path, &meta) {
            Ok(item) => items.push(item),
            Err(e) => {
                eprintln!(
                    "[Cuddle Scanner] Failed to construct FileItem for {:?}: {}",
                    path, e
                );
            }
        }
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::ffi::OsStringExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_test_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cuddle_scan_test_{}_{}", prefix, nanos));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn test_scan_directory_basic_and_depth() {
        let dir = create_test_dir("basic");

        // Create files and nested folder
        fs::write(dir.join("file1.txt"), b"hello").unwrap();
        fs::write(dir.join("file2.md"), b"world").unwrap();
        let sub = dir.join("subfolder");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("nested.txt"), b"deep").unwrap();

        // Non-recursive scan (max_depth: 1)
        let opts_shallow = ScanOptions {
            max_depth: Some(1),
            follow_symlinks: false,
            include_hidden: false,
        };
        let items_shallow = scan_directory(&dir, &opts_shallow).unwrap();
        assert_eq!(items_shallow.len(), 3); // file1.txt, file2.md, subfolder

        // Check that all items have valid inode and dev
        for item in &items_shallow {
            assert!(item.inode > 0);
            assert!(item.dev > 0);
        }

        // Recursive scan (max_depth: None)
        let opts_deep = ScanOptions {
            max_depth: None,
            follow_symlinks: false,
            include_hidden: false,
        };
        let items_deep = scan_directory(&dir, &opts_deep).unwrap();
        assert_eq!(items_deep.len(), 4); // file1.txt, file2.md, subfolder, nested.txt

        // Clean up
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_directory_hidden_files_filter() {
        let dir = create_test_dir("hidden");

        fs::write(dir.join("visible.txt"), b"vis").unwrap();
        fs::write(dir.join(".hidden.txt"), b"hid").unwrap();

        // Default: exclude hidden
        let opts_no_hidden = ScanOptions::default();
        let items = scan_directory(&dir, &opts_no_hidden).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].display_name, "visible.txt");

        // Include hidden
        let opts_with_hidden = ScanOptions {
            include_hidden: true,
            ..Default::default()
        };
        let items_all = scan_directory(&dir, &opts_with_hidden).unwrap();
        assert_eq!(items_all.len(), 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_directory_non_utf8_filename() {
        let dir = create_test_dir("non_utf8");

        // Create a file with invalid UTF-8 bytes 0xFF, 0xFE
        let non_utf8_name = b"archive_\xff\xfe_data.bin".to_vec();
        let non_utf8_path = dir.join(std::ffi::OsString::from_vec(non_utf8_name));
        fs::write(&non_utf8_path, b"binary content").unwrap();

        let items = scan_directory(&dir, &ScanOptions::default()).unwrap();
        assert_eq!(items.len(), 1);

        let item = &items[0];
        assert!(item.display_name.contains('\u{FFFD}'));
        let recovered = item.to_path().unwrap();
        assert_eq!(recovered, non_utf8_path);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_cyclic_symlink_does_not_infinite_loop() {
        let dir = create_test_dir("cyclic");

        let sub_a = dir.join("folder_a");
        fs::create_dir(&sub_a).unwrap();
        let sub_b = sub_a.join("folder_b");
        fs::create_dir(&sub_b).unwrap();

        // Create cyclic symlink: folder_b/loop_to_a -> folder_a
        let loop_link = sub_b.join("loop_to_a");
        let _ = std::os::unix::fs::symlink(&sub_a, &loop_link);

        let opts = ScanOptions {
            max_depth: None,
            follow_symlinks: true,
            include_hidden: false,
        };

        // Must complete safely and terminate without hanging
        let items = scan_directory(&dir, &opts).unwrap();
        assert!(items.len() >= 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_export_typescript_bindings_scanner() {
        let _ = fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        let _ = ScanOptions::export(&cfg);
    }
}
