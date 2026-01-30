//! Compaction Engine
//!
//! Rolls up raw snapshots into hierarchical summaries:
//!   snapshot_raw_data (hourly) → work_summaries (hourly)
//!   work_summaries (hourly)    → work_summaries (daily)
//!   work_summaries (daily)     → work_summaries (weekly)
//!   work_summaries (weekly)    → work_summaries (monthly)
//!
//! Each level uses the previous period's summary as context for LLM generation.
//! Falls back to rule-based summarization when LLM is unavailable.
//!
//! Supports two modes:
//! - **Immediate mode**: Process each hourly summary synchronously (default)
//! - **Batch mode**: Collect all hourly prompts, submit to OpenAI Batch API (50% cheaper, 24h delay)

use chrono::{Duration, NaiveDateTime};
#[cfg(test)]
use chrono::Utc;
use serde::Serialize;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{SnapshotRawData, WorkSummary};

use super::llm::{LlmService, parse_error_usage};
use super::llm_batch::{BatchRequest, HourlyCompactionRequest, LlmBatchService};
use super::llm_usage::save_usage_log;
use super::snapshot::{CommitSnapshot, ToolCallRecord};

// ============ Types ============

/// Result of a compaction cycle
#[derive(Debug, Clone, Serialize)]
pub struct CompactionResult {
    pub hourly_compacted: usize,
    pub daily_compacted: usize,
    pub weekly_compacted: usize,
    pub monthly_compacted: usize,
    pub errors: Vec<String>,
    /// Latest date that was compacted (YYYY-MM-DD format)
    pub latest_compacted_date: Option<String>,
}

// ============ Helpers (time) ============

/// Check if an hour bucket is in the past (completed).
/// Hour buckets are naive local times like "2026-01-26T18:00:00".
fn is_hour_completed(hour_bucket: &str) -> bool {
    let current = chrono::Local::now().format("%Y-%m-%dT%H:00:00").to_string();
    hour_bucket < current.as_str()
}

/// Check if a day is in the past (completed).
/// Accepts date strings like "2026-01-26" or period_start like "2026-01-26T00:00:00+00:00".
fn is_day_completed(date: &str) -> bool {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    // Extract just the date portion (first 10 chars) for comparison
    let day = &date[..date.len().min(10)];
    day < today.as_str()
}

/// Check if a period (weekly/monthly) is in the past (completed).
/// A period is completed when period_end is before today.
fn is_period_completed(period_end: &str) -> bool {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let end_date = &period_end[..period_end.len().min(10)];
    end_date < today.as_str()
}

// ============ Hourly Compaction ============

/// Compact raw hourly snapshots into an hourly summary for a specific hour bucket.
pub async fn compact_hourly(
    pool: &SqlitePool,
    llm: Option<&LlmService>,
    user_id: &str,
    project_path: &str,
    hour_bucket: &str,
) -> Result<(), String> {
    // Fetch all snapshots for this hour
    let snapshots: Vec<SnapshotRawData> = sqlx::query_as(
        "SELECT * FROM snapshot_raw_data WHERE user_id = ? AND project_path = ? AND hour_bucket = ?",
    )
    .bind(user_id)
    .bind(project_path)
    .bind(hour_bucket)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch snapshots: {}", e))?;

    if snapshots.is_empty() {
        return Ok(());
    }

    // Check if summary already exists
    let existing: Option<WorkSummary> = sqlx::query_as(
        "SELECT * FROM work_summaries WHERE user_id = ? AND project_path = ? AND scale = 'hourly' AND period_start = ?",
    )
    .bind(user_id)
    .bind(project_path)
    .bind(hour_bucket)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to check existing summary: {}", e))?;

    if existing.is_some() && is_hour_completed(hour_bucket) {
        return Ok(()); // Only skip if hour is finished; in-progress hours get re-compacted
    }

    // Fetch previous hour's summary for context
    let previous_context = get_previous_summary(pool, user_id, Some(project_path), "hourly", hour_bucket).await;

    // Aggregate data from all snapshots
    let (current_data, snapshot_ids, key_activities, git_summary) = aggregate_snapshots(&snapshots);

    // Compute period_end (hour_bucket + 1 hour)
    // hour_bucket is stored as local time without offset: "2026-01-26T10:00:00"
    let period_end = match NaiveDateTime::parse_from_str(hour_bucket, "%Y-%m-%dT%H:%M:%S") {
        Ok(ndt) => (ndt + Duration::hours(1)).format("%Y-%m-%dT%H:%M:%S").to_string(),
        Err(_) => {
            // Fallback: try RFC 3339 for legacy data
            match chrono::DateTime::parse_from_rfc3339(hour_bucket) {
                Ok(dt) => (dt + Duration::hours(1)).format("%Y-%m-%dT%H:%M:%S").to_string(),
                Err(_) => hour_bucket.to_string(),
            }
        }
    };

    // Generate summary
    let (summary, llm_model) = match llm {
        Some(llm_svc) if llm_svc.is_configured() => {
            let result = llm_svc
                .summarize_work_period(
                    &previous_context.as_deref().unwrap_or(""),
                    &current_data,
                    "hourly",
                )
                .await;
            match result {
                Ok((s, usage)) => {
                    let _ = save_usage_log(pool, user_id, &usage).await;
                    (s, Some("llm".to_string()))
                }
                Err(e) => {
                    if let Some(usage) = parse_error_usage(&e) {
                        let _ = save_usage_log(pool, user_id, &usage).await;
                    }
                    log::warn!("LLM summarization failed, using rule-based: {}", e);
                    (build_rule_based_summary(&current_data, &key_activities, &git_summary), None)
                }
            }
        }
        _ => (build_rule_based_summary(&current_data, &key_activities, &git_summary), None),
    };

    // Save summary
    save_summary(
        pool,
        user_id,
        Some(project_path),
        "hourly",
        hour_bucket,
        &period_end,
        &summary,
        &key_activities,
        &git_summary,
        previous_context.as_deref(),
        &snapshot_ids,
        llm_model.as_deref(),
    )
    .await
}

// ============ Daily Compaction ============

/// Compact hourly summaries into a daily summary.
pub async fn compact_daily(
    pool: &SqlitePool,
    llm: Option<&LlmService>,
    user_id: &str,
    project_path: &str,
    date: &str, // "2026-01-26"
) -> Result<(), String> {
    let period_start = format!("{}T00:00:00+00:00", date);
    let period_end = format!("{}T23:59:59+00:00", date);

    // Check if daily summary already exists
    let existing: Option<WorkSummary> = sqlx::query_as(
        "SELECT * FROM work_summaries WHERE user_id = ? AND project_path = ? AND scale = 'daily' AND period_start = ?",
    )
    .bind(user_id)
    .bind(project_path)
    .bind(&period_start)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to check existing daily summary: {}", e))?;

    if existing.is_some() && is_day_completed(date) {
        return Ok(()); // Only skip if day is finished; in-progress days get re-compacted
    }

    // Fetch all hourly summaries for this day
    let hourlies: Vec<WorkSummary> = sqlx::query_as(
        "SELECT * FROM work_summaries WHERE user_id = ? AND project_path = ? AND scale = 'hourly' AND period_start >= ? AND period_start < ? ORDER BY period_start",
    )
    .bind(user_id)
    .bind(project_path)
    .bind(&period_start)
    .bind(&period_end)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch hourly summaries: {}", e))?;

    if hourlies.is_empty() {
        return Ok(());
    }

    let previous_context =
        get_previous_summary(pool, user_id, Some(project_path), "daily", &period_start).await;

    let current_data = hourlies
        .iter()
        .map(|h| format!("[{}] {}", h.period_start, h.summary))
        .collect::<Vec<_>>()
        .join("\n");

    let key_activities = merge_json_arrays(
        &hourlies
            .iter()
            .filter_map(|h| h.key_activities.clone())
            .collect::<Vec<_>>(),
    );
    let git_summary = merge_json_arrays(
        &hourlies
            .iter()
            .filter_map(|h| h.git_commits_summary.clone())
            .collect::<Vec<_>>(),
    );
    let snapshot_ids = hourlies.iter().map(|h| h.id.clone()).collect::<Vec<_>>();

    let (summary, llm_model) = match llm {
        Some(llm_svc) if llm_svc.is_configured() => {
            let result = llm_svc
                .summarize_work_period(
                    &previous_context.as_deref().unwrap_or(""),
                    &current_data,
                    "daily",
                )
                .await;
            match result {
                Ok((s, usage)) => {
                    let _ = save_usage_log(pool, user_id, &usage).await;
                    (s, Some("llm".to_string()))
                }
                Err(e) => {
                    if let Some(usage) = parse_error_usage(&e) {
                        let _ = save_usage_log(pool, user_id, &usage).await;
                    }
                    log::warn!("LLM daily summarization failed: {}", e);
                    (build_rule_based_summary(&current_data, &key_activities, &git_summary), None)
                }
            }
        }
        _ => (build_rule_based_summary(&current_data, &key_activities, &git_summary), None),
    };

    save_summary(
        pool,
        user_id,
        Some(project_path),
        "daily",
        &period_start,
        &period_end,
        &summary,
        &key_activities,
        &git_summary,
        previous_context.as_deref(),
        &snapshot_ids.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        llm_model.as_deref(),
    )
    .await
}

// ============ Weekly / Monthly Compaction ============

/// Roll up daily summaries into weekly or monthly summaries.
pub async fn compact_period(
    pool: &SqlitePool,
    llm: Option<&LlmService>,
    user_id: &str,
    project_path: Option<&str>,
    scale: &str, // "weekly" | "monthly"
    period_start: &str,
    period_end: &str,
) -> Result<(), String> {
    let source_scale = match scale {
        "weekly" => "daily",
        "monthly" => "weekly",
        _ => return Err(format!("Invalid scale for compact_period: {}", scale)),
    };

    // Check if summary already exists
    let existing: Option<WorkSummary> = sqlx::query_as(
        "SELECT * FROM work_summaries WHERE user_id = ? AND (project_path = ? OR (project_path IS NULL AND ? IS NULL)) AND scale = ? AND period_start = ?",
    )
    .bind(user_id)
    .bind(project_path)
    .bind(project_path)
    .bind(scale)
    .bind(period_start)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to check existing {} summary: {}", scale, e))?;

    if existing.is_some() && is_period_completed(period_end) {
        return Ok(()); // Only skip if period is finished; in-progress periods get re-compacted
    }

    // Fetch source-scale summaries for this period
    let sources: Vec<WorkSummary> = sqlx::query_as(
        "SELECT * FROM work_summaries WHERE user_id = ? AND (project_path = ? OR (project_path IS NULL AND ? IS NULL)) AND scale = ? AND period_start >= ? AND period_start < ? ORDER BY period_start",
    )
    .bind(user_id)
    .bind(project_path)
    .bind(project_path)
    .bind(source_scale)
    .bind(period_start)
    .bind(period_end)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch {} summaries: {}", source_scale, e))?;

    if sources.is_empty() {
        return Ok(());
    }

    let previous_context =
        get_previous_summary(pool, user_id, project_path, scale, period_start).await;

    let current_data = sources
        .iter()
        .map(|s| format!("[{}] {}", s.period_start, s.summary))
        .collect::<Vec<_>>()
        .join("\n");

    let key_activities = merge_json_arrays(
        &sources
            .iter()
            .filter_map(|s| s.key_activities.clone())
            .collect::<Vec<_>>(),
    );
    let git_summary = merge_json_arrays(
        &sources
            .iter()
            .filter_map(|s| s.git_commits_summary.clone())
            .collect::<Vec<_>>(),
    );
    let source_ids = sources.iter().map(|s| s.id.clone()).collect::<Vec<_>>();

    let (summary, llm_model) = match llm {
        Some(llm_svc) if llm_svc.is_configured() => {
            let result = llm_svc
                .summarize_work_period(
                    &previous_context.as_deref().unwrap_or(""),
                    &current_data,
                    scale,
                )
                .await;
            match result {
                Ok((s, usage)) => {
                    let _ = save_usage_log(pool, user_id, &usage).await;
                    (s, Some("llm".to_string()))
                }
                Err(e) => {
                    if let Some(usage) = parse_error_usage(&e) {
                        let _ = save_usage_log(pool, user_id, &usage).await;
                    }
                    log::warn!("LLM {} summarization failed: {}", scale, e);
                    (build_rule_based_summary(&current_data, &key_activities, &git_summary), None)
                }
            }
        }
        _ => (build_rule_based_summary(&current_data, &key_activities, &git_summary), None),
    };

    save_summary(
        pool,
        user_id,
        project_path,
        scale,
        period_start,
        period_end,
        &summary,
        &key_activities,
        &git_summary,
        previous_context.as_deref(),
        &source_ids.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        llm_model.as_deref(),
    )
    .await
}

// ============ Force Recompaction ============

/// Options for force recompaction
#[derive(Debug, Clone, Default)]
pub struct ForceRecompactOptions {
    /// Only recompact summaries from this date (YYYY-MM-DD). If None, all dates.
    pub from_date: Option<String>,
    /// Only recompact summaries up to this date (YYYY-MM-DD). If None, up to now.
    pub to_date: Option<String>,
    /// Only recompact these scales. If empty, all scales.
    pub scales: Vec<String>,
}

/// Result of a force recompaction operation
#[derive(Debug, Clone, Serialize)]
pub struct ForceRecompactResult {
    pub summaries_deleted: usize,
    pub compaction_result: CompactionResult,
}

/// Force recalculate all work_summaries from snapshot_raw_data.
///
/// This operation:
/// 1. Deletes existing work_summaries entries (preserving original work_items and snapshot_raw_data)
/// 2. Re-runs the compaction cycle to regenerate all summaries
///
/// Use this when you've made changes to the compaction logic and want to
/// retroactively apply them to historical data.
pub async fn force_recompact(
    pool: &SqlitePool,
    llm: Option<&LlmService>,
    user_id: &str,
    options: ForceRecompactOptions,
) -> Result<ForceRecompactResult, String> {
    log::info!("Starting force recompaction for user: {}", user_id);

    // Build delete query based on options
    let mut delete_conditions = vec!["user_id = ?".to_string()];
    let mut bind_values: Vec<String> = vec![user_id.to_string()];

    if let Some(ref from_date) = options.from_date {
        delete_conditions.push("period_start >= ?".to_string());
        bind_values.push(format!("{}T00:00:00", from_date));
    }

    if let Some(ref to_date) = options.to_date {
        delete_conditions.push("period_start <= ?".to_string());
        bind_values.push(format!("{}T23:59:59", to_date));
    }

    if !options.scales.is_empty() {
        let scale_placeholders: Vec<&str> = options.scales.iter().map(|_| "?").collect();
        delete_conditions.push(format!("scale IN ({})", scale_placeholders.join(", ")));
        bind_values.extend(options.scales.clone());
    }

    let delete_query = format!(
        "DELETE FROM work_summaries WHERE {}",
        delete_conditions.join(" AND ")
    );

    // Count rows to be deleted
    let count_query = format!(
        "SELECT COUNT(*) as count FROM work_summaries WHERE {}",
        delete_conditions.join(" AND ")
    );

    // Execute count query
    let count_result: (i64,) = {
        let mut query = sqlx::query_as(&count_query);
        for val in &bind_values {
            query = query.bind(val);
        }
        query
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Failed to count summaries: {}", e))?
    };
    let summaries_to_delete = count_result.0 as usize;

    log::info!("Deleting {} existing summaries", summaries_to_delete);

    // Execute delete
    {
        let mut query = sqlx::query(&delete_query);
        for val in &bind_values {
            query = query.bind(val);
        }
        query
            .execute(pool)
            .await
            .map_err(|e| format!("Failed to delete summaries: {}", e))?;
    }

    // Run compaction cycle to regenerate summaries
    log::info!("Running compaction cycle to regenerate summaries");
    let compaction_result = run_compaction_cycle(pool, llm, user_id).await?;

    log::info!(
        "Force recompaction complete: deleted {} summaries, created {} hourly + {} daily + {} monthly",
        summaries_to_delete,
        compaction_result.hourly_compacted,
        compaction_result.daily_compacted,
        compaction_result.monthly_compacted
    );

    Ok(ForceRecompactResult {
        summaries_deleted: summaries_to_delete,
        compaction_result,
    })
}

// ============ Batch Mode for Hourly Compaction ============

/// Pending hourly compaction info
#[derive(Debug, Clone)]
pub struct PendingHourlyCompaction {
    pub project_path: String,
    pub hour_bucket: String,
    pub snapshots: Vec<SnapshotRawData>,
}

/// Collect all pending hourly compactions for batch processing
pub async fn collect_pending_hourly(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<PendingHourlyCompaction>, String> {
    // Find all uncompacted hourly snapshots
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
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to find uncompacted snapshots: {}", e))?;

    let mut result = Vec::new();
    for (project_path, hour_bucket) in uncompacted {
        // Only include completed hours (not current hour)
        if !is_hour_completed(&hour_bucket) {
            continue;
        }

        // Fetch snapshots for this hour
        let snapshots: Vec<SnapshotRawData> = sqlx::query_as(
            "SELECT * FROM snapshot_raw_data WHERE user_id = ? AND project_path = ? AND hour_bucket = ?",
        )
        .bind(user_id)
        .bind(&project_path)
        .bind(&hour_bucket)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to fetch snapshots: {}", e))?;

        if !snapshots.is_empty() {
            result.push(PendingHourlyCompaction {
                project_path,
                hour_bucket,
                snapshots,
            });
        }
    }

    Ok(result)
}

/// Prepare batch requests from pending hourly compactions
pub async fn prepare_hourly_batch_requests(
    pool: &SqlitePool,
    user_id: &str,
    pending: &[PendingHourlyCompaction],
) -> Result<Vec<HourlyCompactionRequest>, String> {
    let mut requests = Vec::new();

    for item in pending {
        // Get previous context
        let previous_context = get_previous_summary(
            pool,
            user_id,
            Some(&item.project_path),
            "hourly",
            &item.hour_bucket,
        )
        .await;

        // Aggregate snapshot data
        let (current_data, snapshot_ids, key_activities, git_summary) =
            aggregate_snapshots(&item.snapshots);

        // Build prompt (same as in compact_hourly)
        let prompt = build_hourly_prompt(&previous_context, &current_data);

        requests.push(HourlyCompactionRequest {
            project_path: item.project_path.clone(),
            hour_bucket: item.hour_bucket.clone(),
            prompt,
            snapshot_ids: snapshot_ids.iter().map(|s| s.to_string()).collect(),
            key_activities,
            git_summary,
            previous_context,
        });
    }

    Ok(requests)
}

/// Build prompt for hourly summarization
fn build_hourly_prompt(context: &Option<String>, current_data: &str) -> String {
    let context_section = match context {
        Some(ctx) if !ctx.is_empty() => format!(
            "\n前一時段摘要（作為前後文參考）：\n{}\n",
            ctx.chars().take(1000).collect::<String>()
        ),
        _ => String::new(),
    };

    format!(
        r#"你是工作記錄助手。請根據以下工作資料，產生簡潔的工作摘要（50-100字）。
{context_section}
本時段的工作資料：
{data}

請用繁體中文回答，格式如下：
1. 第一行是一句話的總結摘要（不要加前綴）
2. 空一行後，用條列式列出具體細節，每個要點以「- 」開頭

重點描述完成了什麼、使用什麼技術、解決什麼問題。
若有 git commit，優先以 commit 訊息作為成果總結。
程式碼中的檔名、函式名、變數名請用 `backtick` 包裹。
直接輸出內容，不要加標題。"#,
        context_section = context_section,
        data = current_data.chars().take(4000).collect::<String>()
    )
}

/// Save completed batch results as hourly summaries
pub async fn save_batch_results_as_summaries(
    pool: &SqlitePool,
    user_id: &str,
    requests: &[HourlyCompactionRequest],
    batch_requests: &[BatchRequest],
) -> Result<usize, String> {
    let mut saved = 0;

    for batch_req in batch_requests {
        if batch_req.status != "completed" {
            continue;
        }

        let response = match &batch_req.response {
            Some(r) => r,
            None => continue,
        };

        // Find matching hourly request
        let hourly_req = requests
            .iter()
            .find(|r| r.project_path == batch_req.project_path && r.hour_bucket == batch_req.hour_bucket);

        let hourly_req = match hourly_req {
            Some(r) => r,
            None => continue,
        };

        // Compute period_end
        let period_end = match NaiveDateTime::parse_from_str(&hourly_req.hour_bucket, "%Y-%m-%dT%H:%M:%S") {
            Ok(ndt) => (ndt + Duration::hours(1)).format("%Y-%m-%dT%H:%M:%S").to_string(),
            Err(_) => hourly_req.hour_bucket.clone(),
        };

        // Save summary
        save_summary(
            pool,
            user_id,
            Some(&hourly_req.project_path),
            "hourly",
            &hourly_req.hour_bucket,
            &period_end,
            response,
            &hourly_req.key_activities,
            &hourly_req.git_summary,
            hourly_req.previous_context.as_deref(),
            &hourly_req.snapshot_ids.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            Some("batch"),
        )
        .await?;

        saved += 1;
    }

    Ok(saved)
}

/// Result of batch compaction submission
#[derive(Debug, Clone, Serialize)]
pub struct BatchCompactionSubmitResult {
    pub job_id: String,
    pub total_requests: usize,
    pub message: String,
}

/// Result of batch compaction processing
#[derive(Debug, Clone, Serialize)]
pub struct BatchCompactionProcessResult {
    pub summaries_saved: usize,
    pub daily_compacted: usize,
    pub monthly_compacted: usize,
    pub errors: Vec<String>,
}

/// Submit hourly compactions as a batch job (Phase 1)
///
/// This collects all pending hourly compactions and submits them to OpenAI Batch API.
/// Returns a job ID that can be used to check status and process results later.
pub async fn submit_hourly_batch(
    pool: &SqlitePool,
    batch_service: &LlmBatchService,
    user_id: &str,
) -> Result<BatchCompactionSubmitResult, String> {
    // Check for existing pending job
    if let Some(existing) = LlmBatchService::get_pending_job(pool, user_id).await? {
        return Err(format!(
            "Already have a pending batch job: {} (status: {})",
            existing.id, existing.status
        ));
    }

    // Collect pending hourly compactions
    let pending = collect_pending_hourly(pool, user_id).await?;
    if pending.is_empty() {
        return Err("No pending hourly compactions to batch".to_string());
    }

    // Prepare batch requests
    let requests = prepare_hourly_batch_requests(pool, user_id, &pending).await?;
    let total = requests.len();

    // Create batch job
    let job_id = batch_service.create_batch_job(pool, user_id, requests).await?;

    // Submit to OpenAI
    let submit_result = batch_service.submit_batch_job(pool, &job_id).await?;

    Ok(BatchCompactionSubmitResult {
        job_id: submit_result.job_id,
        total_requests: total,
        message: format!(
            "Submitted {} hourly compactions to batch. OpenAI batch ID: {}",
            total, submit_result.openai_batch_id
        ),
    })
}

/// Process completed batch and run remaining compaction (Phase 2)
///
/// This should be called after the batch job completes. It:
/// 1. Downloads and saves hourly summaries from batch results
/// 2. Runs daily/weekly/monthly compaction (immediate, not batch)
pub async fn process_completed_batch(
    pool: &SqlitePool,
    llm: Option<&LlmService>,
    batch_service: &LlmBatchService,
    user_id: &str,
    job_id: &str,
) -> Result<BatchCompactionProcessResult, String> {
    // Process batch results
    let _batch_result = batch_service.process_batch_results(pool, job_id).await?;

    // Get completed requests
    let completed_requests = LlmBatchService::get_completed_requests(pool, job_id).await?;

    // Get original requests to match metadata
    let pending = collect_pending_hourly(pool, user_id).await?;
    let hourly_requests = prepare_hourly_batch_requests(pool, user_id, &pending).await?;

    // Save as summaries
    let summaries_saved = save_batch_results_as_summaries(
        pool,
        user_id,
        &hourly_requests,
        &completed_requests,
    )
    .await?;

    // Now run daily/weekly/monthly compaction (immediate mode)
    let mut result = BatchCompactionProcessResult {
        summaries_saved,
        daily_compacted: 0,
        monthly_compacted: 0,
        errors: Vec::new(),
    };

    // Run daily compaction
    let uncompacted_days: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT DISTINCT ws.project_path, DATE(ws.period_start) as day
        FROM work_summaries ws
        LEFT JOIN work_summaries ds ON ds.user_id = ws.user_id
            AND ds.project_path = ws.project_path
            AND ds.scale = 'daily'
            AND DATE(ds.period_start) = DATE(ws.period_start)
        WHERE ws.user_id = ? AND ws.scale = 'hourly' AND ds.id IS NULL
        ORDER BY day
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to find uncompacted days: {}", e))?;

    for (project_path, day) in &uncompacted_days {
        match compact_daily(pool, llm, user_id, project_path, day).await {
            Ok(()) => result.daily_compacted += 1,
            Err(e) => result.errors.push(format!("daily {}/{}: {}", project_path, day, e)),
        }
    }

    // Run monthly compaction
    let now = chrono::Local::now();
    let month_start = now.format("%Y-%m-01T00:00:00+00:00").to_string();
    let month_end = {
        let year = now.format("%Y").to_string().parse::<i32>().unwrap_or(2026);
        let month = now.format("%m").to_string().parse::<u32>().unwrap_or(1);
        let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
        format!("{:04}-{:02}-01T00:00:00+00:00", next_year, next_month)
    };

    let monthly_projects: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT DISTINCT ws.project_path
        FROM work_summaries ws
        WHERE ws.user_id = ? AND ws.scale = 'daily'
            AND ws.period_start >= ? AND ws.period_start < ?
        "#,
    )
    .bind(user_id)
    .bind(&month_start)
    .bind(&month_end)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to find monthly projects: {}", e))?;

    for (project_path,) in &monthly_projects {
        match compact_period(pool, llm, user_id, Some(project_path), "monthly", &month_start, &month_end).await {
            Ok(()) => result.monthly_compacted += 1,
            Err(e) => result.errors.push(format!("monthly {}: {}", project_path, e)),
        }
    }

    Ok(result)
}

// ============ Full Compaction Cycle ============

/// Run all pending compactions for a user.
///
/// Discovers uncompacted hourly snapshots, compacts them to hourly summaries,
/// then rolls up to daily, weekly, and monthly summaries.
pub async fn run_compaction_cycle(
    pool: &SqlitePool,
    llm: Option<&LlmService>,
    user_id: &str,
) -> Result<CompactionResult, String> {
    let mut result = CompactionResult {
        hourly_compacted: 0,
        daily_compacted: 0,
        weekly_compacted: 0,
        monthly_compacted: 0,
        errors: Vec::new(),
        latest_compacted_date: None,
    };

    // 1. Find all uncompacted hourly snapshots
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
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to find uncompacted snapshots: {}", e))?;

    // 2. Also find in-progress hours (current hour that already have a summary but need refresh)
    let current_hour = chrono::Local::now().format("%Y-%m-%dT%H:00:00").to_string();
    let in_progress: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT DISTINCT s.project_path, s.hour_bucket
        FROM snapshot_raw_data s
        WHERE s.user_id = ? AND s.hour_bucket = ?
        "#,
    )
    .bind(user_id)
    .bind(&current_hour)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to find in-progress hours: {}", e))?;

    // Merge uncompacted + in-progress, dedup by (project_path, hour_bucket)
    let mut all_hourly = uncompacted;
    for entry in in_progress {
        if !all_hourly.contains(&entry) {
            all_hourly.push(entry);
        }
    }

    // 3. Compact hourly
    for (project_path, hour_bucket) in &all_hourly {
        match compact_hourly(pool, llm, user_id, project_path, hour_bucket).await {
            Ok(()) => result.hourly_compacted += 1,
            Err(e) => result.errors.push(format!("hourly {}/{}: {}", project_path, hour_bucket, e)),
        }
    }

    // 4. Find days that have hourly summaries but no daily summary
    let uncompacted_days: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT DISTINCT ws.project_path, DATE(ws.period_start) as day
        FROM work_summaries ws
        LEFT JOIN work_summaries ds ON ds.user_id = ws.user_id
            AND ds.project_path = ws.project_path
            AND ds.scale = 'daily'
            AND DATE(ds.period_start) = DATE(ws.period_start)
        WHERE ws.user_id = ? AND ws.scale = 'hourly' AND ds.id IS NULL
        ORDER BY day
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to find uncompacted days: {}", e))?;

    // 5. Also include today for re-compaction (daily summary updates as new hourly data arrives)
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let in_progress_days: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT DISTINCT ws.project_path
        FROM work_summaries ws
        WHERE ws.user_id = ? AND ws.scale = 'hourly' AND DATE(ws.period_start) = ?
        "#,
    )
    .bind(user_id)
    .bind(&today)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to find in-progress days: {}", e))?;

    // Merge uncompacted days + today's in-progress
    let mut all_days = uncompacted_days;
    for (project_path,) in in_progress_days {
        let entry = (project_path, today.clone());
        if !all_days.contains(&entry) {
            all_days.push(entry);
        }
    }

    for (project_path, day) in &all_days {
        match compact_daily(pool, llm, user_id, project_path, day).await {
            Ok(()) => {
                result.daily_compacted += 1;
                // Track the latest compacted date
                if result.latest_compacted_date.as_ref().map_or(true, |d| day > d) {
                    result.latest_compacted_date = Some(day.clone());
                }
            }
            Err(e) => result.errors.push(format!("daily {}/{}: {}", project_path, day, e)),
        }
    }

    // 6. Weekly compaction - find weeks with daily summaries but no weekly summary
    // Use ISO week calculation: weeks start on Monday
    let now = chrono::Local::now();

    // Find all (project_path, iso_week_start) combinations that have daily summaries but no weekly summary
    let uncompacted_weeks: Vec<(String, String, String)> = sqlx::query_as(
        r#"
        SELECT DISTINCT
            ws.project_path,
            DATE(ws.period_start, 'weekday 0', '-6 days') as week_start,
            DATE(ws.period_start, 'weekday 0', '+1 day') as week_end
        FROM work_summaries ws
        LEFT JOIN work_summaries ww ON ww.user_id = ws.user_id
            AND ww.project_path = ws.project_path
            AND ww.scale = 'weekly'
            AND DATE(ww.period_start) = DATE(ws.period_start, 'weekday 0', '-6 days')
        WHERE ws.user_id = ? AND ws.scale = 'daily' AND ww.id IS NULL
        ORDER BY week_start
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to find uncompacted weeks: {}", e))?;

    // Also include the current week for re-compaction
    let current_week_start = now.format("%Y-%m-%d").to_string();
    let in_progress_weeks: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT DISTINCT ws.project_path
        FROM work_summaries ws
        WHERE ws.user_id = ? AND ws.scale = 'daily'
            AND DATE(ws.period_start, 'weekday 0', '-6 days') = DATE(?, 'weekday 0', '-6 days')
        "#,
    )
    .bind(user_id)
    .bind(&current_week_start)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to find in-progress weeks: {}", e))?;

    // Merge uncompacted weeks + current week
    let mut all_weeks = uncompacted_weeks;
    for (project_path,) in in_progress_weeks {
        // Calculate current week bounds
        let week_start_query: Option<(String, String)> = sqlx::query_as(
            "SELECT DATE(?, 'weekday 0', '-6 days'), DATE(?, 'weekday 0', '+1 day')"
        )
        .bind(&current_week_start)
        .bind(&current_week_start)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        if let Some((ws, we)) = week_start_query {
            let entry = (project_path, ws, we);
            if !all_weeks.iter().any(|(p, s, _)| p == &entry.0 && s == &entry.1) {
                all_weeks.push(entry);
            }
        }
    }

    for (project_path, week_start, week_end) in &all_weeks {
        let period_start = format!("{}T00:00:00+00:00", week_start);
        let period_end = format!("{}T00:00:00+00:00", week_end);
        match compact_period(pool, llm, user_id, Some(project_path), "weekly", &period_start, &period_end).await {
            Ok(()) => result.weekly_compacted += 1,
            Err(e) => result.errors.push(format!("weekly {}/{}: {}", project_path, week_start, e)),
        }
    }

    // 7. Monthly compaction for the current month (re-compact while month is in progress)
    let month_start = now.format("%Y-%m-01T00:00:00+00:00").to_string();
    let month_end = {
        let year = now.format("%Y").to_string().parse::<i32>().unwrap_or(2026);
        let month = now.format("%m").to_string().parse::<u32>().unwrap_or(1);
        let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
        format!("{:04}-{:02}-01T00:00:00+00:00", next_year, next_month)
    };

    // Find all projects that have weekly summaries this month
    let monthly_projects: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT DISTINCT ws.project_path
        FROM work_summaries ws
        WHERE ws.user_id = ? AND ws.scale = 'weekly'
            AND ws.period_start >= ? AND ws.period_start < ?
        "#,
    )
    .bind(user_id)
    .bind(&month_start)
    .bind(&month_end)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to find monthly projects: {}", e))?;

    for (project_path,) in &monthly_projects {
        match compact_period(pool, llm, user_id, Some(project_path), "monthly", &month_start, &month_end).await {
            Ok(()) => result.monthly_compacted += 1,
            Err(e) => result.errors.push(format!("monthly {}: {}", project_path, e)),
        }
    }

    log::info!(
        "Compaction cycle complete: {} hourly, {} daily, {} monthly, {} errors",
        result.hourly_compacted,
        result.daily_compacted,
        result.monthly_compacted,
        result.errors.len()
    );

    Ok(result)
}

// ============ Helpers ============

/// Get the previous period's summary for context chaining.
async fn get_previous_summary(
    pool: &SqlitePool,
    user_id: &str,
    project_path: Option<&str>,
    scale: &str,
    current_period_start: &str,
) -> Option<String> {
    let row: Option<WorkSummary> = sqlx::query_as(
        "SELECT * FROM work_summaries WHERE user_id = ? AND (project_path = ? OR (project_path IS NULL AND ? IS NULL)) AND scale = ? AND period_start < ? ORDER BY period_start DESC LIMIT 1",
    )
    .bind(user_id)
    .bind(project_path)
    .bind(project_path)
    .bind(scale)
    .bind(current_period_start)
    .fetch_optional(pool)
    .await
    .ok()?;

    row.map(|r| r.summary)
}

/// Aggregate data from multiple snapshots into a single text block.
fn aggregate_snapshots(snapshots: &[SnapshotRawData]) -> (String, Vec<&str>, String, String) {
    let mut all_user_messages = Vec::new();
    let mut all_tool_calls = Vec::new();
    let mut all_files = Vec::new();
    let mut all_commits = Vec::new();
    let mut snapshot_ids = Vec::new();

    for snapshot in snapshots {
        snapshot_ids.push(snapshot.id.as_str());

        if let Some(ref msgs) = snapshot.user_messages {
            if let Ok(parsed) = serde_json::from_str::<Vec<String>>(msgs) {
                all_user_messages.extend(parsed);
            }
        }

        if let Some(ref tools) = snapshot.tool_calls {
            if let Ok(parsed) = serde_json::from_str::<Vec<ToolCallRecord>>(tools) {
                for tc in &parsed {
                    all_tool_calls.push(format!("{}({})", tc.tool, tc.input_summary));
                }
            }
        }

        if let Some(ref files) = snapshot.files_modified {
            if let Ok(parsed) = serde_json::from_str::<Vec<String>>(files) {
                for f in parsed {
                    if !all_files.contains(&f) {
                        all_files.push(f);
                    }
                }
            }
        }

        if let Some(ref commits) = snapshot.git_commits {
            if let Ok(parsed) = serde_json::from_str::<Vec<CommitSnapshot>>(commits) {
                for c in &parsed {
                    all_commits.push(format!("{}: {} (+{}-{})", c.hash, c.message, c.additions, c.deletions));
                }
            }
        }
    }

    let mut data_parts = Vec::new();
    if !all_user_messages.is_empty() {
        data_parts.push(format!(
            "使用者訊息:\n{}",
            all_user_messages
                .iter()
                .take(10)
                .map(|m| format!("- {}", m))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if !all_tool_calls.is_empty() {
        data_parts.push(format!(
            "工具使用:\n{}",
            all_tool_calls
                .iter()
                .take(20)
                .map(|t| format!("- {}", t))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if !all_files.is_empty() {
        data_parts.push(format!("修改檔案: {}", all_files.join(", ")));
    }
    if !all_commits.is_empty() {
        data_parts.push(format!(
            "Git Commits:\n{}",
            all_commits
                .iter()
                .map(|c| format!("- {}", c))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }

    let current_data = data_parts.join("\n\n");

    let key_activities_json = serde_json::to_string(&all_files).unwrap_or_else(|_| "[]".to_string());
    let git_summary_json = serde_json::to_string(&all_commits).unwrap_or_else(|_| "[]".to_string());

    (current_data, snapshot_ids, key_activities_json, git_summary_json)
}

/// Build a rule-based summary when LLM is not available.
/// Produces a one-line summary followed by bullet-point details in markdown format.
fn build_rule_based_summary(current_data: &str, key_activities: &str, git_summary: &str) -> String {
    let commits: Vec<String> = serde_json::from_str(git_summary).unwrap_or_default();
    let files: Vec<String> = serde_json::from_str(key_activities).unwrap_or_default();

    if commits.is_empty() && files.is_empty() {
        return current_data.chars().take(200).collect();
    }

    // Build one-line summary
    let mut summary_parts = Vec::new();
    if !commits.is_empty() {
        summary_parts.push(format!("{} 筆 commit", commits.len()));
    }
    if !files.is_empty() {
        summary_parts.push(format!("修改 {} 個檔案", files.len()));
    }
    let summary_line = summary_parts.join("，");

    // Build bullet details
    let mut details = Vec::new();
    for commit in commits.iter().take(5) {
        details.push(format!("- {}", commit));
    }
    if !files.is_empty() {
        let file_list: Vec<&str> = files.iter().map(|s| s.as_str()).take(5).collect();
        details.push(format!("- 修改: `{}`", file_list.join("`, `")));
    }

    format!("{}\n\n{}", summary_line, details.join("\n"))
}

/// Merge multiple JSON array strings into one.
fn merge_json_arrays(arrays: &[String]) -> String {
    let mut merged: Vec<serde_json::Value> = Vec::new();
    for arr_str in arrays {
        if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(arr_str) {
            merged.extend(arr);
        }
    }
    serde_json::to_string(&merged).unwrap_or_else(|_| "[]".to_string())
}

/// Save a summary to the work_summaries table.
async fn save_summary(
    pool: &SqlitePool,
    user_id: &str,
    project_path: Option<&str>,
    scale: &str,
    period_start: &str,
    period_end: &str,
    summary: &str,
    key_activities: &str,
    git_commits_summary: &str,
    previous_context: Option<&str>,
    source_snapshot_ids: &[&str],
    llm_model: Option<&str>,
) -> Result<(), String> {
    let id = Uuid::new_v4().to_string();
    let source_ids_json =
        serde_json::to_string(source_snapshot_ids).unwrap_or_else(|_| "[]".to_string());

    sqlx::query(
        r#"
        INSERT INTO work_summaries (id, user_id, project_path, scale, period_start, period_end,
            summary, key_activities, git_commits_summary, previous_context,
            source_snapshot_ids, llm_model)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(user_id, project_path, scale, period_start) DO UPDATE SET
            summary = excluded.summary,
            key_activities = excluded.key_activities,
            git_commits_summary = excluded.git_commits_summary,
            previous_context = excluded.previous_context,
            source_snapshot_ids = excluded.source_snapshot_ids,
            llm_model = excluded.llm_model,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&id)
    .bind(user_id)
    .bind(project_path)
    .bind(scale)
    .bind(period_start)
    .bind(period_end)
    .bind(summary)
    .bind(key_activities)
    .bind(git_commits_summary)
    .bind(previous_context)
    .bind(&source_ids_json)
    .bind(llm_model)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to save {} summary: {}", scale, e))?;

    Ok(())
}

// ============ Tests ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_rule_based_summary_with_commits() {
        let data = "some work data";
        let activities = r#"["src/main.rs", "src/lib.rs"]"#;
        let commits = r#"["abc123: feat: add login (+50-10)"]"#;

        let summary = build_rule_based_summary(data, activities, commits);
        // First line: summary
        assert!(summary.contains("1 筆 commit"));
        assert!(summary.contains("修改 2 個檔案"));
        // Details section
        assert!(summary.contains("- abc123: feat: add login"));
        assert!(summary.contains("- 修改: `src/main.rs`"));
    }

    #[test]
    fn test_build_rule_based_summary_no_commits() {
        let data = "工具使用:\n- Edit(src/main.rs)";
        let activities = r#"["src/main.rs"]"#;
        let commits = "[]";

        let summary = build_rule_based_summary(data, activities, commits);
        // Summary line: only files
        assert!(summary.starts_with("修改 1 個檔案"));
        // Details
        assert!(summary.contains("- 修改: `src/main.rs`"));
    }

    #[test]
    fn test_build_rule_based_summary_fallback() {
        let data = "使用者在進行工作";
        let summary = build_rule_based_summary(data, "[]", "[]");
        assert!(summary.contains("使用者在進行工作"));
    }

    #[test]
    fn test_merge_json_arrays() {
        let arrays = vec![
            r#"["a", "b"]"#.to_string(),
            r#"["c"]"#.to_string(),
        ];
        let merged = merge_json_arrays(&arrays);
        let parsed: Vec<String> = serde_json::from_str(&merged).unwrap();
        assert_eq!(parsed, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_merge_json_arrays_empty() {
        let arrays: Vec<String> = vec![];
        let merged = merge_json_arrays(&arrays);
        assert_eq!(merged, "[]");
    }

    #[test]
    fn test_is_hour_completed_past() {
        assert!(is_hour_completed("2020-01-01T00:00:00"));
    }

    #[test]
    fn test_is_hour_completed_future() {
        assert!(!is_hour_completed("2099-12-31T23:00:00"));
    }

    #[test]
    fn test_is_day_completed_past() {
        assert!(is_day_completed("2020-01-01"));
    }

    #[test]
    fn test_is_day_completed_future() {
        assert!(!is_day_completed("2099-12-31"));
    }

    #[test]
    fn test_is_day_completed_with_period_start_format() {
        // period_start format: "2020-01-01T00:00:00+00:00"
        assert!(is_day_completed("2020-01-01T00:00:00+00:00"));
    }

    #[test]
    fn test_is_period_completed_past() {
        assert!(is_period_completed("2020-02-01T00:00:00+00:00"));
    }

    #[test]
    fn test_is_period_completed_future() {
        assert!(!is_period_completed("2099-02-01T00:00:00+00:00"));
    }

    #[test]
    fn test_aggregate_snapshots_basic() {
        let snapshot = SnapshotRawData {
            id: "snap-1".to_string(),
            user_id: "user1".to_string(),
            session_id: "sess1".to_string(),
            project_path: "/project".to_string(),
            hour_bucket: "2026-01-26T14:00:00+00:00".to_string(),
            user_messages: Some(r#"["help me implement login"]"#.to_string()),
            assistant_messages: Some(r#"["Sure, I will help"]"#.to_string()),
            tool_calls: Some(r#"[{"tool":"Edit","input_summary":"src/main.rs","timestamp":"2026-01-26T14:05:00+00:00"}]"#.to_string()),
            files_modified: Some(r#"["src/main.rs"]"#.to_string()),
            git_commits: Some(r#"[{"hash":"abc123","message":"feat: login","timestamp":"2026-01-26T14:30:00+00:00","additions":50,"deletions":10}]"#.to_string()),
            message_count: 1,
            raw_size_bytes: 100,
            created_at: Utc::now(),
        };

        let snapshots = [snapshot];
        let (data, ids, activities, git) = aggregate_snapshots(&snapshots);
        assert!(data.contains("help me implement login"));
        assert!(data.contains("Edit(src/main.rs)"));
        assert!(data.contains("abc123"));
        assert_eq!(ids.len(), 1);
        assert!(activities.contains("src/main.rs"));
        assert!(git.contains("feat: login"));
    }
}
