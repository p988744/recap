//! Work Items mutations
//!
//! Commands for creating, updating, and deleting work items.

use chrono::Utc;
use tauri::State;
use uuid::Uuid;

use recap_core::auth::verify_token;
use recap_core::models::{CreateWorkItem, UpdateWorkItem, WorkItem};

use crate::commands::AppState;

/// Get the manual projects directory path
fn get_manual_projects_dir() -> Result<std::path::PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    Ok(home.join(".recap").join("manual-projects"))
}

/// Get the project path for a manual project
fn get_manual_project_path(project_name: &str) -> Result<String, String> {
    let dir = get_manual_projects_dir()?;
    Ok(dir.join(project_name).to_string_lossy().to_string())
}

/// Ensure the manual project directory exists
fn ensure_manual_project_dir(project_name: &str) -> Result<String, String> {
    let dir = get_manual_projects_dir()?;
    let project_dir = dir.join(project_name);

    if !project_dir.exists() {
        std::fs::create_dir_all(&project_dir)
            .map_err(|e| format!("Failed to create manual project directory: {}", e))?;
    }

    Ok(project_dir.to_string_lossy().to_string())
}

/// Create a new work item
#[tauri::command]
pub async fn create_work_item(
    state: State<'_, AppState>,
    token: String,
    request: CreateWorkItem,
) -> Result<WorkItem, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let source = request.source.unwrap_or_else(|| "manual".to_string());
    let tags_json = request.tags.map(|t| serde_json::to_string(&t).unwrap_or_default());

    // For manual items with project_name, set project_path to manual-projects directory
    let (title, project_path) = if source == "manual" {
        if let Some(ref project_name) = request.project_name {
            if !project_name.is_empty() {
                // Create the manual project directory and set project_path
                let path = ensure_manual_project_dir(project_name)?;
                (request.title.clone(), Some(path))
            } else {
                (request.title.clone(), None)
            }
        } else {
            (request.title.clone(), None)
        }
    } else {
        // Non-manual items keep their original behavior
        (request.title.clone(), None)
    };

    sqlx::query(
        r#"INSERT INTO work_items (id, user_id, source, source_id, title, description, hours, date,
            jira_issue_key, jira_issue_title, category, tags, project_path, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(&claims.sub)
    .bind(&source)
    .bind(&request.source_id)
    .bind(&title)
    .bind(&request.description)
    .bind(request.hours.unwrap_or(0.0))
    .bind(&request.date)
    .bind(&request.jira_issue_key)
    .bind(&request.jira_issue_title)
    .bind(&request.category)
    .bind(&tags_json)
    .bind(&project_path)
    .bind(now)
    .bind(now)
    .execute(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let item: WorkItem = sqlx::query_as("SELECT * FROM work_items WHERE id = ?")
        .bind(&id)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(item)
}

/// Get a single work item
#[tauri::command]
pub async fn get_work_item(
    state: State<'_, AppState>,
    token: String,
    id: String,
) -> Result<WorkItem, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let item: Option<WorkItem> =
        sqlx::query_as("SELECT * FROM work_items WHERE id = ? AND user_id = ?")
            .bind(&id)
            .bind(&claims.sub)
            .fetch_optional(&db.pool)
            .await
            .map_err(|e| e.to_string())?;

    item.ok_or_else(|| "Work item not found".to_string())
}

/// Update a work item
#[tauri::command]
pub async fn update_work_item(
    state: State<'_, AppState>,
    token: String,
    id: String,
    request: UpdateWorkItem,
) -> Result<WorkItem, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Check ownership
    let existing: Option<WorkItem> =
        sqlx::query_as("SELECT * FROM work_items WHERE id = ? AND user_id = ?")
            .bind(&id)
            .bind(&claims.sub)
            .fetch_optional(&db.pool)
            .await
            .map_err(|e| e.to_string())?;

    if existing.is_none() {
        return Err("Work item not found".to_string());
    }

    let now = Utc::now();

    // Update timestamp
    sqlx::query("UPDATE work_items SET updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(&id)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    // Apply individual updates
    if let Some(title) = &request.title {
        sqlx::query("UPDATE work_items SET title = ? WHERE id = ?")
            .bind(title)
            .bind(&id)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(description) = &request.description {
        sqlx::query("UPDATE work_items SET description = ? WHERE id = ?")
            .bind(description)
            .bind(&id)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(hours) = request.hours {
        sqlx::query("UPDATE work_items SET hours = ? WHERE id = ?")
            .bind(hours)
            .bind(&id)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(date) = &request.date {
        sqlx::query("UPDATE work_items SET date = ? WHERE id = ?")
            .bind(date)
            .bind(&id)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(jira_key) = &request.jira_issue_key {
        sqlx::query("UPDATE work_items SET jira_issue_key = ? WHERE id = ?")
            .bind(jira_key)
            .bind(&id)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(jira_title) = &request.jira_issue_title {
        sqlx::query("UPDATE work_items SET jira_issue_title = ? WHERE id = ?")
            .bind(jira_title)
            .bind(&id)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(category) = &request.category {
        sqlx::query("UPDATE work_items SET category = ? WHERE id = ?")
            .bind(category)
            .bind(&id)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(synced) = request.synced_to_tempo {
        sqlx::query("UPDATE work_items SET synced_to_tempo = ? WHERE id = ?")
            .bind(synced)
            .bind(&id)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    // Handle project_name update - update project_path for manual items
    if let Some(ref project_name) = request.project_name {
        let existing = existing.as_ref().unwrap();

        // Only update project_path for manual source items
        if existing.source == "manual" {
            let project_path = if !project_name.is_empty() {
                // Create the manual project directory and set project_path
                Some(ensure_manual_project_dir(project_name)?)
            } else {
                None
            };

            sqlx::query("UPDATE work_items SET project_path = ? WHERE id = ?")
                .bind(&project_path)
                .bind(&id)
                .execute(&db.pool)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    // Fetch updated item
    let item: WorkItem = sqlx::query_as("SELECT * FROM work_items WHERE id = ?")
        .bind(&id)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(item)
}

/// Delete a work item
#[tauri::command]
pub async fn delete_work_item(
    state: State<'_, AppState>,
    token: String,
    id: String,
) -> Result<(), String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let result = sqlx::query("DELETE FROM work_items WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err("Work item not found".to_string());
    }

    Ok(())
}

/// Map a work item to a Jira issue
#[tauri::command]
pub async fn map_work_item_jira(
    state: State<'_, AppState>,
    token: String,
    work_item_id: String,
    jira_issue_key: String,
    jira_issue_title: Option<String>,
) -> Result<WorkItem, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;
    let now = Utc::now();

    // Check ownership
    let existing: Option<WorkItem> =
        sqlx::query_as("SELECT * FROM work_items WHERE id = ? AND user_id = ?")
            .bind(&work_item_id)
            .bind(&claims.sub)
            .fetch_optional(&db.pool)
            .await
            .map_err(|e| e.to_string())?;

    if existing.is_none() {
        return Err("Work item not found".to_string());
    }

    // Update jira mapping
    sqlx::query(
        "UPDATE work_items SET jira_issue_key = ?, jira_issue_title = ?, updated_at = ? WHERE id = ? AND user_id = ?"
    )
    .bind(&jira_issue_key)
    .bind(&jira_issue_title)
    .bind(now)
    .bind(&work_item_id)
    .bind(&claims.sub)
    .execute(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // Fetch updated item
    let item: WorkItem = sqlx::query_as("SELECT * FROM work_items WHERE id = ?")
        .bind(&work_item_id)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(item)
}
