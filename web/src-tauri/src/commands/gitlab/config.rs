//! GitLab configuration commands
//!
//! Commands for managing GitLab configuration.

use chrono::Utc;
use tauri::State;

use recap_core::auth::verify_token;

use crate::commands::AppState;
use super::types::{ConfigureGitLabRequest, GitLabConfigStatus};

/// Get GitLab configuration status
#[tauri::command]
pub async fn get_gitlab_status(
    state: State<'_, AppState>,
    token: String,
) -> Result<GitLabConfigStatus, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let user: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&claims.sub)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(GitLabConfigStatus {
        configured: user.gitlab_pat.is_some(),
        gitlab_url: user.gitlab_url,
    })
}

/// Configure GitLab
#[tauri::command]
pub async fn configure_gitlab(
    state: State<'_, AppState>,
    token: String,
    request: ConfigureGitLabRequest,
) -> Result<serde_json::Value, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;
    let now = Utc::now();

    sqlx::query("UPDATE users SET gitlab_url = ?, gitlab_pat = ?, updated_at = ? WHERE id = ?")
        .bind(&request.gitlab_url)
        .bind(&request.gitlab_pat)
        .bind(now)
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "message": "GitLab configured successfully" }))
}

/// Remove GitLab configuration
#[tauri::command]
pub async fn remove_gitlab_config(
    state: State<'_, AppState>,
    token: String,
) -> Result<serde_json::Value, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;
    let now = Utc::now();

    sqlx::query("UPDATE users SET gitlab_url = NULL, gitlab_pat = NULL, updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "message": "GitLab configuration removed" }))
}
