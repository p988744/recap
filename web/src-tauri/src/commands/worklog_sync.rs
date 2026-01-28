//! Worklog Sync commands
//!
//! Tauri commands for managing project-to-issue mappings and worklog sync records.

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use recap_core::auth::verify_token;

use super::AppState;

// Types

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectIssueMapping {
    pub project_path: String,
    pub user_id: String,
    pub jira_issue_key: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorklogSyncRecord {
    pub id: String,
    pub user_id: String,
    pub project_path: String,
    pub date: String,
    pub jira_issue_key: String,
    pub hours: f64,
    pub description: Option<String>,
    pub tempo_worklog_id: Option<String>,
    pub synced_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveMappingRequest {
    pub project_path: String,
    pub jira_issue_key: String,
}

#[derive(Debug, Deserialize)]
pub struct GetSyncRecordsRequest {
    pub date_from: String,
    pub date_to: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveSyncRecordRequest {
    pub project_path: String,
    pub date: String,
    pub jira_issue_key: String,
    pub hours: f64,
    pub description: Option<String>,
    pub tempo_worklog_id: Option<String>,
}

// Commands

/// Get all project-to-issue mappings for the current user
#[tauri::command]
pub async fn get_project_issue_mappings(
    state: State<'_, AppState>,
    token: String,
) -> Result<Vec<ProjectIssueMapping>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let mappings = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT project_path, user_id, jira_issue_key, COALESCE(updated_at, '') FROM project_issue_mappings WHERE user_id = ?"
    )
    .bind(&claims.sub)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(mappings
        .into_iter()
        .map(|(project_path, user_id, jira_issue_key, updated_at)| ProjectIssueMapping {
            project_path,
            user_id,
            jira_issue_key,
            updated_at,
        })
        .collect())
}

/// Save or update a project-to-issue mapping
#[tauri::command]
pub async fn save_project_issue_mapping(
    state: State<'_, AppState>,
    token: String,
    request: SaveMappingRequest,
) -> Result<ProjectIssueMapping, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    sqlx::query(
        r#"
        INSERT INTO project_issue_mappings (project_path, user_id, jira_issue_key, updated_at)
        VALUES (?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(project_path, user_id) DO UPDATE SET
            jira_issue_key = excluded.jira_issue_key,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&request.project_path)
    .bind(&claims.sub)
    .bind(&request.jira_issue_key)
    .execute(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(ProjectIssueMapping {
        project_path: request.project_path,
        user_id: claims.sub,
        jira_issue_key: request.jira_issue_key,
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Get worklog sync records for a date range
#[tauri::command]
pub async fn get_worklog_sync_records(
    state: State<'_, AppState>,
    token: String,
    request: GetSyncRecordsRequest,
) -> Result<Vec<WorklogSyncRecord>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let records = sqlx::query_as::<_, (String, String, String, String, String, f64, Option<String>, Option<String>, String)>(
        r#"
        SELECT id, user_id, project_path, date, jira_issue_key, hours,
               description, tempo_worklog_id, COALESCE(synced_at, '')
        FROM worklog_sync_records
        WHERE user_id = ? AND date >= ? AND date <= ?
        ORDER BY date, project_path
        "#,
    )
    .bind(&claims.sub)
    .bind(&request.date_from)
    .bind(&request.date_to)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(records
        .into_iter()
        .map(|(id, user_id, project_path, date, jira_issue_key, hours, description, tempo_worklog_id, synced_at)| {
            WorklogSyncRecord {
                id,
                user_id,
                project_path,
                date,
                jira_issue_key,
                hours,
                description,
                tempo_worklog_id,
                synced_at,
            }
        })
        .collect())
}

/// Save a worklog sync record (called after successful Tempo upload)
#[tauri::command]
pub async fn save_worklog_sync_record(
    state: State<'_, AppState>,
    token: String,
    request: SaveSyncRecordRequest,
) -> Result<WorklogSyncRecord, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let id = Uuid::new_v4().to_string();

    sqlx::query(
        r#"
        INSERT INTO worklog_sync_records (id, user_id, project_path, date, jira_issue_key, hours, description, tempo_worklog_id, synced_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(user_id, project_path, date) DO UPDATE SET
            jira_issue_key = excluded.jira_issue_key,
            hours = excluded.hours,
            description = excluded.description,
            tempo_worklog_id = excluded.tempo_worklog_id,
            synced_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&id)
    .bind(&claims.sub)
    .bind(&request.project_path)
    .bind(&request.date)
    .bind(&request.jira_issue_key)
    .bind(&request.hours)
    .bind(&request.description)
    .bind(&request.tempo_worklog_id)
    .execute(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(WorklogSyncRecord {
        id,
        user_id: claims.sub,
        project_path: request.project_path,
        date: request.date,
        jira_issue_key: request.jira_issue_key,
        hours: request.hours,
        description: request.description,
        tempo_worklog_id: request.tempo_worklog_id,
        synced_at: chrono::Utc::now().to_rfc3339(),
    })
}
