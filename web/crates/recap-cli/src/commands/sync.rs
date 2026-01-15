//! Sync commands
//!
//! Commands for syncing data from various sources.

use anyhow::Result;
use clap::Subcommand;
use serde::Serialize;
use tabled::Tabled;

use crate::output::{print_output, print_success, print_info};
use super::Context;

#[derive(Subcommand)]
pub enum SyncAction {
    /// Run sync from all configured sources
    Run {
        /// Only sync specific source (git, claude, gitlab)
        #[arg(short, long)]
        source: Option<String>,

        /// Specific project paths to sync
        #[arg(short, long)]
        project: Option<Vec<String>>,
    },

    /// Show sync status for all sources
    Status,
}

/// Sync status row for table display
#[derive(Debug, Serialize, Tabled)]
pub struct SyncStatusRow {
    #[tabled(rename = "Source")]
    pub source: String,
    #[tabled(rename = "Path")]
    pub path: String,
    #[tabled(rename = "Last Sync")]
    pub last_sync: String,
    #[tabled(rename = "Items")]
    pub items: String,
    #[tabled(rename = "Status")]
    pub status: String,
}

pub async fn execute(ctx: &Context, action: SyncAction) -> Result<()> {
    match action {
        SyncAction::Run { source, project } => {
            run_sync(ctx, source, project).await
        }
        SyncAction::Status => {
            show_status(ctx).await
        }
    }
}

async fn run_sync(
    ctx: &Context,
    source: Option<String>,
    project_paths: Option<Vec<String>>,
) -> Result<()> {
    // Get default user
    let user_id = get_default_user_id(&ctx.db).await?;

    let sources_to_sync = match source {
        Some(s) => vec![s],
        None => vec!["claude".to_string(), "git".to_string()],
    };

    for src in sources_to_sync {
        print_info(&format!("Syncing {}...", src), ctx.quiet);

        match src.as_str() {
            "claude" => {
                let paths = match &project_paths {
                    Some(p) => p.clone(),
                    None => find_claude_projects()?,
                };

                if paths.is_empty() {
                    print_info("  No Claude projects found.", ctx.quiet);
                } else {
                    print_info(&format!("  Found {} Claude project(s)", paths.len()), ctx.quiet);
                    let result = recap_core::sync_claude_projects(&ctx.db.pool, &user_id, &paths).await;

                    match result {
                        Ok(r) => {
                            print_success(&format!(
                                "    Sessions: {} processed, {} skipped",
                                r.sessions_processed, r.sessions_skipped
                            ), ctx.quiet);
                            print_success(&format!(
                                "    Work items: {} created, {} updated",
                                r.work_items_created, r.work_items_updated
                            ), ctx.quiet);
                        }
                        Err(e) => {
                            print_info(&format!("    Error: {}", e), ctx.quiet);
                        }
                    }
                }
            }
            "git" => {
                // Get configured git repos
                let repos: Vec<(String, String)> = sqlx::query_as(
                    "SELECT path, name FROM git_repos WHERE user_id = ? AND enabled = 1"
                )
                .bind(&user_id)
                .fetch_all(&ctx.db.pool)
                .await?;

                if repos.is_empty() {
                    print_info("  No git repos configured. Use 'recap source add git <path>'", ctx.quiet);
                } else {
                    for (path, name) in repos {
                        print_info(&format!("  Syncing git repo: {} ({})", name, path), ctx.quiet);
                        // Note: Git sync would use the worklog service
                        // For now, just indicate it's configured
                        print_success(&format!("    Git repo {} is configured", name), ctx.quiet);
                    }
                }
            }
            "gitlab" => {
                print_info("  GitLab sync requires API configuration", ctx.quiet);
            }
            _ => {
                print_info(&format!("  Unknown source: {}", src), ctx.quiet);
            }
        }
    }

    print_success("Sync completed", ctx.quiet);
    Ok(())
}

async fn show_status(ctx: &Context) -> Result<()> {
    let statuses: Vec<recap_core::SyncStatus> = sqlx::query_as(
        "SELECT * FROM sync_status ORDER BY source, source_path"
    )
    .fetch_all(&ctx.db.pool)
    .await?;

    if statuses.is_empty() {
        print_info("No sync history found. Run 'recap sync run' to start syncing.", ctx.quiet);
        return Ok(());
    }

    let rows: Vec<SyncStatusRow> = statuses
        .into_iter()
        .map(|s| SyncStatusRow {
            source: s.source,
            path: s.source_path.unwrap_or_else(|| "-".to_string()),
            last_sync: s.last_sync_at
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Never".to_string()),
            items: s.last_item_count.to_string(),
            status: s.status,
        })
        .collect();

    print_output(&rows, ctx.format)?;
    Ok(())
}

async fn get_default_user_id(db: &recap_core::Database) -> Result<String> {
    let user: Option<(String,)> = sqlx::query_as("SELECT id FROM users LIMIT 1")
        .fetch_optional(&db.pool)
        .await?;

    match user {
        Some((id,)) => Ok(id),
        None => Err(anyhow::anyhow!("No user found. Run 'recap work add' first to create a default user.")),
    }
}

fn find_claude_projects() -> Result<Vec<String>> {
    let home = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;

    let claude_projects = std::path::PathBuf::from(&home)
        .join(".claude")
        .join("projects");

    if !claude_projects.exists() {
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();
    for entry in std::fs::read_dir(&claude_projects)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Convert directory name back to path format
                let project_path = name.replace('-', "/");
                projects.push(project_path);
            }
        }
    }

    Ok(projects)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_claude_projects_doesnt_crash() {
        // Just verify it doesn't panic
        let _ = find_claude_projects();
    }
}
