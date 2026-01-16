//! Tray commands
//!
//! Tauri commands for system tray operations.
//! Note: Dynamic menu updates will be enhanced in Phase 2 with BackgroundSyncService.

use tauri::AppHandle;

/// Placeholder for updating tray sync status
/// In Phase 2, this will update the tray menu's last sync time display
#[tauri::command]
pub async fn update_tray_sync_status(_app: AppHandle, last_sync: String) -> Result<(), String> {
    // For Phase 1, just log the sync status
    // Phase 2 will implement full tray state management with BackgroundSyncService
    log::info!("Sync completed at: {}", last_sync);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_status_text_format() {
        let last_sync = "16:30";
        let status_text = format!("上次同步: {}", last_sync);
        assert_eq!(status_text, "上次同步: 16:30");
    }

    #[test]
    fn test_empty_status() {
        let last_sync = "";
        let status_text = if last_sync.is_empty() {
            "上次同步: -".to_string()
        } else {
            format!("上次同步: {}", last_sync)
        };
        assert_eq!(status_text, "上次同步: -");
    }
}
