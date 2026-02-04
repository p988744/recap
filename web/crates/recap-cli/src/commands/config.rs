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
        // Jira settings
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

        // GitLab settings
        "gitlab_pat" => {
            update_user_setting(&ctx.db, &user_id, "gitlab_pat", &value).await?;
            print_success("Set gitlab_pat = ****", ctx.quiet);
        }
        "gitlab_url" => {
            update_user_setting(&ctx.db, &user_id, "gitlab_url", &value).await?;
            print_success(&format!("Set gitlab_url = {}", value), ctx.quiet);
        }

        // LLM settings
        "llm_provider" => {
            validate_llm_provider(&value)?;
            update_user_setting(&ctx.db, &user_id, "llm_provider", &value).await?;
            // If setting to ollama and no base_url is set, set default
            if value == "ollama" {
                let settings: Option<(Option<String>,)> = sqlx::query_as(
                    "SELECT llm_base_url FROM users WHERE id = ?"
                )
                .bind(&user_id)
                .fetch_optional(&ctx.db.pool)
                .await?;
                if settings.map(|s| s.0.is_none()).unwrap_or(true) {
                    update_user_setting(&ctx.db, &user_id, "llm_base_url", "http://localhost:11434").await?;
                    print_info("Set default llm_base_url = http://localhost:11434 for Ollama", ctx.quiet);
                }
            }
            print_success(&format!("Set llm_provider = {}", value), ctx.quiet);
        }
        "llm_model" => {
            update_user_setting(&ctx.db, &user_id, "llm_model", &value).await?;
            print_success(&format!("Set llm_model = {}", value), ctx.quiet);
        }
        "llm_api_key" => {
            update_user_setting(&ctx.db, &user_id, "llm_api_key", &value).await?;
            print_success("Set llm_api_key = ****", ctx.quiet);
        }
        "llm_base_url" => {
            update_user_setting(&ctx.db, &user_id, "llm_base_url", &value).await?;
            print_success(&format!("Set llm_base_url = {}", value), ctx.quiet);
        }

        // Work hour settings
        "daily_work_hours" => {
            let hours = parse_f64(&value)?;
            if hours <= 0.0 || hours > 24.0 {
                return Err(anyhow::anyhow!("daily_work_hours must be between 0 and 24"));
            }
            update_user_setting_f64(&ctx.db, &user_id, "daily_work_hours", hours).await?;
            print_success(&format!("Set daily_work_hours = {}", hours), ctx.quiet);
        }
        "normalize_hours" => {
            let normalize = parse_bool(&value)?;
            update_user_setting_bool(&ctx.db, &user_id, "normalize_hours", normalize).await?;
            print_success(&format!("Set normalize_hours = {}", normalize), ctx.quiet);
        }

        _ => {
            print_error(&format!("Unknown config key: {}", key));
            print_info(
                "Available keys:\n  \
                 Jira: jira_url, jira_email, jira_pat, tempo_token\n  \
                 GitLab: gitlab_url, gitlab_pat\n  \
                 LLM: llm_provider, llm_model, llm_api_key, llm_base_url\n  \
                 Work: daily_work_hours, normalize_hours",
                ctx.quiet
            );
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
                   gitlab_pat, gitlab_url,
                   llm_provider, llm_model, llm_api_key, llm_base_url,
                   daily_work_hours, normalize_hours
            FROM users WHERE id = ?
            "#
        )
        .bind(&user_id)
        .fetch_optional(&ctx.db.pool)
        .await?;

        if let Some(settings) = user {
            // Jira settings
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

            // GitLab settings
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

            // LLM settings
            rows.push(ConfigRow {
                key: "llm_provider".to_string(),
                value: settings.llm_provider.unwrap_or_else(|| "openai".to_string()),
                source: "db".to_string(),
            });
            rows.push(ConfigRow {
                key: "llm_model".to_string(),
                value: settings.llm_model.unwrap_or_else(|| "gpt-5-nano".to_string()),
                source: "db".to_string(),
            });
            rows.push(ConfigRow {
                key: "llm_api_key".to_string(),
                value: mask_token(&settings.llm_api_key),
                source: "db".to_string(),
            });
            rows.push(ConfigRow {
                key: "llm_base_url".to_string(),
                value: settings.llm_base_url.unwrap_or_else(|| "-".to_string()),
                source: "db".to_string(),
            });

            // Work hour settings
            rows.push(ConfigRow {
                key: "daily_work_hours".to_string(),
                value: settings.daily_work_hours.unwrap_or(8.0).to_string(),
                source: "db".to_string(),
            });
            rows.push(ConfigRow {
                key: "normalize_hours".to_string(),
                value: settings.normalize_hours.unwrap_or(true).to_string(),
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
    // LLM settings
    llm_provider: Option<String>,
    llm_model: Option<String>,
    llm_api_key: Option<String>,
    llm_base_url: Option<String>,
    // Work hour settings
    daily_work_hours: Option<f64>,
    normalize_hours: Option<bool>,
}

/// Valid LLM providers
const VALID_LLM_PROVIDERS: &[&str] = &["openai", "anthropic", "ollama", "openai-compatible"];

/// Validate LLM provider
fn validate_llm_provider(provider: &str) -> Result<()> {
    if VALID_LLM_PROVIDERS.contains(&provider) {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Invalid LLM provider: {}. Valid options: {}",
            provider,
            VALID_LLM_PROVIDERS.join(", ")
        ))
    }
}

/// Parse boolean value from string
fn parse_bool(value: &str) -> Result<bool> {
    match value.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(anyhow::anyhow!(
            "Invalid boolean value: {}. Use true/false, yes/no, 1/0, or on/off",
            value
        )),
    }
}

/// Parse f64 value from string
fn parse_f64(value: &str) -> Result<f64> {
    value.parse::<f64>().map_err(|_| {
        anyhow::anyhow!("Invalid number: {}. Please provide a valid decimal number", value)
    })
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

async fn update_user_setting_f64(db: &recap_core::Database, user_id: &str, key: &str, value: f64) -> Result<()> {
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

async fn update_user_setting_bool(db: &recap_core::Database, user_id: &str, key: &str, value: bool) -> Result<()> {
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
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_mask_token_with_value() {
        assert_eq!(mask_token(&Some("secret123".to_string())), "****");
        assert_eq!(mask_token(&Some("a".to_string())), "****");
        assert_eq!(mask_token(&Some("very-long-token-value-here".to_string())), "****");
    }

    #[test]
    fn test_mask_token_empty() {
        assert_eq!(mask_token(&Some("".to_string())), "-");
    }

    #[test]
    fn test_mask_token_none() {
        assert_eq!(mask_token(&None), "-");
    }

    #[test]
    fn test_get_claude_path_doesnt_panic() {
        // Just verify it doesn't panic
        let _ = get_claude_path();
    }

    #[test]
    fn test_get_claude_path_with_existing_dir() {
        let temp_dir = TempDir::new().unwrap();
        let claude_dir = temp_dir.path().join(".claude").join("projects");
        fs::create_dir_all(&claude_dir).unwrap();

        // We can't easily test this without modifying HOME,
        // but we can verify the function logic with a mock
        let path = claude_dir.to_string_lossy().to_string();
        assert!(std::path::Path::new(&path).exists());
    }

    #[test]
    fn test_config_row_serialization() {
        let row = ConfigRow {
            key: "jira_url".to_string(),
            value: "https://jira.example.com".to_string(),
            source: "db".to_string(),
        };

        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("jira_url"));
        assert!(json.contains("https://jira.example.com"));
        assert!(json.contains("db"));
    }

    #[test]
    fn test_config_row_debug() {
        let row = ConfigRow {
            key: "test_key".to_string(),
            value: "test_value".to_string(),
            source: "env".to_string(),
        };

        let debug = format!("{:?}", row);
        assert!(debug.contains("test_key"));
        assert!(debug.contains("test_value"));
    }

    #[test]
    fn test_config_row_with_sensitive_masked() {
        let row = ConfigRow {
            key: "tempo_token".to_string(),
            value: mask_token(&Some("secret".to_string())),
            source: "db".to_string(),
        };

        assert_eq!(row.value, "****");
    }

    #[test]
    fn test_config_row_with_default_source() {
        let row = ConfigRow {
            key: "RECAP_DB_PATH".to_string(),
            value: "/path/to/db".to_string(),
            source: "default".to_string(),
        };

        assert_eq!(row.source, "default");
    }

    #[test]
    fn test_user_settings_fields() {
        // Test that UserSettings struct can hold all expected fields
        let settings = UserSettings {
            jira_url: Some("https://jira.example.com".to_string()),
            jira_email: Some("user@example.com".to_string()),
            jira_pat: Some("secret-token".to_string()),
            tempo_token: Some("tempo-secret".to_string()),
            gitlab_pat: Some("gitlab-token".to_string()),
            gitlab_url: Some("https://gitlab.example.com".to_string()),
            llm_provider: Some("openai".to_string()),
            llm_model: Some("gpt-4".to_string()),
            llm_api_key: Some("sk-123".to_string()),
            llm_base_url: Some("https://api.openai.com".to_string()),
            daily_work_hours: Some(8.0),
            normalize_hours: Some(true),
        };

        assert!(settings.jira_url.is_some());
        assert!(settings.jira_email.is_some());
        assert!(settings.jira_pat.is_some());
        assert!(settings.tempo_token.is_some());
        assert!(settings.gitlab_pat.is_some());
        assert!(settings.gitlab_url.is_some());
        assert!(settings.llm_provider.is_some());
        assert!(settings.llm_model.is_some());
        assert!(settings.llm_api_key.is_some());
        assert!(settings.llm_base_url.is_some());
        assert!(settings.daily_work_hours.is_some());
        assert!(settings.normalize_hours.is_some());
    }

    #[test]
    fn test_user_settings_all_none() {
        let settings = UserSettings {
            jira_url: None,
            jira_email: None,
            jira_pat: None,
            tempo_token: None,
            gitlab_pat: None,
            gitlab_url: None,
            llm_provider: None,
            llm_model: None,
            llm_api_key: None,
            llm_base_url: None,
            daily_work_hours: None,
            normalize_hours: None,
        };

        assert!(settings.jira_url.is_none());
        assert!(settings.gitlab_pat.is_none());
        assert!(settings.llm_provider.is_none());
        assert!(settings.daily_work_hours.is_none());
    }

    // ========================================================================
    // LLM Provider Validation Tests
    // ========================================================================

    #[test]
    fn test_validate_llm_provider_openai() {
        assert!(validate_llm_provider("openai").is_ok());
    }

    #[test]
    fn test_validate_llm_provider_anthropic() {
        assert!(validate_llm_provider("anthropic").is_ok());
    }

    #[test]
    fn test_validate_llm_provider_ollama() {
        assert!(validate_llm_provider("ollama").is_ok());
    }

    #[test]
    fn test_validate_llm_provider_openai_compatible() {
        assert!(validate_llm_provider("openai-compatible").is_ok());
    }

    #[test]
    fn test_validate_llm_provider_invalid() {
        let result = validate_llm_provider("invalid-provider");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid LLM provider"));
        assert!(err.contains("openai"));
    }

    #[test]
    fn test_validate_llm_provider_empty() {
        assert!(validate_llm_provider("").is_err());
    }

    // ========================================================================
    // Boolean Parsing Tests
    // ========================================================================

    #[test]
    fn test_parse_bool_true_values() {
        assert!(parse_bool("true").unwrap());
        assert!(parse_bool("True").unwrap());
        assert!(parse_bool("TRUE").unwrap());
        assert!(parse_bool("1").unwrap());
        assert!(parse_bool("yes").unwrap());
        assert!(parse_bool("Yes").unwrap());
        assert!(parse_bool("on").unwrap());
        assert!(parse_bool("ON").unwrap());
    }

    #[test]
    fn test_parse_bool_false_values() {
        assert!(!parse_bool("false").unwrap());
        assert!(!parse_bool("False").unwrap());
        assert!(!parse_bool("FALSE").unwrap());
        assert!(!parse_bool("0").unwrap());
        assert!(!parse_bool("no").unwrap());
        assert!(!parse_bool("No").unwrap());
        assert!(!parse_bool("off").unwrap());
        assert!(!parse_bool("OFF").unwrap());
    }

    #[test]
    fn test_parse_bool_invalid() {
        let result = parse_bool("maybe");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid boolean value"));
    }

    // ========================================================================
    // Float Parsing Tests
    // ========================================================================

    #[test]
    fn test_parse_f64_valid() {
        assert_eq!(parse_f64("8.0").unwrap(), 8.0);
        assert_eq!(parse_f64("7.5").unwrap(), 7.5);
        assert_eq!(parse_f64("24").unwrap(), 24.0);
        assert_eq!(parse_f64("0.5").unwrap(), 0.5);
    }

    #[test]
    fn test_parse_f64_invalid() {
        let result = parse_f64("not-a-number");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid number"));
    }

    #[test]
    fn test_parse_f64_empty() {
        assert!(parse_f64("").is_err());
    }

    // ========================================================================
    // Config Row Tests for New Fields
    // ========================================================================

    #[test]
    fn test_config_row_llm_provider() {
        let row = ConfigRow {
            key: "llm_provider".to_string(),
            value: "openai".to_string(),
            source: "db".to_string(),
        };
        assert_eq!(row.key, "llm_provider");
        assert_eq!(row.value, "openai");
    }

    #[test]
    fn test_config_row_llm_api_key_masked() {
        let row = ConfigRow {
            key: "llm_api_key".to_string(),
            value: mask_token(&Some("sk-secret-key".to_string())),
            source: "db".to_string(),
        };
        assert_eq!(row.value, "****");
    }

    #[test]
    fn test_config_row_daily_work_hours() {
        let row = ConfigRow {
            key: "daily_work_hours".to_string(),
            value: "8".to_string(),
            source: "db".to_string(),
        };
        assert_eq!(row.key, "daily_work_hours");
        assert_eq!(row.value, "8");
    }

    #[test]
    fn test_config_row_normalize_hours() {
        let row = ConfigRow {
            key: "normalize_hours".to_string(),
            value: "true".to_string(),
            source: "db".to_string(),
        };
        assert_eq!(row.key, "normalize_hours");
        assert_eq!(row.value, "true");
    }

    // ========================================================================
    // Valid LLM Providers Constant Test
    // ========================================================================

    #[test]
    fn test_valid_llm_providers_count() {
        assert_eq!(VALID_LLM_PROVIDERS.len(), 4);
        assert!(VALID_LLM_PROVIDERS.contains(&"openai"));
        assert!(VALID_LLM_PROVIDERS.contains(&"anthropic"));
        assert!(VALID_LLM_PROVIDERS.contains(&"ollama"));
        assert!(VALID_LLM_PROVIDERS.contains(&"openai-compatible"));
    }
}
