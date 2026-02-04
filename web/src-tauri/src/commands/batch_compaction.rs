//! Batch Compaction Commands
//!
//! Tauri commands for batch mode compaction using OpenAI Batch API.
//! Provides 50% cost savings with 24-hour turnaround for non-time-sensitive workloads.

use super::AppState;
use recap_core::auth::verify_token;
use recap_core::services::{
    llm::LlmConfig,
    llm_batch::LlmBatchService,
    compaction::{submit_hourly_batch, process_completed_batch, collect_pending_hourly},
};
use serde::Serialize;
use tauri::State;

// =============================================================================
// Types
// =============================================================================

#[derive(Debug, Serialize)]
pub struct BatchJobStatusResponse {
    pub job_id: String,
    pub status: String,
    pub total_requests: i64,
    pub completed_requests: i64,
    pub failed_requests: i64,
    pub created_at: String,
    pub submitted_at: Option<String>,
    pub completed_at: Option<String>,
    pub openai_batch_id: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BatchSubmitResponse {
    pub success: bool,
    pub job_id: Option<String>,
    pub total_requests: usize,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct BatchProcessResponse {
    pub success: bool,
    pub summaries_saved: usize,
    pub daily_compacted: usize,
    pub monthly_compacted: usize,
    pub errors: Vec<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct PendingHourlyResponse {
    pub count: usize,
    pub items: Vec<PendingHourlyItem>,
}

#[derive(Debug, Serialize)]
pub struct PendingHourlyItem {
    pub project_path: String,
    pub hour_bucket: String,
    pub snapshot_count: usize,
}

#[derive(Debug, Serialize)]
pub struct BatchAvailabilityResponse {
    pub available: bool,
    pub reason: Option<String>,
}

// =============================================================================
// Helper Functions
// =============================================================================

async fn get_llm_config(pool: &sqlx::SqlitePool, user_id: &str) -> Result<LlmConfig, String> {
    let row: (Option<String>, Option<String>, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT llm_provider, llm_model, llm_api_key, llm_base_url FROM users WHERE id = ?",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "User not found".to_string())?;

    Ok(LlmConfig {
        provider: row.0.unwrap_or_else(|| "openai".to_string()),
        model: row.1.unwrap_or_else(|| "gpt-5-nano".to_string()),
        api_key: row.2,
        base_url: row.3,
    })
}

// =============================================================================
// Commands
// =============================================================================

/// Check if batch API is available
#[tauri::command]
pub async fn check_batch_availability(
    state: State<'_, AppState>,
    token: String,
) -> Result<BatchAvailabilityResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;

    let pool = {
        let db = state.db.lock().await;
        db.pool.clone()
    };

    let config = get_llm_config(&pool, &claims.sub).await?;
    let batch_service = LlmBatchService::new(config);

    if batch_service.is_batch_available() {
        Ok(BatchAvailabilityResponse {
            available: true,
            reason: None,
        })
    } else {
        Ok(BatchAvailabilityResponse {
            available: false,
            reason: Some("Batch API requires OpenAI provider with API key".to_string()),
        })
    }
}

/// Get pending hourly compactions count
#[tauri::command]
pub async fn get_pending_hourly_compactions(
    state: State<'_, AppState>,
    token: String,
) -> Result<PendingHourlyResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;

    let pool = {
        let db = state.db.lock().await;
        db.pool.clone()
    };

    let pending = collect_pending_hourly(&pool, &claims.sub).await?;

    let items: Vec<PendingHourlyItem> = pending
        .iter()
        .map(|p| PendingHourlyItem {
            project_path: p.project_path.clone(),
            hour_bucket: p.hour_bucket.clone(),
            snapshot_count: p.snapshots.len(),
        })
        .collect();

    Ok(PendingHourlyResponse {
        count: pending.len(),
        items,
    })
}

/// Get current batch job status
#[tauri::command]
pub async fn get_batch_job_status(
    state: State<'_, AppState>,
    token: String,
) -> Result<Option<BatchJobStatusResponse>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;

    let pool = {
        let db = state.db.lock().await;
        db.pool.clone()
    };

    let job = LlmBatchService::get_pending_job(&pool, &claims.sub).await?;

    match job {
        Some(j) => Ok(Some(BatchJobStatusResponse {
            job_id: j.id,
            status: j.status,
            total_requests: j.total_requests,
            completed_requests: j.completed_requests,
            failed_requests: j.failed_requests,
            created_at: j.created_at.to_rfc3339(),
            submitted_at: j.submitted_at.map(|d| d.to_rfc3339()),
            completed_at: j.completed_at.map(|d| d.to_rfc3339()),
            openai_batch_id: j.openai_batch_id,
            error_message: j.error_message,
        })),
        None => Ok(None),
    }
}

/// Submit hourly compactions as a batch job
#[tauri::command]
pub async fn submit_batch_compaction(
    state: State<'_, AppState>,
    token: String,
) -> Result<BatchSubmitResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;

    let pool = {
        let db = state.db.lock().await;
        db.pool.clone()
    };

    let config = get_llm_config(&pool, &claims.sub).await?;
    let batch_service = LlmBatchService::new(config);

    if !batch_service.is_batch_available() {
        return Ok(BatchSubmitResponse {
            success: false,
            job_id: None,
            total_requests: 0,
            message: "Batch API 需要 OpenAI provider 並設定 API key".to_string(),
        });
    }

    match submit_hourly_batch(&pool, &batch_service, &claims.sub).await {
        Ok(result) => Ok(BatchSubmitResponse {
            success: true,
            job_id: Some(result.job_id),
            total_requests: result.total_requests,
            message: result.message,
        }),
        Err(e) => Ok(BatchSubmitResponse {
            success: false,
            job_id: None,
            total_requests: 0,
            message: e,
        }),
    }
}

/// Check batch job status and update database
#[tauri::command]
pub async fn refresh_batch_status(
    state: State<'_, AppState>,
    token: String,
    job_id: String,
) -> Result<BatchJobStatusResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;

    let pool = {
        let db = state.db.lock().await;
        db.pool.clone()
    };

    let config = get_llm_config(&pool, &claims.sub).await?;
    let batch_service = LlmBatchService::new(config);

    // Update status from OpenAI
    let status = batch_service.check_batch_status(&pool, &job_id).await?;

    // Fetch updated job
    let job: recap_core::services::llm_batch::BatchJob = sqlx::query_as(
        "SELECT * FROM llm_batch_jobs WHERE id = ?",
    )
    .bind(&job_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| format!("Failed to fetch batch job: {}", e))?;

    Ok(BatchJobStatusResponse {
        job_id: job.id,
        status: status.to_string(),
        total_requests: job.total_requests,
        completed_requests: job.completed_requests,
        failed_requests: job.failed_requests,
        created_at: job.created_at.to_rfc3339(),
        submitted_at: job.submitted_at.map(|d| d.to_rfc3339()),
        completed_at: job.completed_at.map(|d| d.to_rfc3339()),
        openai_batch_id: job.openai_batch_id,
        error_message: job.error_message,
    })
}

/// Process completed batch and run remaining compaction
#[tauri::command]
pub async fn process_completed_batch_job(
    state: State<'_, AppState>,
    token: String,
    job_id: String,
) -> Result<BatchProcessResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;

    let pool = {
        let db = state.db.lock().await;
        db.pool.clone()
    };

    let config = get_llm_config(&pool, &claims.sub).await?;
    let batch_service = LlmBatchService::new(config.clone());

    // Create LLM service for daily/weekly/monthly compaction
    let llm = recap_core::services::llm::LlmService::new(config);
    let llm_ref = if llm.is_configured() { Some(&llm) } else { None };

    match process_completed_batch(&pool, llm_ref, &batch_service, &claims.sub, &job_id).await {
        Ok(result) => Ok(BatchProcessResponse {
            success: true,
            summaries_saved: result.summaries_saved,
            daily_compacted: result.daily_compacted,
            monthly_compacted: result.monthly_compacted,
            errors: result.errors,
            message: format!(
                "已處理完成：{} 個小時摘要、{} 個每日摘要、{} 個月度摘要",
                result.summaries_saved,
                result.daily_compacted,
                result.monthly_compacted
            ),
        }),
        Err(e) => Ok(BatchProcessResponse {
            success: false,
            summaries_saved: 0,
            daily_compacted: 0,
            monthly_compacted: 0,
            errors: vec![e.clone()],
            message: e,
        }),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_availability_response() {
        let available = BatchAvailabilityResponse {
            available: true,
            reason: None,
        };
        assert!(available.available);

        let unavailable = BatchAvailabilityResponse {
            available: false,
            reason: Some("No API key".to_string()),
        };
        assert!(!unavailable.available);
        assert!(unavailable.reason.is_some());
    }
}
