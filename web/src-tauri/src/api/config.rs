//! Config API routes

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{auth::AuthUser, db::Database};

/// Config routes
pub fn routes() -> Router<Database> {
    Router::new()
        .route("/", get(get_config))
        .route("/", patch(update_config))
        .route("/llm", patch(update_llm_config))
}

/// User config response (includes all settings)
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

    // Git repos (from local storage, placeholder)
    pub use_git_mode: bool,
    pub git_repos: Vec<String>,
    pub outlook_enabled: bool,
}

/// Extended user row for config
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

/// Get current user configuration
async fn get_config(
    State(db): State<Database>,
    auth: AuthUser,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user: UserConfigRow = sqlx::query_as(
        r#"SELECT
            jira_url, jira_pat, jira_email, tempo_token,
            gitlab_url, gitlab_pat,
            llm_provider, llm_model, llm_api_key, llm_base_url,
            daily_work_hours, normalize_hours
        FROM users WHERE id = ?"#
    )
    .bind(&auth.0.sub)
    .fetch_one(&db.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Determine auth type based on what's configured
    let auth_type = if user.jira_email.is_some() && user.jira_pat.is_some() {
        "basic".to_string()
    } else if user.jira_pat.is_some() {
        "pat".to_string()
    } else {
        "none".to_string()
    };

    let config = ConfigResponse {
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
    };

    Ok(Json(config))
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub daily_work_hours: Option<f64>,
    pub normalize_hours: Option<bool>,
}

/// Update general config settings
async fn update_config(
    State(db): State<Database>,
    auth: AuthUser,
    Json(req): Json<UpdateConfigRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let now = Utc::now();

    if let Some(hours) = req.daily_work_hours {
        sqlx::query("UPDATE users SET daily_work_hours = ?, updated_at = ? WHERE id = ?")
            .bind(hours)
            .bind(now)
            .bind(&auth.0.sub)
            .execute(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(normalize) = req.normalize_hours {
        sqlx::query("UPDATE users SET normalize_hours = ?, updated_at = ? WHERE id = ?")
            .bind(normalize)
            .bind(now)
            .bind(&auth.0.sub)
            .execute(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(Json(serde_json::json!({ "message": "Config updated" })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateLlmConfigRequest {
    pub provider: String,          // "openai", "anthropic", "ollama", "openai-compatible"
    pub model: String,             // Model name
    pub api_key: Option<String>,   // API key (not required for Ollama)
    pub base_url: Option<String>,  // Custom API URL (for ollama or self-hosted)
}

/// Update LLM configuration
async fn update_llm_config(
    State(db): State<Database>,
    auth: AuthUser,
    Json(req): Json<UpdateLlmConfigRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let now = Utc::now();

    // Validate provider
    let valid_providers = ["openai", "anthropic", "ollama", "openai-compatible"];
    if !valid_providers.contains(&req.provider.as_str()) {
        return Err((StatusCode::BAD_REQUEST, "Invalid LLM provider".to_string()));
    }

    // For Ollama, default base_url if not provided
    let base_url = if req.provider == "ollama" && req.base_url.is_none() {
        Some("http://localhost:11434".to_string())
    } else {
        req.base_url
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
    .bind(&req.provider)
    .bind(&req.model)
    .bind(&req.api_key)
    .bind(&base_url)
    .bind(now)
    .bind(&auth.0.sub)
    .execute(&db.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "message": "LLM configuration updated",
        "provider": req.provider,
        "model": req.model
    })))
}
