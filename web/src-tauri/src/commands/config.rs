//! Config commands
//!
//! Tauri commands for configuration operations.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::auth::verify_token;

use super::AppState;

// Types

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    // Jira settings
    pub jira_url: Option<String>,
    pub auth_type: String,
    pub jira_configured: bool,
    pub tempo_configured: bool,

    // LLM settings
    pub llm_provider: String,
    pub llm_model: String,
    pub llm_base_url: Option<String>,
    pub llm_configured: bool,

    // Work settings
    pub daily_work_hours: f64,
    pub normalize_hours: bool,

    // GitLab settings
    pub gitlab_url: Option<String>,
    pub gitlab_configured: bool,

    // Git repos
    pub use_git_mode: bool,
    pub git_repos: Vec<String>,
    pub outlook_enabled: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct UserConfigRow {
    jira_url: Option<String>,
    jira_pat: Option<String>,
    jira_email: Option<String>,
    tempo_token: Option<String>,
    gitlab_url: Option<String>,
    gitlab_pat: Option<String>,
    llm_provider: Option<String>,
    llm_model: Option<String>,
    llm_api_key: Option<String>,
    llm_base_url: Option<String>,
    daily_work_hours: Option<f64>,
    normalize_hours: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub daily_work_hours: Option<f64>,
    pub normalize_hours: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLlmConfigRequest {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateJiraConfigRequest {
    pub jira_url: Option<String>,
    pub jira_pat: Option<String>,
    pub jira_email: Option<String>,
    pub jira_api_token: Option<String>,
    pub auth_type: Option<String>,
    pub tempo_api_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

// Commands

/// Get current user configuration
#[tauri::command]
pub async fn get_config(
    state: State<'_, AppState>,
    token: String,
) -> Result<ConfigResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let user: UserConfigRow = sqlx::query_as(
        r#"SELECT
            jira_url, jira_pat, jira_email, tempo_token,
            gitlab_url, gitlab_pat,
            llm_provider, llm_model, llm_api_key, llm_base_url,
            daily_work_hours, normalize_hours
        FROM users WHERE id = ?"#
    )
    .bind(&claims.sub)
    .fetch_one(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // Determine auth type based on what's configured
    let auth_type = if user.jira_email.is_some() && user.jira_pat.is_some() {
        "basic".to_string()
    } else if user.jira_pat.is_some() {
        "pat".to_string()
    } else {
        "none".to_string()
    };

    Ok(ConfigResponse {
        jira_url: user.jira_url,
        auth_type,
        jira_configured: user.jira_pat.is_some(),
        tempo_configured: user.tempo_token.is_some(),

        llm_provider: user.llm_provider.unwrap_or_else(|| "openai".to_string()),
        llm_model: user.llm_model.unwrap_or_else(|| "gpt-4o-mini".to_string()),
        llm_base_url: user.llm_base_url,
        llm_configured: user.llm_api_key.is_some(),

        daily_work_hours: user.daily_work_hours.unwrap_or(8.0),
        normalize_hours: user.normalize_hours.unwrap_or(true),

        gitlab_url: user.gitlab_url,
        gitlab_configured: user.gitlab_pat.is_some(),

        use_git_mode: false,
        git_repos: vec![],
        outlook_enabled: false,
    })
}

/// Update general config settings
#[tauri::command]
pub async fn update_config(
    state: State<'_, AppState>,
    token: String,
    request: UpdateConfigRequest,
) -> Result<MessageResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;
    let now = Utc::now();

    if let Some(hours) = request.daily_work_hours {
        sqlx::query("UPDATE users SET daily_work_hours = ?, updated_at = ? WHERE id = ?")
            .bind(hours)
            .bind(now)
            .bind(&claims.sub)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(normalize) = request.normalize_hours {
        sqlx::query("UPDATE users SET normalize_hours = ?, updated_at = ? WHERE id = ?")
            .bind(normalize)
            .bind(now)
            .bind(&claims.sub)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(MessageResponse {
        message: "Config updated".to_string(),
    })
}

/// Update LLM configuration
#[tauri::command]
pub async fn update_llm_config(
    state: State<'_, AppState>,
    token: String,
    request: UpdateLlmConfigRequest,
) -> Result<MessageResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;
    let now = Utc::now();

    // Validate provider
    let valid_providers = ["openai", "anthropic", "ollama", "openai-compatible"];
    if !valid_providers.contains(&request.provider.as_str()) {
        return Err("Invalid LLM provider".to_string());
    }

    // For Ollama, default base_url if not provided
    let base_url = if request.provider == "ollama" && request.base_url.is_none() {
        Some("http://localhost:11434".to_string())
    } else {
        request.base_url
    };

    sqlx::query(
        r#"UPDATE users SET
            llm_provider = ?,
            llm_model = ?,
            llm_api_key = ?,
            llm_base_url = ?,
            updated_at = ?
        WHERE id = ?"#
    )
    .bind(&request.provider)
    .bind(&request.model)
    .bind(&request.api_key)
    .bind(&base_url)
    .bind(now)
    .bind(&claims.sub)
    .execute(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(MessageResponse {
        message: "LLM configuration updated".to_string(),
    })
}

/// Update Jira configuration
#[tauri::command]
pub async fn update_jira_config(
    state: State<'_, AppState>,
    token: String,
    request: UpdateJiraConfigRequest,
) -> Result<MessageResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;
    let now = Utc::now();

    // Update Jira URL if provided
    if let Some(url) = &request.jira_url {
        sqlx::query("UPDATE users SET jira_url = ?, updated_at = ? WHERE id = ?")
            .bind(url)
            .bind(now)
            .bind(&claims.sub)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    // Update auth credentials based on auth type
    if request.auth_type.as_deref() == Some("pat") {
        if let Some(pat) = &request.jira_pat {
            sqlx::query("UPDATE users SET jira_pat = ?, jira_email = NULL, updated_at = ? WHERE id = ?")
                .bind(pat)
                .bind(now)
                .bind(&claims.sub)
                .execute(&db.pool)
                .await
                .map_err(|e| e.to_string())?;
        }
    } else if request.auth_type.as_deref() == Some("basic") {
        if let Some(api_token) = &request.jira_api_token {
            sqlx::query("UPDATE users SET jira_pat = ?, updated_at = ? WHERE id = ?")
                .bind(api_token)
                .bind(now)
                .bind(&claims.sub)
                .execute(&db.pool)
                .await
                .map_err(|e| e.to_string())?;
        }
        if let Some(email) = &request.jira_email {
            sqlx::query("UPDATE users SET jira_email = ?, updated_at = ? WHERE id = ?")
                .bind(email)
                .bind(now)
                .bind(&claims.sub)
                .execute(&db.pool)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    // Update Tempo token if provided
    if let Some(tempo_token) = &request.tempo_api_token {
        sqlx::query("UPDATE users SET tempo_token = ?, updated_at = ? WHERE id = ?")
            .bind(tempo_token)
            .bind(now)
            .bind(&claims.sub)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(MessageResponse {
        message: "Jira configuration updated".to_string(),
    })
}
