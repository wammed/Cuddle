use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum ModelError {
    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid path representation")]
    InvalidPath,
}

/// Core File Item representation supporting non-UTF-8 paths via dual-key (Base64 + display string).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct FileItem {
    /// Unique immutable item identifier (UUID v4)
    pub item_id: String,
    /// Exact file path raw bytes encoded in standard Base64
    pub raw_base64: String,
    /// Safe UTF-8 display name (invalid UTF-8 bytes replaced with U+FFFD replacement char)
    pub display_name: String,
    /// Parent directory raw bytes encoded in Base64 (optional if root)
    pub parent_raw_base64: Option<String>,
    /// True if entry is a directory
    pub is_dir: bool,
    /// True if entry is a symlink
    pub is_symlink: bool,
    /// File size in bytes
    pub size: u64,
    /// Modification time in epoch milliseconds
    pub mtime_ms: i64,
    /// True if file is read-only (POSIX permissions check)
    pub readonly: bool,
}

impl FileItem {
    /// Creates a FileItem from a given Path and its filesystem metadata.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, ModelError> {
        let p = path.as_ref();
        let metadata = std::fs::symlink_metadata(p)?;
        Self::from_path_and_metadata(p, &metadata)
    }

    /// Creates a FileItem from Path and pre-fetched metadata.
    pub fn from_path_and_metadata(path: &Path, meta: &std::fs::Metadata) -> Result<Self, ModelError> {
        let item_id = Uuid::new_v4().to_string();
        let raw_bytes = path.as_os_str().as_bytes();
        let raw_base64 = BASE64_STANDARD.encode(raw_bytes);

        // Safe display name: takes file_name or the whole path if root
        let file_name_os = path.file_name().unwrap_or(path.as_os_str());
        let display_name = file_name_os.to_string_lossy().to_string();

        let parent_raw_base64 = path.parent().map(|parent| {
            BASE64_STANDARD.encode(parent.as_os_str().as_bytes())
        });

        let is_symlink = meta.file_type().is_symlink();
        let is_dir = meta.is_dir();
        let size = meta.len();

        let mtime_ms = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let readonly = meta.permissions().readonly();

        Ok(Self {
            item_id,
            raw_base64,
            display_name,
            parent_raw_base64,
            is_dir,
            is_symlink,
            size,
            mtime_ms,
            readonly,
        })
    }

    /// Reconstructs the exact raw PathBuf from the Base64 representation.
    /// Preserves exact bytes on Linux without any UTF-8 mangling.
    pub fn to_path(&self) -> Result<PathBuf, ModelError> {
        decode_path_from_base64(&self.raw_base64)
    }
}

/// Decodes a Base64 string into an exact native PathBuf.
pub fn decode_path_from_base64(raw_base64: &str) -> Result<PathBuf, ModelError> {
    let bytes = BASE64_STANDARD.decode(raw_base64)?;
    let os_str = std::ffi::OsStr::from_bytes(&bytes);
    Ok(PathBuf::from(os_str))
}

/// Encodes an exact native Path into a Base64 string.
pub fn encode_path_to_base64<P: AsRef<Path>>(path: P) -> String {
    let bytes = path.as_ref().as_os_str().as_bytes();
    BASE64_STANDARD.encode(bytes)
}

/// Structured error response type shared across Tauri IPC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct AppErrorDto {
    pub code: String,
    pub message: String,
    pub item_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    #[test]
    fn test_valid_utf8_path_roundtrip() {
        let original_path = PathBuf::from("/home/susie/test_document.txt");
        let b64 = encode_path_to_base64(&original_path);
        let decoded = decode_path_from_base64(&b64).unwrap();
        assert_eq!(original_path, decoded);
    }

    #[test]
    fn test_invalid_utf8_path_dual_key_roundtrip() {
        // Construct a path with invalid UTF-8 bytes: [0x66, 0x69, 0x6c, 0x65, 0xff, 0xfe, 0x2e, 0x74, 0x78, 0x74] ("file\xFF\xFE.txt")
        let invalid_bytes = b"/tmp/invalid_\xFF\xFE_name.txt";
        let invalid_os_str = OsString::from_vec(invalid_bytes.to_vec());
        let original_path = PathBuf::from(invalid_os_str);

        // 1. Encode to Base64
        let b64 = encode_path_to_base64(&original_path);

        // 2. Decode back to PathBuf - must preserve 100% exact byte sequence
        let decoded_path = decode_path_from_base64(&b64).unwrap();
        assert_eq!(original_path, decoded_path);
        assert_eq!(decoded_path.as_os_str().as_bytes(), invalid_bytes);

        // 3. Display name should not panic and should contain replacement characters
        let display_lossy = original_path.file_name().unwrap().to_string_lossy().to_string();
        assert!(display_lossy.contains('\u{FFFD}'));
    }

    #[test]
    fn test_file_item_serialization() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("cuddle_test_item.txt");
        std::fs::write(&test_file, b"sample content").unwrap();

        let item = FileItem::from_path(&test_file).unwrap();
        assert_eq!(item.display_name, "cuddle_test_item.txt");
        assert_eq!(item.size, 14);
        assert!(!item.is_dir);

        // Roundtrip path
        let recovered_path = item.to_path().unwrap();
        assert_eq!(recovered_path, test_file);

        // JSON serialize and deserialize check
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: FileItem = serde_json::from_str(&json).unwrap();
        assert_eq!(item, deserialized);

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_export_typescript_bindings() {
        // Generates TypeScript definitions in `src/types/`
        let _ = std::fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        FileItem::export(&cfg).expect("Failed to export FileItem TypeScript binding");
        AppErrorDto::export(&cfg).expect("Failed to export AppErrorDto TypeScript binding");
    }
}
