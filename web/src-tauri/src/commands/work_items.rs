//! Work Items commands
//!
//! Tauri commands for work item operations.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

use crate::auth::verify_token;
use crate::models::{CreateWorkItem, PaginatedResponse, UpdateWorkItem, WorkItem};

use super::AppState;

// Types

#[derive(Debug, Serialize)]
pub struct WorkItemWithChildren {
    #[serde(flatten)]
    pub item: WorkItem,
    pub child_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct WorkItemFilters {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub source: Option<String>,
    pub category: Option<String>,
    pub jira_mapped: Option<bool>,
    pub synced_to_tempo: Option<bool>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub parent_id: Option<String>,
    pub show_all: Option<bool>,
}

// Grouped view types

#[derive(Debug, Serialize)]
pub struct WorkLogItem {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub hours: f64,
    pub date: String,
    pub source: String,
    pub synced_to_tempo: bool,
}

#[derive(Debug, Serialize)]
pub struct JiraIssueGroup {
    pub jira_key: Option<String>,
    pub jira_title: Option<String>,
    pub total_hours: f64,
    pub logs: Vec<WorkLogItem>,
}

#[derive(Debug, Serialize)]
pub struct ProjectGroup {
    pub project_name: String,
    pub total_hours: f64,
    pub issues: Vec<JiraIssueGroup>,
}

#[derive(Debug, Serialize)]
pub struct DateGroup {
    pub date: String,
    pub total_hours: f64,
    pub projects: Vec<ProjectGroup>,
}

#[derive(Debug, Serialize)]
pub struct GroupedWorkItemsResponse {
    pub by_project: Vec<ProjectGroup>,
    pub by_date: Vec<DateGroup>,
    pub total_hours: f64,
    pub total_items: i64,
}

#[derive(Debug, Deserialize)]
pub struct GroupedQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

// Stats types

#[derive(Debug, Serialize)]
pub struct DailyHours {
    pub date: String,
    pub hours: f64,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct JiraMappingStats {
    pub mapped: i64,
    pub unmapped: i64,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct TempoSyncStats {
    pub synced: i64,
    pub not_synced: i64,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct WorkItemStatsResponse {
    pub total_items: i64,
    pub total_hours: f64,
    pub hours_by_source: HashMap<String, f64>,
    pub hours_by_project: HashMap<String, f64>,
    pub hours_by_category: HashMap<String, f64>,
    pub daily_hours: Vec<DailyHours>,
    pub jira_mapping: JiraMappingStats,
    pub tempo_sync: TempoSyncStats,
}

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

// Timeline types

#[derive(Debug, Serialize)]
pub struct TimelineCommit {
    pub hash: String,
    pub message: String,
    pub time: String,
    pub author: String,
}

#[derive(Debug, Serialize)]
pub struct TimelineSession {
    pub id: String,
    pub project: String,
    pub title: String,
    pub start_time: String,
    pub end_time: String,
    pub hours: f64,
    pub commits: Vec<TimelineCommit>,
}

#[derive(Debug, Serialize)]
pub struct TimelineResponse {
    pub date: String,
    pub sessions: Vec<TimelineSession>,
    pub total_hours: f64,
    pub total_commits: i32,
}

// Batch sync types

#[derive(Debug, Deserialize)]
pub struct BatchSyncRequest {
    pub work_item_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BatchSyncResponse {
    pub synced: i64,
    pub failed: i64,
    pub errors: Vec<String>,
}

// Aggregate types

#[derive(Debug, Deserialize)]
pub struct AggregateRequest {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AggregateResponse {
    pub original_count: usize,
    pub aggregated_count: usize,
    pub deleted_count: usize,
}

// Commands

/// List work items with filters
#[tauri::command]
pub async fn list_work_items(
    state: State<'_, AppState>,
    token: String,
    filters: WorkItemFilters,
) -> Result<PaginatedResponse<WorkItemWithChildren>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let page = filters.page.unwrap_or(1);
    let per_page = filters.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;

    // Build dynamic query
    let mut conditions = vec![format!("user_id = '{}'", claims.sub)];

    if let Some(parent_id) = &filters.parent_id {
        conditions.push(format!("parent_id = '{}'", parent_id.replace('\'', "''")));
    } else if !filters.show_all.unwrap_or(false) {
        conditions.push("parent_id IS NULL".to_string());
    }

    if let Some(source) = &filters.source {
        conditions.push(format!("source = '{}'", source.replace('\'', "''")));
    }

    if let Some(category) = &filters.category {
        conditions.push(format!("category = '{}'", category.replace('\'', "''")));
    }

    if let Some(jira_mapped) = filters.jira_mapped {
        if jira_mapped {
            conditions.push("jira_issue_key IS NOT NULL".to_string());
        } else {
            conditions.push("jira_issue_key IS NULL".to_string());
        }
    }

    if let Some(synced) = filters.synced_to_tempo {
        conditions.push(format!("synced_to_tempo = {}", if synced { 1 } else { 0 }));
    }

    if let Some(start_date) = &filters.start_date {
        conditions.push(format!("date >= '{}'", start_date));
    }

    if let Some(end_date) = &filters.end_date {
        conditions.push(format!("date <= '{}'", end_date));
    }

    let where_clause = conditions.join(" AND ");

    // Count total
    let count_query = format!("SELECT COUNT(*) FROM work_items WHERE {}", where_clause);
    let total: (i64,) = sqlx::query_as(&count_query)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    // Fetch items
    let query = format!(
        "SELECT * FROM work_items WHERE {} ORDER BY date DESC, created_at DESC LIMIT {} OFFSET {}",
        where_clause, per_page, offset
    );

    let items: Vec<WorkItem> = sqlx::query_as(&query)
        .fetch_all(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    // Get child counts
    let mut items_with_children: Vec<WorkItemWithChildren> = Vec::new();
    for item in items {
        let child_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM work_items WHERE parent_id = ?")
            .bind(&item.id)
            .fetch_one(&db.pool)
            .await
            .map_err(|e| e.to_string())?;

        items_with_children.push(WorkItemWithChildren {
            item,
            child_count: child_count.0,
        });
    }

    let pages = (total.0 as f64 / per_page as f64).ceil() as i64;

    Ok(PaginatedResponse {
        items: items_with_children,
        total: total.0,
        page,
        per_page,
        pages,
    })
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

    sqlx::query(
        r#"INSERT INTO work_items (id, user_id, source, source_id, title, description, hours, date,
            jira_issue_key, jira_issue_title, category, tags, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(&claims.sub)
    .bind(&source)
    .bind(&request.source_id)
    .bind(&request.title)
    .bind(&request.description)
    .bind(request.hours.unwrap_or(0.0))
    .bind(&request.date)
    .bind(&request.jira_issue_key)
    .bind(&request.jira_issue_title)
    .bind(&request.category)
    .bind(&tags_json)
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

/// Get work item statistics summary
#[tauri::command]
pub async fn get_stats_summary(
    state: State<'_, AppState>,
    token: String,
    query: StatsQuery,
) -> Result<WorkItemStatsResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Build date filter
    let mut date_filter = String::new();
    if let Some(start) = &query.start_date {
        date_filter.push_str(&format!(" AND date >= '{}'", start));
    }
    if let Some(end) = &query.end_date {
        date_filter.push_str(&format!(" AND date <= '{}'", end));
    }

    let sql = format!("SELECT * FROM work_items WHERE user_id = ?{}", date_filter);
    let work_items: Vec<WorkItem> = sqlx::query_as(&sql)
        .bind(&claims.sub)
        .fetch_all(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    let total_items = work_items.len() as i64;
    let total_hours: f64 = work_items.iter().map(|i| i.hours).sum();

    // Hours by source
    let mut hours_by_source: HashMap<String, f64> = HashMap::new();
    for item in &work_items {
        *hours_by_source.entry(item.source.clone()).or_insert(0.0) += item.hours;
    }

    // Hours by project
    let mut hours_by_project: HashMap<String, f64> = HashMap::new();
    for item in &work_items {
        let project_name = if item.title.starts_with('[') {
            item.title
                .split(']')
                .next()
                .map(|s| s.trim_start_matches('[').to_string())
                .unwrap_or_else(|| "未知專案".to_string())
        } else {
            "未知專案".to_string()
        };
        *hours_by_project.entry(project_name).or_insert(0.0) += item.hours;
    }

    // Hours by category
    let mut hours_by_category: HashMap<String, f64> = HashMap::new();
    for item in &work_items {
        let cat = item.category.clone().unwrap_or_else(|| "未分類".to_string());
        *hours_by_category.entry(cat).or_insert(0.0) += item.hours;
    }

    // Daily hours for heatmap
    let mut daily_map: HashMap<String, (f64, i64)> = HashMap::new();
    for item in &work_items {
        let entry = daily_map.entry(item.date.to_string()).or_insert((0.0, 0));
        entry.0 += item.hours;
        entry.1 += 1;
    }
    let daily_hours: Vec<DailyHours> = daily_map
        .into_iter()
        .map(|(date, (hours, count))| DailyHours { date, hours, count })
        .collect();

    // Jira mapping stats
    let mapped = work_items.iter().filter(|i| i.jira_issue_key.is_some()).count() as i64;
    let unmapped = total_items - mapped;
    let jira_percentage = if total_items > 0 {
        (mapped as f64 / total_items as f64) * 100.0
    } else {
        0.0
    };

    // Tempo sync stats
    let synced = work_items.iter().filter(|i| i.synced_to_tempo).count() as i64;
    let not_synced = total_items - synced;
    let tempo_percentage = if total_items > 0 {
        (synced as f64 / total_items as f64) * 100.0
    } else {
        0.0
    };

    Ok(WorkItemStatsResponse {
        total_items,
        total_hours,
        hours_by_source,
        hours_by_project,
        hours_by_category,
        daily_hours,
        jira_mapping: JiraMappingStats {
            mapped,
            unmapped,
            percentage: jira_percentage,
        },
        tempo_sync: TempoSyncStats {
            synced,
            not_synced,
            percentage: tempo_percentage,
        },
    })
}

/// Get work items grouped by project and date
#[tauri::command]
pub async fn get_grouped_work_items(
    state: State<'_, AppState>,
    token: String,
    query: GroupedQuery,
) -> Result<GroupedWorkItemsResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Build query
    let mut conditions = vec![format!("user_id = '{}'", claims.sub)];
    conditions.push("parent_id IS NULL".to_string());

    if let Some(start) = &query.start_date {
        conditions.push(format!("date >= '{}'", start));
    }
    if let Some(end) = &query.end_date {
        conditions.push(format!("date <= '{}'", end));
    }

    let sql = format!(
        "SELECT * FROM work_items WHERE {} ORDER BY date DESC, title",
        conditions.join(" AND ")
    );

    let items: Vec<WorkItem> = sqlx::query_as(&sql)
        .fetch_all(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    let total_items = items.len() as i64;
    let total_hours: f64 = items.iter().map(|i| i.hours).sum();

    // Helper to extract project name
    fn extract_project(title: &str, description: &Option<String>) -> String {
        if let Some(start) = title.find('[') {
            if let Some(end) = title.find(']') {
                return title[start + 1..end].to_string();
            }
        }
        if let Some(desc) = description {
            if let Some(line) = desc.lines().find(|l| l.starts_with("Project:")) {
                if let Some(name) = line.split('/').last() {
                    return name.to_string();
                }
            }
        }
        "其他".to_string()
    }

    // Group by project
    let mut projects_map: HashMap<String, HashMap<Option<String>, Vec<&WorkItem>>> = HashMap::new();
    for item in &items {
        let project = extract_project(&item.title, &item.description);
        let jira_key = item.jira_issue_key.clone();
        projects_map
            .entry(project)
            .or_default()
            .entry(jira_key)
            .or_default()
            .push(item);
    }

    let mut by_project: Vec<ProjectGroup> = projects_map
        .into_iter()
        .map(|(project_name, issues_map)| {
            let mut issues: Vec<JiraIssueGroup> = issues_map
                .into_iter()
                .map(|(jira_key, items)| {
                    let total_hours: f64 = items.iter().map(|i| i.hours).sum();
                    let jira_title = items.first().and_then(|i| i.jira_issue_title.clone());
                    let logs: Vec<WorkLogItem> = items
                        .into_iter()
                        .map(|i| WorkLogItem {
                            id: i.id.clone(),
                            title: i.title.clone(),
                            description: i.description.clone(),
                            hours: i.hours,
                            date: i.date.to_string(),
                            source: i.source.clone(),
                            synced_to_tempo: i.synced_to_tempo,
                        })
                        .collect();
                    JiraIssueGroup {
                        jira_key,
                        jira_title,
                        total_hours,
                        logs,
                    }
                })
                .collect();
            issues.sort_by(|a, b| b.total_hours.partial_cmp(&a.total_hours).unwrap());
            let total_hours: f64 = issues.iter().map(|i| i.total_hours).sum();
            ProjectGroup {
                project_name,
                total_hours,
                issues,
            }
        })
        .collect();
    by_project.sort_by(|a, b| b.total_hours.partial_cmp(&a.total_hours).unwrap());

    // Group by date
    let mut dates_map: HashMap<String, HashMap<String, Vec<&WorkItem>>> = HashMap::new();
    for item in &items {
        let date = item.date.to_string();
        let project = extract_project(&item.title, &item.description);
        dates_map
            .entry(date)
            .or_default()
            .entry(project)
            .or_default()
            .push(item);
    }

    let mut by_date: Vec<DateGroup> = dates_map
        .into_iter()
        .map(|(date, projects_map)| {
            let mut projects: Vec<ProjectGroup> = projects_map
                .into_iter()
                .map(|(project_name, items)| {
                    let mut jira_map: HashMap<Option<String>, Vec<&WorkItem>> = HashMap::new();
                    for item in items {
                        jira_map.entry(item.jira_issue_key.clone()).or_default().push(item);
                    }
                    let issues: Vec<JiraIssueGroup> = jira_map
                        .into_iter()
                        .map(|(jira_key, items)| {
                            let total_hours: f64 = items.iter().map(|i| i.hours).sum();
                            let jira_title = items.first().and_then(|i| i.jira_issue_title.clone());
                            let logs: Vec<WorkLogItem> = items
                                .into_iter()
                                .map(|i| WorkLogItem {
                                    id: i.id.clone(),
                                    title: i.title.clone(),
                                    description: i.description.clone(),
                                    hours: i.hours,
                                    date: i.date.to_string(),
                                    source: i.source.clone(),
                                    synced_to_tempo: i.synced_to_tempo,
                                })
                                .collect();
                            JiraIssueGroup { jira_key, jira_title, total_hours, logs }
                        })
                        .collect();
                    let total_hours: f64 = issues.iter().map(|i| i.total_hours).sum();
                    ProjectGroup { project_name, total_hours, issues }
                })
                .collect();
            projects.sort_by(|a, b| b.total_hours.partial_cmp(&a.total_hours).unwrap());
            let total_hours: f64 = projects.iter().map(|p| p.total_hours).sum();
            DateGroup { date, total_hours, projects }
        })
        .collect();
    by_date.sort_by(|a, b| b.date.cmp(&a.date));

    Ok(GroupedWorkItemsResponse {
        by_project,
        by_date,
        total_hours,
        total_items,
    })
}

/// Get timeline data for Gantt chart visualization
/// Optimized version with parallel processing and early filtering
#[tauri::command]
pub async fn get_timeline_data(
    _state: State<'_, AppState>,
    token: String,
    date: String,
) -> Result<TimelineResponse, String> {
    let _claims = verify_token(&token).map_err(|e| e.to_string())?;

    use std::path::PathBuf;
    use chrono::NaiveDate;

    let target_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date format: {}", e))?;

    let claude_home = std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".claude").join("projects"));

    let projects_dir = match claude_home {
        Some(dir) if dir.exists() => dir,
        _ => {
            return Ok(TimelineResponse {
                date,
                sessions: Vec::new(),
                total_hours: 0.0,
                total_commits: 0,
            });
        }
    };

    // Phase 1: Collect all candidate files (quick filesystem scan)
    let mut candidate_files: Vec<(PathBuf, String)> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&projects_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let dir_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            if dir_name.starts_with('.') {
                continue;
            }

            let project_name = dir_name.split('-').last().unwrap_or(&dir_name).to_string();

            if let Ok(files) = std::fs::read_dir(&path) {
                for file_entry in files.flatten() {
                    let file_path = file_entry.path();
                    if !file_path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                        continue;
                    }

                    // Quick filter: check file modification date
                    if let Ok(file_meta) = file_entry.metadata() {
                        if let Ok(modified) = file_meta.modified() {
                            let modified_date: chrono::DateTime<chrono::Local> = modified.into();
                            let file_date = modified_date.date_naive();
                            // Allow files modified on target date or day after (for sessions spanning midnight)
                            let day_before = target_date - chrono::Duration::days(1);
                            let day_after = target_date + chrono::Duration::days(1);
                            if file_date < day_before || file_date > day_after {
                                continue;
                            }
                        }
                    }

                    candidate_files.push((file_path, project_name.clone()));
                }
            }
        }
    }

    // Phase 2: Process files in parallel using tokio
    let date_clone = date.clone();
    let sessions: Vec<TimelineSession> = tokio::task::spawn_blocking(move || {
        use std::sync::Mutex;
        use std::thread;

        let results: Mutex<Vec<TimelineSession>> = Mutex::new(Vec::new());
        let chunk_size = (candidate_files.len() / 4).max(1);
        let chunks: Vec<_> = candidate_files.chunks(chunk_size).map(|c| c.to_vec()).collect();

        thread::scope(|s| {
            for chunk in chunks {
                let results_ref = &results;
                let date_ref = &date_clone;
                s.spawn(move || {
                    let mut local_sessions = Vec::new();
                    for (file_path, project_name) in chunk {
                        if let Some(metadata) = parse_session_timestamps_fast(&file_path) {
                            let session_date = metadata.first_ts.split('T').next().unwrap_or("");
                            if session_date != date_ref {
                                continue;
                            }

                            let hours = calculate_hours(&metadata.first_ts, &metadata.last_ts);
                            if hours < 0.08 {
                                continue;
                            }

                            let actual_project_name = metadata.cwd.as_ref()
                                .and_then(|c| c.split('/').last())
                                .unwrap_or(&project_name)
                                .to_string();

                            let project_path_for_git = metadata.cwd.clone().unwrap_or_default();

                            let commits = get_commits_in_range(&project_path_for_git, &metadata.first_ts, &metadata.last_ts);

                            let session_id = file_path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string();

                            let title = metadata.first_msg.unwrap_or_else(|| "Claude Code session".to_string());

                            local_sessions.push(TimelineSession {
                                id: session_id,
                                project: actual_project_name,
                                title,
                                start_time: metadata.first_ts,
                                end_time: metadata.last_ts,
                                hours,
                                commits,
                            });
                        }
                    }
                    results_ref.lock().unwrap().extend(local_sessions);
                });
            }
        });

        results.into_inner().unwrap()
    }).await.map_err(|e| format!("Task failed: {}", e))?;

    let mut sessions = sessions;
    sessions.sort_by(|a, b| a.start_time.cmp(&b.start_time));

    let total_hours: f64 = sessions.iter().map(|s| s.hours).sum();
    let total_commits: i32 = sessions.iter().map(|s| s.commits.len() as i32).sum();

    Ok(TimelineResponse {
        date,
        sessions,
        total_hours,
        total_commits,
    })
}

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

    // Build query with filters
    let mut conditions = vec![format!("user_id = '{}'", claims.sub)];

    if let Some(start) = &request.start_date {
        conditions.push(format!("date >= '{}'", start));
    }
    if let Some(end) = &request.end_date {
        conditions.push(format!("date <= '{}'", end));
    }
    if let Some(source) = &request.source {
        conditions.push(format!("source = '{}'", source.replace('\'', "''")));
    }

    let sql = format!(
        "SELECT * FROM work_items WHERE {} ORDER BY date, title",
        conditions.join(" AND ")
    );

    let work_items: Vec<WorkItem> = sqlx::query_as(&sql)
        .fetch_all(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

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

// Helper functions

struct SessionMetadata {
    first_ts: String,
    last_ts: String,
    first_msg: Option<String>,
    cwd: Option<String>,
}

/// Optimized version: reads only the beginning and end of the file
/// instead of parsing the entire JSONL file line by line
fn parse_session_timestamps_fast(path: &std::path::PathBuf) -> Option<SessionMetadata> {
    use std::io::{BufRead, BufReader, Seek, SeekFrom};

    let file = std::fs::File::open(path).ok()?;
    let file_size = file.metadata().ok()?.len();

    // For small files (< 50KB), use the original approach
    if file_size < 50_000 {
        return parse_session_timestamps_full(path);
    }

    let mut reader = BufReader::new(file);

    let mut first_ts: Option<String> = None;
    let mut last_ts: Option<String> = None;
    let mut first_msg: Option<String> = None;
    let mut cwd: Option<String> = None;
    let mut meaningful_count = 0;

    // Read first 20 lines to get first_ts, cwd, and first_msg
    let mut lines_read = 0;
    let max_initial_lines = 20;

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) {
                    if first_ts.is_none() {
                        if let Some(ts) = msg.get("timestamp").and_then(|v| v.as_str()) {
                            first_ts = Some(ts.to_string());
                        }
                    }

                    if cwd.is_none() {
                        if let Some(c) = msg.get("cwd").and_then(|v| v.as_str()) {
                            cwd = Some(c.to_string());
                        }
                    }

                    if first_msg.is_none() {
                        if let Some(message) = msg.get("message") {
                            if message.get("role").and_then(|r| r.as_str()) == Some("user") {
                                if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                                    let trimmed = content.trim();
                                    if trimmed.len() >= 10
                                        && !trimmed.to_lowercase().starts_with("warmup")
                                        && !trimmed.starts_with("<command-")
                                    {
                                        meaningful_count += 1;
                                        first_msg = Some(trimmed.chars().take(150).collect());
                                    }
                                }
                            }
                        }
                    }
                }

                lines_read += 1;
                if lines_read >= max_initial_lines && first_ts.is_some() && cwd.is_some() {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    // If we couldn't get first_ts, fall back to full parse
    if first_ts.is_none() {
        return parse_session_timestamps_full(path);
    }

    // Read the last ~32KB of the file to find the last timestamp
    let tail_size: u64 = 32_000.min(file_size);
    let seek_pos = file_size.saturating_sub(tail_size);

    if reader.seek(SeekFrom::Start(seek_pos)).is_ok() {
        // Skip partial line if we're not at the start
        if seek_pos > 0 {
            let mut skip_line = String::new();
            let _ = reader.read_line(&mut skip_line);
        }

        // Read remaining lines to find the last timestamp
        for line in reader.lines().flatten() {
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(ts) = msg.get("timestamp").and_then(|v| v.as_str()) {
                    last_ts = Some(ts.to_string());
                }

                // Also look for meaningful messages in case we missed one
                if meaningful_count == 0 {
                    if let Some(message) = msg.get("message") {
                        if message.get("role").and_then(|r| r.as_str()) == Some("user") {
                            if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                                let trimmed = content.trim();
                                if trimmed.len() >= 10
                                    && !trimmed.to_lowercase().starts_with("warmup")
                                    && !trimmed.starts_with("<command-")
                                {
                                    meaningful_count += 1;
                                    if first_msg.is_none() {
                                        first_msg = Some(trimmed.chars().take(150).collect());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if meaningful_count == 0 {
        return None;
    }

    let last_ts = last_ts.or_else(|| first_ts.clone());
    match (first_ts, last_ts) {
        (Some(f), Some(l)) => Some(SessionMetadata {
            first_ts: f,
            last_ts: l,
            first_msg,
            cwd,
        }),
        _ => None,
    }
}

/// Full file parse (used for small files or as fallback)
fn parse_session_timestamps_full(path: &std::path::PathBuf) -> Option<SessionMetadata> {
    use std::io::{BufRead, BufReader};

    let file = std::fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut first_ts: Option<String> = None;
    let mut last_ts: Option<String> = None;
    let mut first_msg: Option<String> = None;
    let mut cwd: Option<String> = None;
    let mut meaningful_count = 0;

    for line in reader.lines().flatten() {
        if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) {
            if let Some(ts) = msg.get("timestamp").and_then(|v| v.as_str()) {
                if first_ts.is_none() {
                    first_ts = Some(ts.to_string());
                }
                last_ts = Some(ts.to_string());
            }

            if cwd.is_none() {
                if let Some(c) = msg.get("cwd").and_then(|v| v.as_str()) {
                    cwd = Some(c.to_string());
                }
            }

            if first_msg.is_none() {
                if let Some(message) = msg.get("message") {
                    if message.get("role").and_then(|r| r.as_str()) == Some("user") {
                        if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                            let trimmed = content.trim();
                            if trimmed.len() >= 10
                                && !trimmed.to_lowercase().starts_with("warmup")
                                && !trimmed.starts_with("<command-")
                            {
                                meaningful_count += 1;
                                first_msg = Some(trimmed.chars().take(150).collect());
                            }
                        }
                    }
                }
            }
        }
    }

    if meaningful_count == 0 {
        return None;
    }

    match (first_ts, last_ts) {
        (Some(f), Some(l)) => Some(SessionMetadata {
            first_ts: f,
            last_ts: l,
            first_msg,
            cwd,
        }),
        _ => None,
    }
}

fn calculate_hours(start: &str, end: &str) -> f64 {
    if let (Ok(start_dt), Ok(end_dt)) = (
        chrono::DateTime::parse_from_rfc3339(start),
        chrono::DateTime::parse_from_rfc3339(end),
    ) {
        let duration = end_dt.signed_duration_since(start_dt);
        let hours = duration.num_minutes() as f64 / 60.0;
        hours.min(8.0).max(0.1)
    } else {
        0.5
    }
}

fn get_commits_in_range(project_path: &str, start: &str, end: &str) -> Vec<TimelineCommit> {
    use std::path::PathBuf;
    use std::process::Command;

    if project_path.is_empty() {
        return Vec::new();
    }

    let project_dir = PathBuf::from(project_path);
    if !project_dir.exists() || !project_dir.join(".git").exists() {
        return Vec::new();
    }

    let output = Command::new("git")
        .arg("log")
        .arg("--since")
        .arg(start)
        .arg("--until")
        .arg(end)
        .arg("--format=%H|%an|%aI|%s")
        .arg("--all")
        .current_dir(&project_dir)
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    if !output.status.success() {
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() >= 4 {
            commits.push(TimelineCommit {
                hash: parts[0].chars().take(8).collect(),
                author: parts[1].to_string(),
                time: parts[2].to_string(),
                message: parts[3].to_string(),
            });
        }
    }

    commits
}

// Commit-centric worklog types

#[derive(Debug, Serialize)]
pub struct CommitCentricWorklog {
    pub date: String,
    pub project: String,
    pub commits: Vec<crate::services::CommitRecord>,
    pub standalone_sessions: Vec<crate::services::StandaloneSession>,
    pub total_commits: i32,
    pub total_hours: f64,
}

#[derive(Debug, Deserialize)]
pub struct CommitCentricQuery {
    pub date: String,
    pub project_path: Option<String>,
}

/// Get commit-centric worklog for a date
/// Returns commits as primary records with session data as supplementary
#[tauri::command]
pub async fn get_commit_centric_worklog(
    _state: State<'_, AppState>,
    token: String,
    query: CommitCentricQuery,
) -> Result<CommitCentricWorklog, String> {
    use chrono::NaiveDate;
    use crate::services::get_commits_for_date;

    let _claims = crate::auth::verify_token(&token).map_err(|e| e.to_string())?;

    let date = NaiveDate::parse_from_str(&query.date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date format: {}", e))?;

    // Determine project path
    let project_path = query.project_path.unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    let project_name = project_path
        .split('/')
        .last()
        .unwrap_or("unknown")
        .to_string();

    // Get commits for the date
    let commits = get_commits_for_date(&project_path, &date);
    let total_commits = commits.len() as i32;

    // Calculate total hours from commits
    let commit_hours: f64 = commits.iter().map(|c| c.hours).sum();

    // Find Claude sessions for this project and date that don't have commits
    let standalone_sessions = find_standalone_sessions(&project_path, &query.date)?;

    // Calculate total hours (commits + standalone sessions)
    let session_hours: f64 = standalone_sessions.iter().map(|s| s.hours).sum();
    let total_hours = commit_hours + session_hours;

    Ok(CommitCentricWorklog {
        date: query.date,
        project: project_name,
        commits,
        standalone_sessions,
        total_commits,
        total_hours,
    })
}

/// Find Claude sessions that don't have associated commits
fn find_standalone_sessions(
    project_path: &str,
    date: &str,
) -> Result<Vec<crate::services::StandaloneSession>, String> {
    use chrono::{DateTime, NaiveDate, Local};
    use crate::services::{build_rule_based_outcome, StandaloneSession};

    let target_date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date: {}", e))?;

    let claude_home = std::env::var("HOME")
        .ok()
        .map(|h| std::path::PathBuf::from(h).join(".claude").join("projects"));

    let projects_dir = match claude_home {
        Some(dir) if dir.exists() => dir,
        _ => return Ok(Vec::new()),
    };

    let mut standalone = Vec::new();

    // Find the Claude project directory for this project
    let project_dir_name = project_path.replace('/', "-");

    if let Ok(entries) = std::fs::read_dir(&projects_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let dir_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Check if this directory matches our project
            if !dir_name.contains(&project_dir_name) && !project_dir_name.contains(&dir_name) {
                continue;
            }

            // Read session files
            if let Ok(files) = std::fs::read_dir(&path) {
                for file_entry in files.flatten() {
                    let file_path = file_entry.path();
                    if !file_path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                        continue;
                    }

                    // Check file modification date
                    if let Ok(metadata) = file_entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            let modified_date: DateTime<Local> = modified.into();
                            let file_date = modified_date.date_naive();
                            if file_date != target_date {
                                continue;
                            }
                        }
                    }

                    // Parse session to check if it has commits
                    if let Some(session_data) = parse_session_for_worklog(&file_path, &target_date) {
                        // Only include if no commits were made during this session
                        if session_data.commit_count == 0 {
                            let outcome = build_rule_based_outcome(
                                &session_data.files_modified,
                                &session_data.tools_used,
                                session_data.first_message.as_deref(),
                            );

                            standalone.push(StandaloneSession {
                                session_id: session_data.session_id,
                                project: project_path.split('/').last().unwrap_or("unknown").to_string(),
                                start_time: session_data.start_time,
                                end_time: session_data.end_time,
                                hours: session_data.hours,
                                outcome,
                                outcome_source: "rule".to_string(),
                                tools_used: session_data.tools_used,
                                files_modified: session_data.files_modified,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(standalone)
}

/// Session data for worklog generation
struct SessionWorklogData {
    session_id: String,
    start_time: String,
    end_time: String,
    hours: f64,
    first_message: Option<String>,
    tools_used: HashMap<String, usize>,
    files_modified: Vec<String>,
    commit_count: usize,
}

/// Parse a session file to extract worklog-relevant data
fn parse_session_for_worklog(
    path: &std::path::PathBuf,
    target_date: &chrono::NaiveDate,
) -> Option<SessionWorklogData> {
    use std::io::{BufRead, BufReader};
    use std::collections::HashMap;

    let file = std::fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    let session_id = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut first_ts: Option<String> = None;
    let mut last_ts: Option<String> = None;
    let mut first_message: Option<String> = None;
    let mut tools_used: HashMap<String, usize> = HashMap::new();
    let mut files_modified: Vec<String> = Vec::new();
    let mut commit_count = 0;

    for line in reader.lines().flatten() {
        if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) {
            // Extract timestamp
            if let Some(ts) = msg.get("timestamp").and_then(|v| v.as_str()) {
                if first_ts.is_none() {
                    first_ts = Some(ts.to_string());
                }
                last_ts = Some(ts.to_string());
            }

            // Extract first meaningful user message
            if first_message.is_none() {
                if let Some(message) = msg.get("message") {
                    if message.get("role").and_then(|r| r.as_str()) == Some("user") {
                        if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                            let trimmed = content.trim();
                            if trimmed.len() >= 10
                                && !trimmed.to_lowercase().starts_with("warmup")
                                && !trimmed.starts_with("<command-")
                            {
                                first_message = Some(trimmed.chars().take(100).collect());
                            }
                        }
                    }
                }
            }

            // Extract tool usage from assistant messages
            if let Some(message) = msg.get("message") {
                if let Some(content) = message.get("content") {
                    if let Some(arr) = content.as_array() {
                        for item in arr {
                            if item.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                                if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                                    *tools_used.entry(name.to_string()).or_insert(0) += 1;

                                    // Track file modifications
                                    if name == "Edit" || name == "Write" {
                                        if let Some(input) = item.get("input") {
                                            if let Some(file_path) = input.get("file_path").and_then(|f| f.as_str()) {
                                                if !files_modified.contains(&file_path.to_string()) {
                                                    files_modified.push(file_path.to_string());
                                                }
                                            }
                                        }
                                    }

                                    // Count git commits
                                    if name == "Bash" {
                                        if let Some(input) = item.get("input") {
                                            if let Some(cmd) = input.get("command").and_then(|c| c.as_str()) {
                                                if cmd.contains("git commit") {
                                                    commit_count += 1;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let (first_ts, last_ts) = (first_ts?, last_ts?);

    // Calculate hours
    let hours = if let (Ok(start), Ok(end)) = (
        chrono::DateTime::parse_from_rfc3339(&first_ts),
        chrono::DateTime::parse_from_rfc3339(&last_ts),
    ) {
        // Check if session is on target date
        let session_date = start.date_naive();
        if session_date != *target_date {
            return None;
        }

        let duration = end.signed_duration_since(start);
        (duration.num_minutes() as f64 / 60.0).max(0.1).min(8.0)
    } else {
        return None;
    };

    Some(SessionWorklogData {
        session_id,
        start_time: first_ts,
        end_time: last_ts,
        hours,
        first_message,
        tools_used,
        files_modified,
        commit_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_jsonl(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_calculate_hours_valid() {
        let start = "2025-01-10T09:00:00+08:00";
        let end = "2025-01-10T11:30:00+08:00";
        let hours = calculate_hours(start, end);
        assert!((hours - 2.5).abs() < 0.01);
    }

    #[test]
    fn test_calculate_hours_max_cap() {
        // Should cap at 8 hours
        let start = "2025-01-10T00:00:00+08:00";
        let end = "2025-01-10T12:00:00+08:00";
        let hours = calculate_hours(start, end);
        assert!((hours - 8.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_hours_invalid_format() {
        let hours = calculate_hours("invalid", "also-invalid");
        assert!((hours - 0.5).abs() < 0.01); // Default fallback
    }

    #[test]
    fn test_parse_session_timestamps_full_basic() {
        let content = r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"This is a meaningful test message"}}
{"timestamp":"2025-01-10T09:30:00+08:00","message":{"role":"assistant","content":"Response"}}
{"timestamp":"2025-01-10T10:00:00+08:00","message":{"role":"user","content":"Another message"}}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_full(&path);
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert_eq!(metadata.first_ts, "2025-01-10T09:00:00+08:00");
        assert_eq!(metadata.last_ts, "2025-01-10T10:00:00+08:00");
        assert_eq!(metadata.cwd, Some("/home/user/project".to_string()));
        assert!(metadata.first_msg.is_some());
    }

    #[test]
    fn test_parse_session_timestamps_full_no_meaningful_message() {
        let content = r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project"}
{"timestamp":"2025-01-10T09:30:00+08:00","message":{"role":"user","content":"short"}}
{"timestamp":"2025-01-10T10:00:00+08:00","message":{"role":"user","content":"warmup test"}}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_full(&path);
        assert!(result.is_none()); // No meaningful message found
    }

    #[test]
    fn test_parse_session_timestamps_full_skip_command_messages() {
        let content = r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"<command-name>test</command-name>"}}
{"timestamp":"2025-01-10T09:30:00+08:00","message":{"role":"user","content":"This is a real meaningful message here"}}
{"timestamp":"2025-01-10T10:00:00+08:00"}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_full(&path);
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert!(metadata.first_msg.unwrap().contains("real meaningful"));
    }

    #[test]
    fn test_parse_session_timestamps_fast_small_file() {
        // Small files should use full parse
        let content = r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"This is a meaningful test message"}}
{"timestamp":"2025-01-10T10:00:00+08:00"}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_fast(&path);
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert_eq!(metadata.first_ts, "2025-01-10T09:00:00+08:00");
        assert_eq!(metadata.last_ts, "2025-01-10T10:00:00+08:00");
    }

    #[test]
    fn test_parse_session_timestamps_fast_large_file() {
        // Create a large file (> 50KB) to test the fast path
        let mut content = String::new();

        // First line with timestamp and meaningful message
        content.push_str(r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"This is a meaningful test message for the session"}}"#);
        content.push('\n');

        // Add padding lines to make file > 50KB
        for i in 0..500 {
            content.push_str(&format!(
                r#"{{"timestamp":"2025-01-10T09:{:02}:00+08:00","message":{{"role":"assistant","content":"Response line {} with some padding text to make this longer and reach the size threshold we need for testing the fast path optimization"}}}}"#,
                i % 60,
                i
            ));
            content.push('\n');
        }

        // Last line with final timestamp
        content.push_str(r#"{"timestamp":"2025-01-10T17:00:00+08:00","message":{"role":"assistant","content":"Final response"}}"#);

        let file = create_test_jsonl(&content);
        let path = file.path().to_path_buf();

        // Verify file is large enough
        let file_size = std::fs::metadata(&path).unwrap().len();
        assert!(file_size > 50_000, "Test file should be > 50KB, got {} bytes", file_size);

        let result = parse_session_timestamps_fast(&path);
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert_eq!(metadata.first_ts, "2025-01-10T09:00:00+08:00");
        assert_eq!(metadata.last_ts, "2025-01-10T17:00:00+08:00");
        assert_eq!(metadata.cwd, Some("/home/user/project".to_string()));
    }

    #[test]
    fn test_get_commits_in_range_empty_path() {
        let commits = get_commits_in_range("", "2025-01-10T00:00:00+08:00", "2025-01-10T23:59:59+08:00");
        assert!(commits.is_empty());
    }

    #[test]
    fn test_get_commits_in_range_nonexistent_path() {
        let commits = get_commits_in_range("/nonexistent/path", "2025-01-10T00:00:00+08:00", "2025-01-10T23:59:59+08:00");
        assert!(commits.is_empty());
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_parse_empty_file() {
        let file = create_test_jsonl("");
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_full(&path);
        assert!(result.is_none());

        let result_fast = parse_session_timestamps_fast(&path);
        assert!(result_fast.is_none());
    }

    #[test]
    fn test_parse_single_line_file() {
        let content = r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"This is a single meaningful message"}}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_full(&path);
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert_eq!(metadata.first_ts, "2025-01-10T09:00:00+08:00");
        assert_eq!(metadata.last_ts, "2025-01-10T09:00:00+08:00");
    }

    #[test]
    fn test_parse_corrupted_json_with_valid_lines() {
        let content = r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"Valid meaningful message here"}}
{this is not valid json at all}
{"timestamp":"2025-01-10T10:00:00+08:00"}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_full(&path);
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert_eq!(metadata.first_ts, "2025-01-10T09:00:00+08:00");
        assert_eq!(metadata.last_ts, "2025-01-10T10:00:00+08:00");
    }

    #[test]
    fn test_parse_message_truncation() {
        // Message longer than 150 chars should be truncated
        let long_message = "A".repeat(200);
        let content = format!(
            r#"{{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{{"role":"user","content":"{}"}}}}"#,
            long_message
        );

        let file = create_test_jsonl(&content);
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_full(&path);
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert!(metadata.first_msg.is_some());
        assert_eq!(metadata.first_msg.unwrap().len(), 150);
    }

    #[test]
    fn test_parse_midnight_crossing_session() {
        // Session that starts before midnight and ends after
        let content = r#"{"timestamp":"2025-01-10T23:30:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"Late night meaningful work session"}}
{"timestamp":"2025-01-11T00:30:00+08:00","message":{"role":"assistant","content":"Response"}}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_full(&path);
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert_eq!(metadata.first_ts, "2025-01-10T23:30:00+08:00");
        assert_eq!(metadata.last_ts, "2025-01-11T00:30:00+08:00");

        // Hours should be 1 hour
        let hours = calculate_hours(&metadata.first_ts, &metadata.last_ts);
        assert!((hours - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_no_timestamp_lines() {
        let content = r#"{"cwd":"/home/user/project","message":{"role":"user","content":"No timestamp here"}}
{"message":{"role":"assistant","content":"Also no timestamp"}}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_full(&path);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_only_assistant_messages() {
        // No user messages, only assistant - should return None
        let content = r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{"role":"assistant","content":"This is an assistant message"}}
{"timestamp":"2025-01-10T10:00:00+08:00","message":{"role":"assistant","content":"Another assistant message"}}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_full(&path);
        assert!(result.is_none()); // No meaningful user message
    }

    #[test]
    fn test_calculate_hours_minimum() {
        // Very short session should have minimum 0.1 hours
        let start = "2025-01-10T09:00:00+08:00";
        let end = "2025-01-10T09:01:00+08:00"; // 1 minute
        let hours = calculate_hours(start, end);
        assert!((hours - 0.1).abs() < 0.01); // Minimum is 0.1
    }

    #[test]
    fn test_calculate_hours_negative_duration() {
        // End before start (edge case)
        let start = "2025-01-10T10:00:00+08:00";
        let end = "2025-01-10T09:00:00+08:00";
        let hours = calculate_hours(start, end);
        // Should return minimum 0.1 due to .max(0.1)
        assert!((hours - 0.1).abs() < 0.01);
    }

    // ==================== Parallel Processing Tests ====================

    #[test]
    fn test_parallel_processing_multiple_files() {
        use std::sync::Arc;
        use std::thread;

        // Create multiple test files
        let files: Vec<_> = (0..10)
            .map(|i| {
                let content = format!(
                    r#"{{"timestamp":"2025-01-10T{:02}:00:00+08:00","cwd":"/home/user/project{}","message":{{"role":"user","content":"Meaningful message for session {}"}}}}
{{"timestamp":"2025-01-10T{:02}:30:00+08:00"}}"#,
                    9 + i % 8,
                    i,
                    i,
                    9 + i % 8
                );
                create_test_jsonl(&content)
            })
            .collect();

        let paths: Vec<_> = files.iter().map(|f| f.path().to_path_buf()).collect();
        let paths = Arc::new(paths);

        // Process in parallel
        let handles: Vec<_> = (0..4)
            .map(|thread_id| {
                let paths = Arc::clone(&paths);
                thread::spawn(move || {
                    let mut results = Vec::new();
                    for (i, path) in paths.iter().enumerate() {
                        if i % 4 == thread_id {
                            if let Some(metadata) = parse_session_timestamps_fast(path) {
                                results.push(metadata);
                            }
                        }
                    }
                    results
                })
            })
            .collect();

        let all_results: Vec<_> = handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect();

        assert_eq!(all_results.len(), 10);
    }

    #[test]
    fn test_thread_safety_concurrent_reads() {
        use std::sync::Arc;
        use std::thread;

        // Create a single large file
        let mut content = String::new();
        content.push_str(r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"This is a meaningful test message for concurrency"}}"#);
        content.push('\n');
        for i in 0..100 {
            content.push_str(&format!(
                r#"{{"timestamp":"2025-01-10T09:{:02}:00+08:00","message":{{"role":"assistant","content":"Response {}"}}}}"#,
                i % 60,
                i
            ));
            content.push('\n');
        }
        content.push_str(r#"{"timestamp":"2025-01-10T17:00:00+08:00"}"#);

        let file = create_test_jsonl(&content);
        let path = Arc::new(file.path().to_path_buf());

        // Read the same file from multiple threads concurrently
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let path = Arc::clone(&path);
                thread::spawn(move || {
                    parse_session_timestamps_fast(&path)
                })
            })
            .collect();

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // All threads should get the same result
        assert!(results.iter().all(|r| r.is_some()));
        let first = results[0].as_ref().unwrap();
        for result in &results[1..] {
            let r = result.as_ref().unwrap();
            assert_eq!(r.first_ts, first.first_ts);
            assert_eq!(r.last_ts, first.last_ts);
        }
    }

    // ==================== Integration Tests ====================

    #[test]
    fn test_timeline_session_creation() {
        // Test the full flow of creating a TimelineSession
        let content = r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/my-project","message":{"role":"user","content":"Working on feature implementation"}}
{"timestamp":"2025-01-10T11:30:00+08:00","message":{"role":"assistant","content":"Done"}}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let metadata = parse_session_timestamps_fast(&path).unwrap();
        let hours = calculate_hours(&metadata.first_ts, &metadata.last_ts);

        let session = TimelineSession {
            id: "test-session".to_string(),
            project: metadata.cwd.as_ref()
                .and_then(|c| c.split('/').last())
                .unwrap_or("unknown")
                .to_string(),
            title: metadata.first_msg.unwrap_or_else(|| "Untitled".to_string()),
            start_time: metadata.first_ts.clone(),
            end_time: metadata.last_ts.clone(),
            hours,
            commits: Vec::new(),
        };

        assert_eq!(session.project, "my-project");
        assert_eq!(session.start_time, "2025-01-10T09:00:00+08:00");
        assert_eq!(session.end_time, "2025-01-10T11:30:00+08:00");
        assert!((session.hours - 2.5).abs() < 0.01);
        assert!(session.title.contains("feature implementation"));
    }

    #[test]
    fn test_session_filtering_by_minimum_hours() {
        // Sessions < 0.08 hours should be filtered out
        let content = r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"Very short meaningful session"}}
{"timestamp":"2025-01-10T09:02:00+08:00"}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let metadata = parse_session_timestamps_fast(&path).unwrap();
        let hours = calculate_hours(&metadata.first_ts, &metadata.last_ts);

        // 2 minutes = 0.033 hours, but minimum is 0.1
        // The actual filtering happens in get_timeline_data where hours < 0.08 is skipped
        assert!(hours >= 0.1); // Due to .max(0.1) in calculate_hours
    }

    #[test]
    fn test_directory_structure_simulation() {
        use tempfile::TempDir;
        use std::fs;

        // Simulate Claude projects directory structure
        let temp_dir = TempDir::new().unwrap();
        let projects_dir = temp_dir.path();

        // Create project directories
        let project1_dir = projects_dir.join("abc123-myproject");
        let project2_dir = projects_dir.join("def456-another");
        let hidden_dir = projects_dir.join(".hidden");

        fs::create_dir_all(&project1_dir).unwrap();
        fs::create_dir_all(&project2_dir).unwrap();
        fs::create_dir_all(&hidden_dir).unwrap();

        // Create session files
        let session1_content = r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/myproject","message":{"role":"user","content":"Working on myproject feature"}}
{"timestamp":"2025-01-10T11:00:00+08:00"}"#;

        let session2_content = r#"{"timestamp":"2025-01-10T14:00:00+08:00","cwd":"/home/user/another","message":{"role":"user","content":"Working on another project"}}
{"timestamp":"2025-01-10T16:00:00+08:00"}"#;

        fs::write(project1_dir.join("session1.jsonl"), session1_content).unwrap();
        fs::write(project2_dir.join("session2.jsonl"), session2_content).unwrap();
        fs::write(hidden_dir.join("hidden.jsonl"), session1_content).unwrap();

        // Verify directory structure
        let entries: Vec<_> = fs::read_dir(projects_dir)
            .unwrap()
            .flatten()
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                e.path().is_dir() && !name.starts_with('.')
            })
            .collect();

        assert_eq!(entries.len(), 2); // Should not include hidden dir
    }

    // ==================== Fast vs Full Parser Consistency Tests ====================

    #[test]
    fn test_fast_and_full_parser_consistency_small_file() {
        let content = r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"This is a meaningful test message"}}
{"timestamp":"2025-01-10T09:30:00+08:00","message":{"role":"assistant","content":"Response"}}
{"timestamp":"2025-01-10T10:00:00+08:00"}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let full_result = parse_session_timestamps_full(&path);
        let fast_result = parse_session_timestamps_fast(&path);

        assert!(full_result.is_some());
        assert!(fast_result.is_some());

        let full = full_result.unwrap();
        let fast = fast_result.unwrap();

        assert_eq!(full.first_ts, fast.first_ts);
        assert_eq!(full.last_ts, fast.last_ts);
        assert_eq!(full.cwd, fast.cwd);
    }

    #[test]
    fn test_fast_parser_large_file_correctness() {
        // Create a large file and verify fast parser gets correct first/last timestamps
        let mut content = String::new();

        // Known first timestamp
        content.push_str(r#"{"timestamp":"2025-01-10T08:00:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"First meaningful message of the day"}}"#);
        content.push('\n');

        // Many middle lines
        for i in 0..600 {
            content.push_str(&format!(
                r#"{{"timestamp":"2025-01-10T{:02}:{:02}:00+08:00","message":{{"role":"assistant","content":"Middle response {} with padding text to increase file size"}}}}"#,
                9 + (i / 60) % 8,
                i % 60,
                i
            ));
            content.push('\n');
        }

        // Known last timestamp
        content.push_str(r#"{"timestamp":"2025-01-10T18:00:00+08:00","message":{"role":"assistant","content":"Final response"}}"#);

        let file = create_test_jsonl(&content);
        let path = file.path().to_path_buf();

        // Verify file is large enough for fast path
        let file_size = std::fs::metadata(&path).unwrap().len();
        assert!(file_size > 50_000);

        let result = parse_session_timestamps_fast(&path);
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert_eq!(metadata.first_ts, "2025-01-10T08:00:00+08:00");
        assert_eq!(metadata.last_ts, "2025-01-10T18:00:00+08:00");
        assert_eq!(metadata.cwd, Some("/home/user/project".to_string()));
    }
}
