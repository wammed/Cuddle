use crate::models::AppErrorDto;
use serde::{Deserialize, Serialize};
use std::process::Command;
use ts_rs::TS;

/// Severity / urgency level of a desktop notification (Task 2-3).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
#[serde(rename_all = "snake_case")]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl Default for NotificationLevel {
    fn default() -> Self {
        Self::Info
    }
}

/// Dispatches a native desktop notification conforming to the FreeDesktop Notifications Specification.
/// Supports COSMIC, GNOME, KDE, and other modern Linux desktop environments via `notify-send`.
/// Gracefully falls back if `notify-send` or a notification daemon is not available.
pub fn notify_desktop(
    title: &str,
    body: &str,
    level: NotificationLevel,
) -> Result<bool, AppErrorDto> {
    let urgency = match level {
        NotificationLevel::Info | NotificationLevel::Success => "normal",
        NotificationLevel::Warning => "normal",
        NotificationLevel::Error => "critical",
    };

    let icon = match level {
        NotificationLevel::Info => "dialog-information",
        NotificationLevel::Success => "emblem-default",
        NotificationLevel::Warning => "dialog-warning",
        NotificationLevel::Error => "dialog-error",
    };

    let mut cmd = Command::new("notify-send");
    cmd.arg("-a")
        .arg("Cuddle")
        .arg("-u")
        .arg(urgency)
        .arg("-i")
        .arg(icon)
        .arg(title)
        .arg(body);

    match cmd.status() {
        Ok(status) => Ok(status.success()),
        Err(e) => {
            eprintln!(
                "[Notification] Desktop notify-send invocation skipped or unavailable: {}",
                e
            );
            Ok(false)
        }
    }
}

/// Tauri IPC command to dispatch desktop notifications from frontend or backend.
#[tauri::command]
pub fn notify_user(
    title: String,
    body: String,
    level: Option<NotificationLevel>,
) -> Result<bool, AppErrorDto> {
    let lvl = level.unwrap_or_default();
    notify_desktop(&title, &body, lvl)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_level_defaults_and_conversion() {
        assert_eq!(NotificationLevel::default(), NotificationLevel::Info);

        let json = serde_json::to_string(&NotificationLevel::Warning).unwrap();
        assert_eq!(json, "\"warning\"");

        let de: NotificationLevel = serde_json::from_str("\"error\"").unwrap();
        assert_eq!(de, NotificationLevel::Error);
    }

    #[test]
    fn test_notify_desktop_does_not_panic() {
        // notify_desktop should not panic even if executed in CI/test environment
        let res = notify_desktop(
            "Cuddle Test",
            "Testing desktop notification abstraction",
            NotificationLevel::Info,
        );
        assert!(res.is_ok());
    }

    #[test]
    fn test_export_typescript_bindings_notify() {
        let _ = std::fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        let _ = NotificationLevel::export(&cfg);
    }
}
