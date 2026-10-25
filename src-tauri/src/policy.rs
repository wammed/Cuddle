use crate::models::{decode_path_from_base64, FileItem, ModelError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use thiserror::Error;
use ts_rs::TS;

#[derive(Error, Debug)]
pub enum PolicyError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Model error: {0}")]
    Model(#[from] ModelError),
    #[error("Operation forbidden in this zone: target is outside home directory, on a mount boundary, or on external storage")]
    ForbiddenZone,
    #[error("Conflict detected: original file has been modified externally since staging")]
    ConflictDetected(String),
    #[error("Insufficient disk space: required {required} bytes (1.5x original size), but only {available} bytes available")]
    InsufficientSpace { required: u64, available: u64 },
    #[error("Symlink escape detected: resolves outside safe boundary")]
    SymlinkEscape,
    #[error("File not found or inaccessible: {0}")]
    NotFound(String),
}

impl PolicyError {
    pub fn error_code(&self) -> &'static str {
        match self {
            PolicyError::ForbiddenZone => "ERR_FORBIDDEN_ZONE",
            PolicyError::ConflictDetected(_) => "ERR_CONFLICT_DETECTED",
            PolicyError::InsufficientSpace { .. } => "ERR_INSUFFICIENT_SPACE",
            PolicyError::SymlinkEscape => "ERR_SYMLINK_ESCAPE",
            PolicyError::NotFound(_) => "ERR_NOT_FOUND",
            PolicyError::Io(_) => "ERR_IO",
            PolicyError::Model(_) => "ERR_INVALID_PATH",
        }
    }
}

/// Snapshot of an original file used for detecting external modifications and TOCTOU races.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct OriginalSnapshotDto {
    pub item_id: String,
    pub raw_base64: String,
    pub display_name: String,
    pub canonical_path: String,
    pub inode: u64,
    pub dev: u64,
    pub size: u64,
    pub mtime_ms: i64,
    pub sha256: Option<String>,
}

/// Policy evaluation report indicating whether staging or original reflection is allowed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct PolicyCheckResultDto {
    pub allowed: bool,
    pub is_in_home: bool,
    pub is_external_or_mount: bool,
    pub can_reflect_original: bool,
    pub conflict_detected: bool,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

/// Parsed entry from `/proc/mounts`.
#[derive(Debug, Clone)]
pub struct MountEntry {
    pub device: String,
    pub mount_point: PathBuf,
    pub fs_type: String,
}

/// Reads and parses all current filesystem mounts from `/proc/mounts`.
pub fn get_system_mounts() -> Vec<MountEntry> {
    let mut mounts = Vec::new();
    let file = match File::open("/proc/mounts") {
        Ok(f) => f,
        Err(_) => return mounts,
    };

    let reader = BufReader::new(file);
    for line in reader.lines().map_while(Result::ok) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            mounts.push(MountEntry {
                device: parts[0].to_string(),
                mount_point: PathBuf::from(parts[1]),
                fs_type: parts[2].to_string(),
            });
        }
    }

    mounts
}

/// Resolves a path to its canonical representation, following symlinks safely.
pub fn canonicalize_path<P: AsRef<Path>>(path: P) -> Result<PathBuf, PolicyError> {
    std::fs::canonicalize(path.as_ref()).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            PolicyError::NotFound(path.as_ref().to_string_lossy().to_string())
        } else {
            PolicyError::Io(e)
        }
    })
}

/// Checks whether a path resides strictly within the user's home directory `$HOME`.
pub fn is_in_home_directory<P: AsRef<Path>>(path: P) -> bool {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return false,
    };

    let canonical_home = match canonicalize_path(&home) {
        Ok(h) => h,
        Err(_) => home,
    };

    let canonical_target = match canonicalize_path(&path) {
        Ok(t) => t,
        Err(_) => return false,
    };

    canonical_target.starts_with(canonical_home)
}

/// Checks whether a path is on an external storage, removable media, or mount boundary.
pub fn is_external_or_mount_boundary<P: AsRef<Path>>(path: P) -> bool {
    let canonical = match canonicalize_path(path.as_ref()) {
        Ok(p) => p,
        Err(_) => return true,
    };

    let home = match dirs::home_dir().and_then(|h| canonicalize_path(&h).ok()) {
        Some(h) => h,
        None => return true,
    };

    // If outside home, automatically considered external/forbidden
    if !canonical.starts_with(&home) {
        return true;
    }

    // Inspect /proc/mounts to see if a mount point is nested between HOME and the target path
    let mounts = get_system_mounts();
    for mount in mounts {
        if mount.mount_point != home && mount.mount_point != Path::new("/") {
            // If mount is inside home and target is inside that mount, it is a nested mount boundary
            if mount.mount_point.starts_with(&home) && canonical.starts_with(&mount.mount_point) {
                return true;
            }
        }
    }

    false
}

/// Computes SHA-256 hash of a file's content.
pub fn compute_file_sha256<P: AsRef<Path>>(path: P) -> Result<String, PolicyError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Checks available disk space using statvfs and verifies against 1.5x multiplier.
pub fn check_disk_space<P: AsRef<Path>>(
    target_dir: P,
    required_bytes: u64,
    multiplier: f64,
) -> Result<(), PolicyError> {
    let canonical_dir = match canonicalize_path(target_dir.as_ref()) {
        Ok(d) => d,
        Err(_) => target_dir.as_ref().to_path_buf(),
    };

    let path_c = match std::ffi::CString::new(canonical_dir.as_os_str().as_encoded_bytes()) {
        Ok(c) => c,
        Err(_) => return Ok(()), // Non-fatal fallback
    };

    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(path_c.as_ptr(), &mut stat) == 0 {
            let available = stat.f_bavail as u64 * stat.f_frsize as u64;
            let needed = (required_bytes as f64 * multiplier.max(1.5)) as u64;

            if available < needed {
                return Err(PolicyError::InsufficientSpace {
                    required: needed,
                    available,
                });
            }
        }
    }

    Ok(())
}

/// Core Safety Operation Policy engine verifying all guardrails.
pub struct OperationPolicy;

impl OperationPolicy {
    /// Validates whether a file is permitted to be copied to workspace-temp (Staging).
    /// Staging is allowed even for external files (read-only copy), provided source exists.
    pub fn validate_staging<P: AsRef<Path>>(path: P) -> Result<(), PolicyError> {
        let p = path.as_ref();
        if !p.exists() {
            return Err(PolicyError::NotFound(p.to_string_lossy().to_string()));
        }
        Ok(())
    }

    /// Validates whether an operation is allowed to reflect changes back into the original file.
    /// STRICT RULE: Modifying originals outside `$HOME`, on mount boundaries, or external storage
    /// is categorically FORBIDDEN.
    pub fn validate_reflection<P: AsRef<Path>>(path: P) -> Result<(), PolicyError> {
        let p = path.as_ref();
        if !p.exists() {
            return Err(PolicyError::NotFound(p.to_string_lossy().to_string()));
        }

        if !is_in_home_directory(p) || is_external_or_mount_boundary(p) {
            return Err(PolicyError::ForbiddenZone);
        }

        Ok(())
    }

    /// Evaluates policy and returns a structured PolicyCheckResultDto.
    pub fn check_policy<P: AsRef<Path>>(path: P, for_reflection: bool) -> PolicyCheckResultDto {
        let p = path.as_ref();
        if !p.exists() {
            return PolicyCheckResultDto {
                allowed: false,
                is_in_home: false,
                is_external_or_mount: false,
                can_reflect_original: false,
                conflict_detected: false,
                error_code: Some("ERR_NOT_FOUND".to_string()),
                error_message: Some(format!("Path does not exist: {}", p.display())),
            };
        }

        let in_home = is_in_home_directory(p);
        let external = is_external_or_mount_boundary(p);
        let can_reflect = in_home && !external;

        if for_reflection && !can_reflect {
            return PolicyCheckResultDto {
                allowed: false,
                is_in_home: in_home,
                is_external_or_mount: external,
                can_reflect_original: false,
                conflict_detected: false,
                error_code: Some(PolicyError::ForbiddenZone.error_code().to_string()),
                error_message: Some(PolicyError::ForbiddenZone.to_string()),
            };
        }

        PolicyCheckResultDto {
            allowed: true,
            is_in_home: in_home,
            is_external_or_mount: external,
            can_reflect_original: can_reflect,
            conflict_detected: false,
            error_code: None,
            error_message: None,
        }
    }

    /// Creates an original snapshot for tracking mutations and TOCTOU races.
    pub fn create_snapshot(
        item: &FileItem,
        compute_sha: bool,
    ) -> Result<OriginalSnapshotDto, PolicyError> {
        let path = item.to_path()?;
        let canonical = canonicalize_path(&path)?;

        let meta = std::fs::symlink_metadata(&canonical)?;
        let sha256 = if compute_sha {
            Some(compute_file_sha256(&canonical)?)
        } else {
            None
        };

        let mtime_ms = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        Ok(OriginalSnapshotDto {
            item_id: item.item_id.clone(),
            raw_base64: item.raw_base64.clone(),
            display_name: item.display_name.clone(),
            canonical_path: canonical.to_string_lossy().to_string(),
            inode: meta.ino(),
            dev: meta.dev(),
            size: meta.len(),
            mtime_ms,
            sha256,
        })
    }

    /// Verifies whether the original file has been mutated externally since snapshot.
    /// Fast check: inode, dev, size, mtime.
    /// Strict check: SHA-256 (when verify_sha256 is true).
    pub fn verify_conflict(
        snapshot: &OriginalSnapshotDto,
        verify_sha256: bool,
    ) -> Result<(), PolicyError> {
        let path = decode_path_from_base64(&snapshot.raw_base64)?;
        if !path.exists() {
            return Err(PolicyError::ConflictDetected(
                "Original file was deleted externally".to_string(),
            ));
        }

        let meta = std::fs::symlink_metadata(&path)?;

        // 1. Inode / Device match (prevent file replacement)
        if meta.ino() != snapshot.inode || meta.dev() != snapshot.dev {
            return Err(PolicyError::ConflictDetected(format!(
                "Original file inode changed from #{} to #{}",
                snapshot.inode,
                meta.ino()
            )));
        }

        // 2. Size match
        if meta.len() != snapshot.size {
            return Err(PolicyError::ConflictDetected(format!(
                "Original file size changed from {} B to {} B",
                snapshot.size,
                meta.len()
            )));
        }

        // 3. mtime match
        let current_mtime_ms = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        if current_mtime_ms != snapshot.mtime_ms {
            return Err(PolicyError::ConflictDetected(format!(
                "Original file modified time changed from {} to {}",
                snapshot.mtime_ms, current_mtime_ms
            )));
        }

        // 4. Cryptographic SHA-256 match (when requested or snapshot has hash)
        if verify_sha256 {
            if let Some(expected_hash) = &snapshot.sha256 {
                let current_hash = compute_file_sha256(&path)?;
                if &current_hash != expected_hash {
                    return Err(PolicyError::ConflictDetected(format!(
                        "Original content hash changed: expected {}, found {}",
                        expected_hash, current_hash
                    )));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn test_home_directory_and_outside_detection() {
        let home = dirs::home_dir().expect("Home directory must exist");
        assert!(is_in_home_directory(&home));

        let home_file = home.join(format!("cuddle_test_{}.tmp", Uuid::new_v4()));
        fs::write(&home_file, b"test").unwrap();
        assert!(is_in_home_directory(&home_file));
        let _ = fs::remove_file(&home_file);

        // Outside home
        assert!(!is_in_home_directory(Path::new("/etc")));
        assert!(!is_in_home_directory(Path::new("/tmp")));
        assert!(!is_in_home_directory(Path::new("/usr/bin")));
    }

    #[test]
    fn test_forbidden_zone_for_reflection_outside_home() {
        let etc_file = Path::new("/etc/hosts");
        if etc_file.exists() {
            // Staging (read-only copy) is permitted
            assert!(OperationPolicy::validate_staging(etc_file).is_ok());

            // Reflection back to original is strictly forbidden
            let reflection_err = OperationPolicy::validate_reflection(etc_file);
            assert!(reflection_err.is_err());
            assert_eq!(
                reflection_err.unwrap_err().error_code(),
                "ERR_FORBIDDEN_ZONE"
            );

            let check = OperationPolicy::check_policy(etc_file, true);
            assert!(!check.allowed);
            assert!(!check.can_reflect_original);
            assert_eq!(check.error_code, Some("ERR_FORBIDDEN_ZONE".to_string()));
        }
    }

    #[test]
    fn test_symlink_escape_detection() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        // Symlink pointing to /etc/passwd
        let escape_symlink = temp_dir.join("escape_link");
        let target = Path::new("/etc/hosts");
        if target.exists() {
            #[cfg(unix)]
            std::os::unix::fs::symlink(target, &escape_symlink).unwrap();

            // Reflection must detect escape and reject
            let res = OperationPolicy::validate_reflection(&escape_symlink);
            assert!(res.is_err());
            assert_eq!(res.unwrap_err().error_code(), "ERR_FORBIDDEN_ZONE");
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_conflict_detection_mtime_and_size() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("original.txt");

        fs::write(&file_path, b"initial content 123").unwrap();
        let item = FileItem::from_path(&file_path).unwrap();

        // Take snapshot with SHA-256
        let snapshot = OperationPolicy::create_snapshot(&item, true).unwrap();
        assert!(snapshot.sha256.is_some());

        // 1. Initial verification must pass without conflict
        assert!(OperationPolicy::verify_conflict(&snapshot, true).is_ok());

        // 2. Modify file externally (append data)
        std::thread::sleep(std::time::Duration::from_millis(15));
        fs::write(&file_path, b"initial content 123 mutated!").unwrap();

        // 3. Verification must fail with conflict detected
        let conflict_res = OperationPolicy::verify_conflict(&snapshot, true);
        assert!(conflict_res.is_err());
        assert_eq!(
            conflict_res.unwrap_err().error_code(),
            "ERR_CONFLICT_DETECTED"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_conflict_detection_sha256_strict() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("same_size.txt");

        // Same length mutation: "AAAA" -> "BBBB"
        fs::write(&file_path, b"AAAA").unwrap();
        let item = FileItem::from_path(&file_path).unwrap();

        let snapshot = OperationPolicy::create_snapshot(&item, true).unwrap();

        // Simulate external mutation with exact same byte length
        std::thread::sleep(std::time::Duration::from_millis(15));
        fs::write(&file_path, b"BBBB").unwrap();

        // Must catch hash difference
        let conflict = OperationPolicy::verify_conflict(&snapshot, true);
        assert!(conflict.is_err());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_disk_space_guardrail_check() {
        let temp_dir = std::env::temp_dir();
        // Check reasonable size (100 KiB with 1.5x multiplier)
        assert!(check_disk_space(&temp_dir, 100 * 1024, 1.5).is_ok());

        // Enormous impossible size (100 Petabytes) must trigger InsufficientSpace
        let impossible_bytes = 100 * 1024 * 1024 * 1024 * 1024 * 1024;
        let res = check_disk_space(&temp_dir, impossible_bytes, 1.5);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().error_code(), "ERR_INSUFFICIENT_SPACE");
    }

    #[test]
    fn test_export_typescript_bindings_policy() {
        let _ = fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        OriginalSnapshotDto::export(&cfg).expect("Failed to export OriginalSnapshotDto");
        PolicyCheckResultDto::export(&cfg).expect("Failed to export PolicyCheckResultDto");
    }
}
