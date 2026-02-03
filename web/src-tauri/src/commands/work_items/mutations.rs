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

/// Generate the filename for a manual work item
fn get_manual_item_filename(date: &NaiveDate, id: &str) -> String {
    format!("{}_{}.md", date.format("%Y-%m-%d"), &id[..8])
}

/// Save a manual work item as a markdown file
fn save_manual_item_file(
    project_path: &str,
    id: &str,
    date: &NaiveDate,
    title: &str,
    description: Option<&str>,
    hours: f64,
    jira_issue_key: Option<&str>,
) -> Result<(), String> {
    let filename = get_manual_item_filename(date, id);
    let file_path = std::path::Path::new(project_path).join(&filename);

    // Build markdown content with YAML frontmatter
    let mut content = String::new();
    content.push_str("---\n");
    content.push_str(&format!("id: {}\n", id));
    content.push_str(&format!("date: {}\n", date.format("%Y-%m-%d")));
    content.push_str(&format!("hours: {}\n", hours));
    if let Some(key) = jira_issue_key {
        if !key.is_empty() {
            content.push_str(&format!("jira_issue_key: {}\n", key));
        }
    }
    content.push_str("---\n\n");
    content.push_str(&format!("# {}\n", title));
    if let Some(desc) = description {
        if !desc.is_empty() {
            content.push_str(&format!("\n{}\n", desc));
        }
    }

    std::fs::write(&file_path, content)
        .map_err(|e| format!("Failed to save manual item file: {}", e))?;

    Ok(())
}

/// Delete a manual work item file
fn delete_manual_item_file(project_path: &str, date: &NaiveDate, id: &str) -> Result<(), String> {
    let filename = get_manual_item_filename(date, id);
    let file_path = std::path::Path::new(project_path).join(&filename);

    if file_path.exists() {
        std::fs::remove_file(&file_path)
            .map_err(|e| format!("Failed to delete manual item file: {}", e))?;
    }

    Ok(())
}

/// Update a manual work item file (delete old, create new if date changed)
fn update_manual_item_file(
    old_project_path: Option<&str>,
    new_project_path: Option<&str>,
    old_date: &NaiveDate,
    new_date: &NaiveDate,
    id: &str,
    title: &str,
    description: Option<&str>,
    hours: f64,
    jira_issue_key: Option<&str>,
) -> Result<(), String> {
    // Delete old file if project or date changed
    if let Some(old_path) = old_project_path {
        if old_date != new_date || old_project_path != new_project_path {
            let _ = delete_manual_item_file(old_path, old_date, id);
        }
    }

    // Save new file
    if let Some(new_path) = new_project_path {
        save_manual_item_file(new_path, id, new_date, title, description, hours, jira_issue_key)?;
    }

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

            // Save as markdown file
            save_manual_item_file(
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

        // Update markdown file
        update_manual_item_file(
            existing_item.project_path.as_deref(),
            item.project_path.as_deref(),
            &existing_item.date,
            &item.date,
            &id,
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

        // Delete markdown file
        if let Some(ref item) = existing {
            if let Some(ref path) = item.project_path {
                let _ = delete_manual_item_file(path, &item.date, &id);
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
