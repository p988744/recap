//! Commit-centric worklog
//!
//! Commands for generating commit-centric worklogs.

use std::collections::HashMap;
use chrono::{DateTime, Local, NaiveDate};
use tauri::State;

use recap_core::services::{build_rule_based_outcome, get_commits_for_date, is_meaningful_message, StandaloneSession};

use crate::commands::AppState;
use super::types::{CommitCentricQuery, CommitCentricWorklog};

/// Get commit-centric worklog for a date
/// Returns commits as primary records with session data as supplementary
#[tauri::command]
pub async fn get_commit_centric_worklog(
    _state: State<'_, AppState>,
    token: String,
    query: CommitCentricQuery,
) -> Result<CommitCentricWorklog, String> {
    let _claims = recap_core::auth::verify_token(&token).map_err(|e| e.to_string())?;

    let date = NaiveDate::parse_from_str(&query.date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date format: {}", e))?;

    // Determine project path
    let project_path = query.project_path.unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    let project_name = std::path::Path::new(&project_path)
        .file_name()
        .and_then(|n| n.to_str())
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
) -> Result<Vec<StandaloneSession>, String> {
    let target_date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date: {}", e))?;

    let claude_home = dirs::home_dir()
        .map(|h| h.join(".claude").join("projects"));

    let projects_dir = match claude_home {
        Some(dir) if dir.exists() => dir,
        _ => return Ok(Vec::new()),
    };

    let mut standalone = Vec::new();

    // Find the Claude project directory for this project
    let project_dir_name = project_path.replace(['/', '\\'], "-");

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
                                project: std::path::Path::new(&project_path).file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string(),
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
    target_date: &NaiveDate,
) -> Option<SessionWorklogData> {
    use std::io::{BufRead, BufReader};

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
                            if is_meaningful_message(content) {
                                first_message = Some(content.trim().chars().take(100).collect());
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
