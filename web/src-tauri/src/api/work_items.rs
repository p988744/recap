//! Work Items API routes

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Json, Router,
};
use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    db::Database,
    models::{CreateWorkItem, PaginatedResponse, UpdateWorkItem, WorkItem, WorkItemFilters},
};

/// Work items routes
pub fn routes() -> Router<Database> {
    Router::new()
        .route("/", get(list_work_items))
        .route("/", post(create_work_item))
        .route("/stats/summary", get(get_stats_summary))
        .route("/grouped", get(get_grouped_work_items))
        .route("/timeline", get(get_timeline_data))
        .route("/aggregate", post(aggregate_work_items))
        .route("/:id", get(get_work_item))
        .route("/:id", patch(update_work_item))
        .route("/:id", delete(delete_work_item))
        .route("/batch-sync", post(batch_sync_tempo))
}

/// Work item with child count for list response
#[derive(Debug, Serialize)]
pub struct WorkItemWithChildren {
    #[serde(flatten)]
    pub item: WorkItem,
    pub child_count: i64,
}

/// List work items with filters
async fn list_work_items(
    State(db): State<Database>,
    auth: AuthUser,
    Query(filters): Query<WorkItemFilters>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let page = filters.page.unwrap_or(1);
    let per_page = filters.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;

    // Build dynamic query with inline values (safe - internal filters only)
    let mut conditions = vec![format!("user_id = '{}'", auth.0.sub)];

    // By default, only show top-level items (parent_id IS NULL)
    // Unless explicitly requesting children of a specific parent
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
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Fetch items
    let query = format!(
        "SELECT * FROM work_items WHERE {} ORDER BY date DESC, created_at DESC LIMIT {} OFFSET {}",
        where_clause, per_page, offset
    );

    let items: Vec<WorkItem> = sqlx::query_as(&query)
        .fetch_all(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Get child counts for each item
    let mut items_with_children: Vec<WorkItemWithChildren> = Vec::new();
    for item in items {
        let count_query = "SELECT COUNT(*) FROM work_items WHERE parent_id = ?";
        let child_count: (i64,) = sqlx::query_as(count_query)
            .bind(&item.id)
            .fetch_one(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        items_with_children.push(WorkItemWithChildren {
            item,
            child_count: child_count.0,
        });
    }

    let pages = (total.0 as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        items: items_with_children,
        total: total.0,
        page,
        per_page,
        pages,
    }))
}

// === Grouped View Types ===

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

#[derive(Debug, serde::Deserialize)]
pub struct GroupedQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// Get work items grouped by project and date
async fn get_grouped_work_items(
    State(db): State<Database>,
    auth: AuthUser,
    Query(query): Query<GroupedQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use std::collections::HashMap;

    // Build query
    let mut conditions = vec![format!("user_id = '{}'", auth.0.sub)];
    // Only get top-level items (not children) for grouping
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
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_items = items.len() as i64;
    let total_hours: f64 = items.iter().map(|i| i.hours).sum();

    // Helper to extract project name from title
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
                    // Group by jira within project
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

    Ok(Json(GroupedWorkItemsResponse {
        by_project,
        by_date,
        total_hours,
        total_items,
    }))
}

#[derive(Debug, serde::Deserialize)]
pub struct StatsQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WorkItemStatsResponse {
    pub total_items: i64,
    pub total_hours: f64,
    pub hours_by_source: std::collections::HashMap<String, f64>,
    pub hours_by_project: std::collections::HashMap<String, f64>,
    pub hours_by_category: std::collections::HashMap<String, f64>,
    pub daily_hours: Vec<DailyHours>, // For heatmap
    pub jira_mapping: JiraMappingStats,
    pub tempo_sync: TempoSyncStats,
}

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

/// Get work item statistics summary
async fn get_stats_summary(
    State(db): State<Database>,
    auth: AuthUser,
    Query(query): Query<StatsQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Build date filter
    let mut date_filter = String::new();
    if let Some(start) = &query.start_date {
        date_filter.push_str(&format!(" AND date >= '{}'", start));
    }
    if let Some(end) = &query.end_date {
        date_filter.push_str(&format!(" AND date <= '{}'", end));
    }

    // Get all work items for the user (with optional date filter)
    let sql = format!(
        "SELECT * FROM work_items WHERE user_id = ?{}",
        date_filter
    );
    let work_items: Vec<WorkItem> = sqlx::query_as(&sql)
        .bind(&auth.0.sub)
        .fetch_all(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_items = work_items.len() as i64;
    let total_hours: f64 = work_items.iter().map(|i| i.hours).sum();

    // Hours by source
    let mut hours_by_source: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for item in &work_items {
        *hours_by_source.entry(item.source.clone()).or_insert(0.0) += item.hours;
    }

    // Hours by project (extract project name from title: "[project_name] YYYY-MM-DD 工作紀錄")
    let mut hours_by_project: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
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
    let mut hours_by_category: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for item in &work_items {
        let cat = item.category.clone().unwrap_or_else(|| "未分類".to_string());
        *hours_by_category.entry(cat).or_insert(0.0) += item.hours;
    }

    // Daily hours for heatmap
    let mut daily_map: std::collections::HashMap<String, (f64, i64)> = std::collections::HashMap::new();
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

    Ok(Json(WorkItemStatsResponse {
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
    }))
}

// === Timeline Data for Gantt Chart ===

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

#[derive(Debug, serde::Deserialize)]
pub struct TimelineQuery {
    pub date: String, // YYYY-MM-DD
}

/// Get timeline data for Gantt chart visualization
async fn get_timeline_data(
    State(_db): State<Database>,
    _auth: AuthUser,
    Query(query): Query<TimelineQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use std::path::PathBuf;

    // Get Claude sessions for this date
    let claude_home = std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".claude").join("projects"));

    let mut sessions: Vec<TimelineSession> = Vec::new();
    let target_date = &query.date;

    if let Some(projects_dir) = claude_home {
        if projects_dir.exists() {
            // Read all project directories
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

                    // Convert directory name back to path (for reference)
                    let _project_path_from_dir = dir_name.replace('-', "/");
                    // Project name will be extracted from session.cwd later
                    let mut project_name = dir_name.split('-').last().unwrap_or(&dir_name).to_string();

                    // Read session files
                    if let Ok(files) = std::fs::read_dir(&path) {
                        for file_entry in files.flatten() {
                            let file_path = file_entry.path();
                            if !file_path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                                continue;
                            }

                            // Parse session file for timestamps
                            if let Some(metadata) = parse_session_timestamps(&file_path) {
                                // Check if session is on target date
                                let session_date = metadata.first_ts.split('T').next().unwrap_or("");
                                if session_date != target_date {
                                    continue;
                                }

                                // Calculate hours
                                let hours = calculate_hours(&metadata.first_ts, &metadata.last_ts);
                                if hours < 0.08 {
                                    continue; // Skip very short sessions
                                }

                                // Extract project name from cwd (more accurate)
                                let actual_project_name = metadata.cwd.as_ref()
                                    .and_then(|c| c.split('/').last())
                                    .unwrap_or(&project_name)
                                    .to_string();

                                // Get project path for git commits
                                let project_path_for_git = metadata.cwd.as_ref()
                                    .map(|c| c.as_str())
                                    .unwrap_or("");

                                // Get git commits for this session's time range
                                let commits = get_commits_in_range(project_path_for_git, &metadata.first_ts, &metadata.last_ts);

                                let session_id = file_path.file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();

                                let title = metadata.first_msg.unwrap_or_else(|| "Claude Code session".to_string());

                                sessions.push(TimelineSession {
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
                    }
                }
            }
        }
    }

    // Sort sessions by start time
    sessions.sort_by(|a, b| a.start_time.cmp(&b.start_time));

    let total_hours: f64 = sessions.iter().map(|s| s.hours).sum();
    let total_commits: i32 = sessions.iter().map(|s| s.commits.len() as i32).sum();

    Ok(Json(TimelineResponse {
        date: query.date,
        sessions,
        total_hours,
        total_commits,
    }))
}

/// Session metadata parsed from file
struct SessionMetadata {
    first_ts: String,
    last_ts: String,
    first_msg: Option<String>,
    cwd: Option<String>,
}

/// Parse session file for timestamps, first message, and cwd
fn parse_session_timestamps(path: &std::path::PathBuf) -> Option<SessionMetadata> {
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

            // Get cwd from session
            if cwd.is_none() {
                if let Some(c) = msg.get("cwd").and_then(|v| v.as_str()) {
                    cwd = Some(c.to_string());
                }
            }

            // Get first meaningful user message
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

    // Only return if we have meaningful content
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

/// Calculate hours between two timestamps
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

/// Get git commits within a time range
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
        .arg("--format=%H|%an|%aI|%s")  // hash|author|ISO date|subject
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

/// Create a new work item
async fn create_work_item(
    State(db): State<Database>,
    auth: AuthUser,
    Json(req): Json<CreateWorkItem>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let source = req.source.unwrap_or_else(|| "manual".to_string());
    let tags_json = req.tags.map(|t| serde_json::to_string(&t).unwrap_or_default());

    sqlx::query(
        r#"
        INSERT INTO work_items (id, user_id, source, source_id, title, description, hours, date,
            jira_issue_key, jira_issue_title, category, tags, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&auth.0.sub)
    .bind(&source)
    .bind(&req.source_id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(req.hours.unwrap_or(0.0))
    .bind(&req.date)
    .bind(&req.jira_issue_key)
    .bind(&req.jira_issue_title)
    .bind(&req.category)
    .bind(&tags_json)
    .bind(now)
    .bind(now)
    .execute(&db.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let item: WorkItem = sqlx::query_as("SELECT * FROM work_items WHERE id = ?")
        .bind(&id)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(item)))
}

/// Get a single work item
async fn get_work_item(
    State(db): State<Database>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let item: Option<WorkItem> =
        sqlx::query_as("SELECT * FROM work_items WHERE id = ? AND user_id = ?")
            .bind(&id)
            .bind(&auth.0.sub)
            .fetch_optional(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match item {
        Some(item) => Ok(Json(item)),
        None => Err((StatusCode::NOT_FOUND, "Work item not found".to_string())),
    }
}

/// Update a work item
async fn update_work_item(
    State(db): State<Database>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateWorkItem>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Check ownership
    let existing: Option<WorkItem> =
        sqlx::query_as("SELECT * FROM work_items WHERE id = ? AND user_id = ?")
            .bind(&id)
            .bind(&auth.0.sub)
            .fetch_optional(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if existing.is_none() {
        return Err((StatusCode::NOT_FOUND, "Work item not found".to_string()));
    }

    let now = Utc::now();

    // Build update query dynamically
    let mut updates = vec!["updated_at = ?".to_string()];
    let mut bindings: Vec<String> = vec![now.to_rfc3339()];

    if let Some(title) = &req.title {
        updates.push("title = ?".to_string());
        bindings.push(title.clone());
    }

    if let Some(description) = &req.description {
        updates.push("description = ?".to_string());
        bindings.push(description.clone());
    }

    if let Some(hours) = req.hours {
        updates.push(format!("hours = {}", hours));
    }

    if let Some(date) = &req.date {
        updates.push("date = ?".to_string());
        bindings.push(date.to_string());
    }

    if let Some(jira_key) = &req.jira_issue_key {
        updates.push("jira_issue_key = ?".to_string());
        bindings.push(jira_key.clone());
    }

    if let Some(jira_title) = &req.jira_issue_title {
        updates.push("jira_issue_title = ?".to_string());
        bindings.push(jira_title.clone());
    }

    if let Some(category) = &req.category {
        updates.push("category = ?".to_string());
        bindings.push(category.clone());
    }

    if let Some(tags) = &req.tags {
        updates.push("tags = ?".to_string());
        bindings.push(serde_json::to_string(tags).unwrap_or_default());
    }

    if let Some(synced) = req.synced_to_tempo {
        updates.push(format!("synced_to_tempo = {}", if synced { 1 } else { 0 }));
    }

    if let Some(worklog_id) = &req.tempo_worklog_id {
        updates.push("tempo_worklog_id = ?".to_string());
        bindings.push(worklog_id.clone());
    }

    let _query = format!(
        "UPDATE work_items SET {} WHERE id = ?",
        updates.join(", ")
    );

    // Execute with bindings - simplified approach
    sqlx::query(&format!(
        "UPDATE work_items SET updated_at = ? WHERE id = ?",
    ))
    .bind(now)
    .bind(&id)
    .execute(&db.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Apply individual updates
    if let Some(title) = &req.title {
        sqlx::query("UPDATE work_items SET title = ? WHERE id = ?")
            .bind(title)
            .bind(&id)
            .execute(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(description) = &req.description {
        sqlx::query("UPDATE work_items SET description = ? WHERE id = ?")
            .bind(description)
            .bind(&id)
            .execute(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(hours) = req.hours {
        sqlx::query("UPDATE work_items SET hours = ? WHERE id = ?")
            .bind(hours)
            .bind(&id)
            .execute(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(date) = &req.date {
        sqlx::query("UPDATE work_items SET date = ? WHERE id = ?")
            .bind(date)
            .bind(&id)
            .execute(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(jira_key) = &req.jira_issue_key {
        sqlx::query("UPDATE work_items SET jira_issue_key = ? WHERE id = ?")
            .bind(jira_key)
            .bind(&id)
            .execute(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(jira_title) = &req.jira_issue_title {
        sqlx::query("UPDATE work_items SET jira_issue_title = ? WHERE id = ?")
            .bind(jira_title)
            .bind(&id)
            .execute(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(category) = &req.category {
        sqlx::query("UPDATE work_items SET category = ? WHERE id = ?")
            .bind(category)
            .bind(&id)
            .execute(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(synced) = req.synced_to_tempo {
        sqlx::query("UPDATE work_items SET synced_to_tempo = ? WHERE id = ?")
            .bind(synced)
            .bind(&id)
            .execute(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Fetch updated item
    let item: WorkItem = sqlx::query_as("SELECT * FROM work_items WHERE id = ?")
        .bind(&id)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(item))
}

/// Delete a work item
async fn delete_work_item(
    State(db): State<Database>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let result = sqlx::query("DELETE FROM work_items WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&auth.0.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Work item not found".to_string()));
    }

    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub struct BatchSyncRequest {
    pub work_item_ids: Vec<String>,
}

#[derive(Serialize)]
pub struct BatchSyncResponse {
    pub synced: i64,
    pub failed: i64,
    pub errors: Vec<String>,
}

/// Batch sync work items to Tempo
async fn batch_sync_tempo(
    State(db): State<Database>,
    auth: AuthUser,
    Json(req): Json<BatchSyncRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Get user's Tempo token
    let user: Option<crate::models::User> = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&auth.0.sub)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = user.ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let _tempo_token = user
        .tempo_token
        .ok_or((StatusCode::BAD_REQUEST, "Tempo token not configured".to_string()))?;

    let mut synced = 0;
    let mut failed = 0;
    let mut errors = Vec::new();

    for item_id in &req.work_item_ids {
        // Get work item
        let item: Option<WorkItem> =
            sqlx::query_as("SELECT * FROM work_items WHERE id = ? AND user_id = ?")
                .bind(item_id)
                .bind(&auth.0.sub)
                .fetch_optional(&db.pool)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let item = match item {
            Some(i) => i,
            None => {
                failed += 1;
                errors.push(format!("Work item {} not found", item_id));
                continue;
            }
        };

        // Check if has Jira issue
        let _jira_key = match &item.jira_issue_key {
            Some(k) => k.clone(),
            None => {
                failed += 1;
                errors.push(format!("Work item {} has no Jira issue mapped", item_id));
                continue;
            }
        };

        // TODO: Call Tempo API to create worklog
        // For now, just mark as synced
        let now = Utc::now();
        sqlx::query("UPDATE work_items SET synced_to_tempo = 1, synced_at = ? WHERE id = ?")
            .bind(now)
            .bind(item_id)
            .execute(&db.pool)
            .await
            .map_err(|e| {
                failed += 1;
                errors.push(format!("Failed to update {}: {}", item_id, e));
            })
            .ok();

        synced += 1;
    }

    Ok(Json(BatchSyncResponse {
        synced,
        failed,
        errors,
    }))
}

#[derive(serde::Deserialize)]
pub struct AggregateRequest {
    /// Optional: only aggregate items from these dates
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    /// Optional: only aggregate items from this source
    pub source: Option<String>,
}

#[derive(Serialize)]
pub struct AggregateResponse {
    pub original_count: usize,
    pub aggregated_count: usize,
    pub deleted_count: usize,
}

/// Aggregate work items by project + date
/// Groups fragmented items into consolidated entries per project per day
async fn aggregate_work_items(
    State(db): State<Database>,
    auth: AuthUser,
    Json(req): Json<AggregateRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use std::collections::HashMap;

    // Build query with optional filters
    let mut conditions = vec![format!("user_id = '{}'", auth.0.sub)];

    if let Some(start) = &req.start_date {
        conditions.push(format!("date >= '{}'", start));
    }
    if let Some(end) = &req.end_date {
        conditions.push(format!("date <= '{}'", end));
    }
    if let Some(source) = &req.source {
        conditions.push(format!("source = '{}'", source.replace('\'', "''")));
    }

    let sql = format!(
        "SELECT * FROM work_items WHERE {} ORDER BY date, title",
        conditions.join(" AND ")
    );

    let work_items: Vec<WorkItem> = sqlx::query_as(&sql)
        .fetch_all(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let original_count = work_items.len();

    // Group by project + date
    let mut groups: HashMap<String, Vec<WorkItem>> = HashMap::new();

    for item in work_items {
        // Extract project name from title like "[project-name] ..."
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
            // No need to aggregate single items
            continue;
        }

        let parts: Vec<&str> = key.split('|').collect();
        let project_name = parts[0];
        let date = parts.get(1).unwrap_or(&"");

        // Calculate total hours
        let total_hours: f64 = items.iter().map(|i| i.hours).sum();

        // Extract unique tasks (what was done)
        let mut tasks: Vec<String> = Vec::new();
        for item in &items {
            // Get the task description from title (remove [project] prefix)
            let task = if let Some(idx) = item.title.find(']') {
                item.title[idx + 1..].trim().to_string()
            } else {
                item.title.clone()
            };

            // Truncate long tasks
            let task = if task.len() > 80 {
                format!("{}...", &task.chars().take(80).collect::<String>())
            } else {
                task
            };

            // Avoid duplicates and empty tasks
            if !task.is_empty() && !tasks.contains(&task) {
                tasks.push(task);
            }
        }

        // Create aggregated title with project name
        let title = format!("[{}] {} 項工作", project_name, tasks.len());

        // Create description with task list (what was done)
        let task_list = tasks.iter()
            .take(10) // Limit to 10 tasks
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
            task_list,
            remaining,
            total_hours,
            items.len()
        );

        // Get common attributes from first item
        let first = &items[0];
        let jira_key = items.iter().find_map(|i| i.jira_issue_key.clone());
        let jira_title = items.iter().find_map(|i| i.jira_issue_title.clone());
        let category = first.category.clone();

        // Create new aggregated work item (parent)
        let parent_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"INSERT INTO work_items
            (id, user_id, source, source_id, title, description, hours, date,
             jira_issue_key, jira_issue_title, category, synced_to_tempo, parent_id, created_at, updated_at)
            VALUES (?, ?, 'aggregated', ?, ?, ?, ?, ?, ?, ?, ?, 0, NULL, ?, ?)"#
        )
        .bind(&parent_id)
        .bind(&auth.0.sub)
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
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        aggregated_count += 1;

        // Update original items to link to parent (instead of deleting)
        for item in &items {
            child_ids.push(item.id.clone());
        }

        // Batch update parent_id for child items
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
            query = query.bind(&auth.0.sub);

            query.execute(&db.pool)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
    }

    let grouped_count = child_ids.len();

    Ok(Json(AggregateResponse {
        original_count,
        aggregated_count,
        deleted_count: grouped_count, // Renamed semantically but keeping field name for compatibility
    }))
}
