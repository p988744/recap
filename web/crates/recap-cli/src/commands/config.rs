//! Config commands
//!
//! Commands for managing CLI configuration.

use anyhow::Result;
use clap::Subcommand;
use serde::Serialize;
use tabled::Tabled;

use crate::output::{print_output, print_success, print_info, print_error};
use super::Context;

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,

    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,

        /// Configuration value
        value: String,
    },

    /// Get a configuration value
    Get {
        /// Configuration key
        key: String,
    },

    /// List all configuration keys and values
    List,
}

/// Config row for table display
#[derive(Debug, Serialize, Tabled)]
pub struct ConfigRow {
    #[tabled(rename = "Key")]
    pub key: String,
    #[tabled(rename = "Value")]
    pub value: String,
    #[tabled(rename = "Source")]
    pub source: String,
}

pub async fn execute(ctx: &Context, action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => show_config(ctx).await,
        ConfigAction::Set { key, value } => set_config(ctx, key, value).await,
        ConfigAction::Get { key } => get_config(ctx, key).await,
        ConfigAction::List => list_config(ctx).await,
    }
}

async fn show_config(ctx: &Context) -> Result<()> {
    let rows = get_all_config(ctx).await?;
    print_output(&rows, ctx.format)?;
    Ok(())
}

async fn list_config(ctx: &Context) -> Result<()> {
    show_config(ctx).await
}

async fn get_config(ctx: &Context, key: String) -> Result<()> {
    let rows = get_all_config(ctx).await?;

    if let Some(row) = rows.iter().find(|r| r.key.eq_ignore_ascii_case(&key)) {
        print_info(&format!("{} = {}", row.key, row.value), ctx.quiet);
    } else {
        print_error(&format!("Config key not found: {}", key));
    }

    Ok(())
}

async fn set_config(ctx: &Context, key: String, value: String) -> Result<()> {
    // Get default user
    let user_id = get_default_user_id(&ctx.db).await?;

    match key.to_lowercase().as_str() {
        "jira_url" => {
            update_user_setting(&ctx.db, &user_id, "jira_url", &value).await?;
            print_success(&format!("Set jira_url = {}", value), ctx.quiet);
        }
        "jira_email" => {
            update_user_setting(&ctx.db, &user_id, "jira_email", &value).await?;
            print_success(&format!("Set jira_email = {}", value), ctx.quiet);
        }
        "jira_pat" => {
            update_user_setting(&ctx.db, &user_id, "jira_pat", &value).await?;
            print_success("Set jira_pat = ****", ctx.quiet);
        }
        "tempo_token" => {
            update_user_setting(&ctx.db, &user_id, "tempo_token", &value).await?;
            print_success("Set tempo_token = ****", ctx.quiet);
        }
        "gitlab_pat" => {
            update_user_setting(&ctx.db, &user_id, "gitlab_pat", &value).await?;
            print_success("Set gitlab_pat = ****", ctx.quiet);
        }
        "gitlab_url" => {
            update_user_setting(&ctx.db, &user_id, "gitlab_url", &value).await?;
            print_success(&format!("Set gitlab_url = {}", value), ctx.quiet);
        }
        _ => {
            print_error(&format!("Unknown config key: {}", key));
            print_info("Available keys: jira_url, jira_email, jira_pat, tempo_token, gitlab_pat, gitlab_url", ctx.quiet);
        }
    }

    Ok(())
}

async fn get_all_config(ctx: &Context) -> Result<Vec<ConfigRow>> {
    let mut rows = Vec::new();

    // Database path
    let db_path = recap_core::db::get_db_path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "Unknown".to_string());

    rows.push(ConfigRow {
        key: "RECAP_DB_PATH".to_string(),
        value: db_path,
        source: if std::env::var("RECAP_DB_PATH").is_ok() { "env" } else { "default" }.to_string(),
    });

    // Claude path
    let claude_path = get_claude_path();
    rows.push(ConfigRow {
        key: "claude_projects_path".to_string(),
        value: claude_path.clone().unwrap_or_else(|| "Not found".to_string()),
        source: if claude_path.is_some() { "detected" } else { "n/a" }.to_string(),
    });

    // User settings from database
    if let Ok(user_id) = get_default_user_id(&ctx.db).await {
        let user: Option<UserSettings> = sqlx::query_as(
            r#"
            SELECT jira_url, jira_email, jira_pat, tempo_token,
                   gitlab_pat, gitlab_url
            FROM users WHERE id = ?
            "#
        )
        .bind(&user_id)
        .fetch_optional(&ctx.db.pool)
        .await?;

        if let Some(settings) = user {
            rows.push(ConfigRow {
                key: "jira_url".to_string(),
                value: settings.jira_url.unwrap_or_else(|| "-".to_string()),
                source: "db".to_string(),
            });
            rows.push(ConfigRow {
                key: "jira_email".to_string(),
                value: settings.jira_email.unwrap_or_else(|| "-".to_string()),
                source: "db".to_string(),
            });
            rows.push(ConfigRow {
                key: "jira_pat".to_string(),
                value: mask_token(&settings.jira_pat),
                source: "db".to_string(),
            });
            rows.push(ConfigRow {
                key: "tempo_token".to_string(),
                value: mask_token(&settings.tempo_token),
                source: "db".to_string(),
            });
            rows.push(ConfigRow {
                key: "gitlab_pat".to_string(),
                value: mask_token(&settings.gitlab_pat),
                source: "db".to_string(),
            });
            rows.push(ConfigRow {
                key: "gitlab_url".to_string(),
                value: settings.gitlab_url.unwrap_or_else(|| "-".to_string()),
                source: "db".to_string(),
            });
        }
    }

    Ok(rows)
}

#[derive(Debug, sqlx::FromRow)]
struct UserSettings {
    jira_url: Option<String>,
    jira_email: Option<String>,
    jira_pat: Option<String>,
    tempo_token: Option<String>,
    gitlab_pat: Option<String>,
    gitlab_url: Option<String>,
}

fn mask_token(token: &Option<String>) -> String {
    match token {
        Some(t) if !t.is_empty() => "****".to_string(),
        _ => "-".to_string(),
    }
}

fn get_claude_path() -> Option<String> {
    let home = dirs::home_dir()?;
    let claude_path = home.join(".claude").join("projects");
    if claude_path.exists() {
        Some(claude_path.to_string_lossy().to_string())
    } else {
        None
    }
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

async fn update_user_setting(db: &recap_core::Database, user_id: &str, key: &str, value: &str) -> Result<()> {
    let query = format!("UPDATE users SET {} = ?, updated_at = ? WHERE id = ?", key);
    let now = chrono::Utc::now();

    sqlx::query(&query)
        .bind(value)
        .bind(now)
        .bind(user_id)
        .execute(&db.pool)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_token() {
        assert_eq!(mask_token(&Some("secret123".to_string())), "****");
        assert_eq!(mask_token(&Some("".to_string())), "-");
        assert_eq!(mask_token(&None), "-");
    }

    #[test]
    fn test_get_claude_path() {
        // Just verify it doesn't panic
        let _ = get_claude_path();
    }
}
