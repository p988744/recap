//! Background Sync Commands
//!
//! Tauri commands for controlling the background sync service.

use super::AppState;
use recap_core::auth::verify_token;
use crate::services::background_sync::{BackgroundSyncConfig, SyncOperationResult, SyncServiceStatus};
use serde::{Deserialize, Serialize};
use tauri::State;

// =============================================================================
// Request/Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct UpdateBackgroundSyncConfigRequest {
    pub enabled: Option<bool>,
    pub interval_minutes: Option<u32>,
    pub sync_git: Option<bool>,
    pub sync_claude: Option<bool>,
    pub sync_gitlab: Option<bool>,
    pub sync_jira: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct BackgroundSyncConfigResponse {
    pub enabled: bool,
    pub interval_minutes: u32,
    pub sync_git: bool,
    pub sync_claude: bool,
    pub sync_gitlab: bool,
    pub sync_jira: bool,
}

impl From<BackgroundSyncConfig> for BackgroundSyncConfigResponse {
    fn from(config: BackgroundSyncConfig) -> Self {
        Self {
            enabled: config.enabled,
            interval_minutes: config.interval_minutes,
            sync_git: config.sync_git,
            sync_claude: config.sync_claude,
            sync_gitlab: config.sync_gitlab,
            sync_jira: config.sync_jira,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BackgroundSyncStatusResponse {
    pub is_running: bool,
    pub is_syncing: bool,
    pub last_sync_at: Option<String>,
    pub next_sync_at: Option<String>,
    pub last_result: Option<String>,
    pub last_error: Option<String>,
}

impl From<SyncServiceStatus> for BackgroundSyncStatusResponse {
    fn from(status: SyncServiceStatus) -> Self {
        Self {
            is_running: status.is_running,
            is_syncing: status.is_syncing,
            last_sync_at: status.last_sync_at,
            next_sync_at: status.next_sync_at,
            last_result: status.last_result,
            last_error: status.last_error,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SyncResultResponse {
    pub source: String,
    pub success: bool,
    pub items_synced: i32,
    pub error: Option<String>,
}

impl From<SyncOperationResult> for SyncResultResponse {
    fn from(result: SyncOperationResult) -> Self {
        Self {
            source: result.source,
            success: result.success,
            items_synced: result.items_synced,
            error: result.error,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TriggerSyncResponse {
    pub results: Vec<SyncResultResponse>,
    pub total_items: i32,
}

// =============================================================================
// Commands
// =============================================================================

/// Get the current background sync configuration
#[tauri::command]
pub async fn get_background_sync_config(
    state: State<'_, AppState>,
    token: String,
) -> Result<BackgroundSyncConfigResponse, String> {
    verify_token(&token).map_err(|e| e.to_string())?;

    let config = state.background_sync.get_config().await;
    Ok(config.into())
}

/// Update the background sync configuration
#[tauri::command]
pub async fn update_background_sync_config(
    state: State<'_, AppState>,
    token: String,
    config: UpdateBackgroundSyncConfigRequest,
) -> Result<BackgroundSyncConfigResponse, String> {
    verify_token(&token).map_err(|e| e.to_string())?;

    // Get current config and apply updates
    let current = state.background_sync.get_config().await;
    let new_config = BackgroundSyncConfig {
        enabled: config.enabled.unwrap_or(current.enabled),
        interval_minutes: config.interval_minutes.unwrap_or(current.interval_minutes),
        sync_git: config.sync_git.unwrap_or(current.sync_git),
        sync_claude: config.sync_claude.unwrap_or(current.sync_claude),
        sync_gitlab: config.sync_gitlab.unwrap_or(current.sync_gitlab),
        sync_jira: config.sync_jira.unwrap_or(current.sync_jira),
    };

    // Validate interval
    if ![5, 15, 30, 60].contains(&new_config.interval_minutes) {
        return Err("間隔時間必須是 5, 15, 30 或 60 分鐘".to_string());
    }

    state.background_sync.update_config(new_config.clone()).await;
    log::info!("Background sync config updated: {:?}", new_config);

    Ok(new_config.into())
}

/// Get the current background sync status
#[tauri::command]
pub async fn get_background_sync_status(
    state: State<'_, AppState>,
    token: String,
) -> Result<BackgroundSyncStatusResponse, String> {
    verify_token(&token).map_err(|e| e.to_string())?;

    let status = state.background_sync.get_status().await;
    Ok(status.into())
}

/// Start the background sync service
#[tauri::command]
pub async fn start_background_sync(
    state: State<'_, AppState>,
    token: String,
) -> Result<(), String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;

    // Set user ID for sync operations
    state.background_sync.set_user_id(claims.sub).await;

    state.background_sync.start().await;
    log::info!("Background sync service started");

    Ok(())
}

/// Stop the background sync service
#[tauri::command]
pub async fn stop_background_sync(
    state: State<'_, AppState>,
    token: String,
) -> Result<(), String> {
    verify_token(&token).map_err(|e| e.to_string())?;

    state.background_sync.stop().await;
    log::info!("Background sync service stopped");

    Ok(())
}

/// Trigger an immediate sync
#[tauri::command]
pub async fn trigger_background_sync(
    state: State<'_, AppState>,
    token: String,
) -> Result<TriggerSyncResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;

    // Ensure user ID is set
    state.background_sync.set_user_id(claims.sub).await;

    let results = state.background_sync.trigger_sync().await;
    let total_items: i32 = results.iter().map(|r| r.items_synced).sum();

    log::info!("Manual sync triggered, {} items synced", total_items);

    Ok(TriggerSyncResponse {
        results: results.into_iter().map(|r| r.into()).collect(),
        total_items,
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_response_from() {
        let config = BackgroundSyncConfig {
            enabled: true,
            interval_minutes: 15,
            sync_git: true,
            sync_claude: true,
            sync_gitlab: false,
            sync_jira: false,
        };

        let response: BackgroundSyncConfigResponse = config.into();
        assert!(response.enabled);
        assert_eq!(response.interval_minutes, 15);
        assert!(response.sync_git);
        assert!(response.sync_claude);
        assert!(!response.sync_gitlab);
        assert!(!response.sync_jira);
    }

    #[test]
    fn test_status_response_from() {
        let status = SyncServiceStatus {
            is_running: true,
            is_syncing: false,
            last_sync_at: Some("2026-01-16T12:00:00Z".to_string()),
            next_sync_at: Some("2026-01-16T12:15:00Z".to_string()),
            last_result: Some("成功同步 5 筆項目".to_string()),
            last_error: None,
        };

        let response: BackgroundSyncStatusResponse = status.into();
        assert!(response.is_running);
        assert!(!response.is_syncing);
        assert_eq!(response.last_sync_at, Some("2026-01-16T12:00:00Z".to_string()));
    }

    #[test]
    fn test_sync_result_response_from() {
        let result = SyncOperationResult {
            source: "git".to_string(),
            success: true,
            items_synced: 3,
            projects_scanned: 0,
            items_created: 0,
            error: None,
        };

        let response: SyncResultResponse = result.into();
        assert_eq!(response.source, "git");
        assert!(response.success);
        assert_eq!(response.items_synced, 3);
    }

    #[test]
    fn test_valid_intervals() {
        let valid_intervals = [5, 15, 30, 60];
        for interval in valid_intervals {
            assert!(valid_intervals.contains(&interval));
        }

        assert!(![5, 15, 30, 60].contains(&10));
        assert!(![5, 15, 30, 60].contains(&45));
    }
}
