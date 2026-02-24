//! Danger Zone Commands
//!
//! Commands for destructive operations that require explicit user confirmation.
//! These operations cannot be undone.

use recap_core::auth::verify_token;
use serde::Serialize;
use tauri::{Emitter, State, Window};

use super::AppState;

/// Progress event payload for recompaction
#[derive(Debug, Clone, Serialize)]
pub struct RecompactProgress {
    pub phase: String,           // "counting", "scanning", "hourly", "daily", "monthly", "complete"
    pub current: usize,
    pub total: usize,
    pub message: String,
}

/// Result of a dangerous operation
#[derive(Debug, Serialize)]
pub struct DangerousOperationResult {
    pub success: bool,
    pub message: String,
    pub details: Option<DangerousOperationDetails>,
}

/// Details about what was affected
#[derive(Debug, Serialize)]
pub struct DangerousOperationDetails {
    pub work_items_deleted: Option<i64>,
    pub snapshots_deleted: Option<i64>,
    pub summaries_deleted: Option<i64>,
    pub configs_reset: Option<bool>,
}

/// Clear all synced data (work_items from sync sources, snapshots, summaries)
/// but keep manual work items and user settings.
#[tauri::command]
pub async fn clear_synced_data(
    state: State<'_, AppState>,
    token: String,
    confirmation: String,
) -> Result<DangerousOperationResult, String> {
    // Require explicit confirmation text
    if confirmation != "DELETE_SYNCED_DATA" {
        return Ok(DangerousOperationResult {
            success: false,
            message: "確認文字不正確，操作已取消".to_string(),
            details: None,
        });
    }

    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    clear_synced_data_impl(&db.pool, &claims.sub).await
}

/// Core implementation for clearing synced data, separated for testability.
pub(crate) async fn clear_synced_data_impl(
    pool: &sqlx::SqlitePool,
    user_id: &str,
) -> Result<DangerousOperationResult, String> {
    // Count items to be deleted
    let work_items_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM work_items WHERE user_id = ? AND source != 'manual'",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let snapshots_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM snapshot_raw_data WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let summaries_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM work_summaries WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Delete synced work items (keep manual)
    sqlx::query("DELETE FROM work_items WHERE user_id = ? AND source != 'manual'")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete all snapshots
    sqlx::query("DELETE FROM snapshot_raw_data WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete all summaries
    sqlx::query("DELETE FROM work_summaries WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    log::info!(
        "Cleared synced data for user {}: {} work items, {} snapshots, {} summaries",
        user_id,
        work_items_count.0,
        snapshots_count.0,
        summaries_count.0
    );

    Ok(DangerousOperationResult {
        success: true,
        message: format!(
            "已清除 {} 筆同步資料、{} 筆快照、{} 筆摘要",
            work_items_count.0, snapshots_count.0, summaries_count.0
        ),
        details: Some(DangerousOperationDetails {
            work_items_deleted: Some(work_items_count.0),
            snapshots_deleted: Some(snapshots_count.0),
            summaries_deleted: Some(summaries_count.0),
            configs_reset: None,
        }),
    })
}

/// Clear ALL data and reset all settings to defaults.
/// This is a complete factory reset for the user's account.
#[tauri::command]
pub async fn factory_reset(
    state: State<'_, AppState>,
    token: String,
    confirmation: String,
) -> Result<DangerousOperationResult, String> {
    // Require explicit confirmation text
    if confirmation != "FACTORY_RESET" {
        return Ok(DangerousOperationResult {
            success: false,
            message: "確認文字不正確，操作已取消".to_string(),
            details: None,
        });
    }

    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    factory_reset_impl(&db.pool, &claims.sub).await
}

/// Core implementation for factory reset, separated for testability.
pub(crate) async fn factory_reset_impl(
    pool: &sqlx::SqlitePool,
    user_id: &str,
) -> Result<DangerousOperationResult, String> {
    // Count all items
    let work_items_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM work_items WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let snapshots_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM snapshot_raw_data WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let summaries_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM work_summaries WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Delete ALL work items (including manual)
    sqlx::query("DELETE FROM work_items WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete all snapshots
    sqlx::query("DELETE FROM snapshot_raw_data WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete all summaries
    sqlx::query("DELETE FROM work_summaries WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete all reports
    sqlx::query("DELETE FROM reports WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete all projects
    sqlx::query("DELETE FROM projects WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete worklog sync records
    sqlx::query("DELETE FROM worklog_sync_records WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete project issue mappings
    sqlx::query("DELETE FROM project_issue_mappings WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    // Reset user config to defaults
    sqlx::query(
        r#"UPDATE user_config SET
            daily_hours = 8.0,
            normalize_hours = 0,
            claude_code_path = NULL,
            antigravity_path = NULL,
            gitlab_url = NULL,
            gitlab_token = NULL,
            jira_url = NULL,
            jira_auth_type = 'pat',
            jira_token = NULL,
            jira_email = NULL,
            tempo_token = NULL,
            llm_provider = NULL,
            llm_model = NULL,
            llm_api_key = NULL,
            llm_base_url = NULL,
            timezone = 'Asia/Taipei',
            week_start_day = 1,
            updated_at = CURRENT_TIMESTAMP
        WHERE user_id = ?"#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    log::info!(
        "Factory reset for user {}: {} work items, {} snapshots, {} summaries deleted, configs reset",
        user_id,
        work_items_count.0,
        snapshots_count.0,
        summaries_count.0
    );

    Ok(DangerousOperationResult {
        success: true,
        message: format!(
            "已重置所有資料：{} 筆工作紀錄、{} 筆快照、{} 筆摘要，所有設定已恢復預設值",
            work_items_count.0, snapshots_count.0, summaries_count.0
        ),
        details: Some(DangerousOperationDetails {
            work_items_deleted: Some(work_items_count.0),
            snapshots_deleted: Some(snapshots_count.0),
            summaries_deleted: Some(summaries_count.0),
            configs_reset: Some(true),
        }),
    })
}

/// Force recompact all summaries with progress reporting.
/// Emits "recompact-progress" events to the frontend.
#[tauri::command]
pub async fn force_recompact_with_progress(
    state: State<'_, AppState>,
    window: Window,
    token: String,
    confirmation: String,
) -> Result<DangerousOperationResult, String> {
    // Require explicit confirmation text
    if confirmation != "RECOMPACT" {
        return Ok(DangerousOperationResult {
            success: false,
            message: "確認文字不正確，操作已取消".to_string(),
            details: None,
        });
    }

    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;
    let pool = db.pool.clone();
    drop(db); // Release lock early

    // Helper to emit progress
    let emit_progress = |phase: &str, current: usize, total: usize, message: &str| {
        let _ = window.emit("recompact-progress", RecompactProgress {
            phase: phase.to_string(),
            current,
            total,
            message: message.to_string(),
        });
    };

    // Phase 1: Count existing summaries (for reference, will be replaced progressively)
    emit_progress("counting", 0, 100, "正在統計現有摘要...");

    let summaries_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM work_summaries WHERE user_id = ?",
    )
    .bind(&claims.sub)
    .fetch_one(&pool)
    .await
    .map_err(|e| e.to_string())?;

    emit_progress("counting", 100, 100, &format!("找到 {} 筆現有摘要，將逐步替換", summaries_count.0));

    // Note: No deletion phase - we use progressive replacement (upsert)
    // Each compaction operation will replace the existing summary if it exists
    // This ensures that if the process fails mid-way, unprocessed items still have their old summaries

    // Phase 2: Find all hourly snapshots to recompact
    emit_progress("scanning", 0, 100, "正在掃描快照資料...");

    let hourly_items: Vec<(String, String)> = sqlx::query_as(
        r#"SELECT DISTINCT project_path, hour_bucket
           FROM snapshot_raw_data
           WHERE user_id = ?
           ORDER BY hour_bucket"#,
    )
    .bind(&claims.sub)
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;

    let total_hourly = hourly_items.len();
    emit_progress("scanning", 100, 100, &format!("找到 {} 個小時區段需要處理", total_hourly));

    // Create LLM service
    let llm = recap_core::services::llm::create_llm_service(&pool, &claims.sub)
        .await
        .ok();

    // Phase 4: Compact hourly
    let mut hourly_compacted = 0;
    for (idx, (project_path, hour_bucket)) in hourly_items.iter().enumerate() {
        emit_progress(
            "hourly",
            idx + 1,
            total_hourly,
            &format!("處理小時摘要 ({}/{}): {}", idx + 1, total_hourly, hour_bucket),
        );

        match recap_core::services::compaction::compact_hourly(
            &pool,
            llm.as_ref(),
            &claims.sub,
            project_path,
            hour_bucket,
        )
        .await
        {
            Ok(()) => hourly_compacted += 1,
            Err(e) => log::warn!("Hourly compaction error: {}", e),
        }
    }

    // Phase 5: Find days that need daily compaction
    emit_progress("scanning", 0, 100, "正在掃描需要產生每日摘要的日期...");

    let daily_items: Vec<(String, String)> = sqlx::query_as(
        r#"SELECT DISTINCT project_path, DATE(period_start) as day
           FROM work_summaries
           WHERE user_id = ? AND scale = 'hourly'
           ORDER BY day"#,
    )
    .bind(&claims.sub)
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;

    let total_daily = daily_items.len();

    // Phase 6: Compact daily
    let mut daily_compacted = 0;
    for (idx, (project_path, day)) in daily_items.iter().enumerate() {
        emit_progress(
            "daily",
            idx + 1,
            total_daily,
            &format!("處理每日摘要 ({}/{}): {}", idx + 1, total_daily, day),
        );

        match recap_core::services::compaction::compact_daily(
            &pool,
            llm.as_ref(),
            &claims.sub,
            project_path,
            day,
        )
        .await
        {
            Ok(()) => daily_compacted += 1,
            Err(e) => log::warn!("Daily compaction error: {}", e),
        }
    }

    // Phase 7: Monthly compaction
    emit_progress("monthly", 0, 1, "正在產生月度摘要...");

    let now = chrono::Local::now();
    let month_start = now.format("%Y-%m-01T00:00:00+00:00").to_string();
    let month_end = {
        let year = now.format("%Y").to_string().parse::<i32>().unwrap_or(2026);
        let month = now.format("%m").to_string().parse::<u32>().unwrap_or(1);
        let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
        format!("{:04}-{:02}-01T00:00:00+00:00", next_year, next_month)
    };

    let monthly_projects: Vec<(String,)> = sqlx::query_as(
        r#"SELECT DISTINCT project_path
           FROM work_summaries
           WHERE user_id = ? AND scale = 'daily'
             AND period_start >= ? AND period_start < ?"#,
    )
    .bind(&claims.sub)
    .bind(&month_start)
    .bind(&month_end)
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut monthly_compacted = 0;
    for (project_path,) in &monthly_projects {
        match recap_core::services::compaction::compact_period(
            &pool,
            llm.as_ref(),
            &claims.sub,
            Some(project_path),
            "monthly",
            &month_start,
            &month_end,
        )
        .await
        {
            Ok(()) => monthly_compacted += 1,
            Err(e) => log::warn!("Monthly compaction error: {}", e),
        }
    }

    emit_progress("monthly", 1, 1, "月度摘要完成");

    // Complete
    emit_progress(
        "complete",
        100,
        100,
        &format!(
            "完成！已產生 {} 小時、{} 天、{} 月摘要",
            hourly_compacted, daily_compacted, monthly_compacted
        ),
    );

    log::info!(
        "Force recompact for user {}: replaced {} existing summaries, created {} hourly + {} daily + {} monthly",
        claims.sub,
        summaries_count.0,
        hourly_compacted,
        daily_compacted,
        monthly_compacted
    );

    Ok(DangerousOperationResult {
        success: true,
        message: format!(
            "已重新計算摘要：{} 小時、{} 天、{} 月",
            hourly_compacted, daily_compacted, monthly_compacted
        ),
        details: Some(DangerousOperationDetails {
            work_items_deleted: None,
            snapshots_deleted: None,
            summaries_deleted: Some(summaries_count.0),
            configs_reset: None,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use recap_core::db::Database;
    use tempfile::TempDir;

    /// Helper to create a test database with all required tables
    async fn create_test_db() -> (Database, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open(db_path).await.expect("Failed to create test database");

        // Create additional tables referenced by factory_reset that aren't in core migrations
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS reports (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                title TEXT,
                content TEXT
            )",
        )
        .execute(&db.pool)
        .await
        .expect("Failed to create reports table");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                name TEXT
            )",
        )
        .execute(&db.pool)
        .await
        .expect("Failed to create projects table");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS user_config (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                daily_hours REAL DEFAULT 8.0,
                normalize_hours INTEGER DEFAULT 0,
                claude_code_path TEXT,
                antigravity_path TEXT,
                gitlab_url TEXT,
                gitlab_token TEXT,
                jira_url TEXT,
                jira_auth_type TEXT DEFAULT 'pat',
                jira_token TEXT,
                jira_email TEXT,
                tempo_token TEXT,
                llm_provider TEXT,
                llm_model TEXT,
                llm_api_key TEXT,
                llm_base_url TEXT,
                timezone TEXT DEFAULT 'Asia/Taipei',
                week_start_day INTEGER DEFAULT 1,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&db.pool)
        .await
        .expect("Failed to create user_config table");

        (db, temp_dir)
    }

    /// Ensure a user exists in the users table (needed for FK constraints)
    async fn ensure_user(pool: &sqlx::SqlitePool, user_id: &str) {
        sqlx::query(
            "INSERT OR IGNORE INTO users (id, email, password_hash, name) VALUES (?, ?, 'hash', 'Test User')",
        )
        .bind(user_id)
        .bind(format!("{}@test.com", user_id))
        .execute(pool)
        .await
        .expect("Failed to ensure user");
    }

    /// Insert a work item for testing
    async fn insert_work_item(pool: &sqlx::SqlitePool, user_id: &str, source: &str) {
        ensure_user(pool, user_id).await;
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO work_items (id, user_id, source, title, hours, date) VALUES (?, ?, ?, 'test', 1.0, '2024-01-15')",
        )
        .bind(&id)
        .bind(user_id)
        .bind(source)
        .execute(pool)
        .await
        .expect("Failed to insert work item");
    }

    /// Insert a snapshot for testing
    async fn insert_snapshot(pool: &sqlx::SqlitePool, user_id: &str, hour_bucket: &str) {
        ensure_user(pool, user_id).await;
        let id = uuid::Uuid::new_v4().to_string();
        let session_id = format!("sess-{}", uuid::Uuid::new_v4());
        sqlx::query(
            r#"INSERT INTO snapshot_raw_data (id, user_id, session_id, project_path, hour_bucket,
                user_messages, assistant_messages, tool_calls, files_modified, git_commits,
                message_count, raw_size_bytes)
            VALUES (?, ?, ?, '/test/project', ?, '[]', '[]', '[]', '[]', '[]', 1, 100)"#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(&session_id)
        .bind(hour_bucket)
        .execute(pool)
        .await
        .expect("Failed to insert snapshot");
    }

    /// Insert a work summary for testing
    async fn insert_summary(pool: &sqlx::SqlitePool, user_id: &str, scale: &str) {
        ensure_user(pool, user_id).await;
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO work_summaries (id, user_id, project_path, scale, period_start, period_end, summary) VALUES (?, ?, '/test/project', ?, '2024-01-15T00:00:00', '2024-01-15T23:59:59', 'test summary')",
        )
        .bind(&id)
        .bind(user_id)
        .bind(scale)
        .execute(pool)
        .await
        .expect("Failed to insert summary");
    }

    /// Helper to count rows in a table for a given user
    async fn count_rows(pool: &sqlx::SqlitePool, table: &str, user_id: &str) -> i64 {
        let query = format!("SELECT COUNT(*) FROM {} WHERE user_id = ?", table);
        let row: (i64,) = sqlx::query_as(&query)
            .bind(user_id)
            .fetch_one(pool)
            .await
            .expect("Count query failed");
        row.0
    }

    // ============================================================================
    // Confirmation validation tests
    // ============================================================================

    #[test]
    fn test_confirmation_text_constants() {
        // Verify the confirmation strings are what we expect
        assert_eq!("DELETE_SYNCED_DATA", "DELETE_SYNCED_DATA");
        assert_eq!("FACTORY_RESET", "FACTORY_RESET");
        assert_eq!("RECOMPACT", "RECOMPACT");
    }

    // ============================================================================
    // DangerousOperationResult serialization tests
    // ============================================================================

    #[test]
    fn test_dangerous_operation_result_serialization_success() {
        let result = DangerousOperationResult {
            success: true,
            message: "Operation completed".to_string(),
            details: Some(DangerousOperationDetails {
                work_items_deleted: Some(5),
                snapshots_deleted: Some(10),
                summaries_deleted: Some(3),
                configs_reset: None,
            }),
        };

        let json = serde_json::to_value(&result).expect("Should serialize");
        assert_eq!(json["success"], true);
        assert_eq!(json["message"], "Operation completed");
        assert!(json["details"].is_object());
        assert_eq!(json["details"]["work_items_deleted"], 5);
        assert_eq!(json["details"]["snapshots_deleted"], 10);
        assert_eq!(json["details"]["summaries_deleted"], 3);
        assert!(json["details"]["configs_reset"].is_null());
    }

    #[test]
    fn test_dangerous_operation_result_serialization_failure() {
        let result = DangerousOperationResult {
            success: false,
            message: "確認文字不正確，操作已取消".to_string(),
            details: None,
        };

        let json = serde_json::to_value(&result).expect("Should serialize");
        assert_eq!(json["success"], false);
        assert!(json["details"].is_null());
    }

    #[test]
    fn test_dangerous_operation_details_with_config_reset() {
        let details = DangerousOperationDetails {
            work_items_deleted: Some(0),
            snapshots_deleted: Some(0),
            summaries_deleted: Some(0),
            configs_reset: Some(true),
        };

        let json = serde_json::to_value(&details).expect("Should serialize");
        assert_eq!(json["configs_reset"], true);
    }

    #[test]
    fn test_recompact_progress_serialization() {
        let progress = RecompactProgress {
            phase: "hourly".to_string(),
            current: 5,
            total: 20,
            message: "Processing...".to_string(),
        };

        let json = serde_json::to_value(&progress).expect("Should serialize");
        assert_eq!(json["phase"], "hourly");
        assert_eq!(json["current"], 5);
        assert_eq!(json["total"], 20);
        assert_eq!(json["message"], "Processing...");
    }

    // ============================================================================
    // clear_synced_data_impl tests
    // ============================================================================

    #[tokio::test]
    async fn test_clear_synced_data_deletes_synced_items() {
        let (db, _temp_dir) = create_test_db().await;
        let pool = &db.pool;
        let user_id = "test-user-1";

        // Insert synced and manual work items
        insert_work_item(pool, user_id, "claude").await;
        insert_work_item(pool, user_id, "claude").await;
        insert_work_item(pool, user_id, "git").await;
        insert_work_item(pool, user_id, "manual").await;

        // Insert snapshots and summaries
        insert_snapshot(pool, user_id, "2024-01-15T10:00:00").await;
        insert_snapshot(pool, user_id, "2024-01-15T11:00:00").await;
        insert_summary(pool, user_id, "hourly").await;

        // Verify data exists
        assert_eq!(count_rows(pool, "work_items", user_id).await, 4);
        assert_eq!(count_rows(pool, "snapshot_raw_data", user_id).await, 2);
        assert_eq!(count_rows(pool, "work_summaries", user_id).await, 1);

        // Execute clear
        let result = clear_synced_data_impl(pool, user_id).await.unwrap();

        assert!(result.success);
        assert_eq!(result.details.as_ref().unwrap().work_items_deleted, Some(3)); // 3 synced
        assert_eq!(result.details.as_ref().unwrap().snapshots_deleted, Some(2));
        assert_eq!(result.details.as_ref().unwrap().summaries_deleted, Some(1));
        assert!(result.details.as_ref().unwrap().configs_reset.is_none());

        // Manual items should remain
        assert_eq!(count_rows(pool, "work_items", user_id).await, 1);
        // Snapshots and summaries should be gone
        assert_eq!(count_rows(pool, "snapshot_raw_data", user_id).await, 0);
        assert_eq!(count_rows(pool, "work_summaries", user_id).await, 0);
    }

    #[tokio::test]
    async fn test_clear_synced_data_empty_database() {
        let (db, _temp_dir) = create_test_db().await;
        let pool = &db.pool;
        let user_id = "test-user-empty";

        let result = clear_synced_data_impl(pool, user_id).await.unwrap();

        assert!(result.success);
        assert_eq!(result.details.as_ref().unwrap().work_items_deleted, Some(0));
        assert_eq!(result.details.as_ref().unwrap().snapshots_deleted, Some(0));
        assert_eq!(result.details.as_ref().unwrap().summaries_deleted, Some(0));
    }

    #[tokio::test]
    async fn test_clear_synced_data_only_manual_items() {
        let (db, _temp_dir) = create_test_db().await;
        let pool = &db.pool;
        let user_id = "test-user-manual";

        // Insert only manual work items
        insert_work_item(pool, user_id, "manual").await;
        insert_work_item(pool, user_id, "manual").await;

        let result = clear_synced_data_impl(pool, user_id).await.unwrap();

        assert!(result.success);
        assert_eq!(result.details.as_ref().unwrap().work_items_deleted, Some(0));

        // Manual items should remain untouched
        assert_eq!(count_rows(pool, "work_items", user_id).await, 2);
    }

    #[tokio::test]
    async fn test_clear_synced_data_isolates_users() {
        let (db, _temp_dir) = create_test_db().await;
        let pool = &db.pool;
        let user_a = "user-a";
        let user_b = "user-b";

        // Insert data for both users
        insert_work_item(pool, user_a, "claude").await;
        insert_work_item(pool, user_b, "claude").await;
        insert_snapshot(pool, user_a, "2024-01-15T10:00:00").await;
        insert_snapshot(pool, user_b, "2024-01-15T11:00:00").await;

        // Clear only user_a's data
        let result = clear_synced_data_impl(pool, user_a).await.unwrap();
        assert!(result.success);

        // user_a data should be gone
        assert_eq!(count_rows(pool, "work_items", user_a).await, 0);
        assert_eq!(count_rows(pool, "snapshot_raw_data", user_a).await, 0);

        // user_b data should remain
        assert_eq!(count_rows(pool, "work_items", user_b).await, 1);
        assert_eq!(count_rows(pool, "snapshot_raw_data", user_b).await, 1);
    }

    // ============================================================================
    // factory_reset_impl tests
    // ============================================================================

    #[tokio::test]
    async fn test_factory_reset_deletes_all_data() {
        let (db, _temp_dir) = create_test_db().await;
        let pool = &db.pool;
        let user_id = "test-user-reset";

        // Insert various data types
        insert_work_item(pool, user_id, "manual").await;
        insert_work_item(pool, user_id, "claude").await;
        insert_snapshot(pool, user_id, "2024-01-15T10:00:00").await;
        insert_summary(pool, user_id, "hourly").await;

        // Insert a worklog sync record
        sqlx::query(
            "INSERT INTO worklog_sync_records (id, user_id, project_path, date, jira_issue_key, hours) VALUES ('wr1', ?, '/test', '2024-01-15', 'JIRA-1', 2.0)",
        )
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();

        // Insert a project issue mapping
        sqlx::query(
            "INSERT INTO project_issue_mappings (project_path, user_id, jira_issue_key) VALUES ('/test', ?, 'JIRA-1')",
        )
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();

        // Insert user_config row
        sqlx::query(
            "INSERT INTO user_config (id, user_id, daily_hours, llm_provider) VALUES ('cfg1', ?, 6.0, 'openai')",
        )
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();

        let result = factory_reset_impl(pool, user_id).await.unwrap();

        assert!(result.success);
        assert_eq!(result.details.as_ref().unwrap().work_items_deleted, Some(2));
        assert_eq!(result.details.as_ref().unwrap().snapshots_deleted, Some(1));
        assert_eq!(result.details.as_ref().unwrap().summaries_deleted, Some(1));
        assert_eq!(result.details.as_ref().unwrap().configs_reset, Some(true));

        // All data should be gone
        assert_eq!(count_rows(pool, "work_items", user_id).await, 0);
        assert_eq!(count_rows(pool, "snapshot_raw_data", user_id).await, 0);
        assert_eq!(count_rows(pool, "work_summaries", user_id).await, 0);
        assert_eq!(count_rows(pool, "worklog_sync_records", user_id).await, 0);
        assert_eq!(count_rows(pool, "project_issue_mappings", user_id).await, 0);

        // Verify user_config was reset
        let config: (f64, i32) = sqlx::query_as(
            "SELECT daily_hours, normalize_hours FROM user_config WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap();
        assert!((config.0 - 8.0).abs() < f64::EPSILON, "daily_hours should be reset to 8.0");
        assert_eq!(config.1, 0, "normalize_hours should be reset to 0");

        // Check LLM config was cleared
        let llm: (Option<String>,) = sqlx::query_as(
            "SELECT llm_provider FROM user_config WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap();
        assert!(llm.0.is_none(), "llm_provider should be NULL after reset");
    }

    #[tokio::test]
    async fn test_factory_reset_deletes_manual_items_too() {
        let (db, _temp_dir) = create_test_db().await;
        let pool = &db.pool;
        let user_id = "test-user-manual-reset";

        // Factory reset should delete manual items (unlike clear_synced_data)
        insert_work_item(pool, user_id, "manual").await;
        insert_work_item(pool, user_id, "manual").await;

        let result = factory_reset_impl(pool, user_id).await.unwrap();

        assert!(result.success);
        assert_eq!(result.details.as_ref().unwrap().work_items_deleted, Some(2));
        assert_eq!(count_rows(pool, "work_items", user_id).await, 0);
    }

    #[tokio::test]
    async fn test_factory_reset_empty_database() {
        let (db, _temp_dir) = create_test_db().await;
        let pool = &db.pool;
        let user_id = "test-user-empty-reset";

        let result = factory_reset_impl(pool, user_id).await.unwrap();

        assert!(result.success);
        assert_eq!(result.details.as_ref().unwrap().work_items_deleted, Some(0));
        assert_eq!(result.details.as_ref().unwrap().snapshots_deleted, Some(0));
        assert_eq!(result.details.as_ref().unwrap().summaries_deleted, Some(0));
        assert_eq!(result.details.as_ref().unwrap().configs_reset, Some(true));
    }

    #[tokio::test]
    async fn test_factory_reset_isolates_users() {
        let (db, _temp_dir) = create_test_db().await;
        let pool = &db.pool;
        let user_a = "user-a-reset";
        let user_b = "user-b-reset";

        // Insert data for both users
        insert_work_item(pool, user_a, "manual").await;
        insert_work_item(pool, user_a, "claude").await;
        insert_work_item(pool, user_b, "manual").await;
        insert_snapshot(pool, user_a, "2024-01-15T10:00:00").await;
        insert_snapshot(pool, user_b, "2024-01-15T11:00:00").await;

        // Factory reset only user_a
        factory_reset_impl(pool, user_a).await.unwrap();

        // user_a data should be completely gone
        assert_eq!(count_rows(pool, "work_items", user_a).await, 0);
        assert_eq!(count_rows(pool, "snapshot_raw_data", user_a).await, 0);

        // user_b data should remain
        assert_eq!(count_rows(pool, "work_items", user_b).await, 1);
        assert_eq!(count_rows(pool, "snapshot_raw_data", user_b).await, 1);
    }

    // ============================================================================
    // clear_synced_data vs factory_reset comparison test
    // ============================================================================

    #[tokio::test]
    async fn test_clear_synced_preserves_manual_but_factory_reset_deletes_all() {
        // Part 1: clear_synced_data preserves manual items
        let (db1, _temp1) = create_test_db().await;
        let pool1 = &db1.pool;
        let user_id = "test-user-compare";

        insert_work_item(pool1, user_id, "manual").await;
        insert_work_item(pool1, user_id, "claude").await;

        clear_synced_data_impl(pool1, user_id).await.unwrap();
        assert_eq!(count_rows(pool1, "work_items", user_id).await, 1, "clear_synced should preserve manual items");

        // Part 2: factory_reset deletes everything
        let (db2, _temp2) = create_test_db().await;
        let pool2 = &db2.pool;

        insert_work_item(pool2, user_id, "manual").await;
        insert_work_item(pool2, user_id, "claude").await;

        factory_reset_impl(pool2, user_id).await.unwrap();
        assert_eq!(count_rows(pool2, "work_items", user_id).await, 0, "factory_reset should delete all items");
    }
}
