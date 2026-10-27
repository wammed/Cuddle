use crate::extractor::{self, IndexOperationMode, RunIndexOptions};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::process::Command;
use ts_rs::TS;

const APP_DIR_NAME: &str = "Cuddle";
const LOCK_FILE_NAME: &str = "cuddle.lock";
const SYSTEMD_SERVICE_NAME: &str = "cuddle-index.service";
const SYSTEMD_TIMER_NAME: &str = "cuddle-index.timer";

/// RAII lock guard holding an exclusive non-blocking file lock.
/// Automatically releases the lock when dropped or when process terminates.
pub struct AppLock {
    _file: File,
    pub path: PathBuf,
}

/// DTO representing the systemd user timer status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct SystemdTimerStatusDto {
    pub is_installed: bool,
    pub is_active: bool,
    pub next_run_time: Option<String>,
    pub unit_path: String,
}

/// Returns the persistent path to the shared lock file:
/// `$XDG_DATA_HOME/Cuddle/cuddle.lock`
pub fn get_lock_file_path() -> Result<PathBuf, std::io::Error> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "XDG data dir not found"))?
        .join(APP_DIR_NAME);
    fs::create_dir_all(&data_dir)?;
    Ok(data_dir.join(LOCK_FILE_NAME))
}

/// Attempts to acquire an exclusive lock non-blockingly.
/// Returns Ok(Some(AppLock)) on success, Ok(None) if already locked by another process.
pub fn try_acquire_lock() -> Result<Option<AppLock>, std::io::Error> {
    let lock_path = get_lock_file_path()?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;

    let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if ret == 0 {
        Ok(Some(AppLock {
            _file: file,
            path: lock_path,
        }))
    } else {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EWOULDBLOCK) || err.raw_os_error() == Some(libc::EAGAIN)
        {
            Ok(None)
        } else {
            Err(err)
        }
    }
}

/// Returns the systemd user units directory: `~/.config/systemd/user/`
pub fn get_systemd_user_dir() -> Result<PathBuf, std::io::Error> {
    let config_dir = dirs::config_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "XDG config dir not found")
    })?;
    let user_systemd = config_dir.join("systemd").join("user");
    fs::create_dir_all(&user_systemd)?;
    Ok(user_systemd)
}

/// Generates systemd .service unit content for the current binary.
pub fn generate_systemd_service_content() -> String {
    let bin_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "%h/.local/bin/cuddle".to_string());

    format!(
        r#"[Unit]
Description=Cuddle File Workbench - Periodic Fulltext Indexer (Section 7.3)
Documentation=https://github.com/wammed/Cuddle
After=default.target

[Service]
Type=oneshot
ExecStart={} --index-update --quiet
StandardOutput=journal
StandardError=journal
Nice=19
CPUSchedulingPolicy=idle
IOSchedulingClass=idle
"#,
        bin_path
    )
}

/// Generates systemd .timer unit content (runs daily at 02:30 AM with persistent catch-up).
pub fn generate_systemd_timer_content() -> String {
    r#"[Unit]
Description=Daily Periodic Fulltext Index Timer for Cuddle (Section 7.3)
Documentation=https://github.com/wammed/Cuddle

[Timer]
# Run daily at 02:30 AM
OnCalendar=*-*-* 02:30:00
Persistent=true
RandomizedDelaySec=15m

[Install]
WantedBy=timers.target
"#
    .to_string()
}

/// Installs and enables the systemd user timer.
pub fn install_systemd_timer() -> Result<SystemdTimerStatusDto, String> {
    let unit_dir = get_systemd_user_dir().map_err(|e| e.to_string())?;
    let service_path = unit_dir.join(SYSTEMD_SERVICE_NAME);
    let timer_path = unit_dir.join(SYSTEMD_TIMER_NAME);

    fs::write(&service_path, generate_systemd_service_content())
        .map_err(|e| format!("Failed to write {}: {}", service_path.display(), e))?;
    fs::write(&timer_path, generate_systemd_timer_content())
        .map_err(|e| format!("Failed to write {}: {}", timer_path.display(), e))?;

    // Reload systemd user daemon
    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output();

    // Enable and start timer
    let enable_output = Command::new("systemctl")
        .args(["--user", "enable", "--now", SYSTEMD_TIMER_NAME])
        .output()
        .map_err(|e| format!("Failed to execute systemctl: {}", e))?;

    if !enable_output.status.success() {
        let stderr = String::from_utf8_lossy(&enable_output.stderr);
        return Err(format!("systemctl enable --now failed: {}", stderr));
    }

    get_systemd_timer_status()
}

/// Disables and removes the systemd user timer and service.
pub fn uninstall_systemd_timer() -> Result<SystemdTimerStatusDto, String> {
    let _ = Command::new("systemctl")
        .args(["--user", "stop", SYSTEMD_TIMER_NAME])
        .output();
    let _ = Command::new("systemctl")
        .args(["--user", "disable", SYSTEMD_TIMER_NAME])
        .output();

    let unit_dir = get_systemd_user_dir().map_err(|e| e.to_string())?;
    let service_path = unit_dir.join(SYSTEMD_SERVICE_NAME);
    let timer_path = unit_dir.join(SYSTEMD_TIMER_NAME);

    let _ = fs::remove_file(timer_path);
    let _ = fs::remove_file(service_path);

    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output();

    get_systemd_timer_status()
}

/// Checks the current systemd user timer status.
pub fn get_systemd_timer_status() -> Result<SystemdTimerStatusDto, String> {
    let unit_dir = get_systemd_user_dir().map_err(|e| e.to_string())?;
    let timer_path = unit_dir.join(SYSTEMD_TIMER_NAME);
    let is_installed = timer_path.exists();

    let is_active = if is_installed {
        Command::new("systemctl")
            .args(["--user", "is-active", SYSTEMD_TIMER_NAME])
            .output()
            .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "active")
            .unwrap_or(false)
    } else {
        false
    };

    let next_run_time = if is_active {
        // Query next trigger time from systemctl list-timers
        Command::new("systemctl")
            .args(["--user", "list-timers", SYSTEMD_TIMER_NAME, "--no-legend"])
            .output()
            .ok()
            .and_then(|out| {
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                let parts: Vec<&str> = text.split_whitespace().collect();
                if parts.len() >= 3 {
                    Some(format!("{} {}", parts[0], parts[1]))
                } else {
                    None
                }
            })
    } else {
        None
    };

    Ok(SystemdTimerStatusDto {
        is_installed,
        is_active,
        next_run_time,
        unit_path: timer_path.to_string_lossy().to_string(),
    })
}

/// Headless periodic indexer execution per Section 7.3.
/// Only runs when the app is NOT active, using lockfile mutual exclusion.
pub fn run_periodic_index_headless(quiet: bool) -> Result<(), String> {
    if !quiet {
        println!("[Cuddle Periodic Indexer] Checking application lockfile per Section 7.3...");
    }

    let lock = match try_acquire_lock() {
        Ok(Some(l)) => l,
        Ok(None) => {
            if !quiet {
                println!(
                    "[Cuddle Periodic Indexer] Cuddle application is currently active. \
                     Skipping periodic index update per specification 7.3."
                );
            }
            return Ok(());
        }
        Err(e) => {
            return Err(format!("Failed to access lockfile: {}", e));
        }
    };

    if !quiet {
        println!(
            "[Cuddle Periodic Indexer] Lock acquired ({:?}). Running incremental index on $HOME...",
            lock.path
        );
    }

    let home_path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let home_b64 = crate::models::encode_path_to_base64(&home_path);

    let opts = RunIndexOptions {
        target_raw_base64: Some(home_b64),
        mode: IndexOperationMode::IncrementalUpdate,
        max_depth: None,
        include_hidden: true,
    };

    let result = extractor::run_index_operation(opts).map_err(|e| e.to_string())?;

    if !quiet {
        println!(
            "[Cuddle Periodic Indexer] Finished in {}ms: {} indexed, {} updated, {} unchanged, {} skipped (total: {} documents).",
            result.elapsed_ms,
            result.indexed_count,
            result.updated_count,
            result.unchanged_count,
            result.skipped_count,
            result.total_documents
        );
    }

    Ok(())
}

/// Inspects CLI arguments on startup. If an indexer command was provided,
/// handles it and returns Some(result). Otherwise returns None.
pub fn handle_cli_args(args: &[String]) -> Option<Result<(), String>> {
    let mut is_index_update = false;
    let mut is_quiet = false;
    let mut is_install = false;
    let mut is_uninstall = false;
    let mut is_status = false;

    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "--index-update" | "--cron-index" => is_index_update = true,
            "--quiet" | "-q" => is_quiet = true,
            "--install-systemd-timer" => is_install = true,
            "--uninstall-systemd-timer" => is_uninstall = true,
            "--systemd-timer-status" => is_status = true,
            "--help" | "-h" => {
                print_cli_help();
                return Some(Ok(()));
            }
            _ => {}
        }
    }

    if is_install {
        return Some(install_systemd_timer().map(|res| {
            println!(
                "Successfully installed and enabled systemd user timer: {}",
                res.unit_path
            );
        }));
    }

    if is_uninstall {
        return Some(uninstall_systemd_timer().map(|_| {
            println!("Successfully uninstalled systemd user timer.");
        }));
    }

    if is_status {
        return Some(get_systemd_timer_status().map(|res| {
            println!("Installed: {}", res.is_installed);
            println!("Active:    {}", res.is_active);
            if let Some(next) = res.next_run_time {
                println!("Next run:  {}", next);
            }
        }));
    }

    if is_index_update {
        return Some(run_periodic_index_headless(is_quiet));
    }

    None
}

fn print_cli_help() {
    println!(
        r#"Cuddle File Workbench - CLI Indexer & Periodic Update Options (Section 7.3)

USAGE:
    cuddle [OPTIONS]

INDEXING OPTIONS:
    --index-update              Run headless incremental index scan on $HOME (respects lockfile)
    --quiet, -q                 Suppress normal output (ideal for systemd / cron)
    --install-systemd-timer     Install and enable daily systemd user timer (~/.config/systemd/user/)
    --uninstall-systemd-timer   Disable and remove systemd user timer
    --systemd-timer-status      Check current systemd timer status
    --help, -h                  Show this help message
"#
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lockfile_mutual_exclusion() {
        let lock1 = try_acquire_lock().unwrap();
        assert!(lock1.is_some(), "First lock acquisition should succeed");

        let lock2 = try_acquire_lock().unwrap();
        assert!(
            lock2.is_none(),
            "Second lock acquisition while first is held must fail"
        );

        drop(lock1);

        let lock3 = try_acquire_lock().unwrap();
        assert!(
            lock3.is_some(),
            "Lock acquisition after drop should succeed again"
        );
    }

    #[test]
    fn test_generate_systemd_contents() {
        let service = generate_systemd_service_content();
        assert!(service.contains("--index-update"));
        assert!(service.contains("Nice=19"));
        assert!(service.contains("IOSchedulingClass=idle"));

        let timer = generate_systemd_timer_content();
        assert!(timer.contains("OnCalendar=*-*-* 02:30:00"));
        assert!(timer.contains("Persistent=true"));
    }

    #[test]
    fn test_cli_args_dispatch() {
        let args = vec!["cuddle".to_string(), "--help".to_string()];
        let res = handle_cli_args(&args);
        assert!(res.is_some());
        assert!(res.unwrap().is_ok());

        let args_normal = vec!["cuddle".to_string()];
        assert!(handle_cli_args(&args_normal).is_none());
    }

    #[test]
    fn test_export_typescript_bindings_periodic_index() {
        let _ = fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        SystemdTimerStatusDto::export(&cfg)
            .expect("Failed to export SystemdTimerStatusDto TypeScript binding");
    }
}
