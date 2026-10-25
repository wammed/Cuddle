use landlock::{
    Access, AccessFs, PathBeneath, PathFd, RestrictionStatus, Ruleset, RulesetAttr,
    RulesetCreatedAttr, RulesetError, RulesetStatus, ABI,
};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SandboxError {
    #[error("Landlock is not supported or not enforced by current kernel")]
    NotSupported,
    #[error("Ruleset error: {0}")]
    Ruleset(#[from] RulesetError),
    #[error("Failed to open path descriptor for {0}: {1}")]
    PathFd(PathBuf, String),
    #[error("Process execution failed: {0}")]
    Io(#[from] std::io::Error),
}

/// Probes the highest supported Landlock ABI on the host Linux kernel.
pub fn probe_landlock_abi() -> Result<ABI, SandboxError> {
    // Try ABIs from newest (V5) down to V1
    let candidate_abis = [ABI::V5, ABI::V4, ABI::V3, ABI::V2, ABI::V1];

    for &abi in &candidate_abis {
        let access_all = AccessFs::from_all(abi);
        if let Ok(ruleset_created) = Ruleset::default().handle_access(access_all) {
            if let Ok(created) = ruleset_created.create() {
                // Testing with a dummy self-restrict is not done here to avoid restricting current test process,
                // but creating a ruleset successfully means the kernel supports this ABI.
                let _ = created;
                return Ok(abi);
            }
        }
    }

    Err(SandboxError::NotSupported)
}

/// Builder for isolating external child processes via Landlock LSM.
#[derive(Debug, Clone)]
pub struct LandlockSandbox {
    read_paths: Vec<PathBuf>,
    write_paths: Vec<PathBuf>,
    abi: ABI,
}

impl LandlockSandbox {
    /// Creates a new sandbox builder. Default ABI is V1 for universal compatibility.
    pub fn new() -> Self {
        Self {
            read_paths: Vec::new(),
            write_paths: Vec::new(),
            abi: ABI::V1,
        }
    }

    /// Sets a specific ABI version.
    pub fn with_abi(mut self, abi: ABI) -> Self {
        self.abi = abi;
        self
    }

    /// Whitelist a path for read-only access.
    pub fn allow_read<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.read_paths.push(path.as_ref().to_path_buf());
        self
    }

    /// Whitelist a path for read and write access.
    pub fn allow_write<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.write_paths.push(path.as_ref().to_path_buf());
        self
    }

    /// Automatically include system directories required to run standard binaries.
    pub fn add_system_defaults(self) -> Self {
        let mut s = self;
        for path in &[
            "/usr",
            "/lib",
            "/lib64",
            "/etc",
            "/bin",
            "/dev/null",
            "/dev/urandom",
        ] {
            if Path::new(path).exists() {
                s = s.allow_read(path);
            }
        }
        s
    }

    /// Apply Landlock ruleset to the calling process/thread.
    /// MUST be called inside `pre_exec` hook of `std::process::Command` to isolate only child.
    pub fn apply(&self) -> Result<RestrictionStatus, SandboxError> {
        let access_all = AccessFs::from_all(self.abi);
        let access_read = AccessFs::from_read(self.abi);

        let mut ruleset = Ruleset::default()
            .handle_access(access_all)
            .map_err(SandboxError::Ruleset)?
            .create()
            .map_err(SandboxError::Ruleset)?;

        // Apply read-only rules
        for p in &self.read_paths {
            if p.exists() {
                if let Ok(fd) = PathFd::new(p) {
                    ruleset = ruleset
                        .add_rule(PathBeneath::new(fd, access_read))
                        .map_err(SandboxError::Ruleset)?;
                }
            }
        }

        // Apply read/write rules
        for p in &self.write_paths {
            if p.exists() {
                if let Ok(fd) = PathFd::new(p) {
                    ruleset = ruleset
                        .add_rule(PathBeneath::new(fd, access_all))
                        .map_err(SandboxError::Ruleset)?;
                }
            }
        }

        let status = ruleset.restrict_self().map_err(SandboxError::Ruleset)?;
        if status.ruleset == RulesetStatus::NotEnforced {
            return Err(SandboxError::NotSupported);
        }

        Ok(status)
    }

    /// Configures a `std::process::Command` with Landlock sandbox pre-exec hook and isolated TMPDIR.
    pub fn configure_command(&self, cmd: &mut Command, isolated_tmpdir: &Path) {
        self.configure_command_with_limits(cmd, isolated_tmpdir, None);
    }

    /// Configures a `std::process::Command` with Landlock sandbox pre-exec hook, isolated TMPDIR,
    /// and an optional memory limit (RLIMIT_AS).
    pub fn configure_command_with_limits(
        &self,
        cmd: &mut Command,
        isolated_tmpdir: &Path,
        memory_limit_bytes: Option<u64>,
    ) {
        // Isolate TMPDIR environment variable
        cmd.env("TMPDIR", isolated_tmpdir);

        let sandbox = self.clone().allow_write(isolated_tmpdir);

        unsafe {
            cmd.pre_exec(move || {
                if let Some(limit_bytes) = memory_limit_bytes {
                    let rlimit = libc::rlimit {
                        rlim_cur: limit_bytes,
                        rlim_max: limit_bytes,
                    };
                    let ret = libc::setrlimit(libc::RLIMIT_AS, &rlimit);
                    if ret != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                sandbox.apply().map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::PermissionDenied, e.to_string())
                })?;
                Ok(())
            });
        }
    }
}

impl Default for LandlockSandbox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_probe_abi() {
        let abi = probe_landlock_abi();
        assert!(abi.is_ok(), "Kernel should support Landlock ABI");
        let version = abi.unwrap();
        println!("Probed kernel Landlock ABI: {:?}", version);
    }

    #[test]
    fn test_landlock_child_isolation() {
        let base_dir =
            std::env::temp_dir().join(format!("cuddle_landlock_test_{}", std::process::id()));
        let allowed_workspace = base_dir.join("workspace_temp");
        let forbidden_original = base_dir.join("original_protected");

        fs::create_dir_all(&allowed_workspace).unwrap();
        fs::create_dir_all(&forbidden_original).unwrap();

        let sandbox = LandlockSandbox::new()
            .add_system_defaults()
            .allow_write(&allowed_workspace);

        // 1. Test writing to allowed workspace -> MUST SUCCEED
        let allowed_file = allowed_workspace.join("output.txt");
        let mut cmd_success = Command::new("sh");
        cmd_success.args([
            "-c",
            &format!("echo 'hello' > '{}'", allowed_file.display()),
        ]);
        sandbox.configure_command(&mut cmd_success, &allowed_workspace);

        let status_success = cmd_success.status().expect("Failed to spawn success test");
        assert!(
            status_success.success(),
            "Writing to allowed workspace must succeed"
        );
        assert!(allowed_file.exists(), "Output file must exist");

        // 2. Test writing to forbidden original directory -> MUST BE BLOCKED BY LANDLOCK
        let forbidden_file = forbidden_original.join("tampered.txt");
        let mut cmd_fail = Command::new("sh");
        cmd_fail.args([
            "-c",
            &format!("echo 'bad' > '{}'", forbidden_file.display()),
        ]);
        sandbox.configure_command(&mut cmd_fail, &allowed_workspace);

        let output_fail = cmd_fail.output().expect("Failed to spawn fail test");
        assert!(
            !output_fail.status.success(),
            "Writing to unlisted directory must fail"
        );
        assert!(
            !forbidden_file.exists(),
            "Forbidden file must NOT be created"
        );

        // Cleanup
        let _ = fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn test_external_tool_magick_isolation() {
        // Skip if neither magick nor convert exists on the host
        let tool = if Path::new("/usr/bin/magick").exists() {
            "magick"
        } else if Path::new("/usr/bin/convert").exists() {
            "convert"
        } else {
            println!("Skipping magick test: tool not found in /usr/bin");
            return;
        };

        let base_dir =
            std::env::temp_dir().join(format!("cuddle_magick_test_{}", std::process::id()));
        let allowed_workspace = base_dir.join("workspace_temp");
        let forbidden_original = base_dir.join("original_protected");

        fs::create_dir_all(&allowed_workspace).unwrap();
        fs::create_dir_all(&forbidden_original).unwrap();

        let sandbox = LandlockSandbox::new()
            .add_system_defaults()
            .allow_write(&allowed_workspace);

        // 1. Success case: magick creates an image inside allowed workspace
        let allowed_png = allowed_workspace.join("sample.png");
        let mut cmd_success = Command::new(tool);
        cmd_success.args(["-size", "16x16", "xc:blue", allowed_png.to_str().unwrap()]);
        sandbox.configure_command(&mut cmd_success, &allowed_workspace);

        let status_success = cmd_success
            .status()
            .expect("Failed to execute magick in allowed workspace");
        assert!(
            status_success.success(),
            "Magick in allowed workspace should succeed"
        );
        assert!(allowed_png.exists(), "Allowed png file must be generated");

        // 2. Blocked case: magick tries to write to forbidden original directory
        let forbidden_png = forbidden_original.join("injected.png");
        let mut cmd_fail = Command::new(tool);
        cmd_fail.args(["-size", "16x16", "xc:red", forbidden_png.to_str().unwrap()]);
        sandbox.configure_command(&mut cmd_fail, &allowed_workspace);

        let output_fail = cmd_fail
            .output()
            .expect("Failed to execute magick targeting forbidden path");
        assert!(
            !output_fail.status.success(),
            "Magick must NOT succeed when writing to forbidden path"
        );
        assert!(
            !forbidden_png.exists(),
            "Forbidden file must never be created by external tool"
        );

        // Cleanup
        let _ = fs::remove_dir_all(&base_dir);
    }
}
