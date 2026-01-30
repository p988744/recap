//! Project timeline commands
//!
//! Commands for querying project timeline data with sessions and commits.

use chrono::{DateTime, Datelike, Local, NaiveDate};
use recap_core::auth::verify_token;
use recap_core::models::{SnapshotRawData, WorkItem};
use serde_json::Value;
use std::collections::HashMap;
use tauri::State;

use super::types::{
    ProjectTimelineRequest, ProjectTimelineResponse, TimelineCommit, TimelineGroup,
    TimelineSession,
};
use crate::commands::AppState;

/// Extract project name from work item title "[ProjectName] ..." pattern
fn extract_project_name(title: &str) -> Option<String> {
    if title.starts_with('[') {
        title
            .split(']')
            .next()
            .map(|s| s.trim_start_matches('[').to_string())
    } else {
        None
    }
}

/// Derive project name from either title pattern or project_path
fn derive_project_name(item: &WorkItem) -> String {
    if let Some(name) = extract_project_name(&item.title) {
        if !name.is_empty() {
            return name;
        }
    }
    if let Some(path) = &item.project_path {
        if let Some(last) = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
        {
            return last.to_string();
        }
    }
    "unknown".to_string()
}

/// Extract local date from a timestamp string
fn extract_local_date(ts: &str) -> String {
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
        let local = dt.with_timezone(&Local);
        return local.format("%Y-%m-%d").to_string();
    }
    ts.get(..10).unwrap_or(ts).to_string()
}

/// Get period label based on time unit
fn get_period_label(date: &NaiveDate, time_unit: &str) -> String {
    match time_unit {
        "day" => date.format("%Y-%m-%d").to_string(),
        "week" => {
            let iso_week = date.iso_week();
            format!("{} W{:02}", iso_week.year(), iso_week.week())
        }
        "month" => date.format("%Y-%m").to_string(),
        "quarter" => {
            let quarter = (date.month() - 1) / 3 + 1;
            format!("{} Q{}", date.year(), quarter)
        }
        "year" => date.format("%Y").to_string(),
        _ => date.format("%Y-%m-%d").to_string(),
    }
}

/// Get period start and end dates based on time unit
fn get_period_bounds(date: &NaiveDate, time_unit: &str) -> (NaiveDate, NaiveDate) {
    match time_unit {
        "day" => (*date, *date),
        "week" => {
            let weekday = date.weekday();
            let days_from_monday = weekday.num_days_from_monday();
            let start = *date - chrono::Duration::days(days_from_monday as i64);
            let end = start + chrono::Duration::days(6);
            (start, end)
        }
        "month" => {
            let start = NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(*date);
            let end = if date.month() == 12 {
                NaiveDate::from_ymd_opt(date.year() + 1, 1, 1)
                    .unwrap_or(*date)
                    .pred_opt()
                    .unwrap_or(*date)
            } else {
                NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1)
                    .unwrap_or(*date)
                    .pred_opt()
                    .unwrap_or(*date)
            };
            (start, end)
        }
        "quarter" => {
            let quarter = (date.month() - 1) / 3;
            let start_month = quarter * 3 + 1;
            let start = NaiveDate::from_ymd_opt(date.year(), start_month, 1).unwrap_or(*date);
            let end_month = start_month + 2;
            let end = if end_month == 12 {
                NaiveDate::from_ymd_opt(date.year() + 1, 1, 1)
                    .unwrap_or(*date)
                    .pred_opt()
                    .unwrap_or(*date)
            } else {
                NaiveDate::from_ymd_opt(date.year(), end_month + 1, 1)
                    .unwrap_or(*date)
                    .pred_opt()
                    .unwrap_or(*date)
            };
            (start, end)
        }
        "year" => {
            let start = NaiveDate::from_ymd_opt(date.year(), 1, 1).unwrap_or(*date);
            let end = NaiveDate::from_ymd_opt(date.year(), 12, 31).unwrap_or(*date);
            (start, end)
        }
        _ => (*date, *date),
    }
}

/// Parse commits from JSON string
fn parse_commits_from_json(json_str: &str) -> Vec<TimelineCommit> {
    if let Ok(commits) = serde_json::from_str::<Vec<Value>>(json_str) {
        commits
            .iter()
            .filter_map(|c| {
                let hash = c.get("hash")?.as_str()?.to_string();
                let short_hash = hash.get(..7).unwrap_or(&hash).to_string();
                let message = c.get("message")?.as_str()?.to_string();
                let author = c
                    .get("author")
                    .and_then(|a| a.as_str())
                    .unwrap_or("")
                    .to_string();
                let time = c
                    .get("timestamp")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                let files_changed = c
                    .get("files_changed")
                    .and_then(|f| f.as_i64())
                    .unwrap_or(0) as i32;
                let insertions = c.get("insertions").and_then(|i| i.as_i64()).unwrap_or(0) as i32;
                let deletions = c.get("deletions").and_then(|d| d.as_i64()).unwrap_or(0) as i32;

                Some(TimelineCommit {
                    hash,
                    short_hash,
                    message,
                    author,
                    time,
                    files_changed,
                    insertions,
                    deletions,
                })
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// Get project timeline with sessions and commits grouped by time period
#[tauri::command(rename_all = "camelCase")]
pub async fn get_project_timeline(
    state: State<'_, AppState>,
    token: String,
    request: ProjectTimelineRequest,
) -> Result<ProjectTimelineResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let limit = request.limit.unwrap_or(10).min(50);
    let time_unit = request.time_unit.as_str();

    // Parse date range
    let range_start = NaiveDate::parse_from_str(&request.range_start, "%Y-%m-%d")
        .map_err(|e| format!("Invalid range_start: {}", e))?;
    let range_end = NaiveDate::parse_from_str(&request.range_end, "%Y-%m-%d")
        .map_err(|e| format!("Invalid range_end: {}", e))?;

    // Apply cursor if present (cursor is the next period's start date to skip to)
    let effective_end = if let Some(ref cursor) = request.cursor {
        NaiveDate::parse_from_str(cursor, "%Y-%m-%d")
            .unwrap_or(range_end)
            .pred_opt()
            .unwrap_or(range_end)
    } else {
        range_end
    };

    // Query work items for this project within date range
    let items: Vec<WorkItem> = sqlx::query_as(
        r#"SELECT * FROM work_items
           WHERE user_id = ? AND date >= ? AND date <= ?
           ORDER BY date DESC, created_at DESC"#,
    )
    .bind(&claims.sub)
    .bind(range_start.format("%Y-%m-%d").to_string())
    .bind(effective_end.format("%Y-%m-%d").to_string())
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // Filter items for this project
    let project_items: Vec<&WorkItem> = items
        .iter()
        .filter(|item| derive_project_name(item) == request.project_name)
        .filter(|item| {
            // Filter by source if specified
            if let Some(ref sources) = request.sources {
                sources.is_empty() || sources.contains(&item.source)
            } else {
                true
            }
        })
        .collect();

    // Query snapshot_raw_data for commits
    let snapshot_start = format!("{}T00:00:00", range_start.format("%Y-%m-%d"));
    let snapshot_end = format!("{}T23:59:59", effective_end.format("%Y-%m-%d"));

    // Get project paths from work items to query snapshots
    let project_paths: Vec<String> = project_items
        .iter()
        .filter_map(|item| item.project_path.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let mut all_snapshots: Vec<SnapshotRawData> = Vec::new();
    for project_path in &project_paths {
        let snapshots: Vec<SnapshotRawData> = sqlx::query_as(
            r#"SELECT * FROM snapshot_raw_data
               WHERE user_id = ? AND project_path = ?
                 AND hour_bucket >= ? AND hour_bucket <= ?
               ORDER BY hour_bucket DESC"#,
        )
        .bind(&claims.sub)
        .bind(project_path)
        .bind(&snapshot_start)
        .bind(&snapshot_end)
        .fetch_all(&db.pool)
        .await
        .map_err(|e| e.to_string())?;
        all_snapshots.extend(snapshots);
    }

    // Build a map of session_id -> commits from snapshots
    let mut session_commits: HashMap<String, Vec<TimelineCommit>> = HashMap::new();
    let mut snapshot_dates: HashMap<String, String> = HashMap::new(); // session_id -> date

    for snapshot in &all_snapshots {
        if let Some(ref git_commits_json) = snapshot.git_commits {
            let commits = parse_commits_from_json(git_commits_json);
            session_commits
                .entry(snapshot.session_id.clone())
                .or_default()
                .extend(commits);
        }
        let local_date = extract_local_date(&snapshot.hour_bucket);
        snapshot_dates.insert(snapshot.session_id.clone(), local_date);
    }

    // Group work items by period
    struct PeriodData {
        period_label: String,
        period_start: NaiveDate,
        period_end: NaiveDate,
        sessions: Vec<TimelineSession>,
        standalone_commits: Vec<TimelineCommit>,
        total_hours: f64,
    }

    let mut periods: HashMap<String, PeriodData> = HashMap::new();

    for item in project_items {
        let (period_start, period_end) = get_period_bounds(&item.date, time_unit);
        let period_label = get_period_label(&item.date, time_unit);

        let period = periods.entry(period_label.clone()).or_insert_with(|| PeriodData {
            period_label: period_label.clone(),
            period_start,
            period_end,
            sessions: Vec::new(),
            standalone_commits: Vec::new(),
            total_hours: 0.0,
        });

        // Get commits for this session
        let commits = item
            .session_id
            .as_ref()
            .and_then(|sid| session_commits.get(sid))
            .cloned()
            .unwrap_or_default();

        // Build session
        let session = TimelineSession {
            id: item.id.clone(),
            source: item.source.clone(),
            title: item.title.clone(),
            start_time: item
                .start_time
                .clone()
                .unwrap_or_else(|| item.created_at.to_rfc3339()),
            end_time: item
                .end_time
                .clone()
                .unwrap_or_else(|| item.created_at.to_rfc3339()),
            hours: item.hours,
            summary: item.description.clone(),
            commits,
        };

        period.sessions.push(session);
        period.total_hours += item.hours;
    }

    // Convert to sorted vector (newest first)
    let mut period_vec: Vec<PeriodData> = periods.into_values().collect();
    period_vec.sort_by(|a, b| b.period_start.cmp(&a.period_start));

    // Apply pagination
    let has_more = period_vec.len() > limit as usize;
    let next_cursor = if has_more && period_vec.len() > limit as usize {
        period_vec
            .get(limit as usize)
            .map(|p| p.period_start.format("%Y-%m-%d").to_string())
    } else {
        None
    };

    // Build TimelineGroup list - summary will be generated on-demand via LLM
    let groups: Vec<TimelineGroup> = period_vec
        .into_iter()
        .take(limit as usize)
        .map(|p| TimelineGroup {
            period_label: p.period_label,
            period_start: p.period_start.format("%Y-%m-%d").to_string(),
            period_end: p.period_end.format("%Y-%m-%d").to_string(),
            total_hours: p.total_hours,
            summary: None, // Generated on-demand via generate_timeline_summary
            sessions: p.sessions,
            standalone_commits: p.standalone_commits,
        })
        .collect();

    Ok(ProjectTimelineResponse {
        groups,
        next_cursor,
        has_more,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_period_label_day() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 30).unwrap();
        assert_eq!(get_period_label(&date, "day"), "2026-01-30");
    }

    #[test]
    fn test_get_period_label_week() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 30).unwrap();
        let label = get_period_label(&date, "week");
        assert!(label.contains("W"));
    }

    #[test]
    fn test_get_period_label_month() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 30).unwrap();
        assert_eq!(get_period_label(&date, "month"), "2026-01");
    }

    #[test]
    fn test_get_period_label_quarter() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 30).unwrap();
        assert_eq!(get_period_label(&date, "quarter"), "2026 Q1");

        let date_q2 = NaiveDate::from_ymd_opt(2026, 5, 15).unwrap();
        assert_eq!(get_period_label(&date_q2, "quarter"), "2026 Q2");
    }

    #[test]
    fn test_get_period_label_year() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 30).unwrap();
        assert_eq!(get_period_label(&date, "year"), "2026");
    }

    #[test]
    fn test_get_period_bounds_day() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 30).unwrap();
        let (start, end) = get_period_bounds(&date, "day");
        assert_eq!(start, date);
        assert_eq!(end, date);
    }

    #[test]
    fn test_get_period_bounds_week() {
        // Jan 30, 2026 is a Friday
        let date = NaiveDate::from_ymd_opt(2026, 1, 30).unwrap();
        let (start, end) = get_period_bounds(&date, "week");
        // Week should start on Monday (Jan 26) and end on Sunday (Feb 1)
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 1, 26).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 2, 1).unwrap());
    }

    #[test]
    fn test_get_period_bounds_month() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let (start, end) = get_period_bounds(&date, "month");
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 1, 31).unwrap());
    }

    #[test]
    fn test_parse_commits_from_json() {
        let json = r#"[
            {
                "hash": "abc123def456789",
                "message": "Add feature",
                "author": "dev",
                "timestamp": "2026-01-30T10:00:00",
                "files_changed": 5,
                "insertions": 100,
                "deletions": 20
            }
        ]"#;
        let commits = parse_commits_from_json(json);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].short_hash, "abc123d");
        assert_eq!(commits[0].message, "Add feature");
        assert_eq!(commits[0].files_changed, 5);
    }

    #[test]
    fn test_parse_commits_from_json_empty() {
        assert!(parse_commits_from_json("[]").is_empty());
        assert!(parse_commits_from_json("invalid").is_empty());
    }

    #[test]
    fn test_extract_local_date() {
        assert_eq!(extract_local_date("2026-01-30T10:00:00"), "2026-01-30");
        assert_eq!(extract_local_date("2026-01-30"), "2026-01-30");
    }
}
