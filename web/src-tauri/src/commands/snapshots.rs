//! Snapshot & Compaction Tauri Commands
//!
//! Provides commands for querying work summaries, viewing snapshot details,
//! and triggering compaction manually.

use chrono::Datelike;
use recap_core::auth::verify_token;
use recap_core::models::{SnapshotRawData, WorkSummary};
use serde::Serialize;
use tauri::State;

use super::AppState;

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
    let snapshot_stats: Vec<(String, String, i32, i32)> = sqlx::query_as(
        r#"SELECT project_path, DATE(hour_bucket) as day,
              SUM(json_array_length(git_commits)) as commit_count,
              SUM(json_array_length(files_modified)) as file_count
           FROM snapshot_raw_data
           WHERE user_id = ? AND DATE(hour_bucket) >= ? AND DATE(hour_bucket) <= ?
           GROUP BY project_path, DATE(hour_bucket)
           ORDER BY day DESC"#,
    )
    .bind(&claims.sub)
    .bind(&start_date)
    .bind(&end_date)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

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
        let project_name = project_path.split('/').last().unwrap_or("unknown").to_string();

        // Parse commit/file counts from summary metadata
        let commit_count = summary.git_commits_summary.as_ref()
            .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(s).ok())
            .map(|v| v.len() as i32)
            .unwrap_or(0);
        let file_count = summary.key_activities.as_ref()
            .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(s).ok())
            .map(|v| v.len() as i32)
            .unwrap_or(0);

        let has_hourly = hourly_exists.iter().any(|(pp, d)| pp == &project_path && d == &date);

        get_or_create_day(&mut days_map, &date);
        if let Some(day) = days_map.get_mut(&date) {
            day.projects.push(WorklogDayProject {
                project_path: project_path.clone(),
                project_name,
                daily_summary: Some(summary.summary.clone()),
                total_commits: commit_count,
                total_files: file_count,
                total_hours: 0.0, // TODO: calculate from hourly data
                has_hourly_data: has_hourly,
            });
        }
    }

    // Add snapshot-only data (projects with snapshots but no daily summary)
    for (project_path, day, commits, files) in &snapshot_stats {
        get_or_create_day(&mut days_map, day);
        if let Some(day_entry) = days_map.get_mut(day.as_str()) {
            // Skip if already have a daily summary for this project
            if day_entry.projects.iter().any(|p| &p.project_path == project_path) {
                continue;
            }
            let project_name = project_path.split('/').last().unwrap_or("unknown").to_string();
            let has_hourly = hourly_exists.iter().any(|(pp, d)| pp == project_path && d == day);

            day_entry.projects.push(WorklogDayProject {
                project_path: project_path.clone(),
                project_name,
                daily_summary: None,
                total_commits: *commits,
                total_files: *files,
                total_hours: 0.0,
                has_hourly_data: has_hourly,
            });
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

    let start = format!("{}T00:00:00", &date);
    let end = format!("{}T23:59:59", &date);

    // Try hourly summaries first
    let summaries: Vec<WorkSummary> = sqlx::query_as(
        r#"SELECT * FROM work_summaries
           WHERE user_id = ? AND scale = 'hourly' AND project_path = ?
             AND period_start >= ? AND period_start <= ?
           ORDER BY period_start"#,
    )
    .bind(&claims.sub)
    .bind(&project_path)
    .bind(&start)
    .bind(&end)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    if !summaries.is_empty() {
        return Ok(summaries.into_iter().map(|s| {
            let hour_start = s.period_start.get(11..16).unwrap_or("??:??").to_string();
            let hour_end = s.period_end.get(11..16).unwrap_or("??:??").to_string();
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
                summary: s.summary,
                files_modified: files,
                git_commits: commits,
            }
        }).collect());
    }

    // Fallback: build from raw snapshots
    let snapshots: Vec<SnapshotRawData> = sqlx::query_as(
        r#"SELECT * FROM snapshot_raw_data
           WHERE user_id = ? AND project_path = ?
             AND hour_bucket >= ? AND hour_bucket <= ?
           ORDER BY hour_bucket"#,
    )
    .bind(&claims.sub)
    .bind(&project_path)
    .bind(&start)
    .bind(&end)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(snapshots.into_iter().map(|s| {
        let hour_start = s.hour_bucket.get(11..16).unwrap_or("??:??").to_string();
        let hour_end_num = hour_start.get(..2).and_then(|h| h.parse::<u32>().ok()).unwrap_or(0) + 1;
        let hour_end = format!("{:02}:00", hour_end_num.min(23));

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
        }
    }).collect())
}

/// Hourly breakdown item
#[derive(Debug, Serialize)]
pub struct HourlyBreakdownItem {
    pub hour_start: String,
    pub hour_end: String,
    pub summary: String,
    pub files_modified: Vec<String>,
    pub git_commits: Vec<GitCommitRef>,
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
