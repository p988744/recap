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
pub(crate) fn session_hours_from_options(first: &Option<String>, last: &Option<String>) -> f64 {
    match (first, last) {
        (Some(start), Some(end)) => calculate_session_hours(start, end),
        _ => 0.5,
    }
}

/// Build description for a single session work item
pub(crate) fn build_session_description(session: &ParsedSession) -> String {
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
pub(crate) fn generate_session_hash(user_id: &str, session_id: &str, project_path: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::session_parser::ToolUsage;

    // ========================================================================
    // Helper Functions
    // ========================================================================

    fn create_test_session() -> ParsedSession {
        ParsedSession {
            cwd: "/Users/test/projects/myapp".to_string(),
            first_timestamp: Some("2026-01-15T10:00:00Z".to_string()),
            last_timestamp: Some("2026-01-15T12:30:00Z".to_string()),
            message_count: 10,
            tool_usage: vec![
                ToolUsage { tool_name: "Read".to_string(), count: 15 },
                ToolUsage { tool_name: "Edit".to_string(), count: 8 },
                ToolUsage { tool_name: "Bash".to_string(), count: 5 },
            ],
            files_modified: vec![
                "src/main.rs".to_string(),
                "src/lib.rs".to_string(),
                "Cargo.toml".to_string(),
            ],
            first_message: Some("Help me implement the new feature".to_string()),
        }
    }

    fn create_empty_session() -> ParsedSession {
        ParsedSession {
            cwd: "/Users/test/project".to_string(),
            first_timestamp: None,
            last_timestamp: None,
            message_count: 0,
            tool_usage: vec![],
            files_modified: vec![],
            first_message: None,
        }
    }

    // ========================================================================
    // session_hours_from_options Tests
    // ========================================================================

    #[test]
    fn test_session_hours_both_timestamps() {
        let first = Some("2026-01-15T10:00:00Z".to_string());
        let last = Some("2026-01-15T12:30:00Z".to_string());

        let hours = session_hours_from_options(&first, &last);

        // Should calculate actual hours (2.5 hours)
        assert!(hours > 2.0 && hours < 3.0);
    }

    #[test]
    fn test_session_hours_missing_first() {
        let first = None;
        let last = Some("2026-01-15T12:30:00Z".to_string());

        let hours = session_hours_from_options(&first, &last);

        // Should default to 0.5
        assert_eq!(hours, 0.5);
    }

    #[test]
    fn test_session_hours_missing_last() {
        let first = Some("2026-01-15T10:00:00Z".to_string());
        let last = None;

        let hours = session_hours_from_options(&first, &last);

        // Should default to 0.5
        assert_eq!(hours, 0.5);
    }

    #[test]
    fn test_session_hours_both_missing() {
        let first = None;
        let last = None;

        let hours = session_hours_from_options(&first, &last);

        // Should default to 0.5
        assert_eq!(hours, 0.5);
    }

    #[test]
    fn test_session_hours_same_timestamp() {
        let ts = Some("2026-01-15T10:00:00Z".to_string());

        let hours = session_hours_from_options(&ts, &ts);

        // Very short session (same time)
        assert!(hours >= 0.0);
    }

    // ========================================================================
    // build_session_description Tests
    // ========================================================================

    #[test]
    fn test_build_description_full_session() {
        let session = create_test_session();

        let desc = build_session_description(&session);

        // Should contain tool usage
        assert!(desc.contains("🔧 Tools:"));
        assert!(desc.contains("Read: 15"));
        assert!(desc.contains("Edit: 8"));
        assert!(desc.contains("Bash: 5"));

        // Should contain files modified
        assert!(desc.contains("📁 Modified files"));
        assert!(desc.contains("src/main.rs"));
        assert!(desc.contains("src/lib.rs"));

        // Should contain project path
        assert!(desc.contains("📂 Project:"));
        assert!(desc.contains("/Users/test/projects/myapp"));
    }

    #[test]
    fn test_build_description_empty_session() {
        let session = create_empty_session();

        let desc = build_session_description(&session);

        // Should not contain tools or files
        assert!(!desc.contains("🔧 Tools:"));
        assert!(!desc.contains("📁 Modified files"));

        // Should still have project path
        assert!(desc.contains("📂 Project:"));
    }

    #[test]
    fn test_build_description_no_tools() {
        let mut session = create_test_session();
        session.tool_usage = vec![];

        let desc = build_session_description(&session);

        assert!(!desc.contains("🔧 Tools:"));
        assert!(desc.contains("📁 Modified files"));
    }

    #[test]
    fn test_build_description_no_files() {
        let mut session = create_test_session();
        session.files_modified = vec![];

        let desc = build_session_description(&session);

        assert!(desc.contains("🔧 Tools:"));
        assert!(!desc.contains("📁 Modified files"));
    }

    #[test]
    fn test_build_description_many_tools() {
        let mut session = create_test_session();
        session.tool_usage = (0..15)
            .map(|i| ToolUsage {
                tool_name: format!("Tool{}", i),
                count: 15 - i,
            })
            .collect();

        let desc = build_session_description(&session);

        // Should limit to 8 tools
        assert!(desc.contains("Tool0"));
        assert!(desc.contains("Tool7"));
        // Tool8 onwards should not be present
        assert!(!desc.contains("Tool8"));
    }

    #[test]
    fn test_build_description_many_files() {
        let mut session = create_test_session();
        session.files_modified = (0..10)
            .map(|i| format!("src/file{}.rs", i))
            .collect();

        let desc = build_session_description(&session);

        // Should show 5 files
        assert!(desc.contains("file0.rs"));
        assert!(desc.contains("file4.rs"));
        // file5 onwards should not be listed
        assert!(!desc.contains("file5.rs"));
        // Should show (+5 more)
        assert!(desc.contains("+5 more"));
    }

    // ========================================================================
    // generate_session_hash Tests
    // ========================================================================

    #[test]
    fn test_generate_session_hash_format() {
        let hash = generate_session_hash("user-123", "session-abc", "/path/to/project");

        // Should start with sess_
        assert!(hash.starts_with("sess_"));
        // Should be hex after prefix
        assert!(hash[5..].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_session_hash_consistency() {
        let hash1 = generate_session_hash("user-123", "session-abc", "/path/to/project");
        let hash2 = generate_session_hash("user-123", "session-abc", "/path/to/project");

        // Same inputs should produce same hash
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_generate_session_hash_different_users() {
        let hash1 = generate_session_hash("user-123", "session-abc", "/path/to/project");
        let hash2 = generate_session_hash("user-456", "session-abc", "/path/to/project");

        // Different users should produce different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_generate_session_hash_different_sessions() {
        let hash1 = generate_session_hash("user-123", "session-abc", "/path/to/project");
        let hash2 = generate_session_hash("user-123", "session-xyz", "/path/to/project");

        // Different sessions should produce different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_generate_session_hash_different_projects() {
        let hash1 = generate_session_hash("user-123", "session-abc", "/path/to/project1");
        let hash2 = generate_session_hash("user-123", "session-abc", "/path/to/project2");

        // Different projects should produce different hashes
        assert_ne!(hash1, hash2);
    }

    // ========================================================================
    // ClaudeSyncResult Tests
    // ========================================================================

    #[test]
    fn test_claude_sync_result_serialization() {
        let result = ClaudeSyncResult {
            sessions_processed: 10,
            sessions_skipped: 2,
            work_items_created: 8,
            work_items_updated: 3,
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["sessions_processed"], 10);
        assert_eq!(parsed["sessions_skipped"], 2);
        assert_eq!(parsed["work_items_created"], 8);
        assert_eq!(parsed["work_items_updated"], 3);
    }

    #[test]
    fn test_claude_sync_result_zero_values() {
        let result = ClaudeSyncResult {
            sessions_processed: 0,
            sessions_skipped: 0,
            work_items_created: 0,
            work_items_updated: 0,
        };

        let json = serde_json::to_string(&result).unwrap();

        assert!(json.contains("\"sessions_processed\":0"));
        assert!(json.contains("\"sessions_skipped\":0"));
    }

    // ========================================================================
    // SyncService Tests
    // ========================================================================

    #[test]
    fn test_get_claude_projects_dir() {
        // This test depends on the actual filesystem
        // Just verify it doesn't panic and returns Option
        let result = SyncService::get_claude_projects_dir();

        // Result should be Some if ~/.claude/projects exists, None otherwise
        // We can't assert a specific value as it depends on the environment
        if let Some(path) = result {
            assert!(path.to_string_lossy().contains(".claude"));
            assert!(path.to_string_lossy().contains("projects"));
        }
    }

    #[test]
    fn test_list_claude_projects_returns_vec() {
        // Just verify it returns a Vec and doesn't panic
        let projects = SyncService::list_claude_projects();

        // Should return a Vec (may be empty if no Claude projects)
        assert!(projects.len() >= 0);
    }
}
