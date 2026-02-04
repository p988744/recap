//! Work Items mutations
//!
//! Commands for creating, updating, and deleting work items.

use chrono::{NaiveDate, Utc};
use tauri::State;
use uuid::Uuid;

use recap_core::auth::verify_token;
use recap_core::models::{CreateWorkItem, UpdateWorkItem, WorkItem};

use crate::commands::AppState;

/// Create a snapshot record for a manual work item
/// This allows manual items to use the same workflow as automatic items
async fn create_manual_snapshot(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    work_item_id: &str,
    project_path: &str,
    date: &NaiveDate,
    title: &str,
    description: Option<&str>,
    hours: f64,
) -> Result<(), String> {
    let snapshot_id = Uuid::new_v4().to_string();
    let session_id = format!("manual:{}", work_item_id);

    // Create hour_bucket from date (use 09:00 as default work start time)
    let hour_bucket = format!("{}T09:00:00", date.format("%Y-%m-%d"));

    // Build user_messages JSON with title and description
    let content = if let Some(desc) = description {
        format!("{}\n\n{}", title, desc)
    } else {
        title.to_string()
    };
    let user_messages = serde_json::json!([{
        "role": "user",
        "content": content,
        "hours": hours
    }]).to_string();

    sqlx::query(
        r#"INSERT OR REPLACE INTO snapshot_raw_data
           (id, user_id, session_id, project_path, hour_bucket, user_messages,
            assistant_messages, tool_calls, files_modified, git_commits,
            message_count, raw_size_bytes, created_at)
           VALUES (?, ?, ?, ?, ?, ?, NULL, NULL, NULL, NULL, 1, 0, CURRENT_TIMESTAMP)"#
    )
    .bind(&snapshot_id)
    .bind(user_id)
    .bind(&session_id)
    .bind(project_path)
    .bind(&hour_bucket)
    .bind(&user_messages)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to create snapshot for manual item: {}", e))?;

    Ok(())
}

/// Update the snapshot record for a manual work item
async fn update_manual_snapshot(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    work_item_id: &str,
    project_path: Option<&str>,
    date: Option<&NaiveDate>,
    title: Option<&str>,
    description: Option<&str>,
    hours: Option<f64>,
) -> Result<(), String> {
    let session_id = format!("manual:{}", work_item_id);

    // Check if snapshot exists
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM snapshot_raw_data WHERE session_id = ? AND user_id = ?"
    )
    .bind(&session_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    if existing.is_none() {
        // No existing snapshot, nothing to update
        return Ok(());
    }

    // Update project_path if provided
    if let Some(path) = project_path {
        sqlx::query("UPDATE snapshot_raw_data SET project_path = ? WHERE session_id = ? AND user_id = ?")
            .bind(path)
            .bind(&session_id)
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    // Update hour_bucket if date changed
    if let Some(naive_date) = date {
        let hour_bucket = format!("{}T09:00:00", naive_date.format("%Y-%m-%d"));

        sqlx::query("UPDATE snapshot_raw_data SET hour_bucket = ? WHERE session_id = ? AND user_id = ?")
            .bind(&hour_bucket)
            .bind(&session_id)
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    // Update user_messages if title or description changed
    if title.is_some() || description.is_some() || hours.is_some() {
        // Fetch current work item to get complete data
        let item: Option<WorkItem> = sqlx::query_as(
            "SELECT * FROM work_items WHERE id = ? AND user_id = ?"
        )
        .bind(work_item_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

        if let Some(item) = item {
            let content = if let Some(desc) = &item.description {
                format!("{}\n\n{}", item.title, desc)
            } else {
                item.title.clone()
            };
            let user_messages = serde_json::json!([{
                "role": "user",
                "content": content,
                "hours": item.hours
            }]).to_string();

            sqlx::query("UPDATE snapshot_raw_data SET user_messages = ? WHERE session_id = ? AND user_id = ?")
                .bind(&user_messages)
                .bind(&session_id)
                .bind(user_id)
                .execute(pool)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

/// Delete the snapshot record for a manual work item
async fn delete_manual_snapshot(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    work_item_id: &str,
) -> Result<(), String> {
    let session_id = format!("manual:{}", work_item_id);

    sqlx::query("DELETE FROM snapshot_raw_data WHERE session_id = ? AND user_id = ?")
        .bind(&session_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get the manual projects directory path
fn get_manual_projects_dir() -> Result<std::path::PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    Ok(home.join(".recap").join("manual-projects"))
}

/// Manual item entry for JSONL file
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ManualItemEntry {
    id: String,
    date: String,
    hours: f64,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    jira_issue_key: Option<String>,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
}

/// Get the JSONL file path for a project
fn get_items_jsonl_path(project_path: &str) -> std::path::PathBuf {
    std::path::Path::new(project_path).join("items.jsonl")
}

/// Read all items from the JSONL file
fn read_items_jsonl(project_path: &str) -> Result<Vec<ManualItemEntry>, String> {
    let file_path = get_items_jsonl_path(project_path);

    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read items.jsonl: {}", e))?;

    let mut items = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let item: ManualItemEntry = serde_json::from_str(line)
            .map_err(|e| format!("Failed to parse JSONL line: {}", e))?;
        items.push(item);
    }

    Ok(items)
}

/// Write all items to the JSONL file
fn write_items_jsonl(project_path: &str, items: &[ManualItemEntry]) -> Result<(), String> {
    let file_path = get_items_jsonl_path(project_path);

    let mut content = String::new();
    for item in items {
        let line = serde_json::to_string(item)
            .map_err(|e| format!("Failed to serialize item: {}", e))?;
        content.push_str(&line);
        content.push('\n');
    }

    std::fs::write(&file_path, content)
        .map_err(|e| format!("Failed to write items.jsonl: {}", e))?;

    Ok(())
}

/// Append a manual work item to the JSONL file
fn append_manual_item_jsonl(
    project_path: &str,
    id: &str,
    date: &NaiveDate,
    title: &str,
    description: Option<&str>,
    hours: f64,
    jira_issue_key: Option<&str>,
) -> Result<(), String> {
    let entry = ManualItemEntry {
        id: id.to_string(),
        date: date.format("%Y-%m-%d").to_string(),
        hours,
        title: title.to_string(),
        description: description.map(|s| s.to_string()),
        jira_issue_key: jira_issue_key.map(|s| s.to_string()),
        created_at: Utc::now().to_rfc3339(),
        updated_at: None,
    };

    let file_path = get_items_jsonl_path(project_path);
    let line = serde_json::to_string(&entry)
        .map_err(|e| format!("Failed to serialize item: {}", e))?;

    // Append to file
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)
        .map_err(|e| format!("Failed to open items.jsonl: {}", e))?;

    writeln!(file, "{}", line)
        .map_err(|e| format!("Failed to append to items.jsonl: {}", e))?;

    Ok(())
}

/// Update a manual work item in the JSONL file
fn update_manual_item_jsonl(
    old_project_path: Option<&str>,
    new_project_path: Option<&str>,
    id: &str,
    date: &NaiveDate,
    title: &str,
    description: Option<&str>,
    hours: f64,
    jira_issue_key: Option<&str>,
) -> Result<(), String> {
    // If project changed, remove from old and add to new
    if old_project_path != new_project_path {
        if let Some(old_path) = old_project_path {
            let _ = delete_manual_item_jsonl(old_path, id);
        }
        if let Some(new_path) = new_project_path {
            append_manual_item_jsonl(new_path, id, date, title, description, hours, jira_issue_key)?;
        }
        return Ok(());
    }

    // Update in place
    if let Some(project_path) = new_project_path {
        let mut items = read_items_jsonl(project_path)?;

        if let Some(item) = items.iter_mut().find(|i| i.id == id) {
            item.date = date.format("%Y-%m-%d").to_string();
            item.title = title.to_string();
            item.description = description.map(|s| s.to_string());
            item.hours = hours;
            item.jira_issue_key = jira_issue_key.map(|s| s.to_string());
            item.updated_at = Some(Utc::now().to_rfc3339());
        }

        write_items_jsonl(project_path, &items)?;
    }

    Ok(())
}

/// Delete a manual work item from the JSONL file
fn delete_manual_item_jsonl(project_path: &str, id: &str) -> Result<(), String> {
    let mut items = read_items_jsonl(project_path)?;
    items.retain(|item| item.id != id);
    write_items_jsonl(project_path, &items)?;
    Ok(())
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

    // Create snapshot and file for manual items with project_path (for unified workflow)
    if source == "manual" {
        if let Some(ref path) = project_path {
            create_manual_snapshot(
                &db.pool,
                &claims.sub,
                &id,
                path,
                &request.date,
                &title,
                request.description.as_deref(),
                request.hours.unwrap_or(0.0),
            ).await?;

            // Append to items.jsonl
            append_manual_item_jsonl(
                path,
                &id,
                &request.date,
                &title,
                request.description.as_deref(),
                request.hours.unwrap_or(0.0),
                request.jira_issue_key.as_deref(),
            )?;
        }
    }

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

    // Update snapshot and file for manual items (for unified workflow)
    if item.source == "manual" {
        let existing_item = existing.as_ref().unwrap();

        update_manual_snapshot(
            &db.pool,
            &claims.sub,
            &id,
            item.project_path.as_deref(),
            request.date.as_ref(),
            request.title.as_deref(),
            request.description.as_deref(),
            request.hours,
        ).await?;

        // Update items.jsonl
        update_manual_item_jsonl(
            existing_item.project_path.as_deref(),
            item.project_path.as_deref(),
            &id,
            &item.date,
            &item.title,
            item.description.as_deref(),
            item.hours,
            item.jira_issue_key.as_deref(),
        )?;
    }

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

    // Check if it's a manual item before deleting
    let existing: Option<WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE id = ? AND user_id = ?"
    )
    .bind(&id)
    .bind(&claims.sub)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let is_manual = existing.as_ref().map(|w| w.source == "manual").unwrap_or(false);

    let result = sqlx::query("DELETE FROM work_items WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err("Work item not found".to_string());
    }

    // Delete associated snapshot and file for manual items
    if is_manual {
        delete_manual_snapshot(&db.pool, &claims.sub, &id).await?;

        // Delete from items.jsonl
        if let Some(ref item) = existing {
            if let Some(ref path) = item.project_path {
                let _ = delete_manual_item_jsonl(path, &id);
            }
        }
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
