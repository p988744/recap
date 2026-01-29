//! Claude Code sessions commands
//!
//! Tauri commands for Claude Code session operations.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use tauri::State;

use recap_core::auth::verify_token;
use recap_core::services::{
    generate_daily_hash, is_meaningful_message, extract_tool_detail,
    calculate_session_hours,
};

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

// SyncResult re-exported from services for API response
pub use recap_core::services::ClaudeSyncResult as SyncResult;

// Helper functions

pub(crate) fn get_claude_home() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude"))
}

// generate_daily_hash, is_meaningful_message, extract_tool_detail, calculate_session_hours
// are imported from crate::services

/// Helper to calculate session hours with Option handling
pub(crate) fn session_hours_from_options(first: &Option<String>, last: &Option<String>) -> f64 {
    match (first, last) {
        (Some(start), Some(end)) => calculate_session_hours(start, end),
        _ => 0.5,
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

pub(crate) fn build_session_description(session: &ClaudeSession, hours: f64) -> String {
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

// get_git_commits_for_date and build_daily_description moved to services/sync.rs

pub(crate) fn extract_session_content(path: &PathBuf) -> String {
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
            let project_name = std::path::Path::new(&project_path)
                .file_name()
                .and_then(|n| n.to_str())
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
                    log::debug!("Skipping session {} - no meaningful messages", session_id);
                    continue;
                }

                let hours = session_hours_from_options(&session.first_timestamp, &session.last_timestamp);
                // Note: calculate_session_hours already enforces minimum 0.1h (6 min)

                let project_name = std::path::Path::new(&session.cwd).file_name().and_then(|n| n.to_str()).unwrap_or(&session.slug);
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
                    log::debug!("Skipping session {} - already exists with hash {}", session_id, content_hash);
                    continue;
                }

                let id = uuid::Uuid::new_v4().to_string();
                let now = Utc::now();
                let description = build_session_description(&session, hours);

                sqlx::query(
                    r#"INSERT INTO work_items
                    (id, user_id, source, source_id, title, description, hours, date, content_hash, hours_source, hours_estimated, created_at, updated_at)
                    VALUES (?, ?, 'claude_code', ?, ?, ?, ?, ?, ?, 'session', ?, ?, ?)"#
                )
                .bind(&id)
                .bind(&claims.sub)
                .bind(&session.agent_id)
                .bind(&title)
                .bind(&description)
                .bind(hours)
                .bind(date)
                .bind(&content_hash)
                .bind(hours)  // hours_estimated = calculated hours
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
    use recap_core::services::create_llm_service;

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
        Ok((summary, _usage)) => Ok(SummarizeResult {
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
/// Delegates to services::sync::sync_claude_projects for the actual implementation
#[tauri::command]
pub async fn sync_claude_projects(
    state: State<'_, AppState>,
    token: String,
    request: SyncProjectsRequest,
) -> Result<SyncResult, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Delegate to service layer
    crate::core_services::sync_claude_projects(&db.pool, &claims.sub, &request.project_paths).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // ==================== get_claude_home Tests ====================

    #[test]
    fn test_get_claude_home() {
        // This test verifies the function works without crashing
        // Result depends on whether HOME env var is set
        let result = get_claude_home();
        if let Some(path) = result {
            assert!(path.to_string_lossy().contains(".claude"));
        }
        // If HOME not set, result will be None
    }

    // ==================== session_hours_from_options Tests ====================

    #[test]
    fn test_session_hours_from_options_valid() {
        let first = Some("2024-01-15T09:00:00+08:00".to_string());
        let last = Some("2024-01-15T11:00:00+08:00".to_string());
        let hours = session_hours_from_options(&first, &last);
        // Should be 2 hours
        assert!((hours - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_session_hours_from_options_none_first() {
        let first = None;
        let last = Some("2024-01-15T11:00:00+08:00".to_string());
        let hours = session_hours_from_options(&first, &last);
        assert!((hours - 0.5).abs() < 0.01); // Default 0.5
    }

    #[test]
    fn test_session_hours_from_options_none_last() {
        let first = Some("2024-01-15T09:00:00+08:00".to_string());
        let last = None;
        let hours = session_hours_from_options(&first, &last);
        assert!((hours - 0.5).abs() < 0.01); // Default 0.5
    }

    #[test]
    fn test_session_hours_from_options_both_none() {
        let hours = session_hours_from_options(&None, &None);
        assert!((hours - 0.5).abs() < 0.01); // Default 0.5
    }

    // ==================== build_session_description Tests ====================

    fn create_test_session() -> ClaudeSession {
        ClaudeSession {
            session_id: "test-session-123".to_string(),
            agent_id: "agent-1".to_string(),
            slug: "test-slug".to_string(),
            file_path: "/tmp/test.jsonl".to_string(),
            cwd: "/home/user/project".to_string(),
            git_branch: Some("main".to_string()),
            first_timestamp: Some("2024-01-15T09:00:00+08:00".to_string()),
            last_timestamp: Some("2024-01-15T11:00:00+08:00".to_string()),
            first_message: Some("Help me fix a bug".to_string()),
            message_count: 10,
            file_size: 1024,
            tool_usage: vec![],
            files_modified: vec![],
            commands_run: vec![],
            user_messages: vec!["Help me fix a bug".to_string()],
        }
    }

    #[test]
    fn test_build_session_description_basic() {
        let session = create_test_session();
        let desc = build_session_description(&session, 2.0);

        assert!(desc.contains("📁 Project: /home/user/project"));
        assert!(desc.contains("🌿 Branch: main"));
        assert!(desc.contains("💬 Messages: 10"));
        assert!(desc.contains("⏱️ Duration: 2.0h"));
    }

    #[test]
    fn test_build_session_description_with_files() {
        let mut session = create_test_session();
        session.files_modified = vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
        ];

        let desc = build_session_description(&session, 1.5);

        assert!(desc.contains("📝 Files Modified"));
        assert!(desc.contains("src/main.rs"));
        assert!(desc.contains("src/lib.rs"));
    }

    #[test]
    fn test_build_session_description_with_tools() {
        let mut session = create_test_session();
        session.tool_usage = vec![
            ToolUsage { tool_name: "Edit".to_string(), count: 5, details: vec![] },
            ToolUsage { tool_name: "Read".to_string(), count: 10, details: vec![] },
        ];

        let desc = build_session_description(&session, 1.0);

        assert!(desc.contains("🔧 Tools:"));
        assert!(desc.contains("Edit: 5"));
        assert!(desc.contains("Read: 10"));
    }

    #[test]
    fn test_build_session_description_with_commands() {
        let mut session = create_test_session();
        session.commands_run = vec![
            "cargo test".to_string(),
            "cargo build".to_string(),
        ];

        let desc = build_session_description(&session, 1.0);

        assert!(desc.contains("💻 Commands:"));
        assert!(desc.contains("$ cargo test"));
        assert!(desc.contains("$ cargo build"));
    }

    #[test]
    fn test_build_session_description_with_user_messages() {
        let mut session = create_test_session();
        session.user_messages = vec!["Help me implement authentication".to_string()];

        let desc = build_session_description(&session, 1.0);

        assert!(desc.contains("📋 Initial Request:"));
        assert!(desc.contains("Help me implement authentication"));
    }

    #[test]
    fn test_build_session_description_no_branch() {
        let mut session = create_test_session();
        session.git_branch = None;

        let desc = build_session_description(&session, 1.0);

        assert!(desc.contains("🌿 Branch: N/A"));
    }

    // ==================== extract_session_content Tests ====================

    #[test]
    fn test_extract_session_content_valid_file() {
        let content = r#"{"timestamp":"2024-01-15T09:00:00+08:00","message":{"role":"user","content":"Help me fix a bug in the authentication module"}}
{"timestamp":"2024-01-15T09:01:00+08:00","message":{"role":"assistant","content":"Sure, let me help you."}}
{"timestamp":"2024-01-15T09:02:00+08:00","message":{"role":"user","content":"The token validation is not working correctly"}}"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let path = file.path().to_path_buf();

        let result = extract_session_content(&path);

        assert!(result.contains("User: Help me fix a bug"));
        assert!(result.contains("User: The token validation"));
        // Assistant messages should not be included
        assert!(!result.contains("Sure, let me help"));
    }

    #[test]
    fn test_extract_session_content_nonexistent_file() {
        let path = PathBuf::from("/nonexistent/path/file.jsonl");
        let result = extract_session_content(&path);
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_session_content_filters_warmup() {
        let content = r#"{"timestamp":"2024-01-15T09:00:00+08:00","message":{"role":"user","content":"warmup message"}}
{"timestamp":"2024-01-15T09:01:00+08:00","message":{"role":"user","content":"Real meaningful user message here"}}"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let path = file.path().to_path_buf();

        let result = extract_session_content(&path);

        // Should filter warmup messages
        assert!(!result.contains("warmup"));
        assert!(result.contains("Real meaningful"));
    }

    #[test]
    fn test_extract_session_content_filters_short_messages() {
        let content = r#"{"timestamp":"2024-01-15T09:00:00+08:00","message":{"role":"user","content":"yes"}}
{"timestamp":"2024-01-15T09:01:00+08:00","message":{"role":"user","content":"This is a longer meaningful message about the project"}}"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let path = file.path().to_path_buf();

        let result = extract_session_content(&path);

        // Should filter short messages (< 10 chars)
        assert!(!result.contains("yes"));
        assert!(result.contains("This is a longer"));
    }

    #[test]
    fn test_extract_session_content_empty_file() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"").unwrap();
        let path = file.path().to_path_buf();

        let result = extract_session_content(&path);
        assert!(result.is_empty());
    }

    // ==================== parse_session_file Tests ====================

    fn create_mock_session_jsonl() -> String {
        let lines = vec![
            r#"{"sessionId":"sess-123","agentId":"agent-456","slug":"test-project","cwd":"/home/user/myproject","gitBranch":"feature/auth","timestamp":"2024-01-15T09:00:00+08:00","message":{"role":"user","content":"Help me implement user authentication"}}"#,
            r#"{"sessionId":"sess-123","timestamp":"2024-01-15T09:05:00+08:00","message":{"role":"assistant","content":[{"type":"tool_use","name":"Read","input":{"file_path":"/home/user/myproject/src/auth.rs"}}]}}"#,
            r#"{"sessionId":"sess-123","timestamp":"2024-01-15T09:10:00+08:00","message":{"role":"user","content":"Now add JWT token validation"}}"#,
            r#"{"sessionId":"sess-123","timestamp":"2024-01-15T10:00:00+08:00","message":{"role":"assistant","content":[{"type":"tool_use","name":"Edit","input":{"file_path":"/home/user/myproject/src/auth.rs"}}]}}"#,
            r#"{"sessionId":"sess-123","timestamp":"2024-01-15T10:30:00+08:00","message":{"role":"assistant","content":[{"type":"tool_use","name":"Bash","input":{"command":"cargo test"}}]}}"#,
        ];
        lines.join("\n")
    }

    #[test]
    fn test_parse_session_file_discovers_session_metadata() {
        let content = create_mock_session_jsonl();
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let path = file.path().to_path_buf();

        let session = parse_session_file(&path);
        assert!(session.is_some(), "Session should be parsed successfully");

        let session = session.unwrap();
        assert_eq!(session.session_id, "sess-123");
        assert_eq!(session.agent_id, "agent-456");
        assert_eq!(session.slug, "test-project");
        assert_eq!(session.cwd, "/home/user/myproject");
        assert_eq!(session.git_branch, Some("feature/auth".to_string()));
    }

    #[test]
    fn test_parse_session_file_discovers_timestamps() {
        let content = create_mock_session_jsonl();
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let path = file.path().to_path_buf();

        let session = parse_session_file(&path).unwrap();

        assert_eq!(session.first_timestamp, Some("2024-01-15T09:00:00+08:00".to_string()));
        assert_eq!(session.last_timestamp, Some("2024-01-15T10:30:00+08:00".to_string()));
    }

    #[test]
    fn test_parse_session_file_discovers_user_messages() {
        let content = create_mock_session_jsonl();
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let path = file.path().to_path_buf();

        let session = parse_session_file(&path).unwrap();

        // Should count meaningful user messages
        assert_eq!(session.message_count, 2);
        // First message should be captured
        assert!(session.first_message.as_ref().unwrap().contains("implement user authentication"));
        // User messages should be collected
        assert!(!session.user_messages.is_empty());
    }

    #[test]
    fn test_parse_session_file_discovers_tool_usage() {
        let content = create_mock_session_jsonl();
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let path = file.path().to_path_buf();

        let session = parse_session_file(&path).unwrap();

        // Should discover tool usage
        assert!(!session.tool_usage.is_empty());

        let tool_names: Vec<&str> = session.tool_usage.iter().map(|t| t.tool_name.as_str()).collect();
        assert!(tool_names.contains(&"Read"));
        assert!(tool_names.contains(&"Edit"));
        assert!(tool_names.contains(&"Bash"));
    }

    #[test]
    fn test_parse_session_file_discovers_files_modified() {
        let content = create_mock_session_jsonl();
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let path = file.path().to_path_buf();

        let session = parse_session_file(&path).unwrap();

        // Should discover files from Read/Edit/Write tools
        assert!(!session.files_modified.is_empty());
        assert!(session.files_modified.iter().any(|f| f.contains("auth.rs")));
    }

    #[test]
    fn test_parse_session_file_discovers_commands() {
        let content = create_mock_session_jsonl();
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let path = file.path().to_path_buf();

        let session = parse_session_file(&path).unwrap();

        // Should discover commands from Bash tool
        assert!(!session.commands_run.is_empty());
        assert!(session.commands_run.iter().any(|c| c.contains("cargo test")));
    }

    #[test]
    fn test_parse_session_file_handles_empty_file() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"").unwrap();
        let path = file.path().to_path_buf();

        let session = parse_session_file(&path);
        // Empty file should still return a session with defaults
        assert!(session.is_some());
    }

    #[test]
    fn test_parse_session_file_handles_invalid_json() {
        let content = "not valid json\nalso not json\n";
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let path = file.path().to_path_buf();

        let session = parse_session_file(&path);
        // Should handle gracefully
        assert!(session.is_some());
    }

    #[test]
    fn test_parse_session_file_extracts_agent_id_from_filename() {
        // When agentId is not in the content, should extract from filename
        let content = r#"{"sessionId":"sess-123","timestamp":"2024-01-15T09:00:00+08:00","message":{"role":"user","content":"Test message here"}}"#;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("agent-abc123.jsonl");
        fs::write(&file_path, content).unwrap();

        let session = parse_session_file(&file_path).unwrap();

        // Should extract agent ID from filename "agent-abc123.jsonl" -> "abc123"
        assert_eq!(session.agent_id, "abc123");
    }

    // ==================== Integration Test: Project Discovery ====================

    #[test]
    fn test_discover_projects_from_directory_structure() {
        use tempfile::tempdir;

        // Create mock Claude projects directory structure
        let temp_dir = tempdir().unwrap();
        let projects_dir = temp_dir.path().join("projects");
        fs::create_dir_all(&projects_dir).unwrap();

        // Create project-a with 2 sessions
        let project_a = projects_dir.join("home-user-project-a");
        fs::create_dir_all(&project_a).unwrap();

        let session_a1 = r#"{"sessionId":"a1","agentId":"agent-1","cwd":"/home/user/project-a","timestamp":"2024-01-15T09:00:00+08:00","message":{"role":"user","content":"First session message"}}"#;
        fs::write(project_a.join("agent-1.jsonl"), session_a1).unwrap();

        let session_a2 = r#"{"sessionId":"a2","agentId":"agent-2","cwd":"/home/user/project-a","timestamp":"2024-01-16T09:00:00+08:00","message":{"role":"user","content":"Second session message"}}"#;
        fs::write(project_a.join("agent-2.jsonl"), session_a2).unwrap();

        // Create project-b with 1 session
        let project_b = projects_dir.join("home-user-project-b");
        fs::create_dir_all(&project_b).unwrap();

        let session_b1 = r#"{"sessionId":"b1","agentId":"agent-3","cwd":"/home/user/project-b","timestamp":"2024-01-17T09:00:00+08:00","message":{"role":"user","content":"Project B session"}}"#;
        fs::write(project_b.join("agent-3.jsonl"), session_b1).unwrap();

        // Scan the projects directory
        let mut projects: Vec<ClaudeProject> = Vec::new();

        for entry in fs::read_dir(&projects_dir).unwrap().flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let dir_name = path.file_name().unwrap().to_str().unwrap().to_string();
            let mut sessions: Vec<ClaudeSession> = Vec::new();

            for file_entry in fs::read_dir(&path).unwrap().flatten() {
                let file_path = file_entry.path();
                if file_path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                    if let Some(session) = parse_session_file(&file_path) {
                        sessions.push(session);
                    }
                }
            }

            if !sessions.is_empty() {
                let project_path = sessions.first()
                    .map(|s| s.cwd.clone())
                    .unwrap_or_else(|| dir_name.replace('-', "/"));
                let project_name = std::path::Path::new(&project_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&dir_name)
                    .to_string();

                projects.push(ClaudeProject {
                    path: project_path,
                    name: project_name,
                    sessions,
                });
            }
        }

        // Verify discovery
        assert_eq!(projects.len(), 2, "Should discover 2 projects");

        let project_a_found = projects.iter().find(|p| p.name == "project-a");
        assert!(project_a_found.is_some(), "Should find project-a");
        assert_eq!(project_a_found.unwrap().sessions.len(), 2, "project-a should have 2 sessions");

        let project_b_found = projects.iter().find(|p| p.name == "project-b");
        assert!(project_b_found.is_some(), "Should find project-b");
        assert_eq!(project_b_found.unwrap().sessions.len(), 1, "project-b should have 1 session");
    }

    #[test]
    fn test_discover_projects_ignores_hidden_directories() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let projects_dir = temp_dir.path().join("projects");
        fs::create_dir_all(&projects_dir).unwrap();

        // Create visible project
        let visible_project = projects_dir.join("visible-project");
        fs::create_dir_all(&visible_project).unwrap();
        let session = r#"{"sessionId":"v1","cwd":"/visible","timestamp":"2024-01-15T09:00:00+08:00","message":{"role":"user","content":"Visible project"}}"#;
        fs::write(visible_project.join("session.jsonl"), session).unwrap();

        // Create hidden project (starts with .)
        let hidden_project = projects_dir.join(".hidden-project");
        fs::create_dir_all(&hidden_project).unwrap();
        let hidden_session = r#"{"sessionId":"h1","cwd":"/hidden","timestamp":"2024-01-15T09:00:00+08:00","message":{"role":"user","content":"Hidden project"}}"#;
        fs::write(hidden_project.join("session.jsonl"), hidden_session).unwrap();

        // Scan (simulating list_claude_sessions logic)
        let mut project_count = 0;
        for entry in fs::read_dir(&projects_dir).unwrap().flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let dir_name = path.file_name().unwrap().to_str().unwrap();
            if dir_name.starts_with('.') {
                continue; // Skip hidden
            }

            project_count += 1;
        }

        assert_eq!(project_count, 1, "Should only count visible project, not hidden");
    }

    #[test]
    fn test_discover_projects_handles_non_jsonl_files() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let project_dir = temp_dir.path().join("projects").join("my-project");
        fs::create_dir_all(&project_dir).unwrap();

        // Create valid session file
        let session = r#"{"sessionId":"s1","cwd":"/project","timestamp":"2024-01-15T09:00:00+08:00","message":{"role":"user","content":"Valid session"}}"#;
        fs::write(project_dir.join("session.jsonl"), session).unwrap();

        // Create non-jsonl files that should be ignored
        fs::write(project_dir.join("readme.txt"), "Some text").unwrap();
        fs::write(project_dir.join("config.json"), "{}").unwrap();
        fs::write(project_dir.join(".hidden"), "hidden file").unwrap();

        // Scan
        let mut session_count = 0;
        for file_entry in fs::read_dir(&project_dir).unwrap().flatten() {
            let file_path = file_entry.path();
            if file_path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                if parse_session_file(&file_path).is_some() {
                    session_count += 1;
                }
            }
        }

        assert_eq!(session_count, 1, "Should only parse .jsonl files");
    }
}
