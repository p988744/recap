//! Claude session commands
//!
//! Commands for listing and viewing Claude Code sessions.

use anyhow::Result;
use chrono::{DateTime, NaiveDate};
use clap::Subcommand;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tabled::Tabled;

use recap_core::{parse_session_fast, parse_session_full, ParsedSession};

use crate::output::{print_output, print_info};
use super::Context;

#[derive(Subcommand)]
pub enum ClaudeAction {
    /// List all Claude sessions
    List {
        /// Filter by project path (substring match)
        #[arg(long, short)]
        project: Option<String>,

        /// Filter by date (YYYY-MM-DD)
        #[arg(long, short)]
        date: Option<String>,
    },

    /// Show session details
    Show {
        /// Session ID (UUID from filename)
        session_id: String,
    },
}

/// Session row for table display
#[derive(Debug, Serialize, Tabled)]
pub struct SessionRow {
    #[tabled(rename = "Session ID")]
    pub session_id: String,
    #[tabled(rename = "Project")]
    pub project: String,
    #[tabled(rename = "Date")]
    pub date: String,
    #[tabled(rename = "Duration")]
    pub duration: String,
    #[tabled(rename = "Messages")]
    pub messages: String,
    #[tabled(rename = "First Message")]
    pub first_message: String,
}

/// Session detail for JSON output
#[derive(Debug, Serialize)]
pub struct SessionDetail {
    pub session_id: String,
    pub project: String,
    pub date: String,
    pub duration: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub message_count: usize,
    pub first_message: Option<String>,
    pub tool_usage: Vec<ToolUsageRow>,
    pub files_modified: Vec<String>,
}

#[derive(Debug, Serialize, Tabled)]
pub struct ToolUsageRow {
    #[tabled(rename = "Tool")]
    pub tool: String,
    #[tabled(rename = "Count")]
    pub count: usize,
}

pub async fn execute(ctx: &Context, action: ClaudeAction) -> Result<()> {
    match action {
        ClaudeAction::List { project, date } => list_sessions(ctx, project, date).await,
        ClaudeAction::Show { session_id } => show_session(ctx, session_id).await,
    }
}

async fn list_sessions(ctx: &Context, project_filter: Option<String>, date_filter: Option<String>) -> Result<()> {
    let claude_home = get_claude_home()
        .ok_or_else(|| anyhow::anyhow!("Claude home directory not found. Expected at ~/.claude"))?;

    let projects_dir = claude_home.join("projects");
    if !projects_dir.exists() {
        print_info("No Claude projects directory found.", ctx.quiet);
        return Ok(());
    }

    // Parse date filter if provided
    let filter_date: Option<NaiveDate> = if let Some(date_str) = &date_filter {
        Some(NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|_| anyhow::anyhow!("Invalid date format. Use YYYY-MM-DD"))?)
    } else {
        None
    };

    let mut rows: Vec<SessionRow> = Vec::new();

    // Iterate through project directories
    let entries = fs::read_dir(&projects_dir)?;
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

        // Iterate through session files
        if let Ok(files) = fs::read_dir(&path) {
            for file_entry in files.flatten() {
                let file_path = file_entry.path();
                if file_path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                    if let Some(session) = parse_session_for_list(&file_path) {
                        // Apply project filter
                        if let Some(ref filter) = project_filter {
                            let project_lower = session.project.to_lowercase();
                            let filter_lower = filter.to_lowercase();
                            if !project_lower.contains(&filter_lower) {
                                continue;
                            }
                        }

                        // Apply date filter
                        if let Some(filter_date) = filter_date {
                            if let Ok(session_date) = NaiveDate::parse_from_str(&session.date, "%Y-%m-%d") {
                                if session_date != filter_date {
                                    continue;
                                }
                            } else {
                                continue;
                            }
                        }

                        rows.push(session);
                    }
                }
            }
        }
    }

    // Sort by date descending
    rows.sort_by(|a, b| b.date.cmp(&a.date));

    if rows.is_empty() {
        print_info("No sessions found matching the criteria.", ctx.quiet);
    } else {
        print_output(&rows, ctx.format)?;
    }

    Ok(())
}

async fn show_session(ctx: &Context, session_id: String) -> Result<()> {
    let claude_home = get_claude_home()
        .ok_or_else(|| anyhow::anyhow!("Claude home directory not found. Expected at ~/.claude"))?;

    let projects_dir = claude_home.join("projects");
    if !projects_dir.exists() {
        return Err(anyhow::anyhow!("No Claude projects directory found."));
    }

    // Find session file by ID
    let session_path = find_session_by_id(&projects_dir, &session_id)?;

    // Parse full session details
    let parsed = parse_session_full(&session_path)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse session file"))?;

    let session_id_from_path = session_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let project_name = extract_project_name(&parsed.cwd);
    let (date, duration, start_time, end_time) = calculate_session_timing(&parsed);

    let detail = SessionDetail {
        session_id: session_id_from_path,
        project: project_name,
        date,
        duration,
        start_time,
        end_time,
        message_count: parsed.message_count,
        first_message: parsed.first_message,
        tool_usage: parsed.tool_usage.iter().map(|t| ToolUsageRow {
            tool: t.tool_name.clone(),
            count: t.count,
        }).collect(),
        files_modified: parsed.files_modified,
    };

    // Print based on format
    match ctx.format {
        crate::output::OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&detail)?);
        }
        crate::output::OutputFormat::Table => {
            print_session_detail_table(&detail, ctx.quiet);
        }
    }

    Ok(())
}

// ============ Helper Functions ============

fn get_claude_home() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude"))
}

fn find_session_by_id(projects_dir: &PathBuf, session_id: &str) -> Result<PathBuf> {
    let entries = fs::read_dir(projects_dir)?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        if let Ok(files) = fs::read_dir(&path) {
            for file_entry in files.flatten() {
                let file_path = file_entry.path();
                if file_path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                    let file_name = file_path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");

                    // Match full ID or prefix
                    if file_name == session_id || file_name.starts_with(session_id) {
                        return Ok(file_path);
                    }
                }
            }
        }
    }

    Err(anyhow::anyhow!("Session not found: {}", session_id))
}

fn parse_session_for_list(path: &PathBuf) -> Option<SessionRow> {
    let metadata = parse_session_fast(path)?;

    let session_id = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let project = extract_project_name(&metadata.cwd.unwrap_or_default());

    let (date, duration) = calculate_date_and_duration(&metadata.first_ts, &metadata.last_ts);

    let first_message = metadata.first_msg
        .map(|m| truncate_string(&m, 40))
        .unwrap_or_else(|| "-".to_string());

    Some(SessionRow {
        session_id: truncate_string(&session_id, 12),
        project,
        date,
        duration,
        messages: metadata.message_count.to_string(),
        first_message,
    })
}

fn extract_project_name(cwd: &str) -> String {
    if cwd.is_empty() {
        return "unknown".to_string();
    }

    std::path::Path::new(&cwd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn calculate_date_and_duration(first_ts: &str, last_ts: &str) -> (String, String) {
    let date = first_ts.split('T').next().unwrap_or("unknown").to_string();

    let duration = if let (Ok(start), Ok(end)) = (
        DateTime::parse_from_rfc3339(first_ts),
        DateTime::parse_from_rfc3339(last_ts),
    ) {
        let diff = end.signed_duration_since(start);
        let hours = diff.num_minutes() as f64 / 60.0;
        if hours < 0.1 {
            "< 0.1h".to_string()
        } else {
            format!("{:.1}h", hours)
        }
    } else {
        "-".to_string()
    };

    (date, duration)
}

fn calculate_session_timing(session: &ParsedSession) -> (String, String, Option<String>, Option<String>) {
    let date = session.first_timestamp.as_ref()
        .map(|ts| ts.split('T').next().unwrap_or("unknown").to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let (duration, start_time, end_time) = if let (Some(first), Some(last)) =
        (&session.first_timestamp, &session.last_timestamp)
    {
        if let (Ok(start), Ok(end)) = (
            DateTime::parse_from_rfc3339(first),
            DateTime::parse_from_rfc3339(last),
        ) {
            let diff = end.signed_duration_since(start);
            let hours = diff.num_minutes() as f64 / 60.0;
            let duration_str = if hours < 0.1 {
                "< 0.1h".to_string()
            } else {
                format!("{:.1}h", hours)
            };

            let start_str = start.format("%H:%M").to_string();
            let end_str = end.format("%H:%M").to_string();

            (duration_str, Some(start_str), Some(end_str))
        } else {
            ("-".to_string(), None, None)
        }
    } else {
        ("-".to_string(), None, None)
    };

    (date, duration, start_time, end_time)
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

fn print_session_detail_table(detail: &SessionDetail, quiet: bool) {
    if !quiet {
        println!("Session: {}", detail.session_id);
        println!("Project: {}", detail.project);
        println!("Date: {}", detail.date);

        if let (Some(start), Some(end)) = (&detail.start_time, &detail.end_time) {
            println!("Duration: {} ({} - {})", detail.duration, start, end);
        } else {
            println!("Duration: {}", detail.duration);
        }

        println!("Messages: {}", detail.message_count);
        println!();

        if let Some(ref msg) = detail.first_message {
            println!("First Message:");
            println!("  {}", msg);
            println!();
        }

        if !detail.tool_usage.is_empty() {
            println!("Tool Usage:");
            for tool in &detail.tool_usage {
                println!("  - {}: {} times", tool.tool, tool.count);
            }
            println!();
        }

        if !detail.files_modified.is_empty() {
            println!("Files Modified:");
            for file in &detail.files_modified {
                println!("  - {}", file);
            }
        }
    }
}

// ============ Tests ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_project_name_full_path() {
        assert_eq!(extract_project_name("/Users/user/projects/recap"), "recap");
        assert_eq!(extract_project_name("/home/dev/my-project"), "my-project");
    }

    #[test]
    fn test_extract_project_name_empty() {
        assert_eq!(extract_project_name(""), "unknown");
    }

    #[test]
    fn test_extract_project_name_single() {
        assert_eq!(extract_project_name("project"), "project");
    }

    #[test]
    fn test_truncate_string_short() {
        assert_eq!(truncate_string("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_string_exact() {
        assert_eq!(truncate_string("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_string_long() {
        assert_eq!(truncate_string("hello world", 8), "hello...");
    }

    #[test]
    fn test_calculate_date_and_duration_valid() {
        let (date, duration) = calculate_date_and_duration(
            "2026-01-16T09:00:00Z",
            "2026-01-16T11:30:00Z"
        );
        assert_eq!(date, "2026-01-16");
        assert_eq!(duration, "2.5h");
    }

    #[test]
    fn test_calculate_date_and_duration_short() {
        let (date, duration) = calculate_date_and_duration(
            "2026-01-16T09:00:00Z",
            "2026-01-16T09:05:00Z"
        );
        assert_eq!(date, "2026-01-16");
        assert_eq!(duration, "< 0.1h");
    }

    #[test]
    fn test_calculate_date_and_duration_invalid() {
        let (date, duration) = calculate_date_and_duration("invalid", "invalid");
        assert_eq!(date, "invalid");
        assert_eq!(duration, "-");
    }

    #[test]
    fn test_get_claude_home() {
        // Should return Some on most systems
        let home = get_claude_home();
        if let Some(path) = home {
            assert!(path.ends_with(".claude"));
        }
    }

    #[test]
    fn test_tool_usage_row() {
        let row = ToolUsageRow {
            tool: "Edit".to_string(),
            count: 5,
        };
        assert_eq!(row.tool, "Edit");
        assert_eq!(row.count, 5);
    }

    #[test]
    fn test_session_row_fields() {
        let row = SessionRow {
            session_id: "abc123".to_string(),
            project: "recap".to_string(),
            date: "2026-01-16".to_string(),
            duration: "1.5h".to_string(),
            messages: "10".to_string(),
            first_message: "Help me...".to_string(),
        };
        assert_eq!(row.session_id, "abc123");
        assert_eq!(row.project, "recap");
    }

    #[test]
    fn test_session_detail_serialization() {
        let detail = SessionDetail {
            session_id: "test-123".to_string(),
            project: "test".to_string(),
            date: "2026-01-16".to_string(),
            duration: "1.0h".to_string(),
            start_time: Some("09:00".to_string()),
            end_time: Some("10:00".to_string()),
            message_count: 5,
            first_message: Some("Test message".to_string()),
            tool_usage: vec![],
            files_modified: vec![],
        };

        let json = serde_json::to_string(&detail).unwrap();
        assert!(json.contains("test-123"));
        assert!(json.contains("2026-01-16"));
    }
}
