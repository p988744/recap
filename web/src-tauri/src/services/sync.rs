//! Sync Service - Background synchronization for work items
//!
//! This service handles automatic synchronization of Claude Code sessions
//! and other data sources. It provides:
//! - Auto-sync on app startup
//! - Periodic background sync
//! - Sync status tracking

use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;
use uuid::Uuid;

use crate::models::{SyncStatus, SyncStatusResponse};

/// Sync Service for managing background synchronization
pub struct SyncService {
    pool: SqlitePool,
}

impl SyncService {
    /// Create a new sync service
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get all sync statuses for a user
    pub async fn get_sync_statuses(&self, user_id: &str) -> Result<Vec<SyncStatusResponse>, String> {
        let statuses: Vec<SyncStatus> = sqlx::query_as(
            "SELECT * FROM sync_status WHERE user_id = ? ORDER BY source, source_path"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(statuses.into_iter().map(SyncStatusResponse::from).collect())
    }

    /// Get or create sync status for a specific source
    pub async fn get_or_create_status(
        &self,
        user_id: &str,
        source: &str,
        source_path: Option<&str>,
    ) -> Result<SyncStatus, String> {
        // Try to find existing status
        let existing: Option<SyncStatus> = sqlx::query_as(
            "SELECT * FROM sync_status WHERE user_id = ? AND source = ? AND (source_path = ? OR (source_path IS NULL AND ? IS NULL))"
        )
        .bind(user_id)
        .bind(source)
        .bind(source_path)
        .bind(source_path)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        if let Some(status) = existing {
            return Ok(status);
        }

        // Create new status
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO sync_status (id, user_id, source, source_path, status, created_at, updated_at)
            VALUES (?, ?, ?, ?, 'idle', ?, ?)
            "#
        )
        .bind(&id)
        .bind(user_id)
        .bind(source)
        .bind(source_path)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let status: SyncStatus = sqlx::query_as("SELECT * FROM sync_status WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(status)
    }

    /// Update sync status to 'syncing'
    pub async fn mark_syncing(&self, status_id: &str) -> Result<(), String> {
        let now = Utc::now();
        sqlx::query(
            "UPDATE sync_status SET status = 'syncing', error_message = NULL, updated_at = ? WHERE id = ?"
        )
        .bind(now)
        .bind(status_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Update sync status to 'success' with item count
    pub async fn mark_success(&self, status_id: &str, item_count: i32) -> Result<(), String> {
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE sync_status
            SET status = 'success',
                last_sync_at = ?,
                last_item_count = ?,
                error_message = NULL,
                updated_at = ?
            WHERE id = ?
            "#
        )
        .bind(now)
        .bind(item_count)
        .bind(now)
        .bind(status_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Update sync status to 'error' with message
    pub async fn mark_error(&self, status_id: &str, error: &str) -> Result<(), String> {
        let now = Utc::now();
        sqlx::query(
            "UPDATE sync_status SET status = 'error', error_message = ?, updated_at = ? WHERE id = ?"
        )
        .bind(error)
        .bind(now)
        .bind(status_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Update sync status to 'idle'
    pub async fn mark_idle(&self, status_id: &str) -> Result<(), String> {
        let now = Utc::now();
        sqlx::query(
            "UPDATE sync_status SET status = 'idle', updated_at = ? WHERE id = ?"
        )
        .bind(now)
        .bind(status_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get Claude projects directory path
    pub fn get_claude_projects_dir() -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        let claude_dir = home.join(".claude").join("projects");
        if claude_dir.exists() {
            Some(claude_dir)
        } else {
            None
        }
    }

    /// List all Claude project directories
    pub fn list_claude_projects() -> Vec<PathBuf> {
        let mut projects = Vec::new();

        if let Some(claude_dir) = Self::get_claude_projects_dir() {
            if let Ok(entries) = std::fs::read_dir(&claude_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        // Check if it has session files
                        if let Ok(files) = std::fs::read_dir(&path) {
                            let has_sessions = files
                                .flatten()
                                .any(|f| f.path().extension().map(|e| e == "jsonl").unwrap_or(false));
                            if has_sessions {
                                projects.push(path);
                            }
                        }
                    }
                }
            }
        }

        projects
    }
}

/// Create a new sync service instance
pub fn create_sync_service(pool: SqlitePool) -> SyncService {
    SyncService::new(pool)
}

// ============ Claude Sync Logic ============

/// Generate content hash for deduplication (user + project + date = unique work item)
fn generate_daily_hash(user_id: &str, project: &str, date: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(user_id.as_bytes());
    hasher.update(project.as_bytes());
    hasher.update(date.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Git commit info
#[derive(Debug, Clone)]
struct GitCommit {
    hash: String,
    message: String,
}

/// Session summary for aggregation
#[derive(Debug, Clone)]
struct SessionSummary {
    hours: f64,
    tool_usage: Vec<ToolUsage>,
    files_modified: Vec<String>,
    first_message: Option<String>,
}

/// Tool usage tracking
#[derive(Debug, Clone)]
struct ToolUsage {
    tool_name: String,
    count: usize,
}

/// Daily work item data (aggregated from multiple sessions)
#[derive(Debug)]
struct DailyWorkItem {
    project_path: String,
    project_name: String,
    date: String,
    sessions: Vec<SessionSummary>,
    total_hours: f64,
    git_commits: Vec<GitCommit>,
}

/// Parsed Claude session data
#[derive(Debug)]
struct ParsedSession {
    cwd: String,
    first_timestamp: Option<String>,
    last_timestamp: Option<String>,
    message_count: usize,
    tool_usage: Vec<ToolUsage>,
    files_modified: Vec<String>,
    first_message: Option<String>,
}

/// Sync result for Claude projects
#[derive(Debug)]
pub struct ClaudeSyncResult {
    pub sessions_processed: usize,
    pub sessions_skipped: usize,
    pub work_items_created: usize,
    pub work_items_updated: usize,
}

/// Session message for parsing JSONL
#[derive(Debug, serde::Deserialize)]
struct SessionMessage {
    cwd: Option<String>,
    timestamp: Option<String>,
    #[serde(rename = "type")]
    msg_type: Option<String>,
    message: Option<MessageContent>,
}

#[derive(Debug, serde::Deserialize)]
struct MessageContent {
    role: Option<String>,
    content: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
struct ToolUseContent {
    #[serde(rename = "type")]
    content_type: Option<String>,
    name: Option<String>,
    input: Option<serde_json::Value>,
}

/// Get git log for a project directory on a specific date
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

/// Calculate session duration in hours from timestamps
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

/// Check if a message is meaningful
fn is_meaningful_message(content: &str) -> bool {
    let trimmed = content.trim().to_lowercase();
    if trimmed == "warmup" || trimmed.starts_with("warmup") {
        return false;
    }
    if trimmed.starts_with("<command-") || trimmed.starts_with("<system-") {
        return false;
    }
    trimmed.len() >= 10
}

/// Extract tool detail from input
fn extract_tool_detail(tool_name: &str, input: &serde_json::Value) -> Option<String> {
    match tool_name {
        "Edit" | "Write" | "Read" => input.get("file_path").and_then(|v| v.as_str()).map(|p| {
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() > 3 {
                format!(".../{}", parts[parts.len() - 3..].join("/"))
            } else {
                p.to_string()
            }
        }),
        "Bash" => input.get("command").and_then(|v| v.as_str()).map(|c| {
            let truncated: String = c.chars().take(60).collect();
            if c.len() > 60 {
                format!("{}...", truncated)
            } else {
                truncated
            }
        }),
        _ => None,
    }
}

/// Parse a session .jsonl file
fn parse_session_file(path: &PathBuf) -> Option<ParsedSession> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut cwd: Option<String> = None;
    let mut first_message: Option<String> = None;
    let mut first_timestamp: Option<String> = None;
    let mut last_timestamp: Option<String> = None;
    let mut meaningful_message_count: usize = 0;

    let mut tool_counts: HashMap<String, usize> = HashMap::new();
    let mut files_modified: Vec<String> = Vec::new();

    for line in reader.lines().flatten() {
        if let Ok(msg) = serde_json::from_str::<SessionMessage>(&line) {
            if cwd.is_none() {
                cwd = msg.cwd;
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
                            }
                        }
                    }
                }

                if message.role.as_deref() == Some("assistant") {
                    if let Some(content) = &message.content {
                        if let serde_json::Value::Array(arr) = content {
                            for item in arr {
                                if let Ok(tool_use) =
                                    serde_json::from_value::<ToolUseContent>(item.clone())
                                {
                                    if tool_use.content_type.as_deref() == Some("tool_use") {
                                        if let Some(tool_name) = &tool_use.name {
                                            *tool_counts.entry(tool_name.clone()).or_insert(0) += 1;

                                            if let Some(input) = &tool_use.input {
                                                if let Some(d) =
                                                    extract_tool_detail(tool_name, input)
                                                {
                                                    if matches!(
                                                        tool_name.as_str(),
                                                        "Edit" | "Write" | "Read"
                                                    ) && !files_modified.contains(&d)
                                                        && files_modified.len() < 20
                                                    {
                                                        files_modified.push(d);
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
            tool_name: name,
            count,
        })
        .collect();

    Some(ParsedSession {
        cwd: cwd.unwrap_or_default(),
        first_timestamp,
        last_timestamp,
        message_count: meaningful_message_count,
        tool_usage,
        files_modified,
        first_message,
    })
}

/// Build daily work description
fn build_daily_description(daily: &DailyWorkItem) -> String {
    let mut parts = vec![];

    if !daily.git_commits.is_empty() {
        let commits_str: Vec<String> = daily
            .git_commits
            .iter()
            .take(15)
            .map(|c| format!("  • {} - {}", c.hash, c.message))
            .collect();
        let more = if daily.git_commits.len() > 15 {
            format!(" (+{} more)", daily.git_commits.len() - 15)
        } else {
            String::new()
        };
        parts.push(format!(
            "🔀 Git Commits ({}{})\n{}",
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
        let session_titles: Vec<String> = daily
            .sessions
            .iter()
            .filter_map(|s| s.first_message.as_ref())
            .take(5)
            .map(|m| {
                let truncated: String = m.chars().take(60).collect();
                if m.len() > 60 {
                    format!("  • {}...", truncated)
                } else {
                    format!("  • {}", truncated)
                }
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
        let tools_str: Vec<_> = tools
            .iter()
            .take(10)
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect();
        parts.push(format!("🔧 使用工具: {}", tools_str.join(", ")));
    }

    let mut all_files: Vec<String> = daily
        .sessions
        .iter()
        .flat_map(|s| s.files_modified.clone())
        .collect();
    all_files.sort();
    all_files.dedup();
    if !all_files.is_empty() {
        let count = all_files.len();
        let display_files: Vec<_> = all_files.iter().take(8).collect();
        let files_str = display_files
            .iter()
            .map(|f| format!("  • {}", f))
            .collect::<Vec<_>>()
            .join("\n");
        let more = if count > 8 {
            format!(" (+{} more)", count - 8)
        } else {
            String::new()
        };
        parts.push(format!(
            "📁 修改檔案 ({}{})\n{}",
            display_files.len(),
            more,
            files_str
        ));
    }

    parts.join("\n\n")
}

/// Sync Claude projects to work items
pub async fn sync_claude_projects(
    pool: &SqlitePool,
    user_id: &str,
    project_paths: &[String],
) -> Result<ClaudeSyncResult, String> {
    let claude_home = dirs::home_dir()
        .map(|h| h.join(".claude"))
        .ok_or("Claude home directory not found")?;

    let projects_dir = claude_home.join("projects");

    // Collect all sessions and group by (project_path, date)
    let mut daily_groups: HashMap<(String, String), Vec<(ParsedSession, f64)>> = HashMap::new();
    let mut sessions_processed = 0;
    let mut sessions_skipped = 0;

    for project_path in project_paths {
        let dir_name = project_path.replace('/', "-");
        let project_dir = projects_dir.join(&dir_name);

        if !project_dir.exists() || !project_dir.is_dir() {
            continue;
        }

        if let Ok(files) = fs::read_dir(&project_dir) {
            for file_entry in files.flatten() {
                let file_path = file_entry.path();
                if !file_path
                    .extension()
                    .map(|e| e == "jsonl")
                    .unwrap_or(false)
                {
                    continue;
                }

                if let Some(session) = parse_session_file(&file_path) {
                    if session.message_count == 0 {
                        sessions_skipped += 1;
                        continue;
                    }

                    let hours =
                        calculate_session_hours(&session.first_timestamp, &session.last_timestamp);
                    if hours < 0.08 {
                        sessions_skipped += 1;
                        continue;
                    }

                    let date = session
                        .first_timestamp
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

    // For each (project, date) group, create or update work item
    let mut created = 0;
    let mut updated = 0;
    let now = Utc::now();

    for ((project_path, date), sessions) in daily_groups {
        let project_name = project_path
            .split('/')
            .last()
            .unwrap_or("unknown")
            .to_string();

        let total_hours: f64 = sessions.iter().map(|(_, h)| h).sum();
        let session_summaries: Vec<SessionSummary> = sessions
            .iter()
            .map(|(s, h)| SessionSummary {
                hours: *h,
                tool_usage: s.tool_usage.clone(),
                files_modified: s.files_modified.clone(),
                first_message: s.first_message.clone(),
            })
            .collect();

        let git_commits = get_git_commits_for_date(&project_path, &date);

        let daily = DailyWorkItem {
            project_path: project_path.clone(),
            project_name: project_name.clone(),
            date: date.clone(),
            sessions: session_summaries,
            total_hours,
            git_commits,
        };

        let content_hash = generate_daily_hash(user_id, &project_path, &date);

        let commit_count = daily.git_commits.len();
        let title = if commit_count > 0 {
            let first_commit_msg = daily
                .git_commits
                .first()
                .map(|c| {
                    let msg: String = c.message.chars().take(40).collect();
                    if c.message.len() > 40 {
                        format!("{}...", msg)
                    } else {
                        msg
                    }
                })
                .unwrap_or_default();
            format!(
                "[{}] {} commits: {}",
                project_name, commit_count, first_commit_msg
            )
        } else {
            format!(
                "[{}] {} 工作紀錄 ({} sessions)",
                project_name,
                date,
                daily.sessions.len()
            )
        };
        let description = build_daily_description(&daily);

        // Check if work item exists
        let existing: Option<(String,)> =
            sqlx::query_as("SELECT id FROM work_items WHERE content_hash = ? AND user_id = ?")
                .bind(&content_hash)
                .bind(user_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?;

        if let Some((existing_id,)) = existing {
            sqlx::query(
                r#"UPDATE work_items
                SET title = ?, description = ?, hours = ?, updated_at = ?
                WHERE id = ?"#,
            )
            .bind(&title)
            .bind(&description)
            .bind(total_hours)
            .bind(now)
            .bind(&existing_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;

            updated += 1;
        } else {
            let id = Uuid::new_v4().to_string();
            sqlx::query(
                r#"INSERT INTO work_items
                (id, user_id, source, title, description, hours, date, content_hash, created_at, updated_at)
                VALUES (?, ?, 'claude_code', ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&id)
            .bind(user_id)
            .bind(&title)
            .bind(&description)
            .bind(total_hours)
            .bind(&date)
            .bind(&content_hash)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;

            created += 1;
        }
    }

    Ok(ClaudeSyncResult {
        sessions_processed,
        sessions_skipped,
        work_items_created: created,
        work_items_updated: updated,
    })
}
