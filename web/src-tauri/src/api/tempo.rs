//! Tempo API routes - Jira/Tempo integration

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::auth::AuthUser;
use crate::db::Database;
use crate::services::tempo::{JiraAuthType, JiraClient, TempoClient, WorklogEntry, WorklogUploader};

/// Create Tempo routes
pub fn routes() -> Router<Database> {
    Router::new()
        .route("/test", get(test_connection))
        .route("/validate/{issue_key}", get(validate_issue))
        .route("/sync", post(sync_worklogs))
        .route("/upload", post(upload_single_worklog))
        .route("/worklogs", get(get_worklogs))
}

// Request/Response types

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
pub struct SyncRequest {
    pub entries: Vec<WorklogEntryRequest>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub success: bool,
    pub total_entries: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<WorklogEntryResponse>,
    pub dry_run: bool,
}

#[derive(Debug, Deserialize)]
pub struct GetWorklogsQuery {
    pub date_from: String,
    pub date_to: String,
}

#[derive(Debug, Serialize)]
pub struct ValidateIssueResponse {
    pub valid: bool,
    pub issue_key: String,
    pub summary: Option<String>,
    pub message: String,
}

// Helper function to get user's Jira/Tempo config
async fn get_user_config(
    db: &Database,
    user_id: &str,
) -> Result<(String, Option<String>, Option<String>, Option<String>), (StatusCode, String)> {
    let row = sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>, Option<String>)>(
        "SELECT jira_url, jira_email, jira_pat, tempo_token FROM users WHERE id = ?",
    )
    .bind(user_id)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or_else(|| (StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let jira_url = row.0.ok_or_else(|| {
        (StatusCode::BAD_REQUEST, "Jira URL not configured".to_string())
    })?;

    if row.2.is_none() {
        return Err((StatusCode::BAD_REQUEST, "Jira PAT not configured".to_string()));
    }

    Ok((jira_url, row.1, row.2, row.3))
}

// Route handlers

/// Test Jira/Tempo connection
async fn test_connection(
    auth: AuthUser,
    State(db): State<Database>,
) -> Result<Json<SuccessResponse>, (StatusCode, String)> {
    let (jira_url, jira_email, jira_pat, _tempo_token) = get_user_config(&db, &auth.0.sub).await?;

    let client = JiraClient::new(
        &jira_url,
        &jira_pat.unwrap(),
        jira_email.as_deref(),
        JiraAuthType::Pat,
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match client.get_myself().await {
        Ok(user) => {
            let display_name = user.display_name
                .or(user.name)
                .unwrap_or_else(|| "Unknown".to_string());
            Ok(Json(SuccessResponse {
                success: true,
                message: format!("Connected as: {}", display_name),
            }))
        }
        Err(e) => Err((StatusCode::BAD_REQUEST, format!("Connection failed: {}", e))),
    }
}

/// Validate a Jira issue key
async fn validate_issue(
    auth: AuthUser,
    State(db): State<Database>,
    Path(issue_key): Path<String>,
) -> Result<Json<ValidateIssueResponse>, (StatusCode, String)> {
    let (jira_url, jira_email, jira_pat, _tempo_token) = get_user_config(&db, &auth.0.sub).await?;

    let client = JiraClient::new(
        &jira_url,
        &jira_pat.unwrap(),
        jira_email.as_deref(),
        JiraAuthType::Pat,
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match client.validate_issue_key(&issue_key).await {
        Ok((valid, summary)) => {
            if valid {
                Ok(Json(ValidateIssueResponse {
                    valid: true,
                    issue_key: issue_key.clone(),
                    summary: Some(summary.clone()),
                    message: format!("{}: {}", issue_key, summary),
                }))
            } else {
                Ok(Json(ValidateIssueResponse {
                    valid: false,
                    issue_key,
                    summary: None,
                    message: "Issue not found".to_string(),
                }))
            }
        }
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

/// Sync multiple worklogs to Tempo/Jira
async fn sync_worklogs(
    auth: AuthUser,
    State(db): State<Database>,
    Json(request): Json<SyncRequest>,
) -> Result<Json<SyncResponse>, (StatusCode, String)> {
    let (jira_url, jira_email, jira_pat, tempo_token) = get_user_config(&db, &auth.0.sub).await?;

    let use_tempo = tempo_token.is_some();

    let mut uploader = WorklogUploader::new(
        &jira_url,
        &jira_pat.unwrap(),
        jira_email.as_deref(),
        "pat",
        tempo_token.as_deref(),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut results = Vec::new();
    let mut successful = 0;
    let mut failed = 0;

    for entry_req in &request.entries {
        let entry = WorklogEntry {
            issue_key: entry_req.issue_key.clone(),
            date: entry_req.date.clone(),
            time_spent_seconds: entry_req.minutes * 60,
            description: entry_req.description.clone(),
            account_id: None,
        };

        if request.dry_run {
            results.push(WorklogEntryResponse {
                id: None,
                issue_key: entry_req.issue_key.clone(),
                date: entry_req.date.clone(),
                minutes: entry_req.minutes,
                hours: entry_req.minutes as f64 / 60.0,
                description: entry_req.description.clone(),
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
                    .bind(&auth.0.sub)
                    .bind(&result.issue_key)
                    .bind(&result.date)
                    .execute(&db.pool)
                    .await;
                }
            }
        }
    }

    Ok(Json(SyncResponse {
        success: failed == 0,
        total_entries: request.entries.len(),
        successful,
        failed,
        results,
        dry_run: request.dry_run,
    }))
}

/// Upload a single worklog entry
async fn upload_single_worklog(
    auth: AuthUser,
    State(db): State<Database>,
    Json(entry_req): Json<WorklogEntryRequest>,
) -> Result<Json<WorklogEntryResponse>, (StatusCode, String)> {
    let (jira_url, jira_email, jira_pat, tempo_token) = get_user_config(&db, &auth.0.sub).await?;

    let use_tempo = tempo_token.is_some();

    let mut uploader = WorklogUploader::new(
        &jira_url,
        &jira_pat.unwrap(),
        jira_email.as_deref(),
        "pat",
        tempo_token.as_deref(),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let entry = WorklogEntry {
        issue_key: entry_req.issue_key.clone(),
        date: entry_req.date.clone(),
        time_spent_seconds: entry_req.minutes * 60,
        description: entry_req.description.clone(),
        account_id: None,
    };

    match uploader.upload_worklog(entry, use_tempo).await {
        Ok(result) => {
            let worklog_id = result.id.clone().or(result.tempo_worklog_id.map(|id| id.to_string()));

            // Update synced_to_tempo status in database
            if let Some(ref id) = worklog_id {
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
                .bind(id)
                .bind(&auth.0.sub)
                .bind(&entry_req.issue_key)
                .bind(&entry_req.date)
                .execute(&db.pool)
                .await;
            }

            Ok(Json(WorklogEntryResponse {
                id: worklog_id,
                issue_key: entry_req.issue_key,
                date: entry_req.date,
                minutes: entry_req.minutes,
                hours: entry_req.minutes as f64 / 60.0,
                description: entry_req.description,
                status: "success".to_string(),
                error_message: None,
            }))
        }
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

/// Get worklogs from Tempo for a date range
async fn get_worklogs(
    auth: AuthUser,
    State(db): State<Database>,
    Query(query): Query<GetWorklogsQuery>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, String)> {
    let (jira_url, _jira_email, _jira_pat, tempo_token) = get_user_config(&db, &auth.0.sub).await?;

    let tempo_token = tempo_token.ok_or_else(|| {
        (StatusCode::BAD_REQUEST, "Tempo token not configured".to_string())
    })?;

    let tempo = TempoClient::new(&jira_url, &tempo_token)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match tempo.get_worklogs(&query.date_from, &query.date_to).await {
        Ok(worklogs) => Ok(Json(worklogs)),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}
