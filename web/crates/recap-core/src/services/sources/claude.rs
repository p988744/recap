//! Claude Code Source Implementation
//!
//! This module implements the SyncSource trait for Claude Code sessions.
//! It discovers Claude Code projects from ~/.claude/projects and syncs
//! sessions to work items.

use async_trait::async_trait;
use sqlx::SqlitePool;
use std::fs;
use std::path::Path;

use super::{SyncSource, SourceProject, SourceSyncResult, WorkItemParams, upsert_work_item, UpsertResult};
use crate::services::sync::{SyncService, DiscoveredProject, resolve_git_root};
use crate::services::session_parser::parse_session_full;
use crate::services::worklog::calculate_session_hours;

/// Claude Code data source
///
/// Syncs work items from local Claude Code sessions stored in ~/.claude/projects
pub struct ClaudeSource;

impl ClaudeSource {
    /// Create a new Claude Code source
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClaudeSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SyncSource for ClaudeSource {
    fn source_name(&self) -> &'static str {
        "claude_code"
    }

    fn display_name(&self) -> &'static str {
        "Claude Code"
    }

    async fn is_available(&self) -> bool {
        SyncService::get_claude_projects_dir().is_some()
    }

    async fn discover_projects(&self) -> Result<Vec<SourceProject>, String> {
        let projects = SyncService::discover_project_paths();
        Ok(projects
            .into_iter()
            .map(|p| {
                let session_count = p.claude_dirs.iter()
                    .filter_map(|dir| fs::read_dir(dir).ok())
                    .flat_map(|entries| entries.flatten())
                    .filter(|entry| {
                        entry.path().extension().map(|e| e == "jsonl").unwrap_or(false)
                    })
                    .count();

                SourceProject {
                    name: p.name,
                    path: p.canonical_path,
                    session_count,
                }
            })
            .collect())
    }

    async fn sync_sessions(
        &self,
        pool: &SqlitePool,
        user_id: &str,
    ) -> Result<SourceSyncResult, String> {
        let projects = SyncService::discover_project_paths();
        let mut result = SourceSyncResult::new(self.source_name());
        result.projects_scanned = projects.len();

        log::debug!("Claude Code: 發現 {} 個專案", projects.len());

        for (idx, project) in projects.iter().enumerate() {
            // Skip root path projects (MCP/no-context sessions)
            if project.canonical_path == "/" || project.canonical_path.is_empty() {
                log::debug!("[{}/{}] 跳過根路徑專案: {}", idx + 1, projects.len(), project.name);
                continue;
            }

            log::debug!("[{}/{}] 處理專案: {} ({})", idx + 1, projects.len(), project.name, project.canonical_path);

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
                    if !file_path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                        continue;
                    }

                    if let Some(session) = parse_session_full(&file_path) {
                        if session.message_count == 0 {
                            result.sessions_skipped += 1;
                            continue;
                        }

                        let hours = session_hours_from_options(
                            &session.first_timestamp,
                            &session.last_timestamp,
                        );

                        // Extract session ID from filename
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

                        // Build title from first message
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

                        let title = format!("[{}] {}", project.name, title_content);
                        let description = build_session_description(&session);

                        let params = WorkItemParams::new(
                            user_id,
                            self.source_name(),
                            &session_id,
                            title,
                            hours,
                            &date,
                        )
                        .with_description(description)
                        .with_project_path(&project.canonical_path)
                        .with_session_id(&session_id)
                        .with_time_range(session.first_timestamp.clone(), session.last_timestamp.clone());

                        match upsert_work_item(pool, params).await {
                            Ok(UpsertResult::Created(_)) => result.work_items_created += 1,
                            Ok(UpsertResult::Updated(_)) => result.work_items_updated += 1,
                            Ok(UpsertResult::Skipped(_)) => result.sessions_skipped += 1,
                            Err(e) => {
                                log::error!("Failed to upsert work item: {}", e);
                                result.sessions_skipped += 1;
                            }
                        }
                        result.sessions_processed += 1;
                    }
                }
            }
        }

        Ok(result)
    }
}

/// Sync discovered projects to work items (backward-compatible function).
///
/// This is a convenience wrapper that syncs specific project paths.
pub async fn sync_claude_projects(
    pool: &SqlitePool,
    user_id: &str,
    project_paths: &[String],
) -> Result<SourceSyncResult, String> {
    let claude_home = dirs::home_dir()
        .map(|h| h.join(".claude"))
        .ok_or("Claude home directory not found")?;

    let projects_dir = claude_home.join("projects");
    let mut result = SourceSyncResult::new("claude_code");

    // Convert project_paths into DiscoveredProject structs
    let mut grouped: std::collections::HashMap<String, Vec<std::path::PathBuf>> =
        std::collections::HashMap::new();

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

    result.projects_scanned = projects.len();

    // Sync each project
    for project in &projects {
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
                if !file_path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                    continue;
                }

                if let Some(session) = parse_session_full(&file_path) {
                    if session.message_count == 0 {
                        result.sessions_skipped += 1;
                        continue;
                    }

                    let hours = session_hours_from_options(
                        &session.first_timestamp,
                        &session.last_timestamp,
                    );

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

                    let title = format!("[{}] {}", project.name, title_content);
                    let description = build_session_description(&session);

                    let params = WorkItemParams::new(
                        user_id,
                        "claude_code",
                        &session_id,
                        title,
                        hours,
                        &date,
                    )
                    .with_description(description)
                    .with_project_path(&project.canonical_path)
                    .with_session_id(&session_id)
                    .with_time_range(session.first_timestamp.clone(), session.last_timestamp.clone());

                    match upsert_work_item(pool, params).await {
                        Ok(UpsertResult::Created(_)) => result.work_items_created += 1,
                        Ok(UpsertResult::Updated(_)) => result.work_items_updated += 1,
                        Ok(UpsertResult::Skipped(_)) => result.sessions_skipped += 1,
                        Err(e) => {
                            log::error!("Failed to upsert work item: {}", e);
                            result.sessions_skipped += 1;
                        }
                    }
                    result.sessions_processed += 1;
                }
            }
        }
    }

    Ok(result)
}

/// Helper to calculate session hours with Option handling
fn session_hours_from_options(first: &Option<String>, last: &Option<String>) -> f64 {
    match (first, last) {
        (Some(start), Some(end)) => calculate_session_hours(start, end),
        _ => 0.5,
    }
}

/// Build description for a single session work item
fn build_session_description(session: &crate::services::session_parser::ParsedSession) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_source_name() {
        let source = ClaudeSource::new();
        assert_eq!(source.source_name(), "claude_code");
        assert_eq!(source.display_name(), "Claude Code");
    }

    #[test]
    fn test_session_hours_from_options() {
        // Both timestamps present
        let first = Some("2026-01-15T09:00:00+08:00".to_string());
        let last = Some("2026-01-15T11:00:00+08:00".to_string());
        let hours = session_hours_from_options(&first, &last);
        assert!((hours - 2.0).abs() < 0.1);

        // Missing first timestamp
        let hours = session_hours_from_options(&None, &last);
        assert!((hours - 0.5).abs() < 0.01);

        // Missing last timestamp
        let hours = session_hours_from_options(&first, &None);
        assert!((hours - 0.5).abs() < 0.01);

        // Both missing
        let hours = session_hours_from_options(&None, &None);
        assert!((hours - 0.5).abs() < 0.01);
    }
}
