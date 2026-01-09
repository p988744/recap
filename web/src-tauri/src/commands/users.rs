//! Users commands
//!
//! Tauri commands for user profile operations.

use chrono::Utc;
use serde::Deserialize;
use tauri::State;

use crate::auth::verify_token;
use crate::models::UserResponse;

use super::AppState;

// Types

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub title: Option<String>,
    pub employee_id: Option<String>,
    pub department_id: Option<String>,
}

// Commands

/// Get current user profile
#[tauri::command]
pub async fn get_profile(
    state: State<'_, AppState>,
    token: String,
) -> Result<UserResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let user: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&claims.sub)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(UserResponse::from(user))
}

/// Update user profile
#[tauri::command]
pub async fn update_profile(
    state: State<'_, AppState>,
    token: String,
    request: UpdateProfileRequest,
) -> Result<UserResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;
    let now = Utc::now();

    if let Some(name) = &request.name {
        sqlx::query("UPDATE users SET name = ?, updated_at = ? WHERE id = ?")
            .bind(name)
            .bind(now)
            .bind(&claims.sub)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(email) = &request.email {
        sqlx::query("UPDATE users SET email = ?, updated_at = ? WHERE id = ?")
            .bind(email)
            .bind(now)
            .bind(&claims.sub)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(title) = &request.title {
        sqlx::query("UPDATE users SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(now)
            .bind(&claims.sub)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(employee_id) = &request.employee_id {
        sqlx::query("UPDATE users SET employee_id = ?, updated_at = ? WHERE id = ?")
            .bind(employee_id)
            .bind(now)
            .bind(&claims.sub)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(department_id) = &request.department_id {
        sqlx::query("UPDATE users SET department_id = ?, updated_at = ? WHERE id = ?")
            .bind(department_id)
            .bind(now)
            .bind(&claims.sub)
            .execute(&db.pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    let user: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&claims.sub)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(UserResponse::from(user))
}
