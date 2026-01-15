//! Source management commands
//!
//! Commands for managing data sources: git repos, Claude, GitLab.

use anyhow::Result;
use clap::Subcommand;
use serde::Serialize;
use tabled::Tabled;

use crate::output::{print_output, print_success, print_error, print_info};
use super::Context;

#[derive(Subcommand)]
pub enum SourceAction {
    /// List all configured sources
    List,

    /// Add a new source
    Add {
        #[command(subcommand)]
        source_type: AddSourceType,
    },

    /// Remove a source
    Remove {
        #[command(subcommand)]
        source_type: RemoveSourceType,
    },
}

#[derive(Subcommand)]
pub enum AddSourceType {
    /// Add a local git repository
    Git {
        /// Path to the git repository
        path: String,
    },
}

#[derive(Subcommand)]
pub enum RemoveSourceType {
    /// Remove a local git repository
    Git {
        /// Path to the git repository
        path: String,
    },
}

/// Source row for table display
#[derive(Debug, Serialize, Tabled)]
pub struct SourceRow {
    #[tabled(rename = "Type")]
    pub source_type: String,
    #[tabled(rename = "Name")]
    pub name: String,
    #[tabled(rename = "Path/URL")]
    pub path: String,
    #[tabled(rename = "Status")]
    pub status: String,
}

pub async fn execute(ctx: &Context, action: SourceAction) -> Result<()> {
    match action {
        SourceAction::List => list_sources(ctx).await,
        SourceAction::Add { source_type } => add_source(ctx, source_type).await,
        SourceAction::Remove { source_type } => remove_source(ctx, source_type).await,
    }
}

async fn list_sources(ctx: &Context) -> Result<()> {
    let mut rows = Vec::new();

    // List git repos
    let git_repos: Vec<recap_core::GitRepo> = sqlx::query_as(
        "SELECT * FROM git_repos WHERE enabled = 1"
    )
    .fetch_all(&ctx.db.pool)
    .await?;

    for repo in git_repos {
        let status = if is_valid_git_repo(&repo.path) {
            "Valid"
        } else {
            "Invalid"
        };

        rows.push(SourceRow {
            source_type: "git".to_string(),
            name: repo.name,
            path: repo.path,
            status: status.to_string(),
        });
    }

    // Check Claude connection
    let claude_path = get_claude_projects_path();
    rows.push(SourceRow {
        source_type: "claude".to_string(),
        name: "Claude Code".to_string(),
        path: claude_path.clone().unwrap_or_else(|| "-".to_string()),
        status: if claude_path.is_some() { "Connected" } else { "Not Found" }.to_string(),
    });

    // List GitLab projects
    let gitlab_projects: Vec<recap_core::GitLabProject> = sqlx::query_as(
        "SELECT * FROM gitlab_projects WHERE enabled = 1"
    )
    .fetch_all(&ctx.db.pool)
    .await?;

    for project in gitlab_projects {
        rows.push(SourceRow {
            source_type: "gitlab".to_string(),
            name: project.name,
            path: project.gitlab_url,
            status: "Configured".to_string(),
        });
    }

    if rows.is_empty() {
        print_info("No sources configured.", ctx.quiet);
        print_info("Use 'recap source add git <path>' to add a git repository.", ctx.quiet);
    } else {
        print_output(&rows, ctx.format)?;
    }

    Ok(())
}

async fn add_source(ctx: &Context, source_type: AddSourceType) -> Result<()> {
    match source_type {
        AddSourceType::Git { path } => add_git_source(ctx, path).await,
    }
}

async fn add_git_source(ctx: &Context, path: String) -> Result<()> {
    // Expand tilde and validate
    let expanded = shellexpand::tilde(&path);
    let expanded_path = expanded.to_string();

    if !is_valid_git_repo(&expanded_path) {
        print_error(&format!("Not a valid git repository: {}", path));
        return Ok(());
    }

    // Get or create default user
    let user_id = get_or_create_default_user(&ctx.db).await?;

    // Extract repo name from path
    let name = std::path::Path::new(&expanded_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Check if already exists
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM git_repos WHERE user_id = ? AND path = ?"
    )
    .bind(&user_id)
    .bind(&expanded_path)
    .fetch_optional(&ctx.db.pool)
    .await?;

    if existing.is_some() {
        print_info(&format!("Git repo already configured: {}", name), ctx.quiet);
        return Ok(());
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    sqlx::query(
        r#"
        INSERT INTO git_repos (id, user_id, path, name, enabled, created_at)
        VALUES (?, ?, ?, ?, 1, ?)
        "#
    )
    .bind(&id)
    .bind(&user_id)
    .bind(&expanded_path)
    .bind(&name)
    .bind(now)
    .execute(&ctx.db.pool)
    .await?;

    print_success(&format!("Added git repo: {} ({})", name, expanded_path), ctx.quiet);
    Ok(())
}

async fn remove_source(ctx: &Context, source_type: RemoveSourceType) -> Result<()> {
    match source_type {
        RemoveSourceType::Git { path } => remove_git_source(ctx, path).await,
    }
}

async fn remove_git_source(ctx: &Context, path: String) -> Result<()> {
    let expanded = shellexpand::tilde(&path);
    let expanded_path = expanded.to_string();

    let result = sqlx::query("DELETE FROM git_repos WHERE path = ?")
        .bind(&expanded_path)
        .execute(&ctx.db.pool)
        .await?;

    if result.rows_affected() > 0 {
        print_success(&format!("Removed git repo: {}", path), ctx.quiet);
    } else {
        print_error(&format!("Git repo not found: {}", path));
    }

    Ok(())
}

fn is_valid_git_repo(path: &str) -> bool {
    std::path::Path::new(path).join(".git").is_dir()
}

fn get_claude_projects_path() -> Option<String> {
    let home = dirs::home_dir()?;
    let claude_path = home.join(".claude").join("projects");
    if claude_path.exists() {
        Some(claude_path.to_string_lossy().to_string())
    } else {
        None
    }
}

async fn get_or_create_default_user(db: &recap_core::Database) -> Result<String> {
    let user: Option<(String,)> = sqlx::query_as("SELECT id FROM users LIMIT 1")
        .fetch_optional(&db.pool)
        .await?;

    if let Some((id,)) = user {
        return Ok(id);
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();
    let password_hash = recap_core::auth::hash_password("cli_user")?;

    sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, name, username, created_at, updated_at)
        VALUES (?, 'cli@localhost', ?, 'CLI User', 'cli', ?, ?)
        "#
    )
    .bind(&id)
    .bind(&password_hash)
    .bind(now)
    .bind(now)
    .execute(&db.pool)
    .await?;

    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_git_repo() {
        // Current project should be a git repo
        let project_path = env!("CARGO_MANIFEST_DIR");
        let git_root = std::path::Path::new(project_path)
            .parent() // crates/
            .and_then(|p| p.parent()) // web/
            .and_then(|p| p.parent()) // recap/
            .unwrap()
            .to_string_lossy()
            .to_string();

        assert!(is_valid_git_repo(&git_root));
        assert!(!is_valid_git_repo("/tmp"));
    }
}
