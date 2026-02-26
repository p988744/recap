//! Project summary commands
//!
//! Unified AI-powered summary generation with caching for project pages and timeline.
//! Supports two summary types:
//! - "report": Project overview summaries (paragraph style)
//! - "timeline": Timeline period summaries (bullet-point style)

use chrono::{Datelike, NaiveDate};
use recap_core::auth::verify_token;
use recap_core::models::WorkItem;
use recap_core::services::llm::{create_llm_service, LlmUsageRecord};
use recap_core::services::llm_usage::save_usage_log;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tauri::{Emitter, State, Window};
use uuid::Uuid;

use super::types::{GenerateSummaryRequest, ProjectSummaryResponse, SummaryFreshness};
use crate::commands::AppState;

// ============ Types ============

/// Summary type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SummaryType {
    Report,
    Timeline,
}

impl SummaryType {
    fn as_str(&self) -> &'static str {
        match self {
            SummaryType::Report => "report",
            SummaryType::Timeline => "timeline",
        }
    }
}

/// Progress event for summary generation
#[derive(Debug, Clone, Serialize)]
pub struct SummaryGenerationProgress {
    pub project_name: String,
    pub summary_type: String,
    pub current: usize,
    pub total: usize,
    pub period_label: String,
    pub phase: String, // "generating", "complete", "error"
    pub message: String,
}

/// Request for a single period summary
#[derive(Debug, Deserialize)]
pub struct PeriodSummaryRequest {
    pub period_start: String,
    pub period_end: String,
    pub period_label: String,
}

/// Batch request for generating multiple summaries
#[derive(Debug, Deserialize)]
pub struct BatchSummaryRequest {
    pub project_name: String,
    pub summary_type: String, // "report" | "timeline"
    pub time_unit: String,    // "day" | "week" | "month" | "quarter" | "year"
    pub periods: Vec<PeriodSummaryRequest>,
}

// ============ Helper Functions ============

/// Extract project name from work item title "[ProjectName] ..." pattern
fn extract_project_name(title: &str) -> Option<String> {
    if title.starts_with('[') {
        title
            .split(']')
            .next()
            .map(|s| s.trim_start_matches('[').to_string())
    } else {
        None
    }
}

/// Derive project name from either title pattern or project_path
fn derive_project_name(item: &WorkItem) -> String {
    if let Some(name) = extract_project_name(&item.title) {
        if !name.is_empty() {
            return name;
        }
    }
    if let Some(path) = &item.project_path {
        if let Some(last) = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
        {
            return last.to_string();
        }
    }
    "unknown".to_string()
}

/// Calculate data hash from work items to detect staleness
fn calculate_data_hash(items: &[WorkItem]) -> String {
    let mut hasher = Sha256::new();
    for item in items {
        hasher.update(item.id.as_bytes());
        hasher.update(item.title.as_bytes());
        hasher.update(format!("{}", item.hours).as_bytes());
        if let Some(desc) = &item.description {
            hasher.update(desc.as_bytes());
        }
    }
    format!("{:x}", hasher.finalize())
}

/// Fetch work items for a project within a date range
async fn fetch_work_items_for_project(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    project_name: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<WorkItem>, String> {
    let all_items: Vec<WorkItem> = sqlx::query_as(
        r#"SELECT * FROM work_items
           WHERE user_id = ? AND date >= ? AND date <= ?
           ORDER BY date DESC, created_at DESC"#,
    )
    .bind(user_id)
    .bind(start_date.format("%Y-%m-%d").to_string())
    .bind(end_date.format("%Y-%m-%d").to_string())
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let project_items: Vec<WorkItem> = all_items
        .into_iter()
        .filter(|item| derive_project_name(item) == project_name)
        .collect();

    Ok(project_items)
}

/// Build prompt for report-style summary (paragraph format)
fn build_report_prompt(
    project_name: &str,
    project_description: Option<&(Option<String>, Option<String>)>,
    work_items: &[WorkItem],
    time_unit: &str,
) -> String {
    let mut prompt = String::new();

    if let Some((goal, tech_stack)) = project_description {
        prompt.push_str(&format!("專案名稱：{}\n", project_name));
        if let Some(g) = goal {
            prompt.push_str(&format!("專案目標：{}\n", g));
        }
        if let Some(t) = tech_stack {
            prompt.push_str(&format!("技術棧：{}\n", t));
        }
        prompt.push('\n');
    }

    prompt.push_str("工作項目：\n");
    for item in work_items.iter().take(50) {
        let hours_str = format!("{:.1}h", item.hours);
        let title = item.title.replace(&format!("[{}] ", project_name), "");
        prompt.push_str(&format!("- {} ({}, {})\n", title, item.date, hours_str));
        if let Some(desc) = &item.description {
            let short_desc: String = desc.chars().take(100).collect();
            if !short_desc.is_empty() {
                prompt.push_str(&format!("  {}\n", short_desc));
            }
        }
    }

    let period_label = match time_unit {
        "day" => "今日",
        "week" => "本週",
        "month" => "本月",
        "quarter" => "本季",
        "year" => "今年",
        _ => "此期間",
    };

    prompt.push_str(&format!(
        r#"
請根據以上工作項目，產生{}的專案工作摘要（100-200字）。

要求：
1. 使用繁體中文
2. 突出主要完成的功能和成果
3. 提及使用的技術或工具
4. 簡潔有力，適合向主管報告
5. 不要列點，用段落式敘述

直接輸出摘要內容，不要加任何前綴或標題。"#,
        period_label
    ));

    prompt
}

/// Build prompt for timeline-style summary (bullet-point format)
fn build_timeline_prompt(project_name: &str, work_items: &[WorkItem], period_label: &str) -> String {
    let mut prompt = String::new();

    prompt.push_str(&format!("專案：{}\n", project_name));
    prompt.push_str(&format!("時間區間：{}\n\n", period_label));

    prompt.push_str("工作項目：\n");
    for item in work_items.iter().take(30) {
        let title = item.title.replace(&format!("[{}] ", project_name), "");
        prompt.push_str(&format!("- {} ({}, {:.1}h)\n", title, item.date, item.hours));
        if let Some(desc) = &item.description {
            let short_desc: String = desc.chars().take(200).collect();
            if !short_desc.is_empty() {
                prompt.push_str(&format!("  摘要：{}\n", short_desc));
            }
        }
    }

    prompt.push_str(
        r#"
請根據以上工作項目，產生此期間的開發脈絡摘要。

要求：
1. 使用繁體中文
2. 用條列式呈現（使用 • 符號）
3. 按時間順序整理開發脈絡和進度
4. 突出主要完成的功能、解決的問題
5. 保持簡潔但完整，每點約 15-30 字
6. 最多 5 點

範例格式：
• 建立專案基礎架構，設定 Tauri + React 開發環境
• 實作使用者認證模組，支援 JWT token 驗證
• 修復時間軸滾動問題，優化頁面效能

直接輸出條列內容，不要加任何前綴或標題。"#,
    );

    prompt
}

/// Call LLM service for summary generation
async fn call_llm_for_summary(
    llm: &recap_core::services::llm::LlmService,
    prompt: &str,
) -> Result<(String, LlmUsageRecord), String> {
    llm.summarize_work_period("", prompt, "weekly").await
}

// ============ Unified Commands ============

/// Get cached summary (works for both report and timeline)
#[tauri::command(rename_all = "camelCase")]
pub async fn get_cached_summary(
    state: State<'_, AppState>,
    token: String,
    project_name: String,
    summary_type: String,
    time_unit: String,
    period_start: String,
) -> Result<Option<String>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let result: Option<(String,)> = sqlx::query_as(
        r#"SELECT summary FROM project_summaries
           WHERE user_id = ? AND project_name = ? AND summary_type = ? AND time_unit = ? AND period_start = ?"#,
    )
    .bind(&claims.sub)
    .bind(&project_name)
    .bind(&summary_type)
    .bind(&time_unit)
    .bind(&period_start)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(result.map(|(s,)| s))
}

/// Get cached summaries in batch (for timeline view)
///
/// Fetches from TWO tables with different data sources:
/// 1. `project_summaries` - LLM-generated reports (sparse, triggered manually)
/// 2. `work_summaries` - Compaction results (dense, generated automatically)
///
/// Scale mapping: Frontend uses "day/week/month", backend compaction uses "daily/weekly/monthly"
#[tauri::command(rename_all = "camelCase")]
pub async fn get_cached_summaries_batch(
    state: State<'_, AppState>,
    token: String,
    project_name: String,
    summary_type: String,
    time_unit: String,
    period_starts: Vec<String>,
) -> Result<HashMap<String, String>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let mut summaries = HashMap::new();

    // Map frontend time_unit to work_summaries scale
    let scale = match time_unit.as_str() {
        "day" => "daily",
        "week" => "weekly",
        "month" => "monthly",
        "year" => "yearly",
        _ => &time_unit,
    };

    for period_start in period_starts {
        // First, try project_summaries table
        let result: Option<(String,)> = sqlx::query_as(
            r#"SELECT summary FROM project_summaries
               WHERE user_id = ? AND project_name = ? AND summary_type = ? AND time_unit = ? AND period_start = ?"#,
        )
        .bind(&claims.sub)
        .bind(&project_name)
        .bind(&summary_type)
        .bind(&time_unit)
        .bind(&period_start)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

        if let Some((summary,)) = result {
            summaries.insert(period_start.clone(), summary);
            continue;
        }

        // If not found, try work_summaries table
        // work_summaries uses project_path (full path) and period_start with timestamp format
        // period_start in work_summaries is like "2026-01-30T00:00:00" or "2026-01-30T00:00:00+00:00"
        let result: Option<(String,)> = sqlx::query_as(
            r#"SELECT summary FROM work_summaries
               WHERE user_id = ?
               AND project_path LIKE '%' || ?
               AND scale = ?
               AND (
                   DATE(period_start) = DATE(?)
                   OR period_start LIKE ? || '%'
               )
               LIMIT 1"#,
        )
        .bind(&claims.sub)
        .bind(&project_name)
        .bind(scale)
        .bind(&period_start)
        .bind(&period_start)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

        if let Some((summary,)) = result {
            summaries.insert(period_start, summary);
        }
    }

    Ok(summaries)
}

/// Trigger background generation of summaries
/// Emits "summary-generation-progress" events to the frontend
#[tauri::command(rename_all = "camelCase")]
pub async fn trigger_summaries_generation(
    state: State<'_, AppState>,
    window: Window,
    token: String,
    request: BatchSummaryRequest,
) -> Result<(), String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let user_id = claims.sub.clone();

    let pool = {
        let db = state.db.lock().await;
        db.pool.clone()
    };

    let summary_type = request.summary_type.clone();
    let time_unit = request.time_unit.clone();
    let project_name = request.project_name.clone();

    // Filter out periods that already have summaries
    let mut periods_to_generate = Vec::new();
    for period in request.periods {
        let exists: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM project_summaries WHERE user_id = ? AND project_name = ? AND summary_type = ? AND time_unit = ? AND period_start = ?",
        )
        .bind(&user_id)
        .bind(&project_name)
        .bind(&summary_type)
        .bind(&time_unit)
        .bind(&period.period_start)
        .fetch_optional(&pool)
        .await
        .map_err(|e| e.to_string())?;

        if exists.is_none() {
            periods_to_generate.push(period);
        }
    }

    if periods_to_generate.is_empty() {
        let _ = window.emit(
            "summary-generation-progress",
            SummaryGenerationProgress {
                project_name: project_name.clone(),
                summary_type: summary_type.clone(),
                current: 0,
                total: 0,
                period_label: String::new(),
                phase: "complete".to_string(),
                message: "所有摘要已存在".to_string(),
            },
        );
        return Ok(());
    }

    let total = periods_to_generate.len();

    // Spawn background task
    tauri::async_runtime::spawn(async move {
        let llm = match create_llm_service(&pool, &user_id).await {
            Ok(llm) => llm,
            Err(e) => {
                let _ = window.emit(
                    "summary-generation-progress",
                    SummaryGenerationProgress {
                        project_name,
                        summary_type,
                        current: 0,
                        total,
                        period_label: String::new(),
                        phase: "error".to_string(),
                        message: format!("LLM 服務錯誤: {}", e),
                    },
                );
                return;
            }
        };

        if !llm.is_configured() {
            let _ = window.emit(
                "summary-generation-progress",
                SummaryGenerationProgress {
                    project_name,
                    summary_type,
                    current: 0,
                    total,
                    period_label: String::new(),
                    phase: "error".to_string(),
                    message: "LLM 服務未設定，請在設定頁面配置 API Key".to_string(),
                },
            );
            return;
        }

        let mut generated_count = 0;

        for (idx, period) in periods_to_generate.iter().enumerate() {
            let _ = window.emit(
                "summary-generation-progress",
                SummaryGenerationProgress {
                    project_name: project_name.clone(),
                    summary_type: summary_type.clone(),
                    current: idx + 1,
                    total,
                    period_label: period.period_label.clone(),
                    phase: "generating".to_string(),
                    message: format!("正在生成 {} 的摘要...", period.period_label),
                },
            );

            let start_date = match NaiveDate::parse_from_str(&period.period_start, "%Y-%m-%d") {
                Ok(d) => d,
                Err(_) => continue,
            };
            let end_date = match NaiveDate::parse_from_str(&period.period_end, "%Y-%m-%d") {
                Ok(d) => d,
                Err(_) => continue,
            };

            let work_items = match fetch_work_items_for_project(
                &pool,
                &user_id,
                &project_name,
                start_date,
                end_date,
            )
            .await
            {
                Ok(items) => items,
                Err(_) => continue,
            };

            if work_items.is_empty() {
                continue;
            }

            // Build prompt based on summary type
            let prompt = if summary_type == "timeline" {
                build_timeline_prompt(&project_name, &work_items, &period.period_label)
            } else {
                build_report_prompt(&project_name, None, &work_items, &time_unit)
            };

            match call_llm_for_summary(&llm, &prompt).await {
                Ok((summary, usage)) => {
                    let _ = save_usage_log(&pool, &user_id, &usage).await;
                    let data_hash = calculate_data_hash(&work_items);

                    let id = Uuid::new_v4().to_string();
                    let _ = sqlx::query(
                        r#"INSERT INTO project_summaries (id, user_id, project_name, summary_type, time_unit, period_start, period_end, period_label, summary, data_hash)
                           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                           ON CONFLICT(user_id, project_name, summary_type, time_unit, period_start) DO UPDATE SET
                               summary = excluded.summary,
                               data_hash = excluded.data_hash,
                               period_label = excluded.period_label,
                               orphaned = 0,
                               orphaned_at = NULL,
                               created_at = CURRENT_TIMESTAMP"#,
                    )
                    .bind(&id)
                    .bind(&user_id)
                    .bind(&project_name)
                    .bind(&summary_type)
                    .bind(&time_unit)
                    .bind(&period.period_start)
                    .bind(&period.period_end)
                    .bind(&period.period_label)
                    .bind(&summary)
                    .bind(&data_hash)
                    .execute(&pool)
                    .await;

                    // Emit individual summary generated event
                    let _ = window.emit(
                        "summary-generated",
                        serde_json::json!({
                            "project_name": project_name,
                            "summary_type": summary_type,
                            "time_unit": time_unit,
                            "period_start": period.period_start,
                            "summary": summary,
                        }),
                    );

                    generated_count += 1;
                }
                Err(e) => {
                    log::warn!(
                        "Failed to generate {} summary for {}: {}",
                        summary_type,
                        period.period_label,
                        e
                    );
                }
            }
        }

        let _ = window.emit(
            "summary-generation-progress",
            SummaryGenerationProgress {
                project_name,
                summary_type,
                current: total,
                total,
                period_label: String::new(),
                phase: "complete".to_string(),
                message: format!("已生成 {} 個摘要", generated_count),
            },
        );
    });

    Ok(())
}

// ============ Completed Period Summary Generation ============

/// Check if a period has ended (is complete)
fn is_period_completed(period_end: &str, time_unit: &str) -> bool {
    let today = chrono::Local::now().date_naive();

    if let Ok(end_date) = NaiveDate::parse_from_str(period_end, "%Y-%m-%d") {
        match time_unit {
            "day" => end_date < today,
            "week" => end_date < today,
            "month" => end_date < today,
            "quarter" => end_date < today,
            "year" => end_date < today,
            _ => end_date < today,
        }
    } else {
        false
    }
}

/// Generate summaries for all completed periods that don't have summaries yet
/// This should be called after sync to pre-generate summaries
#[tauri::command(rename_all = "camelCase")]
pub async fn generate_completed_summaries(
    state: State<'_, AppState>,
    window: Window,
    token: String,
    project_name: String,
    time_unit: String,
) -> Result<(), String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let user_id = claims.sub.clone();

    let pool = {
        let db = state.db.lock().await;
        db.pool.clone()
    };

    // Get date range for this time unit (look back based on time unit)
    let today = chrono::Local::now().date_naive();
    let range_start = match time_unit.as_str() {
        "day" => today - chrono::Duration::days(30),
        "week" => today - chrono::Duration::weeks(12),
        "month" => today - chrono::Duration::days(365),
        "quarter" => today - chrono::Duration::days(730),
        "year" => today - chrono::Duration::days(1825),
        _ => today - chrono::Duration::days(90),
    };

    // Get work items to determine which periods have activity
    let work_items: Vec<(String,)> = sqlx::query_as(
        r#"SELECT DISTINCT date FROM work_items
           WHERE user_id = ? AND date >= ? AND date <= ?
           AND (title LIKE ? OR project_path LIKE ?)
           ORDER BY date DESC"#,
    )
    .bind(&user_id)
    .bind(range_start.format("%Y-%m-%d").to_string())
    .bind(today.format("%Y-%m-%d").to_string())
    .bind(format!("[{}]%", project_name))
    .bind(format!("%/{}", project_name))
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;

    if work_items.is_empty() {
        return Ok(());
    }

    // Build periods from dates based on time_unit
    let mut periods_to_check: Vec<(String, String, String)> = Vec::new(); // (start, end, label)

    for (date_str,) in &work_items {
        if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            let (period_start, period_end, period_label) = match time_unit.as_str() {
                "day" => (
                    date.format("%Y-%m-%d").to_string(),
                    date.format("%Y-%m-%d").to_string(),
                    date.format("%Y-%m-%d").to_string(),
                ),
                "week" => {
                    let week_start = date - chrono::Duration::days(date.weekday().num_days_from_monday() as i64);
                    let week_end = week_start + chrono::Duration::days(6);
                    let week_num = date.iso_week().week();
                    (
                        week_start.format("%Y-%m-%d").to_string(),
                        week_end.format("%Y-%m-%d").to_string(),
                        format!("{} W{:02}", date.year(), week_num),
                    )
                },
                "month" => {
                    let month_start = NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap();
                    let next_month = if date.month() == 12 {
                        NaiveDate::from_ymd_opt(date.year() + 1, 1, 1).unwrap()
                    } else {
                        NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1).unwrap()
                    };
                    let month_end = next_month - chrono::Duration::days(1);
                    (
                        month_start.format("%Y-%m-%d").to_string(),
                        month_end.format("%Y-%m-%d").to_string(),
                        date.format("%Y-%m").to_string(),
                    )
                },
                _ => continue,
            };

            // Only add completed periods
            if is_period_completed(&period_end, &time_unit) {
                if !periods_to_check.iter().any(|(s, _, _)| s == &period_start) {
                    periods_to_check.push((period_start, period_end, period_label));
                }
            }
        }
    }

    if periods_to_check.is_empty() {
        return Ok(());
    }

    // Filter out periods that already have summaries
    let mut periods_to_generate: Vec<PeriodSummaryRequest> = Vec::new();

    for (period_start, period_end, period_label) in periods_to_check {
        let exists: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM project_summaries WHERE user_id = ? AND project_name = ? AND summary_type = 'timeline' AND time_unit = ? AND period_start = ?",
        )
        .bind(&user_id)
        .bind(&project_name)
        .bind(&time_unit)
        .bind(&period_start)
        .fetch_optional(&pool)
        .await
        .map_err(|e| e.to_string())?;

        if exists.is_none() {
            periods_to_generate.push(PeriodSummaryRequest {
                period_start,
                period_end,
                period_label,
            });
        }
    }

    if periods_to_generate.is_empty() {
        log::info!("All completed periods already have summaries for {} ({})", project_name, time_unit);
        return Ok(());
    }

    log::info!(
        "Generating {} summaries for completed periods of {} ({})",
        periods_to_generate.len(),
        project_name,
        time_unit
    );

    // Trigger generation in background
    let total = periods_to_generate.len();
    let summary_type = "timeline".to_string();

    tauri::async_runtime::spawn(async move {
        let llm = match create_llm_service(&pool, &user_id).await {
            Ok(llm) => llm,
            Err(e) => {
                log::error!("Failed to create LLM service: {}", e);
                return;
            }
        };

        if !llm.is_configured() {
            log::warn!("LLM not configured, skipping summary generation");
            return;
        }

        for (idx, period) in periods_to_generate.iter().enumerate() {
            let _ = window.emit(
                "summary-generation-progress",
                SummaryGenerationProgress {
                    project_name: project_name.clone(),
                    summary_type: summary_type.clone(),
                    current: idx + 1,
                    total,
                    period_label: period.period_label.clone(),
                    phase: "generating".to_string(),
                    message: format!("背景生成 {} 的摘要...", period.period_label),
                },
            );

            let start_date = match NaiveDate::parse_from_str(&period.period_start, "%Y-%m-%d") {
                Ok(d) => d,
                Err(_) => continue,
            };
            let end_date = match NaiveDate::parse_from_str(&period.period_end, "%Y-%m-%d") {
                Ok(d) => d,
                Err(_) => continue,
            };

            let work_items = match fetch_work_items_for_project(
                &pool,
                &user_id,
                &project_name,
                start_date,
                end_date,
            )
            .await
            {
                Ok(items) => items,
                Err(_) => continue,
            };

            if work_items.is_empty() {
                continue;
            }

            let prompt = build_timeline_prompt(&project_name, &work_items, &period.period_label);

            match call_llm_for_summary(&llm, &prompt).await {
                Ok((summary, usage)) => {
                    let _ = save_usage_log(&pool, &user_id, &usage).await;
                    let data_hash = calculate_data_hash(&work_items);

                    let id = Uuid::new_v4().to_string();
                    let _ = sqlx::query(
                        r#"INSERT INTO project_summaries (id, user_id, project_name, summary_type, time_unit, period_start, period_end, period_label, summary, data_hash)
                           VALUES (?, ?, ?, 'timeline', ?, ?, ?, ?, ?, ?)
                           ON CONFLICT(user_id, project_name, summary_type, time_unit, period_start) DO UPDATE SET
                               summary = excluded.summary,
                               data_hash = excluded.data_hash,
                               period_label = excluded.period_label,
                               orphaned = 0,
                               orphaned_at = NULL,
                               created_at = CURRENT_TIMESTAMP"#,
                    )
                    .bind(&id)
                    .bind(&user_id)
                    .bind(&project_name)
                    .bind(&time_unit)
                    .bind(&period.period_start)
                    .bind(&period.period_end)
                    .bind(&period.period_label)
                    .bind(&summary)
                    .bind(&data_hash)
                    .execute(&pool)
                    .await;

                    log::info!("Generated summary for {} {}", project_name, period.period_label);
                }
                Err(e) => {
                    log::warn!(
                        "Failed to generate summary for {} {}: {}",
                        project_name,
                        period.period_label,
                        e
                    );
                }
            }
        }

        let _ = window.emit(
            "summary-generation-progress",
            SummaryGenerationProgress {
                project_name,
                summary_type,
                current: total,
                total,
                period_label: String::new(),
                phase: "complete".to_string(),
                message: format!("已完成 {} 個摘要", total),
            },
        );
    });

    Ok(())
}

// ============ Legacy Commands (for backwards compatibility) ============

/// Get cached project summary for a period (legacy)
#[tauri::command(rename_all = "camelCase")]
pub async fn get_project_summary(
    state: State<'_, AppState>,
    token: String,
    project_name: String,
    period_type: String,
    period_start: String,
    period_end: String,
) -> Result<ProjectSummaryResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Map period_type to time_unit
    let time_unit = &period_type;

    let cached: Option<(String, Option<String>, String)> = sqlx::query_as(
        r#"SELECT summary, data_hash, datetime(created_at) as created_at
           FROM project_summaries
           WHERE user_id = ? AND project_name = ? AND summary_type = 'report' AND time_unit = ? AND period_start = ?"#,
    )
    .bind(&claims.sub)
    .bind(&project_name)
    .bind(time_unit)
    .bind(&period_start)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    if let Some((summary, data_hash, created_at)) = cached {
        let start_date = NaiveDate::parse_from_str(&period_start, "%Y-%m-%d")
            .map_err(|e| format!("Invalid period_start: {}", e))?;
        let end_date = NaiveDate::parse_from_str(&period_end, "%Y-%m-%d")
            .map_err(|e| format!("Invalid period_end: {}", e))?;

        let work_items =
            fetch_work_items_for_project(&db.pool, &claims.sub, &project_name, start_date, end_date)
                .await?;

        let current_hash = calculate_data_hash(&work_items);
        let is_stale = data_hash.as_deref() != Some(current_hash.as_str());

        return Ok(ProjectSummaryResponse {
            summary: Some(summary),
            period_type,
            period_start,
            period_end,
            is_stale,
            generated_at: Some(created_at),
        });
    }

    Ok(ProjectSummaryResponse {
        summary: None,
        period_type,
        period_start,
        period_end,
        is_stale: false,
        generated_at: None,
    })
}

/// Generate a new project summary using LLM (legacy)
#[tauri::command(rename_all = "camelCase")]
pub async fn generate_project_summary(
    state: State<'_, AppState>,
    token: String,
    request: GenerateSummaryRequest,
) -> Result<ProjectSummaryResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let pool = {
        let db = state.db.lock().await;
        db.pool.clone()
    };

    let start_date = NaiveDate::parse_from_str(&request.period_start, "%Y-%m-%d")
        .map_err(|e| format!("Invalid period_start: {}", e))?;
    let end_date = NaiveDate::parse_from_str(&request.period_end, "%Y-%m-%d")
        .map_err(|e| format!("Invalid period_end: {}", e))?;

    let time_unit = &request.period_type;

    // Check cache unless force_regenerate
    if !request.force_regenerate {
        let cached: Option<(String, Option<String>, String)> = sqlx::query_as(
            r#"SELECT summary, data_hash, datetime(created_at) as created_at
               FROM project_summaries
               WHERE user_id = ? AND project_name = ? AND summary_type = 'report' AND time_unit = ? AND period_start = ?"#,
        )
        .bind(&claims.sub)
        .bind(&request.project_name)
        .bind(time_unit)
        .bind(&request.period_start)
        .fetch_optional(&pool)
        .await
        .map_err(|e| e.to_string())?;

        if let Some((summary, data_hash, created_at)) = cached {
            let work_items = fetch_work_items_for_project(
                &pool,
                &claims.sub,
                &request.project_name,
                start_date,
                end_date,
            )
            .await?;

            let current_hash = calculate_data_hash(&work_items);
            let is_stale = data_hash.as_deref() != Some(current_hash.as_str());

            if !is_stale {
                return Ok(ProjectSummaryResponse {
                    summary: Some(summary),
                    period_type: request.period_type,
                    period_start: request.period_start,
                    period_end: request.period_end,
                    is_stale: false,
                    generated_at: Some(created_at),
                });
            }
        }
    }

    let work_items = fetch_work_items_for_project(
        &pool,
        &claims.sub,
        &request.project_name,
        start_date,
        end_date,
    )
    .await?;

    if work_items.is_empty() {
        return Ok(ProjectSummaryResponse {
            summary: None,
            period_type: request.period_type,
            period_start: request.period_start,
            period_end: request.period_end,
            is_stale: false,
            generated_at: None,
        });
    }

    let project_desc: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT goal, tech_stack FROM project_descriptions WHERE user_id = ? AND project_name = ?",
    )
    .bind(&claims.sub)
    .bind(&request.project_name)
    .fetch_optional(&pool)
    .await
    .map_err(|e| e.to_string())?;

    let llm = create_llm_service(&pool, &claims.sub).await?;

    if !llm.is_configured() {
        return Err("LLM 服務未設定。請在設定頁面配置 API Key。".to_string());
    }

    let prompt = build_report_prompt(
        &request.project_name,
        project_desc.as_ref(),
        &work_items,
        time_unit,
    );

    let (summary, usage) = call_llm_for_summary(&llm, &prompt).await?;
    let _ = save_usage_log(&pool, &claims.sub, &usage).await;

    let data_hash = calculate_data_hash(&work_items);

    let id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"INSERT INTO project_summaries (id, user_id, project_name, summary_type, time_unit, period_start, period_end, summary, data_hash)
           VALUES (?, ?, ?, 'report', ?, ?, ?, ?, ?)
           ON CONFLICT(user_id, project_name, summary_type, time_unit, period_start) DO UPDATE SET
               summary = excluded.summary,
               data_hash = excluded.data_hash,
               orphaned = 0,
               orphaned_at = NULL,
               created_at = CURRENT_TIMESTAMP"#,
    )
    .bind(&id)
    .bind(&claims.sub)
    .bind(&request.project_name)
    .bind(time_unit)
    .bind(&request.period_start)
    .bind(&request.period_end)
    .bind(&summary)
    .bind(&data_hash)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    let created_at: (String,) = sqlx::query_as(
        "SELECT datetime(created_at) FROM project_summaries WHERE user_id = ? AND project_name = ? AND summary_type = 'report' AND time_unit = ? AND period_start = ?",
    )
    .bind(&claims.sub)
    .bind(&request.project_name)
    .bind(time_unit)
    .bind(&request.period_start)
    .fetch_one(&pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(ProjectSummaryResponse {
        summary: Some(summary),
        period_type: request.period_type,
        period_start: request.period_start,
        period_end: request.period_end,
        is_stale: false,
        generated_at: Some(created_at.0),
    })
}

/// Check if a summary needs regeneration (legacy)
#[tauri::command(rename_all = "camelCase")]
pub async fn check_summary_freshness(
    state: State<'_, AppState>,
    token: String,
    project_name: String,
    period_type: String,
    period_start: String,
    period_end: String,
) -> Result<SummaryFreshness, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let start_date = NaiveDate::parse_from_str(&period_start, "%Y-%m-%d")
        .map_err(|e| format!("Invalid period_start: {}", e))?;
    let end_date = NaiveDate::parse_from_str(&period_end, "%Y-%m-%d")
        .map_err(|e| format!("Invalid period_end: {}", e))?;

    let cached: Option<(Option<String>, String)> = sqlx::query_as(
        "SELECT data_hash, datetime(created_at) FROM project_summaries WHERE user_id = ? AND project_name = ? AND summary_type = 'report' AND time_unit = ? AND period_start = ?",
    )
    .bind(&claims.sub)
    .bind(&project_name)
    .bind(&period_type)
    .bind(&period_start)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let work_items =
        fetch_work_items_for_project(&db.pool, &claims.sub, &project_name, start_date, end_date)
            .await?;

    let last_activity_date = work_items.iter().map(|w| w.date).max().map(|d| d.to_string());

    match cached {
        Some((data_hash, summary_date)) => {
            let current_hash = calculate_data_hash(&work_items);
            let has_new_activity = data_hash.as_deref() != Some(current_hash.as_str());

            Ok(SummaryFreshness {
                project_name,
                has_new_activity,
                last_activity_date,
                last_summary_date: Some(summary_date),
            })
        }
        None => Ok(SummaryFreshness {
            project_name,
            has_new_activity: !work_items.is_empty(),
            last_activity_date,
            last_summary_date: None,
        }),
    }
}

// ============ Background Sync Integration ============

/// Time units for summary generation (ordered by priority)
pub const SUMMARY_TIME_UNITS: &[&str] = &["day", "week", "month", "quarter", "year"];

/// Generate timeline summaries for all completed periods across all projects
/// This is called from the background sync service after data sync completes.
///
/// Parameters:
/// - `pool`: Database connection pool
/// - `user_id`: User ID for the summaries
/// - `time_units`: Which time units to generate (e.g., ["week", "month"])
///
/// Returns: Number of summaries generated
pub async fn generate_all_completed_summaries(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    time_units: &[&str],
) -> Result<usize, String> {
    // Get all distinct projects that have work items
    let projects: Vec<(String,)> = sqlx::query_as(
        r#"SELECT DISTINCT
           CASE
               WHEN title LIKE '[%]%' THEN substr(title, 2, instr(title, ']') - 2)
               WHEN project_path IS NOT NULL THEN replace(project_path, rtrim(project_path, replace(project_path, '/', '')), '')
               ELSE 'unknown'
           END as project_name
           FROM work_items
           WHERE user_id = ?
           HAVING project_name != 'unknown' AND project_name != ''"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    if projects.is_empty() {
        return Ok(0);
    }

    // Initialize LLM service once
    let llm = match recap_core::services::llm::create_llm_service(pool, user_id).await {
        Ok(llm) => llm,
        Err(e) => {
            log::warn!("LLM service not available: {}", e);
            return Ok(0);
        }
    };

    if !llm.is_configured() {
        log::info!("LLM not configured, skipping summary generation");
        return Ok(0);
    }

    let mut total_generated = 0;
    let today = chrono::Local::now().date_naive();

    for (project_name,) in &projects {
        for &time_unit in time_units {
            // Calculate lookback period based on time unit
            let range_start = match time_unit {
                "day" => today - chrono::Duration::days(7),      // Last 7 days
                "week" => today - chrono::Duration::weeks(4),    // Last 4 weeks
                "month" => today - chrono::Duration::days(90),   // Last 3 months
                "quarter" => today - chrono::Duration::days(365), // Last year
                "year" => today - chrono::Duration::days(1095),   // Last 3 years
                _ => today - chrono::Duration::days(30),
            };

            // Find completed periods that don't have summaries
            let periods = find_missing_completed_periods(
                pool,
                user_id,
                project_name,
                time_unit,
                range_start,
                today,
            )
            .await?;

            if periods.is_empty() {
                continue;
            }

            log::info!(
                "Generating {} {} summaries for {}",
                periods.len(),
                time_unit,
                project_name
            );

            for period in &periods {
                let start_date = match NaiveDate::parse_from_str(&period.period_start, "%Y-%m-%d") {
                    Ok(d) => d,
                    Err(_) => continue,
                };
                let end_date = match NaiveDate::parse_from_str(&period.period_end, "%Y-%m-%d") {
                    Ok(d) => d,
                    Err(_) => continue,
                };

                let work_items = match fetch_work_items_for_project(
                    pool,
                    user_id,
                    project_name,
                    start_date,
                    end_date,
                )
                .await
                {
                    Ok(items) => items,
                    Err(_) => continue,
                };

                if work_items.is_empty() {
                    continue;
                }

                let prompt = build_timeline_prompt(project_name, &work_items, &period.period_label);

                match call_llm_for_summary(&llm, &prompt).await {
                    Ok((summary, usage)) => {
                        let _ = recap_core::services::llm_usage::save_usage_log(pool, user_id, &usage).await;
                        let data_hash = calculate_data_hash(&work_items);

                        let id = Uuid::new_v4().to_string();
                        let _ = sqlx::query(
                            r#"INSERT INTO project_summaries (id, user_id, project_name, summary_type, time_unit, period_start, period_end, period_label, summary, data_hash)
                               VALUES (?, ?, ?, 'timeline', ?, ?, ?, ?, ?, ?)
                               ON CONFLICT(user_id, project_name, summary_type, time_unit, period_start) DO UPDATE SET
                                   summary = excluded.summary,
                                   data_hash = excluded.data_hash,
                                   period_label = excluded.period_label,
                                   orphaned = 0,
                                   orphaned_at = NULL,
                                   created_at = CURRENT_TIMESTAMP"#,
                        )
                        .bind(&id)
                        .bind(user_id)
                        .bind(project_name)
                        .bind(time_unit)
                        .bind(&period.period_start)
                        .bind(&period.period_end)
                        .bind(&period.period_label)
                        .bind(&summary)
                        .bind(&data_hash)
                        .execute(pool)
                        .await;

                        total_generated += 1;
                        log::debug!(
                            "Generated {} summary for {} {}",
                            time_unit,
                            project_name,
                            period.period_label
                        );
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to generate {} summary for {} {}: {}",
                            time_unit,
                            project_name,
                            period.period_label,
                            e
                        );
                    }
                }
            }
        }
    }

    if total_generated > 0 {
        log::info!("Generated {} timeline summaries in background", total_generated);
    }

    Ok(total_generated)
}

/// Find completed periods that are missing summaries
async fn find_missing_completed_periods(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    project_name: &str,
    time_unit: &str,
    range_start: NaiveDate,
    today: NaiveDate,
) -> Result<Vec<PeriodSummaryRequest>, String> {
    use chrono::{Datelike, IsoWeek};

    // Get all dates with work items for this project in the range
    let dates: Vec<(String,)> = sqlx::query_as(
        r#"SELECT DISTINCT date FROM work_items
           WHERE user_id = ? AND date >= ? AND date <= ?
           AND (title LIKE ? OR project_path LIKE ?)
           ORDER BY date"#,
    )
    .bind(user_id)
    .bind(range_start.format("%Y-%m-%d").to_string())
    .bind(today.format("%Y-%m-%d").to_string())
    .bind(format!("[{}]%", project_name))
    .bind(format!("%/{}", project_name))
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut periods_map: std::collections::HashMap<String, (String, String, String)> = std::collections::HashMap::new();

    for (date_str,) in &dates {
        if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            let (period_start, period_end, period_label) = match time_unit {
                "day" => (
                    date.format("%Y-%m-%d").to_string(),
                    date.format("%Y-%m-%d").to_string(),
                    date.format("%Y-%m-%d").to_string(),
                ),
                "week" => {
                    let week_start = date - chrono::Duration::days(date.weekday().num_days_from_monday() as i64);
                    let week_end = week_start + chrono::Duration::days(6);
                    let week_num = date.iso_week().week();
                    (
                        week_start.format("%Y-%m-%d").to_string(),
                        week_end.format("%Y-%m-%d").to_string(),
                        format!("{} W{:02}", date.year(), week_num),
                    )
                }
                "month" => {
                    let month_start = NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap();
                    let next_month = if date.month() == 12 {
                        NaiveDate::from_ymd_opt(date.year() + 1, 1, 1).unwrap()
                    } else {
                        NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1).unwrap()
                    };
                    let month_end = next_month - chrono::Duration::days(1);
                    (
                        month_start.format("%Y-%m-%d").to_string(),
                        month_end.format("%Y-%m-%d").to_string(),
                        date.format("%Y-%m").to_string(),
                    )
                }
                "quarter" => {
                    let quarter = (date.month() - 1) / 3;
                    let quarter_start_month = quarter * 3 + 1;
                    let quarter_start = NaiveDate::from_ymd_opt(date.year(), quarter_start_month, 1).unwrap();
                    let next_quarter = if quarter == 3 {
                        NaiveDate::from_ymd_opt(date.year() + 1, 1, 1).unwrap()
                    } else {
                        NaiveDate::from_ymd_opt(date.year(), quarter_start_month + 3, 1).unwrap()
                    };
                    let quarter_end = next_quarter - chrono::Duration::days(1);
                    (
                        quarter_start.format("%Y-%m-%d").to_string(),
                        quarter_end.format("%Y-%m-%d").to_string(),
                        format!("{} Q{}", date.year(), quarter + 1),
                    )
                }
                "year" => {
                    let year_start = NaiveDate::from_ymd_opt(date.year(), 1, 1).unwrap();
                    let year_end = NaiveDate::from_ymd_opt(date.year(), 12, 31).unwrap();
                    (
                        year_start.format("%Y-%m-%d").to_string(),
                        year_end.format("%Y-%m-%d").to_string(),
                        format!("{}", date.year()),
                    )
                }
                _ => continue,
            };

            // Only include completed periods
            if let Ok(end_date) = NaiveDate::parse_from_str(&period_end, "%Y-%m-%d") {
                if end_date < today {
                    periods_map.insert(period_start.clone(), (period_start, period_end, period_label));
                }
            }
        }
    }

    // Filter out periods that already have summaries
    let mut missing_periods = Vec::new();
    for (period_start, period_end, period_label) in periods_map.values() {
        let exists: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM project_summaries WHERE user_id = ? AND project_name = ? AND summary_type = 'timeline' AND time_unit = ? AND period_start = ?",
        )
        .bind(user_id)
        .bind(project_name)
        .bind(time_unit)
        .bind(period_start)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

        if exists.is_none() {
            missing_periods.push(PeriodSummaryRequest {
                period_start: period_start.clone(),
                period_end: period_end.clone(),
                period_label: period_label.clone(),
            });
        }
    }

    // Sort by period_start
    missing_periods.sort_by(|a, b| a.period_start.cmp(&b.period_start));

    Ok(missing_periods)
}

// ============ Tests ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_project_name_from_title() {
        let item = WorkItem {
            id: "1".to_string(),
            user_id: "user".to_string(),
            source: "claude_code".to_string(),
            source_id: None,
            source_url: None,
            title: "[recap] Implement feature X".to_string(),
            description: None,
            hours: 1.0,
            date: NaiveDate::from_ymd_opt(2026, 1, 30).unwrap(),
            jira_issue_key: None,
            jira_issue_suggested: None,
            jira_issue_title: None,
            category: None,
            tags: None,
            yearly_goal_id: None,
            synced_to_tempo: false,
            tempo_worklog_id: None,
            synced_at: None,
            parent_id: None,
            hours_source: None,
            hours_estimated: None,
            commit_hash: None,
            session_id: None,
            start_time: None,
            end_time: None,
            project_path: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert_eq!(derive_project_name(&item), "recap");
    }

    #[test]
    fn test_derive_project_name_from_path() {
        let item = WorkItem {
            id: "1".to_string(),
            user_id: "user".to_string(),
            source: "claude_code".to_string(),
            source_id: None,
            source_url: None,
            title: "Working on something".to_string(),
            description: None,
            hours: 1.0,
            date: NaiveDate::from_ymd_opt(2026, 1, 30).unwrap(),
            jira_issue_key: None,
            jira_issue_suggested: None,
            jira_issue_title: None,
            category: None,
            tags: None,
            yearly_goal_id: None,
            synced_to_tempo: false,
            tempo_worklog_id: None,
            synced_at: None,
            parent_id: None,
            hours_source: None,
            hours_estimated: None,
            commit_hash: None,
            session_id: None,
            start_time: None,
            end_time: None,
            project_path: Some("/home/user/projects/my-app".to_string()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert_eq!(derive_project_name(&item), "my-app");
    }

    #[test]
    fn test_calculate_data_hash() {
        let item1 = WorkItem {
            id: "1".to_string(),
            user_id: "user".to_string(),
            source: "manual".to_string(),
            source_id: None,
            source_url: None,
            title: "Task 1".to_string(),
            description: Some("Description".to_string()),
            hours: 2.0,
            date: NaiveDate::from_ymd_opt(2026, 1, 30).unwrap(),
            jira_issue_key: None,
            jira_issue_suggested: None,
            jira_issue_title: None,
            category: None,
            tags: None,
            yearly_goal_id: None,
            synced_to_tempo: false,
            tempo_worklog_id: None,
            synced_at: None,
            parent_id: None,
            hours_source: None,
            hours_estimated: None,
            commit_hash: None,
            session_id: None,
            start_time: None,
            end_time: None,
            project_path: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let hash1 = calculate_data_hash(&[item1.clone()]);
        let hash2 = calculate_data_hash(&[item1]);
        assert_eq!(hash1, hash2);
        assert!(!hash1.is_empty());
    }
}
