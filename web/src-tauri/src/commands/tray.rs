//! Tray commands
//!
//! Tauri commands for system tray operations.

use tauri::{
    menu::{Menu, MenuItem},
    AppHandle,
};

// =============================================================================
// Helper Functions
// =============================================================================

/// Format time for display in tray menu
fn format_time_for_tray(iso_string: &str) -> String {
    // Parse ISO 8601 and format as HH:MM
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(iso_string) {
        dt.format("%H:%M").to_string()
    } else {
        // Fallback: try to extract time from the string
        if let Some(time_part) = iso_string.split('T').nth(1) {
            time_part.split('+').next()
                .or_else(|| time_part.split('Z').next())
                .map(|t| t.chars().take(5).collect())
                .unwrap_or_else(|| "-".to_string())
        } else {
            "-".to_string()
        }
    }
}

/// Build status text for tray menu
fn build_status_text(last_sync: &str, is_syncing: bool) -> String {
    if is_syncing {
        "同步中...".to_string()
    } else if last_sync.is_empty() {
        "上次同步: -".to_string()
    } else {
        format!("上次同步: {}", format_time_for_tray(last_sync))
    }
}

/// Rebuild and set the tray menu with updated status
fn rebuild_tray_menu(
    app: &AppHandle,
    status_text: &str,
    sync_enabled: bool,
) -> Result<(), String> {
    // Get the tray icon by ID
    let tray = app
        .tray_by_id("main-tray")
        .ok_or_else(|| "Tray icon not found".to_string())?;

    // Build new menu items
    let show_item = MenuItem::with_id(app, "show", "開啟 Recap", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let sync_item = MenuItem::with_id(app, "sync_now", "立即同步", sync_enabled, None::<&str>)
        .map_err(|e| e.to_string())?;
    let separator = MenuItem::with_id(app, "sep1", "─────────────", false, None::<&str>)
        .map_err(|e| e.to_string())?;
    let status_item = MenuItem::with_id(app, "status", status_text, false, None::<&str>)
        .map_err(|e| e.to_string())?;
    let separator2 = MenuItem::with_id(app, "sep2", "─────────────", false, None::<&str>)
        .map_err(|e| e.to_string())?;
    let quit_item = MenuItem::with_id(app, "quit", "結束 Recap", true, None::<&str>)
        .map_err(|e| e.to_string())?;

    // Build the menu
    let menu = Menu::with_items(
        app,
        &[&show_item, &sync_item, &separator, &status_item, &separator2, &quit_item],
    )
    .map_err(|e| e.to_string())?;

    // Set the new menu
    tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;

    log::debug!("Tray menu rebuilt with status: {}", status_text);
    Ok(())
}

// =============================================================================
// Commands
// =============================================================================

/// Update the tray menu's sync status display
#[tauri::command]
pub async fn update_tray_sync_status(
    app: AppHandle,
    last_sync: String,
    is_syncing: Option<bool>,
) -> Result<(), String> {
    let is_syncing = is_syncing.unwrap_or(false);
    let status_text = build_status_text(&last_sync, is_syncing);

    // Rebuild menu with sync button enabled (not currently syncing)
    rebuild_tray_menu(&app, &status_text, !is_syncing)
}

/// Update tray to show syncing state
#[tauri::command]
pub async fn set_tray_syncing(app: AppHandle, syncing: bool) -> Result<(), String> {
    let status_text = if syncing {
        "同步中...".to_string()
    } else {
        "上次同步: -".to_string()
    };

    // Rebuild menu: disable sync button when syncing
    rebuild_tray_menu(&app, &status_text, !syncing)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time_for_tray_valid() {
        let time = format_time_for_tray("2026-01-16T14:30:00+08:00");
        assert_eq!(time, "14:30");
    }

    #[test]
    fn test_format_time_for_tray_utc() {
        let time = format_time_for_tray("2026-01-16T06:30:00Z");
        assert_eq!(time, "06:30");
    }

    #[test]
    fn test_format_time_for_tray_invalid() {
        let time = format_time_for_tray("invalid");
        assert_eq!(time, "-");
    }

    #[test]
    fn test_build_status_text_syncing() {
        let text = build_status_text("", true);
        assert_eq!(text, "同步中...");
    }

    #[test]
    fn test_build_status_text_empty() {
        let text = build_status_text("", false);
        assert_eq!(text, "上次同步: -");
    }

    #[test]
    fn test_build_status_text_with_time() {
        let text = build_status_text("2026-01-16T14:30:00+08:00", false);
        assert_eq!(text, "上次同步: 14:30");
    }
}
