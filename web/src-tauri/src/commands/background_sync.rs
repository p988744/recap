//! Background Sync Commands
//!
//! Tauri commands for controlling the background sync service.

use super::AppState;
use recap_core::auth::verify_token;
use crate::services::background_sync::{BackgroundSyncConfig, SyncOperationResult, SyncServiceStatus};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State, Window};

/// Progress event payload for sync operations
#[derive(Debug, Clone, Serialize)]
pub struct SyncProgress {
    pub phase: String,           // "sources", "snapshots", "compaction", "complete"
    pub current_source: Option<String>,
    pub current: usize,
    pub total: usize,
    pub message: String,
}

// =============================================================================
// Request/Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct UpdateBackgroundSyncConfigRequest {
    pub enabled: Option<bool>,
    pub interval_minutes: Option<u32>,
    pub sync_git: Option<bool>,
    pub sync_claude: Option<bool>,
    pub sync_antigravity: Option<bool>,
    pub sync_gitlab: Option<bool>,
    pub sync_jira: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct BackgroundSyncConfigResponse {
    pub enabled: bool,
    pub interval_minutes: u32,
    pub sync_git: bool,
    pub sync_claude: bool,
    pub sync_antigravity: bool,
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
            sync_antigravity: config.sync_antigravity,
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
        sync_antigravity: config.sync_antigravity.unwrap_or(current.sync_antigravity),
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

/// Trigger an immediate sync with progress reporting.
/// Emits "sync-progress" events to the frontend.
#[tauri::command]
pub async fn trigger_sync_with_progress(
    state: State<'_, AppState>,
    window: Window,
    token: String,
) -> Result<TriggerSyncResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let user_id = claims.sub.clone();

    // Ensure user ID is set
    state.background_sync.set_user_id(user_id.clone()).await;

    // Helper to emit progress
    let emit = |phase: &str, source: Option<&str>, current: usize, total: usize, message: &str| {
        let _ = window.emit("sync-progress", SyncProgress {
            phase: phase.to_string(),
            current_source: source.map(|s| s.to_string()),
            current,
            total,
            message: message.to_string(),
        });
    };

    // Clone pool immediately
    let pool = {
        let db = state.db.lock().await;
        db.pool.clone()
    };

    let config = state.background_sync.get_config().await;
    let sync_config = config.to_sync_config();

    // Phase 1: Sync all enabled sources
    emit("sources", None, 0, 100, "正在同步資料來源...");

    let sources = recap_core::services::sources::get_enabled_sources(&sync_config).await;
    let total_sources = sources.len();
    let mut results = Vec::new();

    for (idx, source) in sources.iter().enumerate() {
        emit(
            "sources",
            Some(source.display_name()),
            idx + 1,
            total_sources,
            &format!("正在同步 {}...", source.display_name()),
        );

        match source.sync_sessions(&pool, &user_id).await {
            Ok(source_result) => {
                let result = SyncOperationResult::from(source_result);
                log::info!(
                    "{} sync complete: {} items",
                    source.display_name(),
                    result.items_synced
                );
                results.push(result);
            }
            Err(e) => {
                log::error!("{} sync error: {}", source.display_name(), e);
                results.push(SyncOperationResult {
                    source: source.source_name().to_string(),
                    success: false,
                    error: Some(e),
                    ..Default::default()
                });
            }
        }
    }

    // Phase 2: Capture hourly snapshots
    emit("snapshots", None, 0, 100, "正在捕獲快照...");

    if config.sync_claude {
        let projects = recap_core::services::SyncService::discover_project_paths();
        let total_projects = projects.len();
        let mut snapshot_count = 0;

        for (idx, project) in projects.iter().enumerate() {
            emit(
                "snapshots",
                Some(&project.name),
                idx + 1,
                total_projects,
                &format!("捕獲快照: {}", project.name),
            );

            match recap_core::services::snapshot::capture_snapshots_for_project(
                &pool,
                &user_id,
                project,
            )
            .await
            {
                Ok(n) => snapshot_count += n,
                Err(e) => log::warn!("Snapshot capture error for {}: {}", project.name, e),
            }
        }

        if snapshot_count > 0 {
            log::info!("Captured {} hourly snapshots", snapshot_count);
        }
    }

    // Phase 3: Run compaction cycle
    emit("compaction", None, 0, 100, "正在處理摘要...");

    if config.sync_claude {
        let llm = recap_core::services::llm::create_llm_service(&pool, &user_id)
            .await
            .ok();

        // Find uncompacted items count for progress
        let uncompacted: Vec<(String, String)> = sqlx::query_as(
            r#"
            SELECT DISTINCT s.project_path, s.hour_bucket
            FROM snapshot_raw_data s
            LEFT JOIN work_summaries ws ON ws.user_id = s.user_id
                AND ws.project_path = s.project_path
                AND ws.scale = 'hourly'
                AND ws.period_start = s.hour_bucket
            WHERE s.user_id = ? AND ws.id IS NULL
            ORDER BY s.hour_bucket
            "#,
        )
        .bind(&user_id)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

        let total_items = uncompacted.len();

        for (idx, (project_path, hour_bucket)) in uncompacted.iter().enumerate() {
            if idx % 5 == 0 || idx == total_items - 1 {
                emit(
                    "compaction",
                    None,
                    idx + 1,
                    total_items,
                    &format!("處理摘要 ({}/{})", idx + 1, total_items),
                );
            }

            let _ = recap_core::services::compaction::compact_hourly(
                &pool,
                llm.as_ref(),
                &user_id,
                project_path,
                hour_bucket,
            )
            .await;
        }

        // Daily compaction
        emit("compaction", None, 100, 100, "處理每日摘要...");

        match recap_core::services::compaction::run_compaction_cycle(&pool, llm.as_ref(), &user_id).await {
            Ok(cr) => {
                if cr.hourly_compacted > 0 || cr.daily_compacted > 0 {
                    log::info!(
                        "Compaction: {} hourly, {} daily summaries",
                        cr.hourly_compacted,
                        cr.daily_compacted
                    );
                }
            }
            Err(e) => log::warn!("Compaction cycle error: {}", e),
        }
    }

    // Complete
    let total_items: i32 = results.iter().map(|r| r.items_synced).sum();
    emit(
        "complete",
        None,
        100,
        100,
        &format!("同步完成，共處理 {} 筆資料", total_items),
    );

    log::info!("Manual sync with progress triggered, {} items synced", total_items);

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
            sync_antigravity: true,
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
