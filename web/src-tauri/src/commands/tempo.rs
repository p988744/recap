//! Tempo commands
//!
//! Tauri commands for Jira/Tempo integration operations.

use serde::{Deserialize, Serialize};
use tauri::State;

use recap_core::auth::verify_token;
use recap_core::services::llm::{create_llm_service, parse_error_usage};
use recap_core::services::llm_usage::save_usage_log;
use recap_core::services::tempo::{JiraAuthType, JiraClient, TempoClient, WorklogEntry, WorklogUploader};

use super::AppState;

// Types

#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct WorklogEntryRequest {
    pub issue_key: String,
    pub date: String,
    pub minutes: i64,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct WorklogEntryResponse {
    pub id: Option<String>,
    pub issue_key: String,
    pub date: String,
    pub minutes: i64,
    pub hours: f64,
    pub description: String,
    pub status: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SyncWorklogsRequest {
    pub entries: Vec<WorklogEntryRequest>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Serialize)]
pub struct SyncWorklogsResponse {
    pub success: bool,
    pub total_entries: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<WorklogEntryResponse>,
    pub dry_run: bool,
}

#[derive(Debug, Deserialize)]
pub struct GetWorklogsRequest {
    pub date_from: String,
    pub date_to: String,
}

#[derive(Debug, Serialize)]
pub struct ValidateIssueResponse {
    pub valid: bool,
    pub issue_key: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub assignee: Option<String>,
    pub issue_type: Option<String>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchIssuesRequest {
    pub query: String,
    #[serde(default = "default_max_results")]
    pub max_results: u32,
}

fn default_max_results() -> u32 {
    20
}

#[derive(Debug, Serialize)]
pub struct JiraIssueItem {
    pub key: String,
    pub summary: String,
    pub issue_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchIssuesResponse {
    pub issues: Vec<JiraIssueItem>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct JiraIssueDetail {
    pub key: String,
    pub summary: String,
    pub description: Option<String>,
    pub assignee: Option<String>,
    pub issue_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SummarizeDescriptionResponse {
    pub summary: String,
}

// Helper function to get user's Jira/Tempo config
async fn get_user_config(
    pool: &sqlx::SqlitePool,
    user_id: &str,
) -> Result<(String, Option<String>, Option<String>, Option<String>), String> {
    let row = sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>, Option<String>)>(
        "SELECT jira_url, jira_email, jira_pat, tempo_token FROM users WHERE id = ?",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "User not found".to_string())?;

    let jira_url = row.0.ok_or_else(|| "Jira URL not configured".to_string())?;

    if row.2.is_none() {
        return Err("Jira PAT not configured".to_string());
    }

    Ok((jira_url, row.1, row.2, row.3))
}

// Helpers

/// Simple fallback: strip markdown, keep first line, truncate.
fn sanitize_description_simple(raw: &str, max_len: usize) -> String {
    let mut lines: Vec<String> = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let stripped = trimmed
            .trim_start_matches("- ")
            .trim_start_matches("* ")
            .trim_start_matches("• ");

        let cleaned: String = stripped
            .replace("**", "")
            .replace('*', "")
            .replace('`', "");

        let cleaned = cleaned.trim().to_string();
        if !cleaned.is_empty() {
            lines.push(cleaned);
        }
    }

    if lines.is_empty() {
        return String::new();
    }

    truncate_str(&lines[0], max_len)
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
    format!("{}...", truncated)
}

/// Max description length for Tempo worklog
const MAX_DESCRIPTION_LEN: usize = 50;

/// Summarize descriptions using LLM, with fallback to simple sanitization.
/// Returns a Vec of sanitized descriptions in the same order as inputs.
async fn summarize_descriptions(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    descriptions: &[String],
) -> Vec<String> {
    // Try creating LLM service
    let llm = match create_llm_service(pool, user_id).await {
        Ok(svc) if svc.is_configured() => Some(svc),
        _ => None,
    };

    let mut results = Vec::with_capacity(descriptions.len());

    for desc in descriptions {
        if desc.trim().is_empty() {
            results.push(String::new());
            continue;
        }

        // If already short enough (original fits within limit), skip LLM
        let stripped = desc.trim();
        if stripped.lines().count() <= 1 && stripped.chars().count() <= MAX_DESCRIPTION_LEN {
            results.push(sanitize_description_simple(desc, MAX_DESCRIPTION_LEN));
            continue;
        }

        // Try LLM
        if let Some(ref llm) = llm {
            match llm.summarize_worklog(desc).await {
                Ok((summary, usage)) => {
                    // Save usage log (best-effort)
                    let _ = save_usage_log(pool, user_id, &usage).await;
                    let trimmed = truncate_str(summary.trim(), MAX_DESCRIPTION_LEN);
                    results.push(trimmed);
                    continue;
                }
                Err(err) => {
                    // Save error usage if available
                    if let Some(usage) = parse_error_usage(&err) {
                        let _ = save_usage_log(pool, user_id, &usage).await;
                    }
                    log::warn!("LLM worklog summary failed, falling back: {}", err);
                }
            }
        }

        // Fallback
        results.push(sanitize_description_simple(desc, MAX_DESCRIPTION_LEN));
    }

    results
}

// Commands

/// Test Jira/Tempo connection
#[tauri::command]
pub async fn test_tempo_connection(
    state: State<'_, AppState>,
    token: String,
) -> Result<SuccessResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let (jira_url, jira_email, jira_pat, _tempo_token) = get_user_config(&db.pool, &claims.sub).await?;

    let client = JiraClient::new(
        &jira_url,
        &jira_pat.unwrap(),
        jira_email.as_deref(),
        JiraAuthType::Pat,
    )
    .map_err(|e| e.to_string())?;

    match client.get_myself().await {
        Ok(user) => {
            let display_name = user.display_name
                .or(user.name)
                .unwrap_or_else(|| "Unknown".to_string());
            Ok(SuccessResponse {
                success: true,
                message: format!("Connected as: {}", display_name),
            })
        }
        Err(e) => Err(format!("Connection failed: {}", e)),
    }
}

/// Validate a Jira issue key
#[tauri::command]
pub async fn validate_jira_issue(
    state: State<'_, AppState>,
    token: String,
    issue_key: String,
) -> Result<ValidateIssueResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let (jira_url, jira_email, jira_pat, _tempo_token) = get_user_config(&db.pool, &claims.sub).await?;

    let client = JiraClient::new(
        &jira_url,
        &jira_pat.unwrap(),
        jira_email.as_deref(),
        JiraAuthType::Pat,
    )
    .map_err(|e| e.to_string())?;

    match client.validate_issue_key(&issue_key).await {
        Ok((valid, issue)) => {
            if valid {
                let fields = issue.as_ref().map(|i| &i.fields);
                let summary = fields.and_then(|f| f.summary.clone()).unwrap_or_default();
                let description = fields.and_then(|f| f.description.clone());
                let assignee = fields.and_then(|f| f.assignee.as_ref()).and_then(|a| a.display_name.clone());
                let issue_type = fields.and_then(|f| f.issue_type.as_ref()).map(|t| t.name.clone());
                Ok(ValidateIssueResponse {
                    valid: true,
                    issue_key: issue_key.clone(),
                    summary: Some(summary.clone()),
                    description,
                    assignee,
                    issue_type,
                    message: format!("{}: {}", issue_key, summary),
                })
            } else {
                Ok(ValidateIssueResponse {
                    valid: false,
                    issue_key,
                    summary: None,
                    description: None,
                    assignee: None,
                    issue_type: None,
                    message: "Issue not found".to_string(),
                })
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Sync multiple worklogs to Tempo/Jira
#[tauri::command]
pub async fn sync_worklogs_to_tempo(
    state: State<'_, AppState>,
    token: String,
    request: SyncWorklogsRequest,
) -> Result<SyncWorklogsResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let (jira_url, jira_email, jira_pat, tempo_token) = get_user_config(&db.pool, &claims.sub).await?;

    let use_tempo = tempo_token.is_some();

    let mut uploader = WorklogUploader::new(
        &jira_url,
        &jira_pat.unwrap(),
        jira_email.as_deref(),
        "pat",
        tempo_token.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    let mut successful = 0;
    let mut failed = 0;

    for entry_req in request.entries.iter() {
        // Descriptions are already summarized by frontend (via summarize_tempo_description)
        let desc = entry_req.description.clone();
        let entry = WorklogEntry {
            issue_key: entry_req.issue_key.clone(),
            date: entry_req.date.clone(),
            time_spent_seconds: entry_req.minutes * 60,
            description: desc.clone(),
            account_id: None,
        };

        if request.dry_run {
            results.push(WorklogEntryResponse {
                id: None,
                issue_key: entry_req.issue_key.clone(),
                date: entry_req.date.clone(),
                minutes: entry_req.minutes,
                hours: entry_req.minutes as f64 / 60.0,
                description: desc,
                status: "pending".to_string(),
                error_message: None,
            });
            continue;
        }

        match uploader.upload_worklog(entry, use_tempo).await {
            Ok(result) => {
                results.push(WorklogEntryResponse {
                    id: result.id.or(result.tempo_worklog_id.map(|id| id.to_string())),
                    issue_key: entry_req.issue_key.clone(),
                    date: entry_req.date.clone(),
                    minutes: entry_req.minutes,
                    hours: entry_req.minutes as f64 / 60.0,
                    description: entry_req.description.clone(),
                    status: "success".to_string(),
                    error_message: None,
                });
                successful += 1;
            }
            Err(e) => {
                results.push(WorklogEntryResponse {
                    id: None,
                    issue_key: entry_req.issue_key.clone(),
                    date: entry_req.date.clone(),
                    minutes: entry_req.minutes,
                    hours: entry_req.minutes as f64 / 60.0,
                    description: entry_req.description.clone(),
                    status: "error".to_string(),
                    error_message: Some(e.to_string()),
                });
                failed += 1;
            }
        }
    }

    // Update synced_to_tempo status in database for successful uploads
    if !request.dry_run && successful > 0 {
        for result in &results {
            if result.status == "success" {
                if let Some(ref worklog_id) = result.id {
                    // Find work items with this issue key and date and mark as synced
                    let _ = sqlx::query(
                        r#"
                        UPDATE work_items
                        SET synced_to_tempo = 1,
                            tempo_worklog_id = ?,
                            synced_at = CURRENT_TIMESTAMP,
                            updated_at = CURRENT_TIMESTAMP
                        WHERE user_id = ?
                          AND jira_issue_key = ?
                          AND date = ?
                          AND synced_to_tempo = 0
                        "#,
                    )
                    .bind(worklog_id)
                    .bind(&claims.sub)
                    .bind(&result.issue_key)
                    .bind(&result.date)
                    .execute(&db.pool)
                    .await;
                }
            }
        }
    }

    Ok(SyncWorklogsResponse {
        success: failed == 0,
        total_entries: request.entries.len(),
        successful,
        failed,
        results,
        dry_run: request.dry_run,
    })
}

/// Get worklogs from Tempo for a date range
#[tauri::command]
pub async fn get_tempo_worklogs(
    state: State<'_, AppState>,
    token: String,
    request: GetWorklogsRequest,
) -> Result<Vec<serde_json::Value>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let (jira_url, _jira_email, _jira_pat, tempo_token) = get_user_config(&db.pool, &claims.sub).await?;

    let tempo_token = tempo_token.ok_or_else(|| "Tempo token not configured".to_string())?;

    let tempo = TempoClient::new(&jira_url, &tempo_token)
        .map_err(|e| e.to_string())?;

    tempo.get_worklogs(&request.date_from, &request.date_to).await
        .map_err(|e| e.to_string())
}

/// Search Jira issues by summary or key
#[tauri::command]
pub async fn search_jira_issues(
    state: State<'_, AppState>,
    token: String,
    request: SearchIssuesRequest,
) -> Result<SearchIssuesResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let (jira_url, jira_email, jira_pat, _tempo_token) = get_user_config(&db.pool, &claims.sub).await?;

    let client = JiraClient::new(
        &jira_url,
        &jira_pat.unwrap(),
        jira_email.as_deref(),
        JiraAuthType::Pat,
    )
    .map_err(|e| e.to_string())?;

    let issues = client
        .search_issues(&request.query, request.max_results)
        .await
        .map_err(|e| e.to_string())?;

    let total = issues.len();
    let items: Vec<JiraIssueItem> = issues
        .into_iter()
        .map(|issue| JiraIssueItem {
            key: issue.key,
            summary: issue.fields.summary.unwrap_or_default(),
            issue_type: issue.fields.issue_type.map(|t| t.name),
        })
        .collect();

    Ok(SearchIssuesResponse {
        issues: items,
        total,
    })
}

/// Batch get full issue details for multiple issue keys
#[tauri::command]
pub async fn batch_get_jira_issues(
    state: State<'_, AppState>,
    token: String,
    issue_keys: Vec<String>,
) -> Result<Vec<JiraIssueDetail>, String> {
    if issue_keys.is_empty() {
        return Ok(Vec::new());
    }

    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let (jira_url, jira_email, jira_pat, _tempo_token) = get_user_config(&db.pool, &claims.sub).await?;

    let client = JiraClient::new(
        &jira_url,
        &jira_pat.unwrap(),
        jira_email.as_deref(),
        JiraAuthType::Pat,
    )
    .map_err(|e| e.to_string())?;

    let issues = client
        .batch_get_issues(&issue_keys)
        .await
        .map_err(|e| e.to_string())?;

    Ok(issues
        .into_iter()
        .map(|issue| JiraIssueDetail {
            key: issue.key,
            summary: issue.fields.summary.unwrap_or_default(),
            description: issue.fields.description,
            assignee: issue.fields.assignee.and_then(|a| a.display_name),
            issue_type: issue.fields.issue_type.map(|t| t.name),
        })
        .collect())
}

/// Summarize a single worklog description using LLM (or fallback).
/// Used by frontend to show per-entry progress before syncing.
#[tauri::command]
pub async fn summarize_tempo_description(
    state: State<'_, AppState>,
    token: String,
    description: String,
) -> Result<SummarizeDescriptionResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let descs = summarize_descriptions(&db.pool, &claims.sub, &[description]).await;
    let summary = descs.into_iter().next().unwrap_or_default();

    Ok(SummarizeDescriptionResponse { summary })
}
