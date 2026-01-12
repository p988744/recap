//! Sync Service - Background synchronization for work items
//!
//! This service handles automatic synchronization of Claude Code sessions
//! and other data sources. It provides:
//! - Auto-sync on app startup
//! - Periodic background sync
//! - Sync status tracking

use chrono::Utc;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use uuid::Uuid;

use crate::models::{SyncStatus, SyncStatusResponse};
use super::session_parser::{generate_daily_hash, parse_session_full, ParsedSession, ToolUsage};
use super::worklog::calculate_session_hours;

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

// Shared functions from session_parser: generate_daily_hash, parse_session_full, ToolUsage
// Shared from worklog: calculate_session_hours

/// Git commit info
#[derive(Debug, Clone)]
struct GitCommit {
    hash: String,
    message: String,
}

/// Session summary for aggregation
#[derive(Debug, Clone)]
struct SessionSummary {
    tool_usage: Vec<ToolUsage>,
    files_modified: Vec<String>,
    first_message: Option<String>,
}

/// Daily work item data (aggregated from multiple sessions)
#[derive(Debug)]
struct DailyWorkItem {
    sessions: Vec<SessionSummary>,
    total_hours: f64,
    git_commits: Vec<GitCommit>,
}

/// Sync result for Claude projects
#[derive(Debug, serde::Serialize)]
pub struct ClaudeSyncResult {
    pub sessions_processed: usize,
    pub sessions_skipped: usize,
    pub work_items_created: usize,
    pub work_items_updated: usize,
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

// calculate_session_hours is imported from super::worklog
// is_meaningful_message, extract_tool_detail, parse_session_full are imported from session_parser

/// Helper to calculate session hours with Option handling
fn session_hours_from_options(first: &Option<String>, last: &Option<String>) -> f64 {
    match (first, last) {
        (Some(start), Some(end)) => calculate_session_hours(start, end),
        _ => 0.5,
    }
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

                if let Some(session) = parse_session_full(&file_path) {
                    if session.message_count == 0 {
                        sessions_skipped += 1;
                        continue;
                    }

                    let hours =
                        session_hours_from_options(&session.first_timestamp, &session.last_timestamp);
                    // Note: calculate_session_hours already enforces minimum 0.1h (6 min)
                    // Sessions with valid timestamps will always have hours >= 0.1

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
            .map(|(s, _h)| SessionSummary {
                tool_usage: s.tool_usage.clone(),
                files_modified: s.files_modified.clone(),
                first_message: s.first_message.clone(),
            })
            .collect();

        let git_commits = get_git_commits_for_date(&project_path, &date);

        // Cross-source deduplication: filter out commits already synced from GitLab
        let git_commits = if !git_commits.is_empty() {
            let commit_hashes: Vec<&str> = git_commits.iter().map(|c| c.hash.as_str()).collect();
            let placeholders = commit_hashes.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let query = format!(
                "SELECT commit_hash FROM work_items WHERE source = 'gitlab' AND commit_hash IN ({}) AND user_id = ?",
                placeholders
            );

            let mut q = sqlx::query_scalar::<_, String>(&query);
            for hash in &commit_hashes {
                q = q.bind(*hash);
            }
            q = q.bind(user_id);

            let existing_hashes: Vec<String> = q.fetch_all(pool).await.unwrap_or_default();

            // Filter out commits that already exist as GitLab work items
            git_commits
                .into_iter()
                .filter(|c| !existing_hashes.contains(&c.hash))
                .collect()
        } else {
            git_commits
        };

        let daily = DailyWorkItem {
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

        // Extract first commit hash for bi-directional dedup tracking
        let primary_commit_hash: Option<String> = daily.git_commits.first().map(|c| c.hash.clone());

        // Check if work item exists, also fetch hours_source to preserve user edits
        let existing: Option<(String, Option<String>)> =
            sqlx::query_as("SELECT id, hours_source FROM work_items WHERE content_hash = ? AND user_id = ?")
                .bind(&content_hash)
                .bind(user_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?;

        if let Some((existing_id, existing_hours_source)) = existing {
            // Preserve user-modified hours - only update hours if not manually set
            let user_modified = existing_hours_source.as_deref() == Some("user_modified");

            if user_modified {
                // User has manually edited hours - only update description/title, keep hours
                sqlx::query(
                    r#"UPDATE work_items
                    SET title = ?, description = ?, hours_estimated = ?, commit_hash = COALESCE(commit_hash, ?), updated_at = ?
                    WHERE id = ?"#,
                )
                .bind(&title)
                .bind(&description)
                .bind(total_hours)
                .bind(&primary_commit_hash)
                .bind(now)
                .bind(&existing_id)
                .execute(pool)
                .await
                .map_err(|e| e.to_string())?;
            } else {
                // Safe to update all fields including hours
                sqlx::query(
                    r#"UPDATE work_items
                    SET title = ?, description = ?, hours = ?, hours_source = 'session', hours_estimated = ?, commit_hash = COALESCE(commit_hash, ?), updated_at = ?
                    WHERE id = ?"#,
                )
                .bind(&title)
                .bind(&description)
                .bind(total_hours)
                .bind(total_hours)
                .bind(&primary_commit_hash)
                .bind(now)
                .bind(&existing_id)
                .execute(pool)
                .await
                .map_err(|e| e.to_string())?;
            }

            updated += 1;
        } else {
            let id = Uuid::new_v4().to_string();
            sqlx::query(
                r#"INSERT INTO work_items
                (id, user_id, source, title, description, hours, date, content_hash, hours_source, hours_estimated, commit_hash, created_at, updated_at)
                VALUES (?, ?, 'claude_code', ?, ?, ?, ?, ?, 'session', ?, ?, ?, ?)"#,
            )
            .bind(&id)
            .bind(user_id)
            .bind(&title)
            .bind(&description)
            .bind(total_hours)
            .bind(&date)
            .bind(&content_hash)
            .bind(total_hours)
            .bind(&primary_commit_hash)
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
