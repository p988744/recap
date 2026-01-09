//! Users API routes

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, put},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;

use crate::{auth::AuthUser, db::Database, models::UserResponse};

/// Users routes
pub fn routes() -> Router<Database> {
    Router::new()
        .route("/profile", get(get_profile))
        .route("/profile", patch(update_profile))
        .route("/gitlab-pat", put(update_gitlab_pat))
        .route("/tempo-token", put(update_tempo_token))
        .route("/jira-config", put(update_jira_config))
}

/// Get current user profile
async fn get_profile(
    State(db): State<Database>,
    auth: AuthUser,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&auth.0.sub)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(UserResponse::from(user)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub title: Option<String>,
    pub employee_id: Option<String>,
    pub department_id: Option<String>,
}

/// Update user profile
async fn update_profile(
    State(db): State<Database>,
    auth: AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let now = Utc::now();

    if let Some(name) = &req.name {
        sqlx::query("UPDATE users SET name = ?, updated_at = ? WHERE id = ?")
            .bind(name)
            .bind(now)
            .bind(&auth.0.sub)
            .execute(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(email) = &req.email {
        sqlx::query("UPDATE users SET email = ?, updated_at = ? WHERE id = ?")
            .bind(email)
            .bind(now)
            .bind(&auth.0.sub)
            .execute(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(title) = &req.title {
        sqlx::query("UPDATE users SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(now)
            .bind(&auth.0.sub)
            .execute(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(employee_id) = &req.employee_id {
        sqlx::query("UPDATE users SET employee_id = ?, updated_at = ? WHERE id = ?")
            .bind(employee_id)
            .bind(now)
            .bind(&auth.0.sub)
            .execute(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(department_id) = &req.department_id {
        sqlx::query("UPDATE users SET department_id = ?, updated_at = ? WHERE id = ?")
            .bind(department_id)
            .bind(now)
            .bind(&auth.0.sub)
            .execute(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    let user: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&auth.0.sub)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(UserResponse::from(user)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateGitLabPatRequest {
    pub gitlab_url: String,
    pub gitlab_pat: String,
}

/// Update GitLab PAT
async fn update_gitlab_pat(
    State(db): State<Database>,
    auth: AuthUser,
    Json(req): Json<UpdateGitLabPatRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let now = Utc::now();

    sqlx::query("UPDATE users SET gitlab_url = ?, gitlab_pat = ?, updated_at = ? WHERE id = ?")
        .bind(&req.gitlab_url)
        .bind(&req.gitlab_pat)
        .bind(now)
        .bind(&auth.0.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({ "message": "GitLab PAT updated" })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateTempoTokenRequest {
    pub tempo_token: String,
}

/// Update Tempo token
async fn update_tempo_token(
    State(db): State<Database>,
    auth: AuthUser,
    Json(req): Json<UpdateTempoTokenRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let now = Utc::now();

    sqlx::query("UPDATE users SET tempo_token = ?, updated_at = ? WHERE id = ?")
        .bind(&req.tempo_token)
        .bind(now)
        .bind(&auth.0.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({ "message": "Tempo token updated" })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateJiraConfigRequest {
    pub jira_url: String,
    pub jira_email: String,
    pub jira_pat: String,
}

/// Update Jira configuration
async fn update_jira_config(
    State(db): State<Database>,
    auth: AuthUser,
    Json(req): Json<UpdateJiraConfigRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let now = Utc::now();

    sqlx::query(
        "UPDATE users SET jira_url = ?, jira_email = ?, jira_pat = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&req.jira_url)
    .bind(&req.jira_email)
    .bind(&req.jira_pat)
    .bind(now)
    .bind(&auth.0.sub)
    .execute(&db.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({ "message": "Jira configuration updated" })))
}
