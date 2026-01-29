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
    pub phase: String,           // "deleting", "hourly", "daily", "monthly", "complete"
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

    // Count items to be deleted
    let work_items_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM work_items WHERE user_id = ? AND source != 'manual'",
    )
    .bind(&claims.sub)
    .fetch_one(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let snapshots_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM snapshot_raw_data WHERE user_id = ?",
    )
    .bind(&claims.sub)
    .fetch_one(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let summaries_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM work_summaries WHERE user_id = ?",
    )
    .bind(&claims.sub)
    .fetch_one(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // Delete synced work items (keep manual)
    sqlx::query("DELETE FROM work_items WHERE user_id = ? AND source != 'manual'")
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete all snapshots
    sqlx::query("DELETE FROM snapshot_raw_data WHERE user_id = ?")
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete all summaries
    sqlx::query("DELETE FROM work_summaries WHERE user_id = ?")
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    log::info!(
        "Cleared synced data for user {}: {} work items, {} snapshots, {} summaries",
        claims.sub,
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

    // Count all items
    let work_items_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM work_items WHERE user_id = ?",
    )
    .bind(&claims.sub)
    .fetch_one(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let snapshots_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM snapshot_raw_data WHERE user_id = ?",
    )
    .bind(&claims.sub)
    .fetch_one(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let summaries_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM work_summaries WHERE user_id = ?",
    )
    .bind(&claims.sub)
    .fetch_one(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // Delete ALL work items (including manual)
    sqlx::query("DELETE FROM work_items WHERE user_id = ?")
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete all snapshots
    sqlx::query("DELETE FROM snapshot_raw_data WHERE user_id = ?")
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete all summaries
    sqlx::query("DELETE FROM work_summaries WHERE user_id = ?")
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete all reports
    sqlx::query("DELETE FROM reports WHERE user_id = ?")
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete all projects
    sqlx::query("DELETE FROM projects WHERE user_id = ?")
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete worklog sync records
    sqlx::query("DELETE FROM worklog_sync_records WHERE user_id = ?")
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete project issue mappings
    sqlx::query("DELETE FROM project_issue_mappings WHERE user_id = ?")
        .bind(&claims.sub)
        .execute(&db.pool)
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
    .bind(&claims.sub)
    .execute(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    log::info!(
        "Factory reset for user {}: {} work items, {} snapshots, {} summaries deleted, configs reset",
        claims.sub,
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

    // Phase 1: Count existing summaries
    emit_progress("counting", 0, 100, "正在統計現有摘要...");

    let summaries_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM work_summaries WHERE user_id = ?",
    )
    .bind(&claims.sub)
    .fetch_one(&pool)
    .await
    .map_err(|e| e.to_string())?;

    // Phase 2: Delete existing summaries
    emit_progress("deleting", 0, 100, &format!("正在刪除 {} 筆現有摘要...", summaries_count.0));

    sqlx::query("DELETE FROM work_summaries WHERE user_id = ?")
        .bind(&claims.sub)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

    emit_progress("deleting", 100, 100, "已刪除現有摘要");

    // Phase 3: Find all uncompacted hourly snapshots
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
        "Force recompact for user {}: deleted {} summaries, created {} hourly + {} daily + {} monthly",
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
