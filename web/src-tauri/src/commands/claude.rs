//! Claude Code sessions commands
//!
//! Tauri commands for Claude Code session operations.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;
use tauri::State;

use crate::auth::verify_token;

use super::AppState;

// Types

#[derive(Debug, Serialize)]
pub struct ClaudeProject {
    pub path: String,
    pub name: String,
    pub sessions: Vec<ClaudeSession>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ToolUsage {
    pub tool_name: String,
    pub count: usize,
    pub details: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ClaudeSession {
    pub session_id: String,
    pub agent_id: String,
    pub slug: String,
    pub cwd: String,
    pub git_branch: Option<String>,
    pub first_message: Option<String>,
    pub message_count: usize,
    pub first_timestamp: Option<String>,
    pub last_timestamp: Option<String>,
    pub file_path: String,
    pub file_size: u64,
    pub tool_usage: Vec<ToolUsage>,
    pub files_modified: Vec<String>,
    pub commands_run: Vec<String>,
    pub user_messages: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SessionMessage {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    #[serde(rename = "agentId")]
    agent_id: Option<String>,
    slug: Option<String>,
    cwd: Option<String>,
    #[serde(rename = "gitBranch")]
    git_branch: Option<String>,
    timestamp: Option<String>,
    message: Option<MessageContent>,
}

#[derive(Debug, Deserialize)]
struct MessageContent {
    role: Option<String>,
    content: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ToolUseContent {
    #[serde(rename = "type")]
    content_type: Option<String>,
    name: Option<String>,
    input: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ImportSessionsRequest {
    pub session_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub imported: usize,
    pub work_items_created: usize,
}

#[derive(Debug, Deserialize)]
pub struct SummarizeRequest {
    pub session_file_path: String,
}

#[derive(Debug, Serialize)]
pub struct SummarizeResult {
    pub summary: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SyncProjectsRequest {
    pub project_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SyncResult {
    pub sessions_processed: usize,
    pub sessions_skipped: usize,
    pub work_items_created: usize,
    pub work_items_updated: usize,
}

// Internal types for aggregation

#[derive(Debug, Clone)]
struct GitCommit {
    hash: String,
    message: String,
}

#[derive(Debug, Clone)]
struct SessionSummary {
    first_message: Option<String>,
    hours: f64,
    tool_usage: Vec<ToolUsage>,
    files_modified: Vec<String>,
}

struct DailyWorkItem {
    project_path: String,
    project_name: String,
    date: String,
    sessions: Vec<SessionSummary>,
    total_hours: f64,
    git_commits: Vec<GitCommit>,
}

// Helper functions

fn get_claude_home() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".claude"))
}

fn generate_daily_hash(user_id: &str, project: &str, date: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(user_id.as_bytes());
    hasher.update(project.as_bytes());
    hasher.update(date.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn is_meaningful_message(content: &str) -> bool {
    let trimmed = content.trim().to_lowercase();
    if trimmed == "warmup" || trimmed.starts_with("warmup") {
        return false;
    }
    if trimmed.starts_with("<command-") || trimmed.starts_with("<system-") {
        return false;
    }
    if trimmed.len() < 10 {
        return false;
    }
    true
}

fn calculate_session_hours(first: &Option<String>, last: &Option<String>) -> f64 {
    match (first, last) {
        (Some(start), Some(end)) => {
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
        _ => 0.5,
    }
}

fn extract_tool_detail(tool_name: &str, input: &serde_json::Value) -> Option<String> {
    match tool_name {
        "Edit" | "Write" | "Read" => {
            input.get("file_path")
                .and_then(|v| v.as_str())
                .map(|p| {
                    let parts: Vec<&str> = p.split('/').collect();
                    if parts.len() > 3 {
                        format!(".../{}", parts[parts.len()-3..].join("/"))
                    } else {
                        p.to_string()
                    }
                })
        }
        "Bash" => {
            input.get("command")
                .and_then(|v| v.as_str())
                .map(|c| {
                    let truncated: String = c.chars().take(60).collect();
                    if c.len() > 60 { format!("{}...", truncated) } else { truncated }
                })
        }
        "Glob" | "Grep" => {
            input.get("pattern")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        }
        "Task" => {
            input.get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.chars().take(50).collect())
        }
        _ => None
    }
}

fn parse_session_file(path: &PathBuf) -> Option<ClaudeSession> {
    let file = fs::File::open(path).ok()?;
    let file_size = file.metadata().ok()?.len();
    let reader = BufReader::new(file);

    let mut session_id: Option<String> = None;
    let mut agent_id: Option<String> = None;
    let mut slug: Option<String> = None;
    let mut cwd: Option<String> = None;
    let mut git_branch: Option<String> = None;
    let mut first_message: Option<String> = None;
    let mut first_timestamp: Option<String> = None;
    let mut last_timestamp: Option<String> = None;
    let mut meaningful_message_count: usize = 0;

    let mut tool_counts: HashMap<String, usize> = HashMap::new();
    let mut tool_details: HashMap<String, Vec<String>> = HashMap::new();
    let mut files_modified: Vec<String> = Vec::new();
    let mut commands_run: Vec<String> = Vec::new();
    let mut user_messages: Vec<String> = Vec::new();

    for line in reader.lines().flatten() {
        if let Ok(msg) = serde_json::from_str::<SessionMessage>(&line) {
            if session_id.is_none() {
                session_id = msg.session_id;
            }
            if agent_id.is_none() {
                agent_id = msg.agent_id;
            }
            if slug.is_none() {
                slug = msg.slug;
            }
            if cwd.is_none() {
                cwd = msg.cwd;
            }
            if git_branch.is_none() {
                git_branch = msg.git_branch;
            }

            if let Some(ts) = &msg.timestamp {
                if first_timestamp.is_none() {
                    first_timestamp = Some(ts.clone());
                }
                last_timestamp = Some(ts.clone());
            }

            if let Some(ref message) = msg.message {
                if message.role.as_deref() == Some("user") {
                    if let Some(content) = &message.content {
                        if let serde_json::Value::String(s) = content {
                            if is_meaningful_message(s) {
                                meaningful_message_count += 1;
                                if first_message.is_none() {
                                    first_message = Some(s.chars().take(200).collect());
                                }
                                if user_messages.len() < 10 {
                                    let truncated: String = s.chars().take(100).collect();
                                    if !user_messages.contains(&truncated) {
                                        user_messages.push(truncated);
                                    }
                                }
                            }
                        }
                    }
                }

                if message.role.as_deref() == Some("assistant") {
                    if let Some(content) = &message.content {
                        if let serde_json::Value::Array(arr) = content {
                            for item in arr {
                                if let Ok(tool_use) = serde_json::from_value::<ToolUseContent>(item.clone()) {
                                    if tool_use.content_type.as_deref() == Some("tool_use") {
                                        if let Some(tool_name) = &tool_use.name {
                                            *tool_counts.entry(tool_name.clone()).or_insert(0) += 1;

                                            if let Some(input) = &tool_use.input {
                                                let detail = extract_tool_detail(tool_name, input);
                                                if let Some(d) = detail {
                                                    let details = tool_details.entry(tool_name.clone()).or_default();
                                                    if details.len() < 10 && !details.contains(&d) {
                                                        details.push(d.clone());

                                                        match tool_name.as_str() {
                                                            "Edit" | "Write" | "Read" => {
                                                                if !files_modified.contains(&d) && files_modified.len() < 20 {
                                                                    files_modified.push(d);
                                                                }
                                                            }
                                                            "Bash" => {
                                                                if commands_run.len() < 10 {
                                                                    commands_run.push(d);
                                                                }
                                                            }
                                                            _ => {}
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
        }
    }

    let tool_usage: Vec<ToolUsage> = tool_counts
        .into_iter()
        .map(|(name, count)| ToolUsage {
            tool_name: name.clone(),
            count,
            details: tool_details.remove(&name).unwrap_or_default(),
        })
        .collect();

    if agent_id.is_none() {
        let filename = path.file_stem()?.to_str()?;
        if filename.starts_with("agent-") {
            agent_id = Some(filename.trim_start_matches("agent-").to_string());
        } else {
            agent_id = Some(filename.to_string());
        }
    }

    Some(ClaudeSession {
        session_id: session_id.unwrap_or_else(|| "unknown".to_string()),
        agent_id: agent_id.unwrap_or_else(|| "unknown".to_string()),
        slug: slug.unwrap_or_else(|| "unnamed".to_string()),
        cwd: cwd.unwrap_or_default(),
        git_branch,
        first_message,
        message_count: meaningful_message_count,
        first_timestamp,
        last_timestamp,
        file_path: path.to_string_lossy().to_string(),
        file_size,
        tool_usage,
        files_modified,
        commands_run,
        user_messages,
    })
}

fn build_session_description(session: &ClaudeSession, hours: f64) -> String {
    let mut desc_parts = vec![
        format!("📁 Project: {}", session.cwd),
        format!("🌿 Branch: {}", session.git_branch.as_deref().unwrap_or("N/A")),
        format!("💬 Messages: {} | ⏱️ Duration: {:.1}h", session.message_count, hours),
    ];

    if !session.files_modified.is_empty() {
        let files: Vec<_> = session.files_modified.iter().take(10).collect();
        let files_str = files.iter().map(|f| format!("  • {}", f)).collect::<Vec<_>>().join("\n");
        let more = if session.files_modified.len() > 10 {
            format!(" (+{} more)", session.files_modified.len() - 10)
        } else {
            String::new()
        };
        desc_parts.push(format!("📝 Files Modified ({}{}):\n{}", files.len(), more, files_str));
    }

    if !session.tool_usage.is_empty() {
        let tools_summary: Vec<_> = session.tool_usage.iter()
            .filter(|t| t.count > 0)
            .map(|t| format!("{}: {}", t.tool_name, t.count))
            .collect();
        if !tools_summary.is_empty() {
            desc_parts.push(format!("🔧 Tools: {}", tools_summary.join(", ")));
        }
    }

    if !session.commands_run.is_empty() {
        let cmds: Vec<_> = session.commands_run.iter().take(5).collect();
        let cmds_str = cmds.iter().map(|c| format!("  $ {}", c)).collect::<Vec<_>>().join("\n");
        desc_parts.push(format!("💻 Commands:\n{}", cmds_str));
    }

    if !session.user_messages.is_empty() {
        let first_msg = &session.user_messages[0];
        let truncated = if first_msg.len() > 150 {
            format!("{}...", &first_msg.chars().take(150).collect::<String>())
        } else {
            first_msg.clone()
        };
        desc_parts.push(format!("📋 Initial Request: {}", truncated));
    }

    desc_parts.join("\n\n")
}

fn get_git_commits_for_date(project_path: &str, date: &str) -> Vec<GitCommit> {
    let project_dir = PathBuf::from(project_path);

    if !project_dir.exists() || !project_dir.join(".git").exists() {
        return Vec::new();
    }

    let since = format!("{} 00:00:00", date);
    let until = format!("{} 23:59:59", date);

    let output = Command::new("git")
        .arg("log")
        .arg("--since")
        .arg(&since)
        .arg("--until")
        .arg(&until)
        .arg("--format=%H|%s")
        .arg("--date=short")
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
        let parts: Vec<&str> = line.splitn(2, '|').collect();
        if parts.len() >= 2 {
            commits.push(GitCommit {
                hash: parts[0].chars().take(8).collect(),
                message: parts[1].to_string(),
            });
        }
    }

    commits
}

fn build_daily_description(daily: &DailyWorkItem) -> String {
    let mut parts = vec![];

    if !daily.git_commits.is_empty() {
        let commits_str: Vec<String> = daily.git_commits.iter()
            .take(15)
            .map(|c| format!("  • {} - {}", c.hash, c.message))
            .collect();
        let more = if daily.git_commits.len() > 15 {
            format!(" (+{} more)", daily.git_commits.len() - 15)
        } else {
            String::new()
        };
        parts.push(format!("🔀 Git Commits ({}{})\n{}",
            daily.git_commits.len(),
            more,
            commits_str.join("\n")
        ));
    }

    parts.push(format!(
        "📊 時間統計: {} 個工作 sessions, 共 {:.1}h",
        daily.sessions.len(),
        daily.total_hours
    ));

    if !daily.sessions.is_empty() {
        let session_titles: Vec<String> = daily.sessions.iter()
            .filter_map(|s| s.first_message.as_ref())
            .take(5)
            .map(|m| {
                let truncated: String = m.chars().take(60).collect();
                if m.len() > 60 { format!("  • {}...", truncated) } else { format!("  • {}", truncated) }
            })
            .collect();
        if !session_titles.is_empty() {
            parts.push(format!("📝 主要任務:\n{}", session_titles.join("\n")));
        }
    }

    let mut tool_totals: HashMap<String, usize> = HashMap::new();
    for session in &daily.sessions {
        for tool in &session.tool_usage {
            *tool_totals.entry(tool.tool_name.clone()).or_insert(0) += tool.count;
        }
    }
    if !tool_totals.is_empty() {
        let mut tools: Vec<_> = tool_totals.into_iter().collect();
        tools.sort_by(|a, b| b.1.cmp(&a.1));
        let tools_str: Vec<_> = tools.iter().take(10).map(|(k, v)| format!("{}: {}", k, v)).collect();
        parts.push(format!("🔧 使用工具: {}", tools_str.join(", ")));
    }

    let mut all_files: Vec<String> = daily.sessions.iter()
        .flat_map(|s| s.files_modified.clone())
        .collect();
    all_files.sort();
    all_files.dedup();
    if !all_files.is_empty() {
        let count = all_files.len();
        let display_files: Vec<_> = all_files.iter().take(8).collect();
        let files_str = display_files.iter().map(|f| format!("  • {}", f)).collect::<Vec<_>>().join("\n");
        let more = if count > 8 { format!(" (+{} more)", count - 8) } else { String::new() };
        parts.push(format!("📁 修改檔案 ({}{})\n{}", display_files.len(), more, files_str));
    }

    parts.join("\n\n")
}

fn extract_session_content(path: &PathBuf) -> String {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return String::new(),
    };
    let reader = BufReader::new(file);

    let mut content_parts: Vec<String> = Vec::new();

    for line in reader.lines().flatten() {
        if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) {
            if let Some(message) = msg.get("message") {
                if message.get("role").and_then(|r| r.as_str()) == Some("user") {
                    if let Some(text) = message.get("content").and_then(|c| c.as_str()) {
                        let trimmed = text.trim();
                        if trimmed.len() >= 10
                            && !trimmed.to_lowercase().starts_with("warmup")
                            && !trimmed.starts_with("<command-")
                        {
                            content_parts.push(format!("User: {}", trimmed.chars().take(200).collect::<String>()));
                        }
                    }
                }
            }
        }

        if content_parts.len() >= 20 {
            break;
        }
    }

    content_parts.join("\n\n")
}

// Commands

/// List all Claude Code sessions from local machine
#[tauri::command]
pub async fn list_claude_sessions(
    _state: State<'_, AppState>,
    token: String,
) -> Result<Vec<ClaudeProject>, String> {
    let _claims = verify_token(&token).map_err(|e| e.to_string())?;

    let claude_home = get_claude_home()
        .ok_or_else(|| "Claude home directory not found".to_string())?;

    let projects_dir = claude_home.join("projects");
    if !projects_dir.exists() {
        return Ok(Vec::new());
    }

    let mut projects: Vec<ClaudeProject> = Vec::new();

    let entries = fs::read_dir(&projects_dir)
        .map_err(|e| e.to_string())?;

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

        let mut sessions: Vec<ClaudeSession> = Vec::new();

        if let Ok(files) = fs::read_dir(&path) {
            for file_entry in files.flatten() {
                let file_path = file_entry.path();
                if file_path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                    if let Some(session) = parse_session_file(&file_path) {
                        sessions.push(session);
                    }
                }
            }
        }

        sessions.sort_by(|a, b| {
            b.last_timestamp.as_ref().unwrap_or(&String::new())
                .cmp(a.last_timestamp.as_ref().unwrap_or(&String::new()))
        });

        if !sessions.is_empty() {
            let project_path = sessions.first()
                .map(|s| s.cwd.clone())
                .unwrap_or_else(|| dir_name.replace('-', "/"));
            let project_name = project_path.split('/').last()
                .unwrap_or(&dir_name)
                .to_string();

            projects.push(ClaudeProject {
                path: project_path,
                name: project_name,
                sessions,
            });
        }
    }

    projects.sort_by(|a, b| {
        let a_latest = a.sessions.first().and_then(|s| s.last_timestamp.as_ref());
        let b_latest = b.sessions.first().and_then(|s| s.last_timestamp.as_ref());
        b_latest.cmp(&a_latest)
    });

    Ok(projects)
}

/// Import selected sessions as work items
#[tauri::command]
pub async fn import_claude_sessions(
    state: State<'_, AppState>,
    token: String,
    request: ImportSessionsRequest,
) -> Result<ImportResult, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let claude_home = get_claude_home()
        .ok_or_else(|| "Claude home directory not found".to_string())?;

    let projects_dir = claude_home.join("projects");
    let mut imported = 0;
    let mut work_items_created = 0;

    let mut session_files: HashMap<String, PathBuf> = HashMap::new();

    if let Ok(projects) = fs::read_dir(&projects_dir) {
        for project_entry in projects.flatten() {
            let project_path = project_entry.path();
            if !project_path.is_dir() {
                continue;
            }

            if let Ok(files) = fs::read_dir(&project_path) {
                for file_entry in files.flatten() {
                    let file_path = file_entry.path();
                    if file_path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                        if let Ok(file) = fs::File::open(&file_path) {
                            let reader = BufReader::new(file);
                            if let Some(Ok(line)) = reader.lines().next() {
                                if let Ok(msg) = serde_json::from_str::<SessionMessage>(&line) {
                                    if let Some(sid) = msg.session_id {
                                        session_files.insert(sid, file_path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    for session_id in &request.session_ids {
        if let Some(file_path) = session_files.get(session_id) {
            if let Some(session) = parse_session_file(file_path) {
                if session.message_count == 0 {
                    continue;
                }

                let hours = calculate_session_hours(&session.first_timestamp, &session.last_timestamp);

                if hours < 0.08 {
                    continue;
                }

                let project_name = session.cwd.split('/').last().unwrap_or(&session.slug);
                let title = if let Some(ref msg) = session.first_message {
                    let truncated = if msg.len() > 80 {
                        format!("{}...", &msg.chars().take(80).collect::<String>())
                    } else {
                        msg.clone()
                    };
                    format!("[{}] {}", project_name, truncated)
                } else {
                    format!("[{}] Claude Code session", project_name)
                };

                let date = session.first_timestamp
                    .as_ref()
                    .and_then(|ts| ts.split('T').next())
                    .unwrap_or("2026-01-01");

                let content_hash = generate_daily_hash(&claims.sub, &session.cwd, date);

                let existing: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM work_items WHERE content_hash = ? AND user_id = ?"
                )
                .bind(&content_hash)
                .bind(&claims.sub)
                .fetch_optional(&db.pool)
                .await
                .map_err(|e| e.to_string())?;

                if existing.is_some() {
                    continue;
                }

                let id = uuid::Uuid::new_v4().to_string();
                let now = Utc::now();
                let description = build_session_description(&session, hours);

                sqlx::query(
                    r#"INSERT INTO work_items
                    (id, user_id, source, source_id, title, description, hours, date, content_hash, created_at, updated_at)
                    VALUES (?, ?, 'claude_code', ?, ?, ?, ?, ?, ?, ?, ?)"#
                )
                .bind(&id)
                .bind(&claims.sub)
                .bind(&session.agent_id)
                .bind(&title)
                .bind(&description)
                .bind(hours)
                .bind(date)
                .bind(&content_hash)
                .bind(now)
                .bind(now)
                .execute(&db.pool)
                .await
                .map_err(|e| e.to_string())?;

                imported += 1;
                work_items_created += 1;
            }
        }
    }

    Ok(ImportResult {
        imported,
        work_items_created,
    })
}

/// Summarize a session using LLM
#[tauri::command]
pub async fn summarize_claude_session(
    state: State<'_, AppState>,
    token: String,
    request: SummarizeRequest,
) -> Result<SummarizeResult, String> {
    use crate::services::create_llm_service;

    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let llm = create_llm_service(&db.pool, &claims.sub).await
        .map_err(|e| e)?;

    if !llm.is_configured() {
        return Ok(SummarizeResult {
            summary: String::new(),
            success: false,
            error: Some("LLM not configured. Please set API key in settings.".to_string()),
        });
    }

    let file_path = PathBuf::from(&request.session_file_path);
    if !file_path.exists() {
        return Err("Session file not found".to_string());
    }

    let content = extract_session_content(&file_path);
    if content.is_empty() {
        return Ok(SummarizeResult {
            summary: String::new(),
            success: false,
            error: Some("No content to summarize".to_string()),
        });
    }

    match llm.summarize_session(&content).await {
        Ok(summary) => Ok(SummarizeResult {
            summary,
            success: true,
            error: None,
        }),
        Err(e) => Ok(SummarizeResult {
            summary: String::new(),
            success: false,
            error: Some(e),
        }),
    }
}

/// Sync selected projects - aggregate sessions by project+date
#[tauri::command]
pub async fn sync_claude_projects(
    state: State<'_, AppState>,
    token: String,
    request: SyncProjectsRequest,
) -> Result<SyncResult, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let claude_home = get_claude_home()
        .ok_or_else(|| "Claude home directory not found".to_string())?;

    let projects_dir = claude_home.join("projects");

    let mut daily_groups: HashMap<(String, String), Vec<(ClaudeSession, f64)>> = HashMap::new();
    let mut sessions_processed = 0;
    let mut sessions_skipped = 0;

    for project_path in &request.project_paths {
        let dir_name = project_path.replace('/', "-");
        let project_dir = projects_dir.join(&dir_name);

        if !project_dir.exists() || !project_dir.is_dir() {
            continue;
        }

        if let Ok(files) = fs::read_dir(&project_dir) {
            for file_entry in files.flatten() {
                let file_path = file_entry.path();
                if !file_path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                    continue;
                }

                if let Some(session) = parse_session_file(&file_path) {
                    if session.message_count == 0 {
                        sessions_skipped += 1;
                        continue;
                    }

                    let hours = calculate_session_hours(&session.first_timestamp, &session.last_timestamp);
                    if hours < 0.08 {
                        sessions_skipped += 1;
                        continue;
                    }

                    let date = session.first_timestamp
                        .as_ref()
                        .and_then(|ts| ts.split('T').next())
                        .unwrap_or("2026-01-01")
                        .to_string();

                    let key = (session.cwd.clone(), date);
                    daily_groups.entry(key).or_default().push((session, hours));
                    sessions_processed += 1;
                }
            }
        }
    }

    let mut created = 0;
    let mut updated = 0;
    let now = Utc::now();

    for ((project_path, date), sessions) in daily_groups {
        let project_name = project_path.split('/').last().unwrap_or("unknown").to_string();

        let total_hours: f64 = sessions.iter().map(|(_, h)| h).sum();
        let session_summaries: Vec<SessionSummary> = sessions.iter().map(|(s, h)| {
            SessionSummary {
                first_message: s.first_message.clone(),
                hours: *h,
                tool_usage: s.tool_usage.clone(),
                files_modified: s.files_modified.clone(),
            }
        }).collect();

        let git_commits = get_git_commits_for_date(&project_path, &date);

        let daily = DailyWorkItem {
            project_path: project_path.clone(),
            project_name: project_name.clone(),
            date: date.clone(),
            sessions: session_summaries,
            total_hours,
            git_commits,
        };

        let content_hash = generate_daily_hash(&claims.sub, &project_path, &date);

        let commit_count = daily.git_commits.len();
        let title = if commit_count > 0 {
            let first_commit_msg = daily.git_commits.first()
                .map(|c| {
                    let msg: String = c.message.chars().take(40).collect();
                    if c.message.len() > 40 { format!("{}...", msg) } else { msg }
                })
                .unwrap_or_default();
            format!("[{}] {} commits: {}", project_name, commit_count, first_commit_msg)
        } else {
            format!("[{}] {} 工作紀錄 ({} sessions)", project_name, date, daily.sessions.len())
        };
        let description = build_daily_description(&daily);

        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM work_items WHERE content_hash = ? AND user_id = ?"
        )
        .bind(&content_hash)
        .bind(&claims.sub)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

        if let Some((existing_id,)) = existing {
            sqlx::query(
                r#"UPDATE work_items
                SET title = ?, description = ?, hours = ?, updated_at = ?
                WHERE id = ?"#
            )
            .bind(&title)
            .bind(&description)
            .bind(total_hours)
            .bind(now)
            .bind(&existing_id)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;

            updated += 1;
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                r#"INSERT INTO work_items
                (id, user_id, source, title, description, hours, date, content_hash, created_at, updated_at)
                VALUES (?, ?, 'claude_code', ?, ?, ?, ?, ?, ?, ?)"#
            )
            .bind(&id)
            .bind(&claims.sub)
            .bind(&title)
            .bind(&description)
            .bind(total_hours)
            .bind(&date)
            .bind(&content_hash)
            .bind(now)
            .bind(now)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;

            created += 1;
        }
    }

    Ok(SyncResult {
        sessions_processed,
        sessions_skipped,
        work_items_created: created,
        work_items_updated: updated,
    })
}
