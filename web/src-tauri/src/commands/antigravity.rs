//! Antigravity (Gemini Code) session commands
//!
//! Tauri commands for Antigravity session operations.
//! Antigravity sessions are stored in ~/.gemini/antigravity/

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use tauri::State;

use recap_core::auth::verify_token;
use recap_core::services::{
    calculate_session_hours, extract_tool_detail, generate_daily_hash, is_meaningful_message,
};

use super::AppState;

// Types

#[derive(Debug, Serialize)]
pub struct AntigravityProject {
    pub path: String,
    pub name: String,
    pub sessions: Vec<AntigravitySession>,
}

#[derive(Debug, Serialize, Clone)]
pub struct AntigravityToolUsage {
    pub tool_name: String,
    pub count: usize,
    pub details: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AntigravitySession {
    pub session_id: String,
    pub task_summary: Option<String>,
    pub walkthrough_summary: Option<String>,
    pub cwd: String,
    pub git_branch: Option<String>,
    pub first_message: Option<String>,
    pub message_count: usize,
    pub first_timestamp: Option<String>,
    pub last_timestamp: Option<String>,
    pub file_path: String,
    pub file_size: u64,
    pub artifact_count: usize,
    pub tool_usage: Vec<AntigravityToolUsage>,
    pub files_modified: Vec<String>,
    pub commands_run: Vec<String>,
    pub user_messages: Vec<String>,
}

/// Message structure for Antigravity session files
#[derive(Debug, Deserialize)]
struct AntigravityMessage {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    timestamp: Option<String>,
    cwd: Option<String>,
    #[serde(rename = "gitBranch")]
    git_branch: Option<String>,
    #[serde(rename = "taskSummary")]
    task_summary: Option<String>,
    #[serde(rename = "walkthroughSummary")]
    walkthrough_summary: Option<String>,
    message: Option<MessageContent>,
    artifact: Option<ArtifactContent>,
}

#[derive(Debug, Deserialize)]
struct MessageContent {
    role: Option<String>,
    content: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ArtifactContent {
    #[serde(rename = "type")]
    artifact_type: Option<String>,
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ToolUseContent {
    #[serde(rename = "type")]
    content_type: Option<String>,
    name: Option<String>,
    input: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct SyncProjectsRequest {
    pub project_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AntigravitySyncResult {
    pub sessions_processed: usize,
    pub sessions_skipped: usize,
    pub work_items_created: usize,
    pub work_items_updated: usize,
}

// Helper functions

pub(crate) fn get_antigravity_home() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".gemini").join("antigravity"))
}

/// Helper to calculate session hours with Option handling
fn session_hours_from_options(first: &Option<String>, last: &Option<String>) -> f64 {
    match (first, last) {
        (Some(start), Some(end)) => calculate_session_hours(start, end),
        _ => 0.5,
    }
}

fn parse_session_file(path: &PathBuf) -> Option<AntigravitySession> {
    let file = fs::File::open(path).ok()?;
    let file_size = file.metadata().ok()?.len();
    let reader = BufReader::new(file);

    let mut session_id: Option<String> = None;
    let mut cwd: Option<String> = None;
    let mut git_branch: Option<String> = None;
    let mut task_summary: Option<String> = None;
    let mut walkthrough_summary: Option<String> = None;
    let mut first_message: Option<String> = None;
    let mut first_timestamp: Option<String> = None;
    let mut last_timestamp: Option<String> = None;
    let mut meaningful_message_count: usize = 0;
    let mut artifact_count: usize = 0;

    let mut tool_counts: HashMap<String, usize> = HashMap::new();
    let mut tool_details: HashMap<String, Vec<String>> = HashMap::new();
    let mut files_modified: Vec<String> = Vec::new();
    let mut commands_run: Vec<String> = Vec::new();
    let mut user_messages: Vec<String> = Vec::new();

    for line in reader.lines().flatten() {
        if let Ok(msg) = serde_json::from_str::<AntigravityMessage>(&line) {
            if session_id.is_none() {
                session_id = msg.session_id;
            }
            if cwd.is_none() {
                cwd = msg.cwd;
            }
            if git_branch.is_none() {
                git_branch = msg.git_branch;
            }
            if task_summary.is_none() {
                task_summary = msg.task_summary;
            }
            if walkthrough_summary.is_none() {
                walkthrough_summary = msg.walkthrough_summary;
            }

            if let Some(ts) = &msg.timestamp {
                if first_timestamp.is_none() {
                    first_timestamp = Some(ts.clone());
                }
                last_timestamp = Some(ts.clone());
            }

            // Count artifacts
            if msg.artifact.is_some() {
                artifact_count += 1;
                if let Some(ref artifact) = msg.artifact {
                    if let Some(ref path) = artifact.path {
                        if !files_modified.contains(path) && files_modified.len() < 20 {
                            files_modified.push(path.clone());
                        }
                    }
                }
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

                if message.role.as_deref() == Some("assistant")
                    || message.role.as_deref() == Some("model")
                {
                    if let Some(content) = &message.content {
                        if let serde_json::Value::Array(arr) = content {
                            for item in arr {
                                if let Ok(tool_use) =
                                    serde_json::from_value::<ToolUseContent>(item.clone())
                                {
                                    let is_tool_use = tool_use.content_type.as_deref()
                                        == Some("tool_use")
                                        || tool_use.content_type.as_deref()
                                            == Some("function_call");

                                    if is_tool_use {
                                        if let Some(tool_name) = &tool_use.name {
                                            *tool_counts.entry(tool_name.clone()).or_insert(0) += 1;

                                            if let Some(input) = &tool_use.input {
                                                let detail = extract_tool_detail(tool_name, input);
                                                if let Some(d) = detail {
                                                    let details = tool_details
                                                        .entry(tool_name.clone())
                                                        .or_default();
                                                    if details.len() < 10 && !details.contains(&d) {
                                                        details.push(d.clone());

                                                        match tool_name.as_str() {
                                                            "Edit" | "Write" | "Read"
                                                            | "edit_file" | "write_file"
                                                            | "read_file" => {
                                                                if !files_modified.contains(&d)
                                                                    && files_modified.len() < 20
                                                                {
                                                                    files_modified.push(d);
                                                                }
                                                            }
                                                            "Bash" | "run_command"
                                                            | "execute_command" => {
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

    // Generate session ID from filename if not found in content
    if session_id.is_none() {
        let filename = path.file_stem()?.to_str()?;
        session_id = Some(filename.to_string());
    }

    Some(AntigravitySession {
        session_id: session_id.unwrap_or_else(|| "unknown".to_string()),
        task_summary,
        walkthrough_summary,
        cwd: cwd.unwrap_or_default(),
        git_branch,
        first_message,
        message_count: meaningful_message_count,
        first_timestamp,
        last_timestamp,
        file_path: path.to_string_lossy().to_string(),
        file_size,
        artifact_count,
        tool_usage: tool_counts
            .into_iter()
            .map(|(name, count)| AntigravityToolUsage {
                tool_name: name.clone(),
                count,
                details: tool_details.remove(&name).unwrap_or_default(),
            })
            .collect(),
        files_modified,
        commands_run,
        user_messages,
    })
}

fn build_session_description(session: &AntigravitySession, hours: f64) -> String {
    let mut desc_parts = vec![
        format!("📁 Project: {}", session.cwd),
        format!(
            "🌿 Branch: {}",
            session.git_branch.as_deref().unwrap_or("N/A")
        ),
        format!(
            "💬 Messages: {} | ⏱️ Duration: {:.1}h",
            session.message_count, hours
        ),
    ];

    if session.artifact_count > 0 {
        desc_parts.push(format!("📦 Artifacts: {}", session.artifact_count));
    }

    if !session.files_modified.is_empty() {
        let files: Vec<_> = session.files_modified.iter().take(10).collect();
        let files_str = files
            .iter()
            .map(|f| format!("  • {}", f))
            .collect::<Vec<_>>()
            .join("\n");
        let more = if session.files_modified.len() > 10 {
            format!(" (+{} more)", session.files_modified.len() - 10)
        } else {
            String::new()
        };
        desc_parts.push(format!(
            "📝 Files Modified ({}{}):\n{}",
            files.len(),
            more,
            files_str
        ));
    }

    if !session.tool_usage.is_empty() {
        let tools_summary: Vec<_> = session
            .tool_usage
            .iter()
            .filter(|t| t.count > 0)
            .map(|t| format!("{}: {}", t.tool_name, t.count))
            .collect();
        if !tools_summary.is_empty() {
            desc_parts.push(format!("🔧 Tools: {}", tools_summary.join(", ")));
        }
    }

    if !session.commands_run.is_empty() {
        let cmds: Vec<_> = session.commands_run.iter().take(5).collect();
        let cmds_str = cmds
            .iter()
            .map(|c| format!("  $ {}", c))
            .collect::<Vec<_>>()
            .join("\n");
        desc_parts.push(format!("💻 Commands:\n{}", cmds_str));
    }

    if let Some(ref summary) = session.task_summary {
        desc_parts.push(format!("📋 Task Summary: {}", summary));
    } else if !session.user_messages.is_empty() {
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

// Commands

/// Check if Antigravity is installed (directory exists)
#[tauri::command]
pub async fn check_antigravity_installed(
    _state: State<'_, AppState>,
    token: String,
) -> Result<bool, String> {
    let _claims = verify_token(&token).map_err(|e| e.to_string())?;

    let antigravity_home = get_antigravity_home();
    Ok(antigravity_home.map(|p| p.exists()).unwrap_or(false))
}

/// List all Antigravity sessions from local machine
#[tauri::command]
pub async fn list_antigravity_sessions(
    _state: State<'_, AppState>,
    token: String,
) -> Result<Vec<AntigravityProject>, String> {
    let _claims = verify_token(&token).map_err(|e| e.to_string())?;

    let antigravity_home =
        get_antigravity_home().ok_or_else(|| "Antigravity home directory not found".to_string())?;

    if !antigravity_home.exists() {
        return Ok(Vec::new());
    }

    let mut projects: Vec<AntigravityProject> = Vec::new();

    // Scan for project directories
    let entries = fs::read_dir(&antigravity_home).map_err(|e| e.to_string())?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Skip hidden directories
        if dir_name.starts_with('.') {
            continue;
        }

        let mut sessions: Vec<AntigravitySession> = Vec::new();

        // Scan for session files (.jsonl)
        if let Ok(files) = fs::read_dir(&path) {
            for file_entry in files.flatten() {
                let file_path = file_entry.path();
                if file_path
                    .extension()
                    .map(|e| e == "jsonl")
                    .unwrap_or(false)
                {
                    if let Some(session) = parse_session_file(&file_path) {
                        sessions.push(session);
                    }
                }
            }
        }

        // Sort sessions by last timestamp (newest first)
        sessions.sort_by(|a, b| {
            b.last_timestamp
                .as_ref()
                .unwrap_or(&String::new())
                .cmp(a.last_timestamp.as_ref().unwrap_or(&String::new()))
        });

        if !sessions.is_empty() {
            let project_path = sessions
                .first()
                .map(|s| s.cwd.clone())
                .unwrap_or_else(|| dir_name.replace('-', "/"));
            let project_name = std::path::Path::new(&project_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&dir_name)
                .to_string();

            projects.push(AntigravityProject {
                path: project_path,
                name: project_name,
                sessions,
            });
        }
    }

    // Sort projects by latest session timestamp
    projects.sort_by(|a, b| {
        let a_latest = a
            .sessions
            .first()
            .and_then(|s| s.last_timestamp.as_ref());
        let b_latest = b
            .sessions
            .first()
            .and_then(|s| s.last_timestamp.as_ref());
        b_latest.cmp(&a_latest)
    });

    Ok(projects)
}

/// Sync selected Antigravity projects - aggregate sessions by project+date
#[tauri::command]
pub async fn sync_antigravity_projects(
    state: State<'_, AppState>,
    token: String,
    request: SyncProjectsRequest,
) -> Result<AntigravitySyncResult, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let antigravity_home =
        get_antigravity_home().ok_or_else(|| "Antigravity home directory not found".to_string())?;

    let mut sessions_processed = 0;
    let mut sessions_skipped = 0;
    let mut work_items_created = 0;
    let mut work_items_updated = 0;

    // For each requested project path, find matching sessions
    for project_path in &request.project_paths {
        // Find the directory that corresponds to this project
        let encoded_path = project_path.replace(['/', '\\'], "-");
        let project_dir = antigravity_home.join(&encoded_path);

        if !project_dir.exists() {
            // Try scanning all directories to find matching project
            if let Ok(entries) = fs::read_dir(&antigravity_home) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }

                    // Scan sessions in this directory
                    if let Ok(files) = fs::read_dir(&path) {
                        for file_entry in files.flatten() {
                            let file_path = file_entry.path();
                            if file_path
                                .extension()
                                .map(|e| e == "jsonl")
                                .unwrap_or(false)
                            {
                                if let Some(session) = parse_session_file(&file_path) {
                                    // Check if this session belongs to the requested project
                                    if session.cwd == *project_path
                                        || session.cwd.ends_with(project_path)
                                    {
                                        match process_session(&db.pool, &claims.sub, &session).await
                                        {
                                            Ok(ProcessResult::Created) => {
                                                sessions_processed += 1;
                                                work_items_created += 1;
                                            }
                                            Ok(ProcessResult::Updated) => {
                                                sessions_processed += 1;
                                                work_items_updated += 1;
                                            }
                                            Ok(ProcessResult::Skipped) => {
                                                sessions_skipped += 1;
                                            }
                                            Err(e) => {
                                                log::error!("Failed to process session: {}", e);
                                                sessions_skipped += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Process sessions from the known directory
            if let Ok(files) = fs::read_dir(&project_dir) {
                for file_entry in files.flatten() {
                    let file_path = file_entry.path();
                    if file_path
                        .extension()
                        .map(|e| e == "jsonl")
                        .unwrap_or(false)
                    {
                        if let Some(session) = parse_session_file(&file_path) {
                            match process_session(&db.pool, &claims.sub, &session).await {
                                Ok(ProcessResult::Created) => {
                                    sessions_processed += 1;
                                    work_items_created += 1;
                                }
                                Ok(ProcessResult::Updated) => {
                                    sessions_processed += 1;
                                    work_items_updated += 1;
                                }
                                Ok(ProcessResult::Skipped) => {
                                    sessions_skipped += 1;
                                }
                                Err(e) => {
                                    log::error!("Failed to process session: {}", e);
                                    sessions_skipped += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(AntigravitySyncResult {
        sessions_processed,
        sessions_skipped,
        work_items_created,
        work_items_updated,
    })
}

enum ProcessResult {
    Created,
    Updated,
    Skipped,
}

async fn process_session(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    session: &AntigravitySession,
) -> Result<ProcessResult, String> {
    if session.message_count == 0 {
        return Ok(ProcessResult::Skipped);
    }

    let hours = session_hours_from_options(&session.first_timestamp, &session.last_timestamp);

    let date = session
        .first_timestamp
        .as_ref()
        .and_then(|ts| ts.split('T').next())
        .unwrap_or("2026-01-01");

    let content_hash = generate_daily_hash(user_id, &session.cwd, date);

    // Check if already exists
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT id FROM work_items WHERE content_hash = ? AND user_id = ?")
            .bind(&content_hash)
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;

    if existing.is_some() {
        return Ok(ProcessResult::Skipped);
    }

    let project_name = std::path::Path::new(&session.cwd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown");

    let title = if let Some(ref summary) = session.task_summary {
        format!("[{}] {}", project_name, summary.chars().take(80).collect::<String>())
    } else if let Some(ref msg) = session.first_message {
        let truncated = if msg.len() > 80 {
            format!("{}...", &msg.chars().take(80).collect::<String>())
        } else {
            msg.clone()
        };
        format!("[{}] {}", project_name, truncated)
    } else {
        format!("[{}] Gemini Code session", project_name)
    };

    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();
    let description = build_session_description(session, hours);

    sqlx::query(
        r#"INSERT INTO work_items
        (id, user_id, source, source_id, title, description, hours, date, content_hash, hours_source, hours_estimated, created_at, updated_at)
        VALUES (?, ?, 'antigravity', ?, ?, ?, ?, ?, ?, 'session', ?, ?, ?)"#
    )
    .bind(&id)
    .bind(user_id)
    .bind(&session.session_id)
    .bind(&title)
    .bind(&description)
    .bind(hours)
    .bind(date)
    .bind(&content_hash)
    .bind(hours)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(ProcessResult::Created)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // ==================== get_antigravity_home Tests ====================

    #[test]
    fn test_get_antigravity_home() {
        let result = get_antigravity_home();
        if let Some(path) = result {
            assert!(path.to_string_lossy().contains(".gemini"));
            assert!(path.to_string_lossy().contains("antigravity"));
        }
    }

    // ==================== session_hours_from_options Tests ====================

    #[test]
    fn test_session_hours_from_options_valid() {
        let first = Some("2024-01-15T09:00:00+08:00".to_string());
        let last = Some("2024-01-15T11:00:00+08:00".to_string());
        let hours = session_hours_from_options(&first, &last);
        assert!((hours - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_session_hours_from_options_both_none() {
        let hours = session_hours_from_options(&None, &None);
        assert!((hours - 0.5).abs() < 0.01);
    }

    // ==================== parse_session_file Tests ====================

    fn create_mock_antigravity_session() -> String {
        let lines = vec![
            r#"{"sessionId":"ag-sess-123","cwd":"/home/user/myproject","gitBranch":"main","taskSummary":"Implement user auth","timestamp":"2024-01-15T09:00:00+08:00","message":{"role":"user","content":"Help me add login functionality"}}"#,
            r#"{"sessionId":"ag-sess-123","timestamp":"2024-01-15T09:10:00+08:00","message":{"role":"model","content":[{"type":"function_call","name":"read_file","input":{"file_path":"/home/user/myproject/src/auth.rs"}}]}}"#,
            r#"{"sessionId":"ag-sess-123","timestamp":"2024-01-15T09:30:00+08:00","artifact":{"type":"file","path":"/home/user/myproject/src/login.rs"}}"#,
            r#"{"sessionId":"ag-sess-123","timestamp":"2024-01-15T10:00:00+08:00","message":{"role":"user","content":"Now add logout as well"}}"#,
        ];
        lines.join("\n")
    }

    #[test]
    fn test_parse_session_file_discovers_metadata() {
        let content = create_mock_antigravity_session();
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let path = file.path().to_path_buf();

        let session = parse_session_file(&path);
        assert!(session.is_some());

        let session = session.unwrap();
        assert_eq!(session.session_id, "ag-sess-123");
        assert_eq!(session.cwd, "/home/user/myproject");
        assert_eq!(session.git_branch, Some("main".to_string()));
        assert_eq!(session.task_summary, Some("Implement user auth".to_string()));
    }

    #[test]
    fn test_parse_session_file_discovers_timestamps() {
        let content = create_mock_antigravity_session();
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let path = file.path().to_path_buf();

        let session = parse_session_file(&path).unwrap();

        assert_eq!(
            session.first_timestamp,
            Some("2024-01-15T09:00:00+08:00".to_string())
        );
        assert_eq!(
            session.last_timestamp,
            Some("2024-01-15T10:00:00+08:00".to_string())
        );
    }

    #[test]
    fn test_parse_session_file_counts_artifacts() {
        let content = create_mock_antigravity_session();
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let path = file.path().to_path_buf();

        let session = parse_session_file(&path).unwrap();

        assert_eq!(session.artifact_count, 1);
        assert!(session.files_modified.contains(&"/home/user/myproject/src/login.rs".to_string()));
    }

    #[test]
    fn test_parse_session_file_discovers_user_messages() {
        let content = create_mock_antigravity_session();
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let path = file.path().to_path_buf();

        let session = parse_session_file(&path).unwrap();

        assert_eq!(session.message_count, 2);
        assert!(session
            .first_message
            .as_ref()
            .unwrap()
            .contains("login functionality"));
    }

    #[test]
    fn test_parse_session_file_discovers_tool_usage() {
        let content = create_mock_antigravity_session();
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let path = file.path().to_path_buf();

        let session = parse_session_file(&path).unwrap();

        let tool_names: Vec<&str> = session
            .tool_usage
            .iter()
            .map(|t| t.tool_name.as_str())
            .collect();
        assert!(tool_names.contains(&"read_file"));
    }

    #[test]
    fn test_parse_session_file_handles_empty_file() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"").unwrap();
        let path = file.path().to_path_buf();

        let session = parse_session_file(&path);
        assert!(session.is_some());
    }

    #[test]
    fn test_parse_session_file_handles_invalid_json() {
        let content = "not valid json\nalso not json\n";
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let path = file.path().to_path_buf();

        let session = parse_session_file(&path);
        assert!(session.is_some());
    }

    #[test]
    fn test_parse_session_file_extracts_session_id_from_filename() {
        let content = r#"{"timestamp":"2024-01-15T09:00:00+08:00","message":{"role":"user","content":"Test message here"}}"#;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("my-session-abc123.jsonl");
        fs::write(&file_path, content).unwrap();

        let session = parse_session_file(&file_path).unwrap();
        assert_eq!(session.session_id, "my-session-abc123");
    }

    // ==================== build_session_description Tests ====================

    fn create_test_session() -> AntigravitySession {
        AntigravitySession {
            session_id: "test-session-123".to_string(),
            task_summary: Some("Implement login feature".to_string()),
            walkthrough_summary: None,
            file_path: "/tmp/test.jsonl".to_string(),
            cwd: "/home/user/project".to_string(),
            git_branch: Some("main".to_string()),
            first_timestamp: Some("2024-01-15T09:00:00+08:00".to_string()),
            last_timestamp: Some("2024-01-15T11:00:00+08:00".to_string()),
            first_message: Some("Help me add login".to_string()),
            message_count: 10,
            file_size: 1024,
            artifact_count: 3,
            tool_usage: vec![],
            files_modified: vec![],
            commands_run: vec![],
            user_messages: vec!["Help me add login".to_string()],
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
        assert!(desc.contains("📦 Artifacts: 3"));
    }

    #[test]
    fn test_build_session_description_with_task_summary() {
        let session = create_test_session();
        let desc = build_session_description(&session, 1.5);

        assert!(desc.contains("📋 Task Summary: Implement login feature"));
    }

    #[test]
    fn test_build_session_description_with_files() {
        let mut session = create_test_session();
        session.files_modified = vec!["src/main.rs".to_string(), "src/lib.rs".to_string()];

        let desc = build_session_description(&session, 1.5);

        assert!(desc.contains("📝 Files Modified"));
        assert!(desc.contains("src/main.rs"));
        assert!(desc.contains("src/lib.rs"));
    }

    #[test]
    fn test_build_session_description_no_branch() {
        let mut session = create_test_session();
        session.git_branch = None;

        let desc = build_session_description(&session, 1.0);

        assert!(desc.contains("🌿 Branch: N/A"));
    }

    // ==================== Integration Test: Project Discovery ====================

    #[test]
    fn test_discover_antigravity_projects_from_directory() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let antigravity_dir = temp_dir.path();

        // Create project-a with 2 sessions
        let project_a = antigravity_dir.join("home-user-project-a");
        fs::create_dir_all(&project_a).unwrap();

        let session_a1 = r#"{"sessionId":"a1","cwd":"/home/user/project-a","timestamp":"2024-01-15T09:00:00+08:00","message":{"role":"user","content":"First session message"}}"#;
        fs::write(project_a.join("session-1.jsonl"), session_a1).unwrap();

        let session_a2 = r#"{"sessionId":"a2","cwd":"/home/user/project-a","timestamp":"2024-01-16T09:00:00+08:00","message":{"role":"user","content":"Second session message"}}"#;
        fs::write(project_a.join("session-2.jsonl"), session_a2).unwrap();

        // Create project-b with 1 session
        let project_b = antigravity_dir.join("home-user-project-b");
        fs::create_dir_all(&project_b).unwrap();

        let session_b1 = r#"{"sessionId":"b1","cwd":"/home/user/project-b","timestamp":"2024-01-17T09:00:00+08:00","message":{"role":"user","content":"Project B session"}}"#;
        fs::write(project_b.join("session-1.jsonl"), session_b1).unwrap();

        // Scan the directory
        let mut projects: Vec<AntigravityProject> = Vec::new();

        for entry in fs::read_dir(&antigravity_dir).unwrap().flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let dir_name = path.file_name().unwrap().to_str().unwrap().to_string();
            let mut sessions: Vec<AntigravitySession> = Vec::new();

            for file_entry in fs::read_dir(&path).unwrap().flatten() {
                let file_path = file_entry.path();
                if file_path
                    .extension()
                    .map(|e| e == "jsonl")
                    .unwrap_or(false)
                {
                    if let Some(session) = parse_session_file(&file_path) {
                        sessions.push(session);
                    }
                }
            }

            if !sessions.is_empty() {
                let project_path = sessions
                    .first()
                    .map(|s| s.cwd.clone())
                    .unwrap_or_else(|| dir_name.replace('-', "/"));
                let project_name = std::path::Path::new(&project_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&dir_name)
                    .to_string();

                projects.push(AntigravityProject {
                    path: project_path,
                    name: project_name,
                    sessions,
                });
            }
        }

        assert_eq!(projects.len(), 2, "Should discover 2 projects");

        let project_a_found = projects.iter().find(|p| p.name == "project-a");
        assert!(project_a_found.is_some(), "Should find project-a");
        assert_eq!(
            project_a_found.unwrap().sessions.len(),
            2,
            "project-a should have 2 sessions"
        );

        let project_b_found = projects.iter().find(|p| p.name == "project-b");
        assert!(project_b_found.is_some(), "Should find project-b");
        assert_eq!(
            project_b_found.unwrap().sessions.len(),
            1,
            "project-b should have 1 session"
        );
    }

    #[test]
    fn test_discover_projects_ignores_hidden_directories() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let antigravity_dir = temp_dir.path();

        // Create visible project
        let visible_project = antigravity_dir.join("visible-project");
        fs::create_dir_all(&visible_project).unwrap();
        let session = r#"{"sessionId":"v1","cwd":"/visible","timestamp":"2024-01-15T09:00:00+08:00","message":{"role":"user","content":"Visible project"}}"#;
        fs::write(visible_project.join("session.jsonl"), session).unwrap();

        // Create hidden project
        let hidden_project = antigravity_dir.join(".hidden-project");
        fs::create_dir_all(&hidden_project).unwrap();
        let hidden_session = r#"{"sessionId":"h1","cwd":"/hidden","timestamp":"2024-01-15T09:00:00+08:00","message":{"role":"user","content":"Hidden project"}}"#;
        fs::write(hidden_project.join("session.jsonl"), hidden_session).unwrap();

        // Scan (simulating list_antigravity_sessions logic)
        let mut project_count = 0;
        for entry in fs::read_dir(&antigravity_dir).unwrap().flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let dir_name = path.file_name().unwrap().to_str().unwrap();
            if dir_name.starts_with('.') {
                continue;
            }

            project_count += 1;
        }

        assert_eq!(project_count, 1, "Should only count visible project");
    }
}
