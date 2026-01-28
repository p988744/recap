//! LLM Usage Logging Module
//!
//! Provides functions to save and query LLM usage records.

use serde::Serialize;
use sqlx::SqlitePool;
use uuid::Uuid;

use super::llm::LlmUsageRecord;
use super::llm_pricing::estimate_cost;

/// Save an LLM usage record to the database.
pub async fn save_usage_log(
    pool: &SqlitePool,
    user_id: &str,
    record: &LlmUsageRecord,
) -> Result<(), String> {
    let id = Uuid::new_v4().to_string();
    let estimated_cost = estimate_cost(
        &record.provider,
        &record.model,
        record.prompt_tokens,
        record.completion_tokens,
    );

    sqlx::query(
        r#"INSERT INTO llm_usage_logs
           (id, user_id, provider, model, prompt_tokens, completion_tokens, total_tokens,
            estimated_cost, purpose, duration_ms, status, error_message)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(user_id)
    .bind(&record.provider)
    .bind(&record.model)
    .bind(record.prompt_tokens)
    .bind(record.completion_tokens)
    .bind(record.total_tokens)
    .bind(estimated_cost)
    .bind(&record.purpose)
    .bind(record.duration_ms)
    .bind(&record.status)
    .bind(&record.error_message)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to save LLM usage log: {}", e))?;

    Ok(())
}

/// Aggregated usage statistics
#[derive(Debug, Serialize)]
pub struct LlmUsageStats {
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

/// Get aggregated usage stats for a date range.
pub async fn get_usage_stats(
    pool: &SqlitePool,
    user_id: &str,
    start_date: &str,
    end_date: &str,
) -> Result<LlmUsageStats, String> {
    let row: (i64, i64, i64, Option<i64>, Option<i64>, Option<i64>, Option<f64>, Option<f64>) = sqlx::query_as(
        r#"SELECT
            COUNT(*) as total_calls,
            SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) as success_calls,
            SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) as error_calls,
            SUM(prompt_tokens) as total_prompt_tokens,
            SUM(completion_tokens) as total_completion_tokens,
            SUM(total_tokens) as total_tokens,
            SUM(estimated_cost) as total_cost,
            AVG(duration_ms) as avg_duration_ms
           FROM llm_usage_logs
           WHERE user_id = ? AND DATE(created_at) >= ? AND DATE(created_at) <= ?"#,
    )
    .bind(user_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to get usage stats: {}", e))?;

    let total_calls = row.0;
    let total_tokens = row.5.unwrap_or(0);
    let avg_tokens_per_call = if total_calls > 0 {
        total_tokens as f64 / total_calls as f64
    } else {
        0.0
    };

    Ok(LlmUsageStats {
        total_calls,
        success_calls: row.1,
        error_calls: row.2,
        total_prompt_tokens: row.3.unwrap_or(0),
        total_completion_tokens: row.4.unwrap_or(0),
        total_tokens,
        total_cost: row.6.unwrap_or(0.0),
        avg_duration_ms: row.7.unwrap_or(0.0),
        avg_tokens_per_call,
    })
}

/// Daily usage data point
#[derive(Debug, Serialize)]
pub struct DailyUsage {
    pub date: String,
    pub calls: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub cost: f64,
}

/// Get daily usage breakdown for a date range.
pub async fn get_usage_by_day(
    pool: &SqlitePool,
    user_id: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<DailyUsage>, String> {
    let rows: Vec<(String, i64, Option<i64>, Option<i64>, Option<i64>, Option<f64>)> = sqlx::query_as(
        r#"SELECT
            DATE(created_at) as date,
            COUNT(*) as calls,
            SUM(prompt_tokens) as prompt_tokens,
            SUM(completion_tokens) as completion_tokens,
            SUM(total_tokens) as total_tokens,
            SUM(estimated_cost) as cost
           FROM llm_usage_logs
           WHERE user_id = ? AND DATE(created_at) >= ? AND DATE(created_at) <= ?
           GROUP BY DATE(created_at)
           ORDER BY date"#,
    )
    .bind(user_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to get daily usage: {}", e))?;

    Ok(rows
        .into_iter()
        .map(|(date, calls, pt, ct, tt, cost)| DailyUsage {
            date,
            calls,
            prompt_tokens: pt.unwrap_or(0),
            completion_tokens: ct.unwrap_or(0),
            total_tokens: tt.unwrap_or(0),
            cost: cost.unwrap_or(0.0),
        })
        .collect())
}

/// Usage breakdown by model
#[derive(Debug, Serialize)]
pub struct ModelUsage {
    pub provider: String,
    pub model: String,
    pub calls: i64,
    pub total_tokens: i64,
    pub cost: f64,
}

/// Get usage breakdown by model for a date range.
pub async fn get_usage_by_model(
    pool: &SqlitePool,
    user_id: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<ModelUsage>, String> {
    let rows: Vec<(String, String, i64, Option<i64>, Option<f64>)> = sqlx::query_as(
        r#"SELECT
            provider,
            model,
            COUNT(*) as calls,
            SUM(total_tokens) as total_tokens,
            SUM(estimated_cost) as cost
           FROM llm_usage_logs
           WHERE user_id = ? AND DATE(created_at) >= ? AND DATE(created_at) <= ?
           GROUP BY provider, model
           ORDER BY cost DESC"#,
    )
    .bind(user_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to get model usage: {}", e))?;

    Ok(rows
        .into_iter()
        .map(|(provider, model, calls, tt, cost)| ModelUsage {
            provider,
            model,
            calls,
            total_tokens: tt.unwrap_or(0),
            cost: cost.unwrap_or(0.0),
        })
        .collect())
}

/// Single usage log entry
#[derive(Debug, Serialize)]
pub struct LlmUsageLog {
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

/// Get paginated usage logs for a date range.
pub async fn get_usage_logs(
    pool: &SqlitePool,
    user_id: &str,
    start_date: &str,
    end_date: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<LlmUsageLog>, String> {
    let rows: Vec<(String, String, String, Option<i64>, Option<i64>, Option<i64>, Option<f64>, String, Option<i64>, String, Option<String>, String)> = sqlx::query_as(
        r#"SELECT
            id, provider, model, prompt_tokens, completion_tokens, total_tokens,
            estimated_cost, purpose, duration_ms, status, error_message,
            datetime(created_at) as created_at
           FROM llm_usage_logs
           WHERE user_id = ? AND DATE(created_at) >= ? AND DATE(created_at) <= ?
           ORDER BY created_at DESC
           LIMIT ? OFFSET ?"#,
    )
    .bind(user_id)
    .bind(start_date)
    .bind(end_date)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to get usage logs: {}", e))?;

    Ok(rows
        .into_iter()
        .map(|(id, provider, model, pt, ct, tt, cost, purpose, dur, status, err, created_at)| {
            LlmUsageLog {
                id,
                provider,
                model,
                prompt_tokens: pt,
                completion_tokens: ct,
                total_tokens: tt,
                estimated_cost: cost,
                purpose,
                duration_ms: dur,
                status,
                error_message: err,
                created_at,
            }
        })
        .collect())
}
