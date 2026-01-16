//! Work Items sync operations
//!
//! Commands for batch sync and aggregation of work items.

use std::collections::HashMap;
use chrono::Utc;
use tauri::State;
use uuid::Uuid;

use recap_core::auth::verify_token;
use recap_core::models::WorkItem;

use crate::commands::AppState;
use super::query_builder::SafeQueryBuilder;
use super::types::{
    AggregateRequest, AggregateResponse, BatchSyncRequest, BatchSyncResponse,
};

/// Batch sync work items to Tempo
#[tauri::command]
pub async fn batch_sync_tempo(
    state: State<'_, AppState>,
    token: String,
    request: BatchSyncRequest,
) -> Result<BatchSyncResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Get user's Tempo token
    let user: Option<crate::models::User> = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&claims.sub)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    let user = user.ok_or("User not found".to_string())?;

    let _tempo_token = user
        .tempo_token
        .ok_or("Tempo token not configured".to_string())?;

    let mut synced = 0;
    let mut failed = 0;
    let mut errors = Vec::new();

    for item_id in &request.work_item_ids {
        let item: Option<WorkItem> =
            sqlx::query_as("SELECT * FROM work_items WHERE id = ? AND user_id = ?")
                .bind(item_id)
                .bind(&claims.sub)
                .fetch_optional(&db.pool)
                .await
                .map_err(|e| e.to_string())?;

        let item = match item {
            Some(i) => i,
            None => {
                failed += 1;
                errors.push(format!("Work item {} not found", item_id));
                continue;
            }
        };

        if item.jira_issue_key.is_none() {
            failed += 1;
            errors.push(format!("Work item {} has no Jira issue mapped", item_id));
            continue;
        }

        // TODO: Call Tempo API to create worklog
        let now = Utc::now();
        if let Err(e) = sqlx::query("UPDATE work_items SET synced_to_tempo = 1, synced_at = ? WHERE id = ?")
            .bind(now)
            .bind(item_id)
            .execute(&db.pool)
            .await
        {
            failed += 1;
            errors.push(format!("Failed to update {}: {}", item_id, e));
            continue;
        }

        synced += 1;
    }

    Ok(BatchSyncResponse {
        synced,
        failed,
        errors,
    })
}

/// Aggregate work items by project + date
#[tauri::command]
pub async fn aggregate_work_items(
    state: State<'_, AppState>,
    token: String,
    request: AggregateRequest,
) -> Result<AggregateResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Build parameterized query safely
    let mut builder = SafeQueryBuilder::new();
    builder.add_string_condition("user_id", "=", &claims.sub);

    if let Some(start) = &request.start_date {
        builder.add_string_condition("date", ">=", start);
    }
    if let Some(end) = &request.end_date {
        builder.add_string_condition("date", "<=", end);
    }
    if let Some(source) = &request.source {
        builder.add_string_condition("source", "=", source);
    }

    let work_items: Vec<WorkItem> = builder
        .fetch_all(
            &db.pool,
            "SELECT * FROM work_items",
            "ORDER BY date, title",
            None,
            None,
        )
        .await?;

    let original_count = work_items.len();

    // Group by project + date
    let mut groups: HashMap<String, Vec<WorkItem>> = HashMap::new();

    for item in work_items {
        let project = if let Some(start_idx) = item.title.find('[') {
            if let Some(end_idx) = item.title.find(']') {
                item.title[start_idx + 1..end_idx].to_string()
            } else {
                "其他".to_string()
            }
        } else if let Some(desc) = &item.description {
            if let Some(line) = desc.lines().find(|l| l.starts_with("Project:")) {
                line.split('/').last().unwrap_or("其他").to_string()
            } else {
                "其他".to_string()
            }
        } else {
            "其他".to_string()
        };

        let key = format!("{}|{}", project, item.date);
        groups.entry(key).or_default().push(item);
    }

    let mut aggregated_count = 0;
    let mut child_ids: Vec<String> = Vec::new();

    for (key, items) in groups {
        if items.len() <= 1 {
            continue;
        }

        let parts: Vec<&str> = key.split('|').collect();
        let project_name = parts[0];
        let date = parts.get(1).unwrap_or(&"");

        let total_hours: f64 = items.iter().map(|i| i.hours).sum();

        // Extract unique tasks
        let mut tasks: Vec<String> = Vec::new();
        for item in &items {
            let task = if let Some(idx) = item.title.find(']') {
                item.title[idx + 1..].trim().to_string()
            } else {
                item.title.clone()
            };

            let task = if task.len() > 80 {
                format!("{}...", &task.chars().take(80).collect::<String>())
            } else {
                task
            };

            if !task.is_empty() && !tasks.contains(&task) {
                tasks.push(task);
            }
        }

        let title = format!("[{}] {} 項工作", project_name, tasks.len());

        let task_list = tasks.iter()
            .take(10)
            .enumerate()
            .map(|(i, t)| format!("{}. {}", i + 1, t))
            .collect::<Vec<_>>()
            .join("\n");

        let remaining = if tasks.len() > 10 {
            format!("\n...還有 {} 項", tasks.len() - 10)
        } else {
            String::new()
        };

        let description = format!(
            "工作內容：\n{}{}\n\n總時數：{:.1}h | 原始項目數：{}",
            task_list, remaining, total_hours, items.len()
        );

        let first = &items[0];
        let jira_key = items.iter().find_map(|i| i.jira_issue_key.clone());
        let jira_title = items.iter().find_map(|i| i.jira_issue_title.clone());
        let category = first.category.clone();

        let parent_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"INSERT INTO work_items
            (id, user_id, source, source_id, title, description, hours, date,
             jira_issue_key, jira_issue_title, category, synced_to_tempo, parent_id, created_at, updated_at)
            VALUES (?, ?, 'aggregated', ?, ?, ?, ?, ?, ?, ?, ?, 0, NULL, ?, ?)"#
        )
        .bind(&parent_id)
        .bind(&claims.sub)
        .bind(format!("agg-{}-{}", project_name, date))
        .bind(&title)
        .bind(&description)
        .bind(total_hours)
        .bind(date)
        .bind(&jira_key)
        .bind(&jira_title)
        .bind(&category)
        .bind(now)
        .bind(now)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

        aggregated_count += 1;

        for item in &items {
            child_ids.push(item.id.clone());
        }

        // Update parent_id for child items
        for chunk in items.chunks(100) {
            let placeholders: Vec<&str> = chunk.iter().map(|_| "?").collect();
            let sql = format!(
                "UPDATE work_items SET parent_id = ? WHERE id IN ({}) AND user_id = ?",
                placeholders.join(",")
            );

            let mut query = sqlx::query(&sql);
            query = query.bind(&parent_id);
            for item in chunk {
                query = query.bind(&item.id);
            }
            query = query.bind(&claims.sub);

            query.execute(&db.pool)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    let grouped_count = child_ids.len();

    Ok(AggregateResponse {
        original_count,
        aggregated_count,
        deleted_count: grouped_count,
    })
}
