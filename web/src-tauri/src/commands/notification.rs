//! Notification commands
//!
//! Tauri commands for system notifications.

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

// =============================================================================
// Types
// =============================================================================

/// Notification type for categorization
#[derive(Debug, Clone, Copy)]
pub enum NotificationType {
    /// Sync completed successfully
    SyncSuccess,
    /// Sync completed with errors
    SyncError,
    /// Authentication required
    AuthRequired,
    /// Source configuration issue
    SourceError,
}

impl NotificationType {
    fn title(&self) -> &'static str {
        match self {
            NotificationType::SyncSuccess => "同步完成",
            NotificationType::SyncError => "同步錯誤",
            NotificationType::AuthRequired => "需要重新登入",
            NotificationType::SourceError => "來源設定錯誤",
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Send a system notification
pub fn send_notification(
    app: &AppHandle,
    notification_type: NotificationType,
    body: &str,
) -> Result<(), String> {
    app.notification()
        .builder()
        .title(notification_type.title())
        .body(body)
        .show()
        .map_err(|e| e.to_string())
}

// =============================================================================
// Commands
// =============================================================================

/// Send a notification from frontend
#[tauri::command]
pub async fn send_sync_notification(
    app: AppHandle,
    success: bool,
    message: String,
) -> Result<(), String> {
    let notification_type = if success {
        NotificationType::SyncSuccess
    } else {
        NotificationType::SyncError
    };

    send_notification(&app, notification_type, &message)
}

/// Send an auth required notification
#[tauri::command]
pub async fn send_auth_notification(app: AppHandle, message: String) -> Result<(), String> {
    send_notification(&app, NotificationType::AuthRequired, &message)
}

/// Send a source error notification
#[tauri::command]
pub async fn send_source_error_notification(
    app: AppHandle,
    source: String,
    error: String,
) -> Result<(), String> {
    let body = format!("{}: {}", source, error);
    send_notification(&app, NotificationType::SourceError, &body)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_type_title() {
        assert_eq!(NotificationType::SyncSuccess.title(), "同步完成");
        assert_eq!(NotificationType::SyncError.title(), "同步錯誤");
        assert_eq!(NotificationType::AuthRequired.title(), "需要重新登入");
        assert_eq!(NotificationType::SourceError.title(), "來源設定錯誤");
    }
}
