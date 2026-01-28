//! LLM Usage Tauri Commands
//!
//! Provides commands for querying LLM token usage statistics and logs.

use recap_core::auth::verify_token;
use recap_core::services::llm_usage;
use serde::Serialize;
use tauri::State;

use super::AppState;

/// Response for usage stats
#[derive(Debug, Serialize)]
pub struct LlmUsageStatsResponse {
    pub total_calls: i64,
    pub success_calls: i64,
    pub error_calls: i64,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub avg_duration_ms: f64,
    pub avg_tokens_per_call: f64,
}

/// Response for daily usage
#[derive(Debug, Serialize)]
pub struct DailyUsageResponse {
    pub date: String,
    pub calls: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub cost: f64,
}

/// Response for model usage
#[derive(Debug, Serialize)]
pub struct ModelUsageResponse {
    pub provider: String,
    pub model: String,
    pub calls: i64,
    pub total_tokens: i64,
    pub cost: f64,
}

/// Response for usage log entry
#[derive(Debug, Serialize)]
pub struct LlmUsageLogResponse {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub estimated_cost: Option<f64>,
    pub purpose: String,
    pub duration_ms: Option<i64>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: String,
}

/// Get aggregated LLM usage statistics for a date range.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_llm_usage_stats(
    state: State<'_, AppState>,
    token: String,
    start_date: String,
    end_date: String,
) -> Result<LlmUsageStatsResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let stats = llm_usage::get_usage_stats(&db.pool, &claims.sub, &start_date, &end_date).await?;

    Ok(LlmUsageStatsResponse {
        total_calls: stats.total_calls,
        success_calls: stats.success_calls,
        error_calls: stats.error_calls,
        total_prompt_tokens: stats.total_prompt_tokens,
        total_completion_tokens: stats.total_completion_tokens,
        total_tokens: stats.total_tokens,
        total_cost: stats.total_cost,
        avg_duration_ms: stats.avg_duration_ms,
        avg_tokens_per_call: stats.avg_tokens_per_call,
    })
}

/// Get daily LLM usage breakdown for a date range.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_llm_usage_daily(
    state: State<'_, AppState>,
    token: String,
    start_date: String,
    end_date: String,
) -> Result<Vec<DailyUsageResponse>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let daily = llm_usage::get_usage_by_day(&db.pool, &claims.sub, &start_date, &end_date).await?;

    Ok(daily
        .into_iter()
        .map(|d| DailyUsageResponse {
            date: d.date,
            calls: d.calls,
            prompt_tokens: d.prompt_tokens,
            completion_tokens: d.completion_tokens,
            total_tokens: d.total_tokens,
            cost: d.cost,
        })
        .collect())
}

/// Get LLM usage breakdown by model for a date range.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_llm_usage_by_model(
    state: State<'_, AppState>,
    token: String,
    start_date: String,
    end_date: String,
) -> Result<Vec<ModelUsageResponse>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let models = llm_usage::get_usage_by_model(&db.pool, &claims.sub, &start_date, &end_date).await?;

    Ok(models
        .into_iter()
        .map(|m| ModelUsageResponse {
            provider: m.provider,
            model: m.model,
            calls: m.calls,
            total_tokens: m.total_tokens,
            cost: m.cost,
        })
        .collect())
}

/// Get paginated LLM usage logs for a date range.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_llm_usage_logs(
    state: State<'_, AppState>,
    token: String,
    start_date: String,
    end_date: String,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<LlmUsageLogResponse>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let logs = llm_usage::get_usage_logs(
        &db.pool,
        &claims.sub,
        &start_date,
        &end_date,
        limit.unwrap_or(50),
        offset.unwrap_or(0),
    )
    .await?;

    Ok(logs
        .into_iter()
        .map(|l| LlmUsageLogResponse {
            id: l.id,
            provider: l.provider,
            model: l.model,
            prompt_tokens: l.prompt_tokens,
            completion_tokens: l.completion_tokens,
            total_tokens: l.total_tokens,
            estimated_cost: l.estimated_cost,
            purpose: l.purpose,
            duration_ms: l.duration_ms,
            status: l.status,
            error_message: l.error_message,
            created_at: l.created_at,
        })
        .collect())
}
