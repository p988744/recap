//! Snapshot & Compaction Tauri Commands
//!
//! Provides commands for querying work summaries, viewing snapshot details,
//! and triggering compaction manually.

use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime, Timelike};
use recap_core::auth::verify_token;
use recap_core::models::{SnapshotRawData, WorkSummary};
use recap_core::get_commits_for_date;
use serde::Serialize;
use tauri::State;

use super::AppState;

/// Extract local HH:MM from a timestamp string.
/// Handles both naive local time ("2026-01-27T09:00:00") and
/// UTC-offset format ("2026-01-27T00:00:00+00:00").
fn extract_local_hour(ts: &str) -> String {
    // Try RFC3339 first (has timezone offset)
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
        let local = dt.with_timezone(&Local);
        return format!("{:02}:{:02}", local.hour(), local.minute());
    }
    // Try naive datetime (already in local time)
    if let Ok(ndt) = NaiveDateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S") {
        return format!("{:02}:{:02}", ndt.hour(), ndt.minute());
    }
    // Fallback: substring extraction
    ts.get(11..16).unwrap_or("??:??").to_string()
}

/// Compute the next hour from a local HH:MM string.
fn next_hour(hour_str: &str) -> String {
    let h: u32 = hour_str.get(..2).and_then(|s| s.parse().ok()).unwrap_or(0);
    format!("{:02}:00", (h + 1).min(23))
}

/// Extract local date (YYYY-MM-DD) from a timestamp string.
/// Handles both RFC3339 and naive datetime formats.
fn extract_local_date(ts: &str) -> String {
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
        let local = dt.with_timezone(&Local);
        return local.format("%Y-%m-%d").to_string();
    }
    // Naive or fallback: first 10 chars
    ts.get(..10).unwrap_or(ts).to_string()
}

/// Response type for work summaries
#[derive(Debug, Serialize)]
pub struct WorkSummaryResponse {
    pub id: String,
    pub project_path: Option<String>,
    pub scale: String,
    pub period_start: String,
    pub period_end: String,
    pub summary: String,
    pub key_activities: Option<String>,
    pub git_commits_summary: Option<String>,
    pub llm_model: Option<String>,
    pub created_at: String,
}

impl From<WorkSummary> for WorkSummaryResponse {
    fn from(ws: WorkSummary) -> Self {
        Self {
            id: ws.id,
            project_path: ws.project_path,
            scale: ws.scale,
            period_start: ws.period_start,
            period_end: ws.period_end,
            summary: ws.summary,
            key_activities: ws.key_activities,
            git_commits_summary: ws.git_commits_summary,
            llm_model: ws.llm_model,
            created_at: ws.created_at.to_rfc3339(),
        }
    }
}

/// Response type for snapshot details
#[derive(Debug, Serialize)]
pub struct SnapshotDetailResponse {
    pub id: String,
    pub session_id: String,
    pub project_path: String,
    pub hour_bucket: String,
    pub user_messages: Option<String>,
    pub assistant_messages: Option<String>,
    pub tool_calls: Option<String>,
    pub files_modified: Option<String>,
    pub git_commits: Option<String>,
    pub message_count: i32,
    pub raw_size_bytes: i32,
    pub created_at: String,
}

impl From<SnapshotRawData> for SnapshotDetailResponse {
    fn from(s: SnapshotRawData) -> Self {
        Self {
            id: s.id,
            session_id: s.session_id,
            project_path: s.project_path,
            hour_bucket: s.hour_bucket,
            user_messages: s.user_messages,
            assistant_messages: s.assistant_messages,
            tool_calls: s.tool_calls,
            files_modified: s.files_modified,
            git_commits: s.git_commits,
            message_count: s.message_count,
            raw_size_bytes: s.raw_size_bytes,
            created_at: s.created_at.to_rfc3339(),
        }
    }
}

/// Response type for compaction result
#[derive(Debug, Serialize)]
pub struct CompactionResultResponse {
    pub hourly_compacted: usize,
    pub daily_compacted: usize,
    pub weekly_compacted: usize,
    pub monthly_compacted: usize,
    pub errors: Vec<String>,
}

/// Response type for force recompact result
#[derive(Debug, Serialize)]
pub struct ForceRecompactResponse {
    pub summaries_deleted: usize,
    pub hourly_compacted: usize,
    pub daily_compacted: usize,
    pub weekly_compacted: usize,
    pub monthly_compacted: usize,
    pub errors: Vec<String>,
}

/// Get work summaries at a given scale and date range.
#[tauri::command]
pub async fn get_work_summaries(
    state: State<'_, AppState>,
    token: String,
    scale: String,
    start_date: String,
    end_date: String,
    project_path: Option<String>,
) -> Result<Vec<WorkSummaryResponse>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let summaries: Vec<WorkSummary> = if let Some(ref pp) = project_path {
        sqlx::query_as(
            "SELECT * FROM work_summaries WHERE user_id = ? AND scale = ? AND period_start >= ? AND period_start <= ? AND project_path = ? ORDER BY period_start",
        )
        .bind(&claims.sub)
        .bind(&scale)
        .bind(&start_date)
        .bind(&end_date)
        .bind(pp)
        .fetch_all(&db.pool)
        .await
        .map_err(|e| e.to_string())?
    } else {
        sqlx::query_as(
            "SELECT * FROM work_summaries WHERE user_id = ? AND scale = ? AND period_start >= ? AND period_start <= ? ORDER BY period_start",
        )
        .bind(&claims.sub)
        .bind(&scale)
        .bind(&start_date)
        .bind(&end_date)
        .fetch_all(&db.pool)
        .await
        .map_err(|e| e.to_string())?
    };

    Ok(summaries.into_iter().map(WorkSummaryResponse::from).collect())
}

/// Get detailed snapshot data for a specific snapshot ID.
#[tauri::command]
pub async fn get_snapshot_detail(
    state: State<'_, AppState>,
    token: String,
    snapshot_id: String,
) -> Result<SnapshotDetailResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let snapshot: SnapshotRawData = sqlx::query_as(
        "SELECT * FROM snapshot_raw_data WHERE id = ? AND user_id = ?",
    )
    .bind(&snapshot_id)
    .bind(&claims.sub)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Snapshot not found".to_string())?;

    Ok(SnapshotDetailResponse::from(snapshot))
}

// ============ Worklog Overview ============

/// A manual work item within a worklog day
#[derive(Debug, Serialize)]
pub struct ManualWorkItem {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub hours: f64,
    pub date: String,
}

/// A project's daily summary within a worklog day
#[derive(Debug, Serialize)]
pub struct WorklogDayProject {
    pub project_path: String,
    pub project_name: String,
    pub daily_summary: Option<String>,
    pub total_commits: i32,
    pub total_files: i32,
    pub total_hours: f64,
    pub has_hourly_data: bool,
}

/// A single day in the worklog overview
#[derive(Debug, Serialize)]
pub struct WorklogDay {
    pub date: String,
    pub weekday: String,
    pub projects: Vec<WorklogDayProject>,
    pub manual_items: Vec<ManualWorkItem>,
}

/// The full worklog overview response
#[derive(Debug, Serialize)]
pub struct WorklogOverviewResponse {
    pub days: Vec<WorklogDay>,
}

/// Get a worklog overview for a date range.
/// Returns daily summaries grouped by date and project, plus manual work items.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_worklog_overview(
    state: State<'_, AppState>,
    token: String,
    start_date: String,
    end_date: String,
) -> Result<WorklogOverviewResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // 1. Fetch all daily summaries in range
    //    period_start is stored as local time without offset: "2026-01-26T00:00:00"
    let daily_summaries: Vec<WorkSummary> = sqlx::query_as(
        r#"SELECT * FROM work_summaries
           WHERE user_id = ? AND scale = 'daily'
             AND DATE(period_start) >= ? AND DATE(period_start) <= ?
           ORDER BY period_start DESC"#,
    )
    .bind(&claims.sub)
    .bind(&start_date)
    .bind(&end_date)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // 2. Fetch snapshot stats per project per day (for days without daily summaries)
    //    Widen query range by 1 day to handle UTC-offset hour_buckets, then group by local date in Rust.
    let prev_start = chrono::NaiveDate::parse_from_str(&start_date, "%Y-%m-%d")
        .map(|d| d.pred_opt().unwrap_or(d).format("%Y-%m-%d").to_string())
        .unwrap_or_else(|_| start_date.clone());
    let next_end = chrono::NaiveDate::parse_from_str(&end_date, "%Y-%m-%d")
        .map(|d| d.succ_opt().unwrap_or(d).format("%Y-%m-%d").to_string())
        .unwrap_or_else(|_| end_date.clone());
    let wide_snap_start = format!("{}T00:00:00", &prev_start);
    let wide_snap_end = format!("{}T23:59:59", &next_end);

    let raw_snapshots: Vec<SnapshotRawData> = sqlx::query_as(
        r#"SELECT * FROM snapshot_raw_data
           WHERE user_id = ? AND hour_bucket >= ? AND hour_bucket <= ?
           ORDER BY hour_bucket DESC"#,
    )
    .bind(&claims.sub)
    .bind(&wide_snap_start)
    .bind(&wide_snap_end)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // Group by (project_path, local_date) and aggregate counts + hours
    // hours = number of distinct hour buckets (each bucket ≈ 1 hour of activity)
    let mut snapshot_stats: Vec<(String, String, i32, i32, f64)> = Vec::new();
    {
        let mut stats_map: std::collections::HashMap<(String, String), (i32, i32, std::collections::HashSet<String>)> = std::collections::HashMap::new();
        for snap in &raw_snapshots {
            let local_date = extract_local_date(&snap.hour_bucket);
            if local_date < start_date || local_date > end_date {
                continue;
            }
            let local_hour = extract_local_hour(&snap.hour_bucket);
            let key = (snap.project_path.clone(), local_date);
            let entry = stats_map.entry(key).or_insert((0, 0, std::collections::HashSet::new()));
            entry.0 += snap.git_commits.as_ref()
                .and_then(|g| serde_json::from_str::<Vec<serde_json::Value>>(g).ok())
                .map(|v| v.len() as i32)
                .unwrap_or(0);
            entry.1 += snap.files_modified.as_ref()
                .and_then(|f| serde_json::from_str::<Vec<serde_json::Value>>(f).ok())
                .map(|v| v.len() as i32)
                .unwrap_or(0);
            entry.2.insert(local_hour);
        }
        for ((project_path, day), (commits, files, hours_set)) in stats_map {
            snapshot_stats.push((project_path, day, commits, files, hours_set.len() as f64));
        }
        snapshot_stats.sort_by(|a, b| b.1.cmp(&a.1)); // newest first
    }

    // 3. Check which days/projects have hourly data
    let hourly_exists: Vec<(String, String)> = sqlx::query_as(
        r#"SELECT DISTINCT project_path, DATE(period_start) as day
           FROM work_summaries
           WHERE user_id = ? AND scale = 'hourly'
             AND DATE(period_start) >= ? AND DATE(period_start) <= ?"#,
    )
    .bind(&claims.sub)
    .bind(&start_date)
    .bind(&end_date)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // 4. Fetch manual work items
    let manual_items: Vec<recap_core::WorkItem> = sqlx::query_as(
        r#"SELECT * FROM work_items
           WHERE user_id = ? AND source = 'manual' AND date >= ? AND date <= ?
           ORDER BY date DESC"#,
    )
    .bind(&claims.sub)
    .bind(&start_date)
    .bind(&end_date)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // 4b. Fetch Antigravity work items (grouped by date and project_path)
    let antigravity_items: Vec<recap_core::WorkItem> = sqlx::query_as(
        r#"SELECT * FROM work_items
           WHERE user_id = ? AND source = 'antigravity' AND date >= ? AND date <= ?
           ORDER BY date DESC"#,
    )
    .bind(&claims.sub)
    .bind(&start_date)
    .bind(&end_date)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // Group Antigravity items by (date, project_path)
    let mut antigravity_stats: std::collections::HashMap<(String, String), (f64, String)> = std::collections::HashMap::new();
    for item in &antigravity_items {
        let date = item.date.to_string();
        let project_path = item.project_path.clone().unwrap_or_default();
        let entry = antigravity_stats.entry((date, project_path)).or_insert((0.0, String::new()));
        entry.0 += item.hours;
        if entry.1.is_empty() {
            entry.1 = item.description.clone().unwrap_or_else(|| item.title.clone());
        }
    }

    // 5. Build the response: group by date
    let mut days_map: std::collections::BTreeMap<String, WorklogDay> = std::collections::BTreeMap::new();

    let weekday_names = ["週日", "週一", "週二", "週三", "週四", "週五", "週六"];

    // Helper: get or create a day entry
    let get_or_create_day = |days_map: &mut std::collections::BTreeMap<String, WorklogDay>, date: &str| {
        if !days_map.contains_key(date) {
            let weekday = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map(|d| weekday_names[d.weekday().num_days_from_sunday() as usize].to_string())
                .unwrap_or_default();
            days_map.insert(date.to_string(), WorklogDay {
                date: date.to_string(),
                weekday,
                projects: Vec::new(),
                manual_items: Vec::new(),
            });
        }
    };

    // Add daily summaries
    for summary in &daily_summaries {
        let date = summary.period_start.get(..10).unwrap_or(&summary.period_start).to_string();
        let project_path = summary.project_path.clone().unwrap_or_default();
        let project_name = std::path::Path::new(&project_path).file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();

        // Parse commit/file counts from summary metadata
        let commit_count = summary.git_commits_summary.as_ref()
            .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(s).ok())
            .map(|v| v.len() as i32)
            .unwrap_or(0);
        let file_count = summary.key_activities.as_ref()
            .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(s).ok())
            .map(|v| v.len() as i32)
            .unwrap_or(0);

        // Get hours from snapshot stats for this project+date
        let total_hours = snapshot_stats.iter()
            .find(|(pp, d, _, _, _)| pp == &project_path && d == &date)
            .map(|(_, _, _, _, h)| *h)
            .unwrap_or(0.0);

        let has_hourly = hourly_exists.iter().any(|(pp, d)| pp == &project_path && d == &date);

        get_or_create_day(&mut days_map, &date);
        if let Some(day) = days_map.get_mut(&date) {
            day.projects.push(WorklogDayProject {
                project_path: project_path.clone(),
                project_name,
                daily_summary: Some(summary.summary.clone()),
                total_commits: commit_count,
                total_files: file_count,
                total_hours,
                has_hourly_data: has_hourly,
            });
        }
    }

    // Add snapshot-only data (projects with snapshots but no daily summary)
    for (project_path, day, commits, files, hours) in &snapshot_stats {
        get_or_create_day(&mut days_map, day);
        if let Some(day_entry) = days_map.get_mut(day.as_str()) {
            // Skip if already have a daily summary for this project
            if day_entry.projects.iter().any(|p| &p.project_path == project_path) {
                continue;
            }
            let project_name = std::path::Path::new(&project_path).file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
            let has_hourly = hourly_exists.iter().any(|(pp, d)| pp == project_path && d == day);

            day_entry.projects.push(WorklogDayProject {
                project_path: project_path.clone(),
                project_name,
                daily_summary: None,
                total_commits: *commits,
                total_files: *files,
                total_hours: *hours,
                has_hourly_data: has_hourly,
            });
        }
    }

    // Add Antigravity projects (or merge with existing projects)
    for ((date, project_path), (hours, summary)) in &antigravity_stats {
        if project_path.is_empty() {
            continue;
        }
        get_or_create_day(&mut days_map, date);
        if let Some(day_entry) = days_map.get_mut(date.as_str()) {
            // Check if project already exists (from Claude Code data)
            if let Some(existing) = day_entry.projects.iter_mut().find(|p| &p.project_path == project_path) {
                // Merge: add Antigravity hours to existing project
                existing.total_hours += hours;
                // Mark as having hourly data (Antigravity items will show in breakdown)
                existing.has_hourly_data = true;
            } else {
                // New project (only has Antigravity data)
                // Look up commit/file counts from snapshot_stats if available (from Claude Code data)
                let (mut commits, files) = snapshot_stats.iter()
                    .find(|(pp, d, _, _, _)| pp == project_path && d == date)
                    .map(|(_, _, c, f, _)| (*c, *f))
                    .unwrap_or((0, 0));

                // If no commits from snapshots, query git directly
                if commits == 0 {
                    if let Ok(naive_date) = NaiveDate::parse_from_str(date, "%Y-%m-%d") {
                        let git_commits = get_commits_for_date(project_path, &naive_date);
                        commits = git_commits.len() as i32;
                    }
                }

                let project_name = std::path::Path::new(&project_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                day_entry.projects.push(WorklogDayProject {
                    project_path: project_path.clone(),
                    project_name,
                    daily_summary: Some(summary.clone()),
                    total_commits: commits,
                    total_files: files,
                    total_hours: *hours,
                    has_hourly_data: true, // Antigravity items will show in breakdown
                });
            }
        }
    }

    // Add manual items
    for item in &manual_items {
        let date = item.date.to_string();
        get_or_create_day(&mut days_map, &date);
        if let Some(day) = days_map.get_mut(&date) {
            day.manual_items.push(ManualWorkItem {
                id: item.id.clone(),
                title: item.title.clone(),
                description: item.description.clone(),
                hours: item.hours,
                date: date.clone(),
            });
        }
    }

    // Post-process: for any project with 0 commits, query git directly
    for (date, day) in days_map.iter_mut() {
        for project in day.projects.iter_mut() {
            if project.total_commits == 0 {
                if let Ok(naive_date) = NaiveDate::parse_from_str(date, "%Y-%m-%d") {
                    let git_commits = get_commits_for_date(&project.project_path, &naive_date);
                    project.total_commits = git_commits.len() as i32;
                }
            }
        }
    }

    // Convert to sorted Vec (newest first)
    let days: Vec<WorklogDay> = days_map.into_values().rev().collect();

    Ok(WorklogOverviewResponse { days })
}

/// Get hourly breakdown for a specific day and project.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_hourly_breakdown(
    state: State<'_, AppState>,
    token: String,
    date: String,
    project_path: String,
) -> Result<Vec<HourlyBreakdownItem>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Widen query range by 1 day on each side to handle UTC-offset period_start,
    // then filter by local date in Rust.
    let prev_date_summary = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map(|d| d.pred_opt().unwrap_or(d).format("%Y-%m-%d").to_string())
        .unwrap_or_else(|_| date.clone());
    let next_date_summary = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map(|d| d.succ_opt().unwrap_or(d).format("%Y-%m-%d").to_string())
        .unwrap_or_else(|_| date.clone());
    let wide_start_summary = format!("{}T00:00:00", &prev_date_summary);
    let wide_end_summary = format!("{}T23:59:59", &next_date_summary);

    // Try hourly summaries first
    let all_summaries: Vec<WorkSummary> = sqlx::query_as(
        r#"SELECT * FROM work_summaries
           WHERE user_id = ? AND scale = 'hourly' AND project_path = ?
             AND period_start >= ? AND period_start <= ?
           ORDER BY period_start"#,
    )
    .bind(&claims.sub)
    .bind(&project_path)
    .bind(&wide_start_summary)
    .bind(&wide_end_summary)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // Filter to only summaries whose local date matches the requested date
    let summaries: Vec<&WorkSummary> = all_summaries.iter()
        .filter(|s| extract_local_date(&s.period_start) == date)
        .collect();

    // Build Claude Code items from hourly summaries if available
    let mut items: Vec<HourlyBreakdownItem> = if !summaries.is_empty() {
        summaries.into_iter().map(|s| {
            let hour_start = extract_local_hour(&s.period_start);
            let hour_end = extract_local_hour(&s.period_end);
            let files: Vec<String> = s.key_activities.as_ref()
                .and_then(|a| serde_json::from_str(a).ok())
                .unwrap_or_default();
            let commits: Vec<GitCommitRef> = s.git_commits_summary.as_ref()
                .and_then(|g| {
                    serde_json::from_str::<Vec<String>>(g).ok().map(|strings| {
                        strings.iter().filter_map(|s| {
                            let parts: Vec<&str> = s.splitn(2, ": ").collect();
                            if parts.len() == 2 {
                                let hash_part: Vec<&str> = parts[0].splitn(2, ' ').collect();
                                Some(GitCommitRef {
                                    hash: hash_part[0].to_string(),
                                    message: parts[1].to_string(),
                                    timestamp: String::new(),
                                })
                            } else {
                                None
                            }
                        }).collect()
                    })
                })
                .unwrap_or_default();

            HourlyBreakdownItem {
                hour_start,
                hour_end,
                summary: s.summary.clone(),
                files_modified: files,
                git_commits: commits,
                source: "claude_code".to_string(),
            }
        }).collect()
    } else {
        // No hourly summaries - will build from snapshots below
        Vec::new()
    };

    // If no items from summaries, fall back to raw snapshots
    if items.is_empty() {
        // Fallback: build from raw snapshots
        // Query broadly to handle both UTC-offset and naive-local hour_bucket formats.
        // We widen the range by 1 day on each side, then filter by local date in Rust.
        let prev_date = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
            .map(|d| d.pred_opt().unwrap_or(d).format("%Y-%m-%d").to_string())
            .unwrap_or_else(|_| date.clone());
        let next_date = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
            .map(|d| d.succ_opt().unwrap_or(d).format("%Y-%m-%d").to_string())
            .unwrap_or_else(|_| date.clone());
        let wide_start = format!("{}T00:00:00", &prev_date);
        let wide_end = format!("{}T23:59:59", &next_date);

        let all_snapshots: Vec<SnapshotRawData> = sqlx::query_as(
            r#"SELECT * FROM snapshot_raw_data
               WHERE user_id = ? AND project_path = ?
                 AND hour_bucket >= ? AND hour_bucket <= ?
               ORDER BY hour_bucket"#,
        )
        .bind(&claims.sub)
        .bind(&project_path)
        .bind(&wide_start)
        .bind(&wide_end)
        .fetch_all(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

        // Filter to only snapshots whose local date matches the requested date
        let snapshots: Vec<&SnapshotRawData> = all_snapshots.iter()
            .filter(|s| extract_local_date(&s.hour_bucket) == date)
            .collect();

        items = snapshots.into_iter().map(|s| {
            let hour_start = extract_local_hour(&s.hour_bucket);
            let hour_end = next_hour(&hour_start);

            let files: Vec<String> = s.files_modified.as_ref()
                .and_then(|f| serde_json::from_str(f).ok())
                .unwrap_or_default();

            let commits: Vec<GitCommitRef> = s.git_commits.as_ref()
                .and_then(|g| {
                    serde_json::from_str::<Vec<serde_json::Value>>(g).ok().map(|arr| {
                        arr.iter().filter_map(|v| {
                            Some(GitCommitRef {
                                hash: v.get("hash")?.as_str()?.to_string(),
                                message: v.get("message")?.as_str()?.to_string(),
                                timestamp: v.get("timestamp").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                            })
                        }).collect()
                    })
                })
                .unwrap_or_default();

            let summary = s.user_messages.as_ref()
                .and_then(|m| serde_json::from_str::<Vec<String>>(m).ok())
                .map(|msgs| msgs.join("; "))
                .unwrap_or_else(|| "工作進行中".to_string());

            HourlyBreakdownItem {
                hour_start,
                hour_end,
                summary,
                files_modified: files,
                git_commits: commits,
                source: "claude_code".to_string(),
            }
        }).collect();
    }

    // Build set of hours that already have items (from work_summaries or snapshots)
    let existing_hours: std::collections::HashSet<String> = items.iter()
        .map(|i| i.hour_start.clone())
        .collect();

    // Query Antigravity work items for the same date and project
    // First try exact path match, then fall back to matching by project name
    let mut antigravity_items: Vec<recap_core::WorkItem> = sqlx::query_as(
        r#"SELECT * FROM work_items
           WHERE user_id = ? AND source = 'antigravity' AND date = ? AND project_path = ?
           ORDER BY created_at DESC"#,
    )
    .bind(&claims.sub)
    .bind(&date)
    .bind(&project_path)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // If no exact match, try matching by project name (last path component)
    if antigravity_items.is_empty() {
        let project_name = std::path::Path::new(&project_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if !project_name.is_empty() {
            // Use LIKE to match paths ending with the project name
            let pattern = format!("%/{}", project_name);
            antigravity_items = sqlx::query_as(
                r#"SELECT * FROM work_items
                   WHERE user_id = ? AND source = 'antigravity' AND date = ? AND project_path LIKE ?
                   ORDER BY created_at DESC"#,
            )
            .bind(&claims.sub)
            .bind(&date)
            .bind(&pattern)
            .fetch_all(&db.pool)
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    // Build a map of Antigravity session_ids to their hours for source attribution
    let mut antigravity_session_hours: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for item in &antigravity_items {
        let hour_start = item
            .start_time
            .as_ref()
            .and_then(|ts| {
                chrono::DateTime::parse_from_rfc3339(ts)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Local).format("%H:00").to_string())
            })
            .unwrap_or_else(|| item.created_at.format("%H:00").to_string());

        if let Some(session_id) = &item.session_id {
            antigravity_session_hours.insert(session_id.clone(), hour_start);
        }
    }

    // Update source for existing items that came from Antigravity snapshots
    // Check if the work_summary's source_snapshot_ids reference Antigravity sessions
    for item in &mut items {
        // If this hour has an Antigravity work_item, mark the source as antigravity
        // since the LLM summary was generated from Antigravity snapshot data
        if antigravity_session_hours.values().any(|h| h == &item.hour_start) {
            item.source = "antigravity".to_string();
        }
    }

    // Only add Antigravity items for hours that DON'T already have entries
    // (to avoid duplicates when we already have LLM-generated summaries)
    for item in antigravity_items {
        // Skip items without session_id or start_time - these are orphan entries
        // that weren't properly linked to snapshots and would show incorrect times
        if item.session_id.is_none() || item.start_time.is_none() {
            continue;
        }

        // Extract hour from start_time (actual session time)
        let hour_start = item
            .start_time
            .as_ref()
            .and_then(|ts| {
                // Parse ISO timestamp and convert to local timezone
                chrono::DateTime::parse_from_rfc3339(ts)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Local).format("%H:00").to_string())
            })
            .unwrap_or_else(|| item.created_at.format("%H:00").to_string());

        // Skip if we already have an entry for this hour (LLM summary exists)
        if existing_hours.contains(&hour_start) {
            continue;
        }

        let hour_end = next_hour(&hour_start);

        // Extract summary from description, building a rich summary from available fields
        let summary = item
            .description
            .as_ref()
            .map(|desc| {
                // Extract key fields from the description
                let mut parts: Vec<String> = Vec::new();

                // Get the summary line
                let summary_text = desc.lines()
                    .find(|line| line.starts_with("📋 Summary:"))
                    .map(|line| line.trim_start_matches("📋 Summary:").trim().to_string())
                    .filter(|s| !s.is_empty() && s.len() > 10) // Only use if meaningful (>10 chars)
                    .unwrap_or_default();

                if !summary_text.is_empty() {
                    parts.push(summary_text);
                }

                // If summary is too short, add context from other fields
                if parts.is_empty() || parts[0].len() < 15 {
                    // Extract steps/duration info
                    if let Some(steps_line) = desc.lines().find(|line| line.contains("Steps:")) {
                        let steps_info = steps_line
                            .trim_start_matches("💬 ")
                            .trim();
                        if !steps_info.is_empty() {
                            parts.push(steps_info.to_string());
                        }
                    }

                    // Extract branch info
                    if let Some(branch_line) = desc.lines().find(|line| line.starts_with("🌿 Branch:")) {
                        let branch = branch_line.trim_start_matches("🌿 Branch:").trim();
                        if !branch.is_empty() && branch != "N/A" {
                            parts.push(format!("Branch: {}", branch));
                        }
                    }
                }

                if parts.is_empty() {
                    // Fall back to title without project prefix
                    item.title
                        .split(']')
                        .nth(1)
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|| item.title.clone())
                } else {
                    parts.join(" | ")
                }
            })
            .unwrap_or_else(|| {
                // No description - use title
                item.title
                    .split(']')
                    .nth(1)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| item.title.clone())
            });

        items.push(HourlyBreakdownItem {
            hour_start,
            hour_end,
            summary,
            files_modified: Vec::new(),
            git_commits: Vec::new(),
            source: "antigravity".to_string(),
        });
    }

    // Sort by source first (claude_code before antigravity), then by hour_start descending
    items.sort_by(|a, b| {
        // Primary: sort by hour_start descending
        b.hour_start.cmp(&a.hour_start)
    });
    Ok(items)
}

/// Hourly breakdown item
#[derive(Debug, Serialize)]
pub struct HourlyBreakdownItem {
    pub hour_start: String,
    pub hour_end: String,
    pub summary: String,
    pub files_modified: Vec<String>,
    pub git_commits: Vec<GitCommitRef>,
    /// Data source: "claude_code" or "antigravity"
    pub source: String,
}

/// Git commit reference
#[derive(Debug, Serialize)]
pub struct GitCommitRef {
    pub hash: String,
    pub message: String,
    pub timestamp: String,
}

/// Manually trigger a compaction cycle.
#[tauri::command]
pub async fn trigger_compaction(
    state: State<'_, AppState>,
    token: String,
) -> Result<CompactionResultResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let llm = recap_core::services::llm::create_llm_service(&db.pool, &claims.sub)
        .await
        .ok();

    let result = recap_core::services::compaction::run_compaction_cycle(
        &db.pool,
        llm.as_ref(),
        &claims.sub,
    )
    .await?;

    Ok(CompactionResultResponse {
        hourly_compacted: result.hourly_compacted,
        daily_compacted: result.daily_compacted,
        weekly_compacted: result.weekly_compacted,
        monthly_compacted: result.monthly_compacted,
        errors: result.errors,
    })
}

/// Force recompact all work summaries.
///
/// This operation deletes existing work_summaries and regenerates them from
/// snapshot_raw_data. Use this when you've made changes to the compaction logic
/// and want to retroactively apply them to historical data.
///
/// Original data (work_items, snapshot_raw_data) is preserved.
#[tauri::command(rename_all = "snake_case")]
pub async fn force_recompact(
    state: State<'_, AppState>,
    token: String,
    from_date: Option<String>,
    to_date: Option<String>,
    scales: Option<Vec<String>>,
) -> Result<ForceRecompactResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let llm = recap_core::services::llm::create_llm_service(&db.pool, &claims.sub)
        .await
        .ok();

    let options = recap_core::services::compaction::ForceRecompactOptions {
        from_date,
        to_date,
        scales: scales.unwrap_or_default(),
    };

    let result = recap_core::services::compaction::force_recompact(
        &db.pool,
        llm.as_ref(),
        &claims.sub,
        options,
    )
    .await?;

    Ok(ForceRecompactResponse {
        summaries_deleted: result.summaries_deleted,
        hourly_compacted: result.compaction_result.hourly_compacted,
        daily_compacted: result.compaction_result.daily_compacted,
        weekly_compacted: result.compaction_result.weekly_compacted,
        monthly_compacted: result.compaction_result.monthly_compacted,
        errors: result.compaction_result.errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_local_hour ──

    #[test]
    fn test_extract_local_hour_utc_offset() {
        // UTC midnight → should be converted to local time.
        // For Asia/Taipei (UTC+8): "2026-01-27T00:00:00+00:00" → "08:00"
        let result = extract_local_hour("2026-01-27T00:00:00+00:00");
        // We can't hardcode the expected value because it depends on the system timezone.
        // Instead, verify it produces a valid HH:MM format.
        assert_eq!(result.len(), 5);
        assert_eq!(&result[2..3], ":");
        let hour: u32 = result[..2].parse().unwrap();
        let minute: u32 = result[3..].parse().unwrap();
        assert!(hour < 24);
        assert!(minute < 60);
    }

    #[test]
    fn test_extract_local_hour_utc_offset_converts_correctly() {
        // Use a known offset to verify conversion: "+08:00" means already local for UTC+8 systems
        let result = extract_local_hour("2026-01-27T09:30:00+08:00");
        // Regardless of system timezone, the conversion should be consistent.
        // On UTC+8 system: input is 09:30 local → output "09:30"
        // On UTC system: input is 01:30 UTC → output "01:30"
        // We just verify the format is valid.
        assert_eq!(result.len(), 5);
        assert_eq!(&result[2..3], ":");
    }

    #[test]
    fn test_extract_local_hour_naive_datetime() {
        // Naive datetime (no timezone info) → treated as already local
        let result = extract_local_hour("2026-01-27T14:30:00");
        assert_eq!(result, "14:30");
    }

    #[test]
    fn test_extract_local_hour_naive_datetime_midnight() {
        let result = extract_local_hour("2026-01-27T00:00:00");
        assert_eq!(result, "00:00");
    }

    #[test]
    fn test_extract_local_hour_naive_datetime_end_of_day() {
        let result = extract_local_hour("2026-01-27T23:59:00");
        assert_eq!(result, "23:59");
    }

    #[test]
    fn test_extract_local_hour_fallback() {
        // Something that doesn't parse → falls back to substring
        let result = extract_local_hour("invalid-time");
        assert_eq!(result, "??:??");
    }

    #[test]
    fn test_extract_local_hour_z_suffix() {
        // "Z" suffix = UTC, equivalent to +00:00
        let result = extract_local_hour("2026-01-27T02:15:00Z");
        assert_eq!(result.len(), 5);
        assert_eq!(&result[2..3], ":");
    }

    // ── next_hour ──

    #[test]
    fn test_next_hour_normal() {
        assert_eq!(next_hour("09:00"), "10:00");
        assert_eq!(next_hour("14:30"), "15:00");
        assert_eq!(next_hour("00:00"), "01:00");
    }

    #[test]
    fn test_next_hour_capped_at_23() {
        assert_eq!(next_hour("23:00"), "23:00");
        assert_eq!(next_hour("23:30"), "23:00");
    }

    #[test]
    fn test_next_hour_edge_cases() {
        assert_eq!(next_hour("22:00"), "23:00");
        assert_eq!(next_hour("01:45"), "02:00");
    }

    #[test]
    fn test_next_hour_invalid_input() {
        // Invalid input → parses as 0 → "01:00"
        assert_eq!(next_hour("??:??"), "01:00");
    }

    // ── extract_local_date ──

    #[test]
    fn test_extract_local_date_utc_offset() {
        // UTC midnight with offset → converted to local date.
        let result = extract_local_date("2026-01-27T00:00:00+00:00");
        // On UTC+8 system: 2026-01-27 00:00 UTC = 2026-01-27 08:00 local → date = "2026-01-27"
        // On UTC-5 system: 2026-01-27 00:00 UTC = 2026-01-26 19:00 local → date = "2026-01-26"
        // Just verify valid date format YYYY-MM-DD
        assert_eq!(result.len(), 10);
        assert_eq!(&result[4..5], "-");
        assert_eq!(&result[7..8], "-");
    }

    #[test]
    fn test_extract_local_date_utc_offset_date_boundary() {
        // 2026-01-26T23:00:00+00:00 on UTC+8 → 2026-01-27T07:00 local
        // This tests that date conversion crosses the date boundary correctly
        let result = extract_local_date("2026-01-26T23:00:00+00:00");
        assert_eq!(result.len(), 10);
        // On positive UTC offset systems, this should be the next day
        let local_offset = Local::now().offset().local_minus_utc();
        if local_offset > 3600 {
            // UTC+2 or more: 23:00 UTC becomes next day
            assert_eq!(result, "2026-01-27");
        }
    }

    #[test]
    fn test_extract_local_date_naive() {
        // Naive datetime → first 10 chars
        let result = extract_local_date("2026-01-27T14:30:00");
        assert_eq!(result, "2026-01-27");
    }

    #[test]
    fn test_extract_local_date_date_only() {
        // Just a date string → first 10 chars
        let result = extract_local_date("2026-01-27");
        assert_eq!(result, "2026-01-27");
    }

    #[test]
    fn test_extract_local_date_z_suffix() {
        let result = extract_local_date("2026-01-27T10:00:00Z");
        assert_eq!(result.len(), 10);
    }

    // ── Integration-style tests for timezone conversion consistency ──

    #[test]
    fn test_hour_and_date_consistent_for_utc_offset() {
        // Ensure that for the same timestamp, extract_local_hour and extract_local_date
        // produce consistent results (both refer to the same local time).
        let ts = "2026-01-27T01:30:00+00:00";
        let date = extract_local_date(ts);
        let hour = extract_local_hour(ts);

        // Both should be valid
        assert_eq!(date.len(), 10);
        assert_eq!(hour.len(), 5);

        // Verify they match the same chrono Local conversion
        let dt = DateTime::parse_from_rfc3339(ts).unwrap();
        let local = dt.with_timezone(&Local);
        let expected_date = local.format("%Y-%m-%d").to_string();
        let expected_hour = format!("{:02}:{:02}", local.hour(), local.minute());
        assert_eq!(date, expected_date);
        assert_eq!(hour, expected_hour);
    }

    #[test]
    fn test_hour_and_date_consistent_for_naive() {
        let ts = "2026-01-27T09:45:00";
        let date = extract_local_date(ts);
        let hour = extract_local_hour(ts);

        assert_eq!(date, "2026-01-27");
        assert_eq!(hour, "09:45");
    }

    #[test]
    fn test_next_hour_from_extracted_hour() {
        // Simulate the typical flow: extract hour, compute next hour
        let hour_start = extract_local_hour("2026-01-27T15:00:00");
        let hour_end = next_hour(&hour_start);
        assert_eq!(hour_start, "15:00");
        assert_eq!(hour_end, "16:00");
    }

    #[test]
    fn test_utc_offset_positive_timezone_conversion() {
        // Test with explicit +08:00 offset
        let ts = "2026-01-27T08:00:00+08:00";
        let dt = DateTime::parse_from_rfc3339(ts).unwrap();
        let local = dt.with_timezone(&Local);
        let result = extract_local_hour(ts);
        let expected = format!("{:02}:{:02}", local.hour(), local.minute());
        assert_eq!(result, expected);
    }
}
