//! Sync Service - Background synchronization for work items
//!
//! This service handles automatic synchronization of Claude Code sessions
//! and other data sources. It provides:
//! - Auto-sync on app startup
//! - Periodic background sync
//! - Sync status tracking

use chrono::Utc;
use sqlx::SqlitePool;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

use crate::models::{SyncStatus, SyncStatusResponse};
use super::session_parser::{parse_session_full, ParsedSession};
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

// Shared functions from session_parser: parse_session_full, ParsedSession
// Shared from worklog: calculate_session_hours

/// Sync result for Claude projects
#[derive(Debug, serde::Serialize)]
pub struct ClaudeSyncResult {
    pub sessions_processed: usize,
    pub sessions_skipped: usize,
    pub work_items_created: usize,
    pub work_items_updated: usize,
}

/// Helper to calculate session hours with Option handling
fn session_hours_from_options(first: &Option<String>, last: &Option<String>) -> f64 {
    match (first, last) {
        (Some(start), Some(end)) => calculate_session_hours(start, end),
        _ => 0.5,
    }
}

/// Build description for a single session work item
fn build_session_description(session: &ParsedSession) -> String {
    let mut parts = vec![];

    // Tool usage summary
    if !session.tool_usage.is_empty() {
        let mut tools: Vec<_> = session.tool_usage.iter().collect();
        tools.sort_by(|a, b| b.count.cmp(&a.count));
        let tools_str: Vec<_> = tools
            .iter()
            .take(8)
            .map(|t| format!("{}: {}", t.tool_name, t.count))
            .collect();
        parts.push(format!("🔧 Tools: {}", tools_str.join(", ")));
    }

    // Files modified
    if !session.files_modified.is_empty() {
        let count = session.files_modified.len();
        let display_files: Vec<_> = session.files_modified.iter().take(5).collect();
        let files_str = display_files
            .iter()
            .map(|f| format!("  • {}", f))
            .collect::<Vec<_>>()
            .join("\n");
        let more = if count > 5 {
            format!(" (+{} more)", count - 5)
        } else {
            String::new()
        };
        parts.push(format!(
            "📁 Modified files ({}{})\n{}",
            display_files.len(),
            more,
            files_str
        ));
    }

    // Project path info
    parts.push(format!("📂 Project: {}", session.cwd));

    parts.join("\n\n")
}

/// Generate a unique hash for session-based deduplication
fn generate_session_hash(user_id: &str, session_id: &str, project_path: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    format!("session:{}:{}:{}", user_id, project_path, session_id).hash(&mut hasher);
    format!("sess_{:x}", hasher.finish())
}

/// Sync Claude projects to work items
/// NEW: Stores individual sessions (not aggregated by date) for Timeline consistency
pub async fn sync_claude_projects(
    pool: &SqlitePool,
    user_id: &str,
    project_paths: &[String],
) -> Result<ClaudeSyncResult, String> {
    let claude_home = dirs::home_dir()
        .map(|h| h.join(".claude"))
        .ok_or("Claude home directory not found")?;

    let projects_dir = claude_home.join("projects");

    let mut sessions_processed = 0;
    let mut sessions_skipped = 0;
    let mut created = 0;
    let mut updated = 0;
    let now = Utc::now();

    for project_path in project_paths {
        // Handle path encoding: /Users/foo -> -Users-foo or Users-foo
        // Try both with and without leading dash
        let dir_name_with_dash = project_path.replace('/', "-");
        let dir_name_without_dash = project_path.trim_start_matches('/').replace('/', "-");

        let project_dir = if projects_dir.join(&dir_name_with_dash).exists() {
            projects_dir.join(&dir_name_with_dash)
        } else if projects_dir.join(&dir_name_without_dash).exists() {
            projects_dir.join(&dir_name_without_dash)
        } else {
            log::debug!("Claude project directory not found for path: {}", project_path);
            continue;
        };

        if !project_dir.is_dir() {
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

                    // Extract session ID from filename (UUID.jsonl -> UUID)
                    let session_id = file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let date = session
                        .first_timestamp
                        .as_ref()
                        .and_then(|ts| ts.split('T').next())
                        .unwrap_or("2026-01-01")
                        .to_string();

                    let project_name = session.cwd
                        .split('/')
                        .last()
                        .unwrap_or("unknown")
                        .to_string();

                    // Build title from first message or fallback
                    let title_content = session.first_message
                        .as_ref()
                        .map(|m| {
                            let truncated: String = m.chars().take(60).collect();
                            if m.len() > 60 {
                                format!("{}...", truncated)
                            } else {
                                truncated
                            }
                        })
                        .unwrap_or_else(|| "Claude Code session".to_string());

                    let title = format!("[{}] {}", project_name, title_content);
                    let description = build_session_description(&session);

                    // Generate unique content hash for this session
                    let content_hash = generate_session_hash(user_id, &session_id, &session.cwd);

                    // Check if work item exists
                    let existing: Option<(String, Option<String>)> =
                        sqlx::query_as("SELECT id, hours_source FROM work_items WHERE content_hash = ? AND user_id = ?")
                            .bind(&content_hash)
                            .bind(user_id)
                            .fetch_optional(pool)
                            .await
                            .map_err(|e| e.to_string())?;

                    if let Some((existing_id, existing_hours_source)) = existing {
                        // Preserve user-modified hours
                        let user_modified = existing_hours_source.as_deref() == Some("user_modified");

                        if user_modified {
                            sqlx::query(
                                r#"UPDATE work_items
                                SET title = ?, description = ?, hours_estimated = ?,
                                    start_time = ?, end_time = ?, project_path = ?,
                                    session_id = ?, updated_at = ?
                                WHERE id = ?"#,
                            )
                            .bind(&title)
                            .bind(&description)
                            .bind(hours)
                            .bind(&session.first_timestamp)
                            .bind(&session.last_timestamp)
                            .bind(&session.cwd)
                            .bind(&session_id)
                            .bind(now)
                            .bind(&existing_id)
                            .execute(pool)
                            .await
                            .map_err(|e| e.to_string())?;
                        } else {
                            sqlx::query(
                                r#"UPDATE work_items
                                SET title = ?, description = ?, hours = ?, hours_source = 'session',
                                    hours_estimated = ?, start_time = ?, end_time = ?, project_path = ?,
                                    session_id = ?, updated_at = ?
                                WHERE id = ?"#,
                            )
                            .bind(&title)
                            .bind(&description)
                            .bind(hours)
                            .bind(hours)
                            .bind(&session.first_timestamp)
                            .bind(&session.last_timestamp)
                            .bind(&session.cwd)
                            .bind(&session_id)
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
                            (id, user_id, source, title, description, hours, date, content_hash,
                             hours_source, hours_estimated, session_id, start_time, end_time, project_path,
                             created_at, updated_at)
                            VALUES (?, ?, 'claude_code', ?, ?, ?, ?, ?, 'session', ?, ?, ?, ?, ?, ?, ?)"#,
                        )
                        .bind(&id)
                        .bind(user_id)
                        .bind(&title)
                        .bind(&description)
                        .bind(hours)
                        .bind(&date)
                        .bind(&content_hash)
                        .bind(hours)
                        .bind(&session_id)
                        .bind(&session.first_timestamp)
                        .bind(&session.last_timestamp)
                        .bind(&session.cwd)
                        .bind(now)
                        .bind(now)
                        .execute(pool)
                        .await
                        .map_err(|e| e.to_string())?;

                        created += 1;
                    }

                    sessions_processed += 1;
                }
            }
        }
    }

    Ok(ClaudeSyncResult {
        sessions_processed,
        sessions_skipped,
        work_items_created: created,
        work_items_updated: updated,
    })
}
