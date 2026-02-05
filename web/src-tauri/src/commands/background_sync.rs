//! Background Sync Commands
//!
//! Tauri commands for controlling the background sync service.

use super::AppState;
use chrono::Utc;
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
    pub compaction_interval_minutes: Option<u32>,
    pub sync_git: Option<bool>,
    pub sync_claude: Option<bool>,
    pub sync_antigravity: Option<bool>,
    pub sync_gitlab: Option<bool>,
    pub sync_jira: Option<bool>,
    pub auto_generate_summaries: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct BackgroundSyncConfigResponse {
    pub enabled: bool,
    pub interval_minutes: u32,
    pub compaction_interval_minutes: u32,
    pub sync_git: bool,
    pub sync_claude: bool,
    pub sync_antigravity: bool,
    pub sync_gitlab: bool,
    pub sync_jira: bool,
    pub auto_generate_summaries: bool,
}

impl From<BackgroundSyncConfig> for BackgroundSyncConfigResponse {
    fn from(config: BackgroundSyncConfig) -> Self {
        Self {
            enabled: config.enabled,
            interval_minutes: config.interval_minutes,
            compaction_interval_minutes: config.compaction_interval_minutes,
            sync_git: config.sync_git,
            sync_claude: config.sync_claude,
            sync_antigravity: config.sync_antigravity,
            sync_gitlab: config.sync_gitlab,
            sync_jira: config.sync_jira,
            auto_generate_summaries: config.auto_generate_summaries,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BackgroundSyncStatusResponse {
    pub is_running: bool,
    pub is_syncing: bool,
    pub is_compacting: bool,
    pub last_sync_at: Option<String>,
    pub last_compaction_at: Option<String>,
    pub next_sync_at: Option<String>,
    pub next_compaction_at: Option<String>,
    pub last_result: Option<String>,
    pub last_error: Option<String>,
}

impl From<SyncServiceStatus> for BackgroundSyncStatusResponse {
    fn from(status: SyncServiceStatus) -> Self {
        Self {
            is_running: status.is_running,
            is_syncing: status.is_syncing,
            is_compacting: status.is_compacting,
            last_sync_at: status.last_sync_at,
            last_compaction_at: status.last_compaction_at,
            next_sync_at: status.next_sync_at,
            next_compaction_at: status.next_compaction_at,
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
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let user_id = claims.sub;

    // Get current config and apply updates
    let current = state.background_sync.get_config().await;
    let new_config = BackgroundSyncConfig {
        enabled: config.enabled.unwrap_or(current.enabled),
        interval_minutes: config.interval_minutes.unwrap_or(current.interval_minutes),
        compaction_interval_minutes: config.compaction_interval_minutes.unwrap_or(current.compaction_interval_minutes),
        sync_git: config.sync_git.unwrap_or(current.sync_git),
        sync_claude: config.sync_claude.unwrap_or(current.sync_claude),
        sync_antigravity: config.sync_antigravity.unwrap_or(current.sync_antigravity),
        sync_gitlab: config.sync_gitlab.unwrap_or(current.sync_gitlab),
        sync_jira: config.sync_jira.unwrap_or(current.sync_jira),
        auto_generate_summaries: config.auto_generate_summaries.unwrap_or(current.auto_generate_summaries),
    };

    // Validate data sync interval
    if ![5, 15, 30, 60].contains(&new_config.interval_minutes) {
        return Err("資料同步間隔必須是 5, 15, 30 或 60 分鐘".to_string());
    }

    // Validate compaction interval (30min, 1h, 3h, 6h, 12h, 24h)
    if ![30, 60, 180, 360, 720, 1440].contains(&new_config.compaction_interval_minutes) {
        return Err("壓縮間隔必須是 30 分鐘、1、3、6、12 或 24 小時".to_string());
    }

    // Update in-memory config
    state.background_sync.update_config(new_config.clone()).await;

    // Persist to database
    let pool = {
        let db = state.db.lock().await;
        db.pool.clone()
    };

    sqlx::query(
        r#"
        UPDATE users SET
            sync_enabled = ?,
            sync_interval_minutes = ?,
            compaction_interval_minutes = ?,
            auto_generate_summaries = ?,
            sync_git = ?,
            sync_claude = ?,
            sync_antigravity = ?
        WHERE id = ?
        "#
    )
    .bind(new_config.enabled)
    .bind(new_config.interval_minutes)
    .bind(new_config.compaction_interval_minutes)
    .bind(new_config.auto_generate_summaries)
    .bind(new_config.sync_git)
    .bind(new_config.sync_claude)
    .bind(new_config.sync_antigravity)
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to persist sync config: {}", e))?;

    log::info!("Background sync config updated and persisted: {:?}", new_config);

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
    let user_id = claims.sub.clone();

    // Set user ID for sync operations
    state.background_sync.set_user_id(user_id.clone()).await;

    // Load config from database
    let pool = {
        let db = state.db.lock().await;
        db.pool.clone()
    };

    let config_row: Option<(
        Option<bool>,
        Option<i32>,
        Option<i32>,
        Option<bool>,
        Option<bool>,
        Option<bool>,
        Option<bool>,
    )> = sqlx::query_as(
        r#"
        SELECT
            sync_enabled,
            sync_interval_minutes,
            compaction_interval_minutes,
            auto_generate_summaries,
            sync_git,
            sync_claude,
            sync_antigravity
        FROM users WHERE id = ?
        "#
    )
    .bind(&user_id)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();

    if let Some((enabled, interval, compaction, auto_summaries, git, claude, antigravity)) = config_row {
        let config = BackgroundSyncConfig {
            enabled: enabled.unwrap_or(true),
            interval_minutes: interval.unwrap_or(15) as u32,
            compaction_interval_minutes: compaction.unwrap_or(60) as u32,
            auto_generate_summaries: auto_summaries.unwrap_or(true),
            sync_git: git.unwrap_or(true),
            sync_claude: claude.unwrap_or(true),
            sync_antigravity: antigravity.unwrap_or(true),
            sync_gitlab: false,
            sync_jira: false,
        };
        state.background_sync.update_config(config).await;
        log::info!("Loaded sync config from database");
    }

    // Initialize timestamps from database (restore last known sync/compaction times)
    state.background_sync.initialize_timestamps_from_db(&user_id).await;

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
///
/// Uses the service lifecycle to ensure proper state management:
/// 1. Calls begin_sync_operation() at start
/// 2. Calls complete_sync_operation() at end (always, even on error)
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

    // Begin sync operation (lifecycle: Idle -> Syncing)
    state.background_sync.begin_sync_operation().await
        .map_err(|e| e.to_string())?;

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
    log::info!("========== 開始資料壓縮 ==========");
    emit("compaction", None, 0, 100, "正在處理摘要...");

    if config.sync_claude {
        let llm = recap_core::services::llm::create_llm_service(&pool, &user_id)
            .await
            .ok();

        log::info!("LLM 服務: {}", if llm.is_some() { "已啟用" } else { "未設定" });

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
        log::info!("---------- Phase 3a: 小時摘要壓縮 ----------");
        log::info!("待處理: {} 個未壓縮的小時區塊", total_items);

        let mut hourly_success = 0;
        let mut hourly_errors = 0;
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

            match recap_core::services::compaction::compact_hourly(
                &pool,
                llm.as_ref(),
                &user_id,
                project_path,
                hour_bucket,
            )
            .await {
                Ok(_) => {
                    hourly_success += 1;
                    log::debug!("[{}/{}] 壓縮成功: {} @ {}", idx + 1, total_items, project_path, hour_bucket);
                }
                Err(e) => {
                    hourly_errors += 1;
                    log::warn!("[{}/{}] 壓縮失敗: {} @ {} - {}", idx + 1, total_items, project_path, hour_bucket, e);
                }
            }
        }

        if total_items > 0 {
            log::info!("小時摘要壓縮完成: {} 成功, {} 失敗", hourly_success, hourly_errors);
        }

        // Daily compaction
        log::info!("---------- Phase 3b: 每日摘要壓縮 ----------");
        emit("compaction", None, 100, 100, "處理每日摘要...");

        match recap_core::services::compaction::run_compaction_cycle(&pool, llm.as_ref(), &user_id).await {
            Ok(cr) => {
                // 單行摘要 log - 方便事後追蹤每次壓縮紀錄
                let now_local = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                let error_count = cr.errors.len();
                log::info!(
                    "[COMPACTION] {} | 小時:{} 每日:{} 每月:{} 錯誤:{}",
                    now_local,
                    cr.hourly_compacted,
                    cr.daily_compacted,
                    cr.monthly_compacted,
                    error_count
                );
            }
            Err(e) => {
                let now_local = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                log::warn!("[COMPACTION] {} | 錯誤: {}", now_local, e);
            }
        }
    } else {
        log::debug!("Claude 同步未啟用，跳過資料壓縮");
    }

    // Phase 4: Generate timeline summaries for completed periods
    log::info!("---------- Phase 4: 時間軸摘要 ----------");
    if config.auto_generate_summaries {
        emit("summaries", None, 0, 100, "生成時間軸摘要...");

        let time_units = ["week", "month", "quarter", "year"];
        log::info!("處理時間單位: {:?}", time_units);
        match crate::commands::projects::summaries::generate_all_completed_summaries(
            &pool,
            &user_id,
            &time_units,
        )
        .await
        {
            Ok(count) => {
                if count > 0 {
                    emit("summaries", None, 100, 100, &format!("已生成 {} 個時間軸摘要", count));
                    log::info!("時間軸摘要生成完成: {} 個新摘要", count);
                } else {
                    emit("summaries", None, 100, 100, "時間軸摘要已是最新");
                    log::info!("時間軸摘要已是最新，無需生成");
                }
            }
            Err(e) => {
                emit("summaries", None, 100, 100, &format!("摘要生成錯誤: {}", e));
                log::warn!("時間軸摘要生成錯誤: {}", e);
            }
        }
    } else {
        log::info!("自動生成摘要未啟用，跳過時間軸摘要");
    }

    log::debug!("========== 資料壓縮結束 ==========");

    // Record compaction completion (updates last_compaction_at and next_compaction_at)
    if config.sync_claude {
        state.background_sync.record_compaction_completed().await;
    }

    // Complete
    let total_items: i32 = results.iter().map(|r| r.items_synced).sum();
    let total_projects: i32 = results.iter().map(|r| r.projects_scanned).sum();
    let total_created: i32 = results.iter().map(|r| r.items_created).sum();

    // 單行摘要 log - 方便事後追蹤每次手動同步紀錄
    let now_local = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    log::info!(
        "[MANUAL_SYNC] {} | 來源:{} 專案:{} 資料:{} 新增:{}",
        now_local, results.len(), total_projects, total_items, total_created
    );

    emit(
        "complete",
        None,
        100,
        100,
        &format!("同步完成，共處理 {} 筆資料", total_items),
    );

    // Complete sync operation (lifecycle: Syncing -> Idle)
    // This automatically records results and updates last_sync_at
    state.background_sync.complete_sync_operation(&results).await;

    // Persist sync status to database
    let now = Utc::now();
    if let Err(e) = sqlx::query(
        r#"
        UPDATE sync_status
        SET status = 'success',
            last_sync_at = ?,
            last_item_count = ?,
            error_message = NULL,
            updated_at = ?
        WHERE user_id = ?
        "#
    )
    .bind(&now)
    .bind(total_items)
    .bind(&now)
    .bind(&user_id)
    .execute(&pool)
    .await
    {
        log::warn!("Failed to persist sync status to database: {}", e);
    }

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
            compaction_interval_minutes: 30,
            sync_git: true,
            sync_claude: true,
            sync_antigravity: true,
            sync_gitlab: false,
            sync_jira: false,
            auto_generate_summaries: true,
        };

        let response: BackgroundSyncConfigResponse = config.into();
        assert!(response.enabled);
        assert_eq!(response.interval_minutes, 15);
        assert_eq!(response.compaction_interval_minutes, 30);
        assert!(response.sync_git);
        assert!(response.sync_claude);
        assert!(!response.sync_gitlab);
        assert!(!response.sync_jira);
        assert!(response.auto_generate_summaries);
    }

    #[test]
    fn test_status_response_from() {
        let status = SyncServiceStatus {
            is_running: true,
            is_syncing: false,
            is_compacting: false,
            last_sync_at: Some("2026-01-16T12:00:00Z".to_string()),
            last_compaction_at: Some("2026-01-16T10:00:00Z".to_string()),
            next_sync_at: Some("2026-01-16T12:15:00Z".to_string()),
            next_compaction_at: Some("2026-01-16T16:00:00Z".to_string()),
            last_result: Some("成功同步 5 筆項目".to_string()),
            last_error: None,
        };

        let response: BackgroundSyncStatusResponse = status.into();
        assert!(response.is_running);
        assert!(!response.is_syncing);
        assert!(!response.is_compacting);
        assert_eq!(response.last_sync_at, Some("2026-01-16T12:00:00Z".to_string()));
        assert_eq!(response.last_compaction_at, Some("2026-01-16T10:00:00Z".to_string()));
        assert_eq!(response.next_compaction_at, Some("2026-01-16T16:00:00Z".to_string()));
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
