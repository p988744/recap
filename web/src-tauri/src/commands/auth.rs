//! Auth commands
//!
//! Tauri commands for authentication operations.

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::{
    auth::{create_token, hash_password, verify_password},
    models::UserResponse,
};

use super::AppState;

// Request/Response types

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub name: String,
    pub email: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Serialize)]
pub struct AppStatus {
    pub has_users: bool,
    pub user_count: i64,
    pub first_user: Option<UserResponse>,
    pub local_mode: bool,
}

// Commands

/// Get app status (has_users, local_mode, etc.)
#[tauri::command]
pub async fn get_app_status(state: State<'_, AppState>) -> Result<AppStatus, String> {
    let db = state.db.lock().await;

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    let first_user: Option<crate::models::User> = if count.0 > 0 {
        sqlx::query_as("SELECT * FROM users ORDER BY created_at LIMIT 1")
            .fetch_optional(&db.pool)
            .await
            .map_err(|e| e.to_string())?
    } else {
        None
    };

    Ok(AppStatus {
        has_users: count.0 > 0,
        user_count: count.0,
        first_user: first_user.map(UserResponse::from),
        local_mode: true,
    })
}

/// Register a new user
#[tauri::command]
pub async fn register_user(
    state: State<'_, AppState>,
    request: RegisterRequest,
) -> Result<UserResponse, String> {
    let db = state.db.lock().await;

    // Check if username already exists
    let existing_username: Option<(i64,)> = sqlx::query_as("SELECT COUNT(*) FROM users WHERE username = ?")
        .bind(&request.username)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    if existing_username.map(|r| r.0).unwrap_or(0) > 0 {
        return Err("Username already exists".to_string());
    }

    // Generate email if not provided
    let email = request.email.clone().unwrap_or_else(|| format!("{}@local", &request.username));

    // Check if email already exists
    let existing: Option<(i64,)> = sqlx::query_as("SELECT COUNT(*) FROM users WHERE email = ?")
        .bind(&email)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    if existing.map(|r| r.0).unwrap_or(0) > 0 {
        return Err("Email already registered".to_string());
    }

    // Check if this is the first user
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    let is_first_user = count.0 == 0;

    // Hash password
    let password_hash = hash_password(&request.password)
        .map_err(|e| e.to_string())?;

    // Create user
    let user_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    sqlx::query(
        r#"
        INSERT INTO users (id, username, email, password_hash, name, title, is_admin, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&user_id)
    .bind(&request.username)
    .bind(&email)
    .bind(&password_hash)
    .bind(&request.name)
    .bind(&request.title)
    .bind(is_first_user)
    .bind(now)
    .bind(now)
    .execute(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // Fetch created user
    let user: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(UserResponse::from(user))
}

/// Login and get token
#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    request: LoginRequest,
) -> Result<TokenResponse, String> {
    let db = state.db.lock().await;

    // Find user by username
    let user: Option<crate::models::User> = sqlx::query_as("SELECT * FROM users WHERE username = ?")
        .bind(&request.username)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    let user = user.ok_or("Invalid credentials".to_string())?;

    // Verify password
    let valid = verify_password(&request.password, &user.password_hash)
        .map_err(|e| e.to_string())?;

    if !valid {
        return Err("Invalid credentials".to_string());
    }

    if !user.is_active {
        return Err("Account is disabled".to_string());
    }

    // Create token
    let token = create_token(&user)
        .map_err(|e| e.to_string())?;

    Ok(TokenResponse {
        access_token: token,
        token_type: "bearer".to_string(),
        expires_in: 7 * 24 * 60 * 60, // 7 days in seconds
    })
}

/// Auto-login for local mode (uses first user)
#[tauri::command]
pub async fn auto_login(state: State<'_, AppState>) -> Result<TokenResponse, String> {
    let db = state.db.lock().await;

    // Get first user
    let user: Option<crate::models::User> =
        sqlx::query_as("SELECT * FROM users ORDER BY created_at LIMIT 1")
            .fetch_optional(&db.pool)
            .await
            .map_err(|e| e.to_string())?;

    let user = user.ok_or("No user found".to_string())?;

    if !user.is_active {
        return Err("Account is disabled".to_string());
    }

    // Create token
    let token = create_token(&user)
        .map_err(|e| e.to_string())?;

    Ok(TokenResponse {
        access_token: token,
        token_type: "bearer".to_string(),
        expires_in: 7 * 24 * 60 * 60,
    })
}

/// Get current user by token
#[tauri::command]
pub async fn get_current_user(
    state: State<'_, AppState>,
    token: String,
) -> Result<UserResponse, String> {
    let db = state.db.lock().await;

    // Verify token and get claims
    let claims = crate::auth::verify_token(&token)
        .map_err(|e| e.to_string())?;

    let user: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&claims.sub)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(UserResponse::from(user))
}
