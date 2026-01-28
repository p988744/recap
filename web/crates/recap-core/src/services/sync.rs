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
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::models::{SyncStatus, SyncStatusResponse};
use super::session_parser::{extract_cwd, parse_session_full, ParsedSession};
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

// ============ Git Root Resolution ============

/// Resolve the git repository root for a given path.
/// Walks up from the path looking for a `.git` directory or `.git` file (worktree).
/// Returns the git root path, or the original path if no git root is found.
pub fn resolve_git_root(path: &str) -> String {
    let mut current = PathBuf::from(path);

    // Walk up the directory tree
    loop {
        let git_path = current.join(".git");
        if git_path.exists() {
            // Found a .git dir or file — this is the git root
            return current.to_string_lossy().to_string();
        }

        if !current.pop() {
            // Reached filesystem root without finding .git
            break;
        }
    }

    // No git root found, return original path
    path.to_string()
}

// ============ Project Discovery ============

/// A discovered Claude project, potentially grouping multiple Claude dirs
/// that all resolve to the same git root.
#[derive(Debug, Clone)]
pub struct DiscoveredProject {
    /// The canonical project path (git root resolved)
    pub canonical_path: String,
    /// All matching Claude project directories
    pub claude_dirs: Vec<PathBuf>,
    /// Project name (last component of canonical_path)
    pub name: String,
}

impl SyncService {
    /// Discover Claude project paths using multiple strategies:
    /// 1. Read `sessions-index.json` for `projectPath`
    /// 2. Use `extract_cwd()` on first `.jsonl` file found
    /// 3. Decode directory name back to path (e.g. `-Users-foo-bar` → `/Users/foo/bar`)
    ///
    /// After extracting raw paths, calls `resolve_git_root()` to canonicalize.
    /// Groups all dirs that resolve to the same git root into one `DiscoveredProject`.
    pub fn discover_project_paths() -> Vec<DiscoveredProject> {
        let claude_dir = match Self::get_claude_projects_dir() {
            Some(dir) => dir,
            None => return vec![],
        };

        let entries = match fs::read_dir(&claude_dir) {
            Ok(e) => e,
            Err(_) => return vec![],
        };

        // Map: git_root -> (Vec<claude_dirs>)
        let mut grouped: HashMap<String, Vec<PathBuf>> = HashMap::new();

        for entry in entries.flatten() {
            let dir_path = entry.path();
            if !dir_path.is_dir() {
                continue;
            }

            // Check if this dir has any session files
            let has_sessions = fs::read_dir(&dir_path)
                .map(|files| {
                    files.flatten().any(|f| {
                        f.path()
                            .extension()
                            .map(|e| e == "jsonl")
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false);

            if !has_sessions {
                continue;
            }

            // Try to extract the project path using multiple strategies
            let raw_path = Self::extract_project_path_from_dir(&dir_path);

            if let Some(raw) = raw_path {
                let git_root = resolve_git_root(&raw);

                // Skip root filesystem path — these are MCP/no-context sessions
                // stored in ~/.claude/projects/-/ with no real project directory
                if git_root == "/" || git_root.is_empty() {
                    log::debug!("Skipping root path project from {:?}", dir_path);
                    continue;
                }

                grouped.entry(git_root).or_default().push(dir_path);
            }
        }

        grouped
            .into_iter()
            .map(|(canonical_path, claude_dirs)| {
                let name = Path::new(&canonical_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                DiscoveredProject {
                    canonical_path,
                    claude_dirs,
                    name,
                }
            })
            .collect()
    }

    /// Try to extract a project path from a Claude project directory.
    /// Uses priority order: sessions-index.json → extract_cwd → dir name decode.
    fn extract_project_path_from_dir(dir_path: &Path) -> Option<String> {
        // Strategy 1: Read sessions-index.json
        let index_path = dir_path.join("sessions-index.json");
        if index_path.exists() {
            if let Ok(content) = fs::read_to_string(&index_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    // Try entries[0].projectPath
                    if let Some(project_path) = json
                        .get("entries")
                        .and_then(|e| e.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|entry| entry.get("projectPath"))
                        .and_then(|v| v.as_str())
                    {
                        if !project_path.is_empty() {
                            return Some(project_path.to_string());
                        }
                    }
                    // Also try top-level projectPath
                    if let Some(project_path) = json
                        .get("projectPath")
                        .and_then(|v| v.as_str())
                    {
                        if !project_path.is_empty() {
                            return Some(project_path.to_string());
                        }
                    }
                }
            }
        }

        // Strategy 2: Use extract_cwd() on first JSONL file
        if let Ok(files) = fs::read_dir(dir_path) {
            for file in files.flatten() {
                let path = file.path();
                if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                    if let Some(cwd) = extract_cwd(&path) {
                        return Some(cwd);
                    }
                }
            }
        }

        // Strategy 3: Decode directory name back to path
        let dir_name = dir_path.file_name()?.to_string_lossy().to_string();
        Some(decode_dir_name_to_path(&dir_name))
    }
}

/// Decode a Claude project directory name back to a filesystem path.
/// e.g. `-Users-foo-bar` → `/Users/foo/bar`
fn decode_dir_name_to_path(dir_name: &str) -> String {
    // Claude encodes paths by replacing / with -
    // A leading dash means the path started with /
    if dir_name.starts_with('-') {
        // Has leading slash: -Users-foo-bar → /Users/foo/bar
        format!("/{}", dir_name.trim_start_matches('-').replace('-', "/"))
    } else {
        // No leading slash: Users-foo-bar → Users/foo/bar
        dir_name.replace('-', "/")
    }
}

// ============ Claude Sync Logic ============

// Shared functions from session_parser: parse_session_full, ParsedSession
// Shared from worklog: calculate_session_hours

/// Sync result for Claude projects
#[derive(Debug, serde::Serialize)]
pub struct ClaudeSyncResult {
    pub projects_scanned: usize,
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

/// Generate a unique hash for session-based deduplication (legacy, includes project_path)
#[cfg(test)]
fn generate_session_hash_legacy(user_id: &str, session_id: &str, project_path: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    format!("session:{}:{}:{}", user_id, project_path, session_id).hash(&mut hasher);
    format!("sess_{:x}", hasher.finish())
}

/// Generate a unique hash for session-based deduplication (v2, uses only user_id + session_id).
/// Since session_id is a UUID and already globally unique, including project_path
/// is unnecessary and causes duplicate work items when the same session is seen
/// from different sub-folder cwd values.
fn generate_session_hash(user_id: &str, session_id: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    format!("session:{}:{}", user_id, session_id).hash(&mut hasher);
    format!("sess_{:x}", hasher.finish())
}

/// Find an existing work item by either the new hash or by session_id fallback.
/// This handles the transition from old hashes (which included project_path)
/// to new hashes (user_id + session_id only).
async fn find_existing_work_item(
    pool: &SqlitePool,
    user_id: &str,
    content_hash: &str,
    session_id: &str,
) -> Result<Option<(String, Option<String>, Option<String>)>, String> {
    // First try: exact content_hash match (new or legacy)
    let existing: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT id, hours_source, content_hash FROM work_items WHERE content_hash = ? AND user_id = ?",
    )
    .bind(content_hash)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    if existing.is_some() {
        return Ok(existing);
    }

    // Second try: session_id fallback for old hash migration
    let fallback: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT id, hours_source, content_hash FROM work_items WHERE session_id = ? AND source = 'claude_code' AND user_id = ?",
    )
    .bind(session_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(fallback)
}

/// Sync discovered projects to work items.
/// Uses `DiscoveredProject` to iterate over all Claude dirs for each project,
/// using the canonical (git root) path for grouping and naming.
pub async fn sync_discovered_projects(
    pool: &SqlitePool,
    user_id: &str,
    projects: &[DiscoveredProject],
) -> Result<ClaudeSyncResult, String> {
    let mut sessions_processed = 0;
    let mut sessions_skipped = 0;
    let mut created = 0;
    let mut updated = 0;
    let now = Utc::now();

    for project in projects {
        // Skip root path projects (MCP/no-context sessions)
        if project.canonical_path == "/" || project.canonical_path.is_empty() {
            continue;
        }

        for claude_dir in &project.claude_dirs {
            if !claude_dir.is_dir() {
                continue;
            }

            let files = match fs::read_dir(claude_dir) {
                Ok(f) => f,
                Err(_) => continue,
            };

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

                    // Use the canonical (git root) project name
                    let project_name = &project.name;

                    // Build title from first message or fallback
                    let title_content = session
                        .first_message
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

                    // Use canonical path (git root) as project_path
                    let project_path = &project.canonical_path;

                    // Generate new content hash (user_id + session_id only)
                    let content_hash = generate_session_hash(user_id, &session_id);

                    // Find existing work item with dual lookup (new hash + session_id fallback)
                    let existing =
                        find_existing_work_item(pool, user_id, &content_hash, &session_id).await?;

                    if let Some((existing_id, existing_hours_source, old_hash)) = existing {
                        // Migrate hash if it changed
                        let needs_hash_migration =
                            old_hash.as_deref() != Some(&content_hash);

                        // Preserve user-modified hours
                        let user_modified =
                            existing_hours_source.as_deref() == Some("user_modified");

                        if user_modified {
                            sqlx::query(
                                r#"UPDATE work_items
                                SET title = ?, description = ?, hours_estimated = ?,
                                    start_time = ?, end_time = ?, project_path = ?,
                                    session_id = ?, content_hash = ?, updated_at = ?
                                WHERE id = ?"#,
                            )
                            .bind(&title)
                            .bind(&description)
                            .bind(hours)
                            .bind(&session.first_timestamp)
                            .bind(&session.last_timestamp)
                            .bind(project_path)
                            .bind(&session_id)
                            .bind(&content_hash)
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
                                    session_id = ?, content_hash = ?, updated_at = ?
                                WHERE id = ?"#,
                            )
                            .bind(&title)
                            .bind(&description)
                            .bind(hours)
                            .bind(hours)
                            .bind(&session.first_timestamp)
                            .bind(&session.last_timestamp)
                            .bind(project_path)
                            .bind(&session_id)
                            .bind(&content_hash)
                            .bind(now)
                            .bind(&existing_id)
                            .execute(pool)
                            .await
                            .map_err(|e| e.to_string())?;
                        }

                        if needs_hash_migration {
                            log::info!(
                                "Migrated hash for session {} from {:?} to {}",
                                session_id,
                                old_hash,
                                content_hash
                            );
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
                        .bind(project_path)
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
        projects_scanned: projects.len(),
        sessions_processed,
        sessions_skipped,
        work_items_created: created,
        work_items_updated: updated,
    })
}

/// Sync Claude projects to work items (backward-compatible wrapper).
/// Converts project_paths into `DiscoveredProject` structs with git root resolution
/// and delegates to `sync_discovered_projects`.
pub async fn sync_claude_projects(
    pool: &SqlitePool,
    user_id: &str,
    project_paths: &[String],
) -> Result<ClaudeSyncResult, String> {
    let claude_home = dirs::home_dir()
        .map(|h| h.join(".claude"))
        .ok_or("Claude home directory not found")?;

    let projects_dir = claude_home.join("projects");

    // Convert project_paths into DiscoveredProject structs
    let mut grouped: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for project_path in project_paths {
        // Handle path encoding: /Users/foo -> -Users-foo or Users-foo
        let dir_name_with_dash = project_path.replace('/', "-");
        let dir_name_without_dash = project_path.trim_start_matches('/').replace('/', "-");

        let project_dir = if projects_dir.join(&dir_name_with_dash).exists() {
            projects_dir.join(&dir_name_with_dash)
        } else if projects_dir.join(&dir_name_without_dash).exists() {
            projects_dir.join(&dir_name_without_dash)
        } else {
            log::debug!(
                "Claude project directory not found for path: {}",
                project_path
            );
            continue;
        };

        if !project_dir.is_dir() {
            continue;
        }

        let git_root = resolve_git_root(project_path);
        grouped.entry(git_root).or_default().push(project_dir);
    }

    let projects: Vec<DiscoveredProject> = grouped
        .into_iter()
        .map(|(canonical_path, claude_dirs)| {
            let name = Path::new(&canonical_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            DiscoveredProject {
                canonical_path,
                claude_dirs,
                name,
            }
        })
        .collect();

    sync_discovered_projects(pool, user_id, &projects).await
}

// ============ Tests ============

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_resolve_git_root_real_repo() {
        // The recap project itself is a git repo
        let project_root = env!("CARGO_MANIFEST_DIR");
        let sub_path = format!("{}/src/services", project_root);
        let result = resolve_git_root(&sub_path);
        // Should resolve to the repo root (parent of crates/)
        assert!(
            result.contains("recap"),
            "Expected git root containing 'recap', got: {}",
            result
        );
        // The result should not be the sub-path itself
        assert!(
            !result.ends_with("/src/services"),
            "Should resolve to git root, not sub-path: {}",
            result
        );
    }

    #[test]
    fn test_resolve_git_root_non_git_path() {
        let path = "/tmp/some/random/path/that/does/not/exist";
        let result = resolve_git_root(path);
        assert_eq!(result, path, "Non-git path should return original");
    }

    #[test]
    fn test_resolve_git_root_root_path() {
        let result = resolve_git_root("/");
        assert_eq!(result, "/", "Root path should return root");
    }

    #[test]
    fn test_resolve_git_root_with_git_dir() {
        // Create a temp dir with .git directory
        let dir = std::env::temp_dir().join("recap_test_git_root");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::create_dir(dir.join(".git")).unwrap();
        let sub_dir = dir.join("src").join("deep");
        fs::create_dir_all(&sub_dir).unwrap();

        let result = resolve_git_root(&sub_dir.to_string_lossy());
        assert_eq!(
            result,
            dir.to_string_lossy(),
            "Should resolve to the dir containing .git"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_git_root_with_git_file_worktree() {
        // Create a temp dir with .git file (worktree)
        let dir = std::env::temp_dir().join("recap_test_git_worktree");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(".git"), "gitdir: /some/other/path/.git/worktrees/foo").unwrap();

        let result = resolve_git_root(&dir.to_string_lossy());
        assert_eq!(
            result,
            dir.to_string_lossy(),
            "Should resolve to the dir containing .git file (worktree)"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_decode_dir_name_to_path_with_leading_dash() {
        assert_eq!(
            decode_dir_name_to_path("-Users-foo-bar"),
            "/Users/foo/bar"
        );
    }

    #[test]
    fn test_decode_dir_name_to_path_without_leading_dash() {
        assert_eq!(
            decode_dir_name_to_path("Users-foo-bar"),
            "Users/foo/bar"
        );
    }

    #[test]
    fn test_decode_dir_name_to_path_single_segment() {
        assert_eq!(decode_dir_name_to_path("project"), "project");
    }

    #[test]
    fn test_generate_session_hash_consistent() {
        let hash1 = generate_session_hash("user1", "session-abc");
        let hash2 = generate_session_hash("user1", "session-abc");
        assert_eq!(hash1, hash2, "Same inputs should produce same hash");
    }

    #[test]
    fn test_generate_session_hash_different_sessions() {
        let hash1 = generate_session_hash("user1", "session-abc");
        let hash2 = generate_session_hash("user1", "session-def");
        assert_ne!(hash1, hash2, "Different sessions should produce different hashes");
    }

    #[test]
    fn test_generate_session_hash_different_users() {
        let hash1 = generate_session_hash("user1", "session-abc");
        let hash2 = generate_session_hash("user2", "session-abc");
        assert_ne!(hash1, hash2, "Different users should produce different hashes");
    }

    #[test]
    fn test_generate_session_hash_prefix() {
        let hash = generate_session_hash("user1", "session-abc");
        assert!(hash.starts_with("sess_"), "Hash should start with sess_ prefix");
    }

    #[test]
    fn test_generate_session_hash_v2_differs_from_legacy() {
        let new_hash = generate_session_hash("user1", "session-abc");
        let legacy_hash = generate_session_hash_legacy("user1", "session-abc", "/some/path");
        assert_ne!(
            new_hash, legacy_hash,
            "New hash should differ from legacy hash (different inputs)"
        );
    }

    #[test]
    fn test_discovered_project_name() {
        let project = DiscoveredProject {
            canonical_path: "/Users/foo/MyProject".to_string(),
            claude_dirs: vec![PathBuf::from("/tmp/test")],
            name: "MyProject".to_string(),
        };
        assert_eq!(project.name, "MyProject");
    }
}
