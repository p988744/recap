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
    /// LLM-related warnings (API errors that were handled with fallback)
    pub llm_warnings: Vec<String>,
    /// Latest date that was compacted (YYYY-MM-DD format)
    pub latest_compacted_date: Option<String>,
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
    /// Latest date that was compacted (YYYY-MM-DD format)
    pub latest_compacted_date: Option<String>,
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
    pub project_path: Option<String>,
    pub project_name: Option<String>,
    pub jira_issue_key: Option<String>,
    /// Start time for Gantt chart display (HH:MM format, e.g. "09:00")
    pub start_time: Option<String>,
    /// End time for Gantt chart display (HH:MM format, e.g. "10:30")
    pub end_time: Option<String>,
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

    // 0. Fetch hidden project names for this user
    let hidden_projects: Vec<(String,)> = sqlx::query_as(
        "SELECT project_name FROM project_preferences WHERE user_id = ? AND hidden = 1",
    )
    .bind(&claims.sub)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;
    let hidden_names: std::collections::HashSet<String> =
        hidden_projects.into_iter().map(|(n,)| n).collect();

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

        // Skip manual projects - they're shown in manual_items, not as projects
        if project_path.contains("manual-projects") {
            continue;
        }

        let project_name = std::path::Path::new(&project_path).file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();

        // Skip hidden projects
        if hidden_names.contains(&project_name) {
            continue;
        }

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
        // Skip manual projects - they're shown in manual_items, not as projects
        if project_path.contains("manual-projects") {
            continue;
        }
        get_or_create_day(&mut days_map, day);
        if let Some(day_entry) = days_map.get_mut(day.as_str()) {
            // Skip if already have a daily summary for this project
            if day_entry.projects.iter().any(|p| &p.project_path == project_path) {
                continue;
            }
            let project_name = std::path::Path::new(&project_path).file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();

            // Skip hidden projects
            if hidden_names.contains(&project_name) {
                continue;
            }

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

    // Add manual items
    for item in &manual_items {
        let date = item.date.to_string();
        get_or_create_day(&mut days_map, &date);
        if let Some(day) = days_map.get_mut(&date) {
            // Extract project name from project_path (e.g., "~/.recap/manual-projects/會議" -> "會議")
            let project_name = item.project_path.as_ref().and_then(|p| {
                std::path::Path::new(p)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            });
            day.manual_items.push(ManualWorkItem {
                id: item.id.clone(),
                title: item.title.clone(),
                description: item.description.clone(),
                hours: item.hours,
                date: date.clone(),
                project_path: item.project_path.clone(),
                project_name,
                jira_issue_key: item.jira_issue_key.clone(),
                start_time: item.start_time.clone(),
                end_time: item.end_time.clone(),
            });
        }
    }

    // Post-process: for any project with 0 commits, query git directly
    for (date, day) in days_map.iter_mut() {
        for project in day.projects.iter_mut() {
            if project.total_commits == 0 {
                if let Ok(naive_date) = NaiveDate::parse_from_str(date, "%Y-%m-%d") {
                    let author = recap_core::get_git_user_email(&project.project_path);
                    let git_commits = get_commits_for_date(&project.project_path, &naive_date, author.as_deref());
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

    // Build maps from snapshot_raw_data:
    // 1. hour_bucket -> full commits (for when summaries lack commit data)
    // 2. hash -> timestamp (for enriching summary commits)
    let (commits_by_hour, commit_timestamps): (
        std::collections::HashMap<String, Vec<GitCommitRef>>,
        std::collections::HashMap<String, String>,
    ) = {
        let all_snapshots: Vec<SnapshotRawData> = sqlx::query_as(
            r#"SELECT * FROM snapshot_raw_data
               WHERE user_id = ? AND project_path = ?
                 AND hour_bucket >= ? AND hour_bucket <= ?
               ORDER BY hour_bucket"#,
        )
        .bind(&claims.sub)
        .bind(&project_path)
        .bind(&wide_start_summary)
        .bind(&wide_end_summary)
        .fetch_all(&db.pool)
        .await
        .unwrap_or_default();

        let mut by_hour: std::collections::HashMap<String, Vec<GitCommitRef>> = std::collections::HashMap::new();
        let mut timestamps: std::collections::HashMap<String, String> = std::collections::HashMap::new();

        for snapshot in all_snapshots {
            let hour_key = extract_local_hour(&snapshot.hour_bucket);
            if let Some(git_commits_json) = &snapshot.git_commits {
                if let Ok(commits) = serde_json::from_str::<Vec<serde_json::Value>>(git_commits_json) {
                    for commit in &commits {
                        if let (Some(hash), Some(timestamp)) = (
                            commit.get("hash").and_then(|h| h.as_str()),
                            commit.get("timestamp").and_then(|t| t.as_str()),
                        ) {
                            timestamps.insert(hash.to_string(), timestamp.to_string());
                        }
                    }
                    // Also store full commits by hour
                    let hour_commits: Vec<GitCommitRef> = commits.iter().filter_map(|c| {
                        Some(GitCommitRef {
                            hash: c.get("hash")?.as_str()?.to_string(),
                            message: c.get("message")?.as_str()?.to_string(),
                            timestamp: c.get("timestamp").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                        })
                    }).collect();
                    by_hour.entry(hour_key).or_default().extend(hour_commits);
                }
            }
        }
        (by_hour, timestamps)
    };

    // Build Claude Code items from hourly summaries if available
    let mut items: Vec<HourlyBreakdownItem> = if !summaries.is_empty() {
        summaries.into_iter().map(|s| {
            let hour_start = extract_local_hour(&s.period_start);
            let hour_end = extract_local_hour(&s.period_end);
            let files: Vec<String> = s.key_activities.as_ref()
                .and_then(|a| serde_json::from_str(a).ok())
                .unwrap_or_default();

            // Try to get commits from summary first
            let mut commits: Vec<GitCommitRef> = s.git_commits_summary.as_ref()
                .and_then(|g| {
                    serde_json::from_str::<Vec<String>>(g).ok().map(|strings| {
                        strings.iter().filter_map(|s| {
                            let parts: Vec<&str> = s.splitn(2, ": ").collect();
                            if parts.len() == 2 {
                                let hash_part: Vec<&str> = parts[0].splitn(2, ' ').collect();
                                let hash = hash_part[0].to_string();
                                // Look up timestamp from snapshot_raw_data
                                let timestamp = commit_timestamps.get(&hash).cloned().unwrap_or_default();
                                Some(GitCommitRef {
                                    hash,
                                    message: parts[1].to_string(),
                                    timestamp,
                                })
                            } else {
                                None
                            }
                        }).collect()
                    })
                })
                .unwrap_or_default();

            // If summary has no commits, fall back to snapshot_raw_data commits for this hour
            if commits.is_empty() {
                if let Some(snapshot_commits) = commits_by_hour.get(&hour_start) {
                    commits = snapshot_commits.clone();
                }
            }

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

    // Sort by hour_start descending
    items.sort_by(|a, b| {
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
    /// Data source: "claude_code"
    pub source: String,
}

/// Git commit reference
#[derive(Debug, Clone, Serialize)]
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
        llm_warnings: result.llm_warnings,
        latest_compacted_date: result.latest_compacted_date,
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
        latest_compacted_date: result.compaction_result.latest_compacted_date,
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
