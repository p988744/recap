//! Work Items queries
//!
//! Commands for listing, getting, and querying work items.

use std::collections::HashMap;
use tauri::State;

use recap_core::auth::verify_token;
use recap_core::models::{PaginatedResponse, WorkItem};

use crate::commands::AppState;
use super::query_builder::SafeQueryBuilder;
use super::types::{
    DailyHours, JiraMappingStats, StatsQuery, TempoSyncStats, TimelineQuery, TimelineResponse, TimelineSession,
    WorkItemFilters, WorkItemStatsResponse, WorkItemWithChildren,
};

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

    // Build parameterized query safely
    let mut builder = SafeQueryBuilder::new();

    // Always filter by user_id
    builder.add_string_condition("user_id", "=", &claims.sub);

    // Exclude hidden projects globally
    builder.add_raw_condition(
        "NOT EXISTS (SELECT 1 FROM project_preferences pp WHERE pp.user_id = work_items.user_id AND pp.hidden = 1 AND work_items.title LIKE '[' || pp.project_name || ']%')"
    );

    if let Some(parent_id) = &filters.parent_id {
        builder.add_string_condition("parent_id", "=", parent_id);
    } else if !filters.show_all.unwrap_or(false) {
        builder.add_null_condition("parent_id", true);
    }

    if let Some(source) = &filters.source {
        builder.add_string_condition("source", "=", source);
    }

    if let Some(category) = &filters.category {
        builder.add_string_condition("category", "=", category);
    }

    if let Some(jira_mapped) = filters.jira_mapped {
        builder.add_null_condition("jira_issue_key", !jira_mapped);
    }

    if let Some(synced) = filters.synced_to_tempo {
        builder.add_int_condition("synced_to_tempo", "=", if synced { 1 } else { 0 });
    }

    if let Some(start_date) = &filters.start_date {
        builder.add_string_condition("date", ">=", start_date);
    }

    if let Some(end_date) = &filters.end_date {
        builder.add_string_condition("date", "<=", end_date);
    }

    // Count total
    let total = builder.count(&db.pool, "work_items").await?;

    // Fetch items
    let items: Vec<WorkItem> = builder
        .fetch_all(
            &db.pool,
            "SELECT * FROM work_items",
            "ORDER BY date DESC, created_at DESC",
            Some(per_page),
            Some(offset),
        )
        .await?;

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

    let pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(PaginatedResponse {
        items: items_with_children,
        total,
        page,
        per_page,
        pages,
    })
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

    // Build parameterized query safely
    let mut builder = SafeQueryBuilder::new();
    builder.add_string_condition("user_id", "=", &claims.sub);

    if let Some(start) = &query.start_date {
        builder.add_string_condition("date", ">=", start);
    }
    if let Some(end) = &query.end_date {
        builder.add_string_condition("date", "<=", end);
    }

    // Exclude hidden projects
    builder.add_raw_condition(
        "NOT EXISTS (SELECT 1 FROM project_preferences pp WHERE pp.user_id = work_items.user_id AND pp.hidden = 1 AND work_items.title LIKE '[' || pp.project_name || ']%')"
    );

    let work_items: Vec<WorkItem> = builder
        .fetch_all(&db.pool, "SELECT * FROM work_items", "", None, None)
        .await?;

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

/// Get timeline data for Gantt chart visualization
/// NOW reads from work_items database for consistency with Stats
#[tauri::command]
pub async fn get_timeline_data(
    state: State<'_, AppState>,
    token: String,
    query: TimelineQuery,
) -> Result<TimelineResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Determine which sources to filter by
    // Default to both claude_code and antigravity if not specified or empty
    let sources = match &query.sources {
        Some(s) if !s.is_empty() => s.clone(),
        _ => vec!["claude_code".to_string(), "antigravity".to_string()],
    };

    // Build the source placeholders for SQL IN clause
    let source_placeholders: String = sources.iter().map(|_| "?").collect::<Vec<_>>().join(", ");

    // Query work_items for the given date with start_time (session timing)
    // Filter by selected sources
    // Exclude hidden projects
    let sql = format!(
        r#"SELECT * FROM work_items
           WHERE user_id = ? AND date = ? AND source IN ({})
           AND NOT EXISTS (
               SELECT 1 FROM project_preferences pp
               WHERE pp.user_id = work_items.user_id
               AND pp.hidden = 1
               AND work_items.title LIKE '[' || pp.project_name || ']%'
           )
           ORDER BY start_time ASC"#,
        source_placeholders
    );

    let mut query_builder = sqlx::query_as::<_, crate::models::WorkItem>(&sql)
        .bind(&claims.sub)
        .bind(&query.date);

    for source in &sources {
        query_builder = query_builder.bind(source);
    }

    let items: Vec<crate::models::WorkItem> = query_builder
        .fetch_all(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    // Convert work items to timeline sessions
    let mut sessions: Vec<TimelineSession> = Vec::new();

    for item in items {
        // Extract project name from title [project_name] ...
        let project = if item.title.starts_with('[') {
            item.title
                .split(']')
                .next()
                .map(|s| s.trim_start_matches('[').to_string())
                .unwrap_or_else(|| "unknown".to_string())
        } else {
            item.project_path
                .as_ref()
                .and_then(|p| std::path::Path::new(p).file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string()
        };

        // Extract title content (remove [project_name] prefix)
        let title = if item.title.starts_with('[') {
            item.title
                .split(']')
                .nth(1)
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| item.title.clone())
        } else {
            item.title.clone()
        };

        // Use start_time/end_time if available, otherwise use date boundaries
        let start_time = item.start_time.clone()
            .unwrap_or_else(|| format!("{}T09:00:00+08:00", query.date));
        let end_time = item.end_time.clone()
            .unwrap_or_else(|| format!("{}T17:00:00+08:00", query.date));

        // Get commits for this session's time range
        let project_path = item.project_path.clone().unwrap_or_default();
        let author = crate::core_services::get_git_user_email(&project_path);
        let commits = crate::core_services::get_commits_in_time_range(&project_path, &start_time, &end_time, author.as_deref());

        sessions.push(TimelineSession {
            id: item.session_id.unwrap_or_else(|| item.id.clone()),
            project,
            title,
            start_time,
            end_time,
            hours: item.hours,
            commits,
        });
    }

    let total_hours: f64 = sessions.iter().map(|s| s.hours).sum();
    let total_commits: i32 = sessions.iter().map(|s| s.commits.len() as i32).sum();

    Ok(TimelineResponse {
        date: query.date,
        sessions,
        total_hours,
        total_commits,
    })
}
