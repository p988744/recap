//! Quota Timer Tauri Commands
//!
//! Commands for managing the background quota polling service.

use std::sync::Arc;

use recap_core::auth::verify_token;
use recap_core::services::quota::{
    ClaudeQuotaProvider, QuotaPollingConfig, QuotaPollingState, QuotaPollingStatus, QuotaProvider,
    QuotaStore, SharedPollingState,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tokio::sync::RwLock;

use super::AppState;

// ============================================================================
// State Management
// ============================================================================

/// State for the quota polling service
///
/// This is managed separately from AppState because it has its own lifecycle.
pub struct QuotaPollingServiceState {
    /// Shared polling state
    pub state: SharedPollingState,
    /// Shutdown signal sender
    pub shutdown_tx: Arc<RwLock<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl Default for QuotaPollingServiceState {
    fn default() -> Self {
        Self::new()
    }
}

impl QuotaPollingServiceState {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(QuotaPollingState::new(
                QuotaPollingConfig::default(),
            ))),
            shutdown_tx: Arc::new(RwLock::new(None)),
        }
    }
}

// ============================================================================
// DTOs
// ============================================================================

/// Request to update quota polling configuration
#[derive(Debug, Deserialize)]
pub struct UpdatePollingConfigRequest {
    /// Whether polling is enabled
    pub enabled: Option<bool>,
    /// Polling interval in minutes
    pub interval_minutes: Option<u32>,
    /// Warning threshold percentage
    pub warning_threshold: Option<f64>,
    /// Critical threshold percentage
    pub critical_threshold: Option<f64>,
    /// Whether to show notifications
    pub notify_on_threshold: Option<bool>,
    /// Whether to update tray
    pub update_tray: Option<bool>,
}

/// Response for quota polling status
#[derive(Debug, Serialize)]
pub struct PollingStatusResponse {
    /// Whether the service is currently running
    pub is_running: bool,
    /// Whether a poll is currently in progress
    pub is_polling: bool,
    /// Last poll timestamp (ISO 8601)
    pub last_poll_at: Option<String>,
    /// Next scheduled poll timestamp (ISO 8601)
    pub next_poll_at: Option<String>,
    /// Last error message
    pub last_error: Option<String>,
    /// Current quota percentage for Claude (5-hour window)
    pub claude_percent: Option<f64>,
    /// Current configuration
    pub config: QuotaPollingConfigDto,
}

/// DTO for quota polling configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct QuotaPollingConfigDto {
    pub enabled: bool,
    pub interval_minutes: u32,
    pub warning_threshold: f64,
    pub critical_threshold: f64,
    pub notify_on_threshold: bool,
    pub update_tray: bool,
}

impl From<QuotaPollingConfig> for QuotaPollingConfigDto {
    fn from(config: QuotaPollingConfig) -> Self {
        Self {
            enabled: config.enabled,
            interval_minutes: config.interval_minutes,
            warning_threshold: config.warning_threshold,
            critical_threshold: config.critical_threshold,
            notify_on_threshold: config.notify_on_threshold,
            update_tray: config.update_tray,
        }
    }
}

impl From<QuotaPollingConfigDto> for QuotaPollingConfig {
    fn from(dto: QuotaPollingConfigDto) -> Self {
        Self {
            enabled: dto.enabled,
            interval_minutes: dto.interval_minutes,
            warning_threshold: dto.warning_threshold,
            critical_threshold: dto.critical_threshold,
            notify_on_threshold: dto.notify_on_threshold,
            update_tray: dto.update_tray,
        }
    }
}

// ============================================================================
// Commands
// ============================================================================

/// Start the quota polling service
#[tauri::command(rename_all = "snake_case")]
pub async fn start_quota_polling(
    app: AppHandle,
    app_state: State<'_, AppState>,
    polling_state: State<'_, QuotaPollingServiceState>,
    token: String,
) -> Result<PollingStatusResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    log::info!(
        "[quota:timer] Starting quota polling for user {}",
        claims.sub
    );

    // Check if already running
    {
        let state = polling_state.state.read().await;
        if state.is_running {
            log::info!("[quota:timer] Polling already running");
            return Ok(build_status_response(&state));
        }
    }

    // Start the polling loop
    let state_clone = Arc::clone(&polling_state.state);
    let shutdown_tx_clone = Arc::clone(&polling_state.shutdown_tx);
    let db_clone = Arc::clone(&app_state.db);
    let user_id = claims.sub.clone();
    let app_handle = app.clone();

    // Create shutdown channel
    let (tx, mut rx) = tokio::sync::oneshot::channel::<()>();
    {
        let mut shutdown_tx = shutdown_tx_clone.write().await;
        *shutdown_tx = Some(tx);
    }

    // Mark as started
    {
        let mut state = state_clone.write().await;
        state.start();
    }

    // Spawn the polling loop
    tokio::spawn(async move {
        log::info!("[quota:timer] Polling loop started");

        loop {
            // Get interval from config
            let interval_secs = {
                let state = state_clone.read().await;
                if !state.is_running || !state.config.enabled {
                    log::info!("[quota:timer] Polling disabled, exiting loop");
                    break;
                }
                state.interval_secs()
            };

            // Wait for interval or shutdown
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)) => {
                    // Time to poll
                }
                _ = &mut rx => {
                    log::info!("[quota:timer] Received shutdown signal");
                    break;
                }
            }

            // Check if still running
            {
                let state = state_clone.read().await;
                if !state.is_running || !state.config.enabled {
                    log::info!("[quota:timer] Polling stopped");
                    break;
                }
            }

            // Perform poll
            let result = perform_quota_poll(
                &state_clone,
                &db_clone,
                &user_id,
                &app_handle,
            )
            .await;

            if let Err(e) = result {
                log::error!("[quota:timer] Poll error: {}", e);
                let mut state = state_clone.write().await;
                state.complete_poll(Some(e));
            }
        }

        // Mark as stopped
        {
            let mut state = state_clone.write().await;
            state.stop();
        }

        log::info!("[quota:timer] Polling loop exited");
    });

    // Return current status
    let state = polling_state.state.read().await;
    Ok(build_status_response(&state))
}

/// Stop the quota polling service
#[tauri::command(rename_all = "snake_case")]
pub async fn stop_quota_polling(
    polling_state: State<'_, QuotaPollingServiceState>,
    token: String,
) -> Result<PollingStatusResponse, String> {
    let _claims = verify_token(&token).map_err(|e| e.to_string())?;
    log::info!("[quota:timer] Stopping quota polling");

    // Send shutdown signal
    {
        let mut shutdown_tx = polling_state.shutdown_tx.write().await;
        if let Some(tx) = shutdown_tx.take() {
            let _ = tx.send(());
        }
    }

    // Mark as stopped
    {
        let mut state = polling_state.state.write().await;
        state.stop();
    }

    let state = polling_state.state.read().await;
    Ok(build_status_response(&state))
}

/// Get the current polling status
#[tauri::command(rename_all = "snake_case")]
pub async fn get_quota_polling_status(
    polling_state: State<'_, QuotaPollingServiceState>,
    token: String,
) -> Result<PollingStatusResponse, String> {
    let _claims = verify_token(&token).map_err(|e| e.to_string())?;

    let state = polling_state.state.read().await;
    Ok(build_status_response(&state))
}

/// Update the polling configuration
#[tauri::command(rename_all = "snake_case")]
pub async fn update_quota_polling_config(
    polling_state: State<'_, QuotaPollingServiceState>,
    token: String,
    config: UpdatePollingConfigRequest,
) -> Result<PollingStatusResponse, String> {
    let _claims = verify_token(&token).map_err(|e| e.to_string())?;
    log::info!("[quota:timer] Updating polling config: {:?}", config);

    {
        let mut state = polling_state.state.write().await;
        let mut new_config = state.config.clone();

        // Apply updates
        if let Some(enabled) = config.enabled {
            new_config.enabled = enabled;
        }
        if let Some(interval) = config.interval_minutes {
            new_config.interval_minutes = interval;
        }
        if let Some(warning) = config.warning_threshold {
            new_config.warning_threshold = warning;
        }
        if let Some(critical) = config.critical_threshold {
            new_config.critical_threshold = critical;
        }
        if let Some(notify) = config.notify_on_threshold {
            new_config.notify_on_threshold = notify;
        }
        if let Some(update_tray) = config.update_tray {
            new_config.update_tray = update_tray;
        }

        state.update_config(new_config);
    }

    let state = polling_state.state.read().await;
    Ok(build_status_response(&state))
}

/// Trigger a manual quota poll
#[tauri::command(rename_all = "snake_case")]
pub async fn trigger_quota_poll(
    app: AppHandle,
    app_state: State<'_, AppState>,
    polling_state: State<'_, QuotaPollingServiceState>,
    token: String,
) -> Result<PollingStatusResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    log::info!("[quota:timer] Manual poll triggered for user {}", claims.sub);

    let state_clone = Arc::clone(&polling_state.state);
    let db_clone = Arc::clone(&app_state.db);

    perform_quota_poll(&state_clone, &db_clone, &claims.sub, &app).await?;

    let state = polling_state.state.read().await;
    Ok(build_status_response(&state))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Build the status response from the current state
fn build_status_response(state: &QuotaPollingState) -> PollingStatusResponse {
    let claude_percent = state.status.current_quotas.get("claude").copied();

    PollingStatusResponse {
        is_running: state.is_running,
        is_polling: state.status.is_polling,
        last_poll_at: state.status.last_poll_at.clone(),
        next_poll_at: state.status.next_poll_at.clone(),
        last_error: state.status.last_error.clone(),
        claude_percent,
        config: state.config.clone().into(),
    }
}

/// Perform a single quota poll
async fn perform_quota_poll(
    state: &SharedPollingState,
    db: &Arc<tokio::sync::Mutex<recap_core::Database>>,
    user_id: &str,
    app: &AppHandle,
) -> Result<(), String> {
    log::debug!("[quota:timer] Performing quota poll for user {}", user_id);

    // Mark poll as starting
    {
        let mut state = state.write().await;
        state.begin_poll();
    }

    // Get config for thresholds
    let (config, mut alert_state) = {
        let state_guard = state.read().await;
        (state_guard.config.clone(), state_guard.alert_state.clone())
    };

    // Create provider and fetch quota
    let provider = ClaudeQuotaProvider::new().with_user_id(user_id);

    if !provider.is_available().await {
        log::debug!("[quota:timer] Claude provider not available");
        let mut state = state.write().await;
        state.complete_poll(Some("Claude provider not available".to_string()));
        return Ok(());
    }

    let snapshots = match provider.fetch_quota().await {
        Ok(s) => s,
        Err(e) => {
            log::error!("[quota:timer] Failed to fetch quota: {}", e);
            let mut state = state.write().await;
            state.complete_poll(Some(e.to_string()));
            return Err(e.to_string());
        }
    };

    if snapshots.is_empty() {
        log::debug!("[quota:timer] No quota data returned");
        let mut state = state.write().await;
        state.complete_poll(None);
        return Ok(());
    }

    // Save to database
    {
        let db_guard = db.lock().await;
        let store = QuotaStore::new(db_guard.pool.clone());
        if let Err(e) = store.save_snapshots(user_id, &snapshots, None).await {
            log::error!("[quota:timer] Failed to save snapshots: {}", e);
        }
    }

    // Find the 5-hour window (most relevant for rate limiting)
    let five_hour = snapshots
        .iter()
        .find(|s| s.window_type.to_string() == "5_hour");

    let claude_percent = five_hour.map(|s| s.used_percent);

    // Update state with current quota
    {
        let mut state_guard = state.write().await;
        if let Some(percent) = claude_percent {
            state_guard.update_quota("claude", percent);
        }
    }

    // Check for threshold crossings and send notifications
    for snapshot in &snapshots {
        let alert = alert_state.should_alert(
            snapshot.provider,
            &snapshot.window_type.to_string(),
            snapshot.used_percent,
            config.warning_threshold,
            config.critical_threshold,
        );

        if let Some(level) = alert {
            if config.notify_on_threshold {
                send_quota_notification(
                    app,
                    level,
                    &snapshot.provider.to_string(),
                    &snapshot.window_type.to_string(),
                    snapshot.used_percent,
                );
            }
        }
    }

    // Update tray if configured
    if config.update_tray {
        if let Err(e) = update_tray_quota(app, claude_percent).await {
            log::warn!("[quota:timer] Failed to update tray: {}", e);
        }
    }

    // Save alert state back
    {
        let mut state_guard = state.write().await;
        state_guard.alert_state = alert_state;
        state_guard.complete_poll(None);
    }

    log::info!(
        "[quota:timer] Poll complete, Claude 5-hour: {:?}%",
        claude_percent
    );

    Ok(())
}

/// Send a quota threshold notification
fn send_quota_notification(
    app: &AppHandle,
    level: recap_core::services::quota::AlertLevel,
    provider: &str,
    window_type: &str,
    percent: f64,
) {
    use tauri_plugin_notification::NotificationExt;

    let (title, body) = match level {
        recap_core::services::quota::AlertLevel::Warning => (
            "API 配額警告",
            format!(
                "{} {} 配額已使用 {:.0}%，請注意使用量",
                provider, window_type, percent
            ),
        ),
        recap_core::services::quota::AlertLevel::Critical => (
            "API 配額緊急",
            format!(
                "{} {} 配額已使用 {:.0}%，即將達到上限！",
                provider, window_type, percent
            ),
        ),
        recap_core::services::quota::AlertLevel::Normal => return, // No notification for normal
    };

    log::info!("[quota:timer] Sending notification: {} - {}", title, body);

    if let Err(e) = app
        .notification()
        .builder()
        .title(title)
        .body(&body)
        .show()
    {
        log::error!("[quota:timer] Failed to send notification: {}", e);
    }
}

/// Update the tray with the current quota
async fn update_tray_quota(app: &AppHandle, claude_percent: Option<f64>) -> Result<(), String> {
    let tray = app
        .tray_by_id("main-tray")
        .ok_or_else(|| "Tray icon not found".to_string())?;

    let title = match claude_percent {
        Some(percent) => format!("{:.0}%", percent),
        None => "—".to_string(),
    };

    tray.set_title(Some(&title)).map_err(|e| e.to_string())?;
    log::debug!("[quota:timer] Updated tray title: {}", title);

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_dto_conversion() {
        let config = QuotaPollingConfig::default();
        let dto: QuotaPollingConfigDto = config.clone().into();

        assert_eq!(dto.enabled, config.enabled);
        assert_eq!(dto.interval_minutes, config.interval_minutes);
        assert_eq!(dto.warning_threshold, config.warning_threshold);
        assert_eq!(dto.critical_threshold, config.critical_threshold);
    }

    #[test]
    fn test_build_status_response() {
        let config = QuotaPollingConfig::default();
        let mut state = QuotaPollingState::new(config);
        state.start();
        state.update_quota("claude", 75.5);

        let response = build_status_response(&state);

        assert!(response.is_running);
        assert!(!response.is_polling);
        assert_eq!(response.claude_percent, Some(75.5));
    }

    #[test]
    fn test_quota_polling_service_state_default() {
        let state = QuotaPollingServiceState::default();
        // Should be able to read the state
        let _config = &state.state;
    }
}
