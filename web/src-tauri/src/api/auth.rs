//! Auth API routes

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{create_token, hash_password, verify_password, AuthUser},
    db::Database,
    models::UserResponse,
};

/// Auth routes
pub fn routes() -> Router<Database> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/status", get(status))
        .route("/auto-login", post(auto_login))
        .route("/me", get(me))
}

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

/// Register a new user
async fn register(
    State(db): State<Database>,
    Json(req): Json<RegisterRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Check if username already exists
    let existing_username: Option<(i64,)> = sqlx::query_as("SELECT COUNT(*) FROM users WHERE username = ?")
        .bind(&req.username)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if existing_username.map(|r| r.0).unwrap_or(0) > 0 {
        return Err((StatusCode::BAD_REQUEST, "Username already exists".to_string()));
    }

    // Generate email if not provided
    let email = req.email.clone().unwrap_or_else(|| format!("{}@local", &req.username));

    // Check if email already exists
    let existing: Option<(i64,)> = sqlx::query_as("SELECT COUNT(*) FROM users WHERE email = ?")
        .bind(&email)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if existing.map(|r| r.0).unwrap_or(0) > 0 {
        return Err((StatusCode::BAD_REQUEST, "Email already registered".to_string()));
    }

    // Check if this is the first user
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let is_first_user = count.0 == 0;

    // Hash password
    let password_hash = hash_password(&req.password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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
    .bind(&req.username)
    .bind(&email)
    .bind(&password_hash)
    .bind(&req.name)
    .bind(&req.title)
    .bind(is_first_user)
    .bind(now)
    .bind(now)
    .execute(&db.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Fetch created user
    let user: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(UserResponse::from(user))))
}

/// Login and get token
async fn login(
    State(db): State<Database>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Find user by username
    let user: Option<crate::models::User> = sqlx::query_as("SELECT * FROM users WHERE username = ?")
        .bind(&req.username)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = user.ok_or((StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()))?;

    // Verify password
    let valid = verify_password(&req.password, &user.password_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !valid {
        return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()));
    }

    if !user.is_active {
        return Err((StatusCode::FORBIDDEN, "Account is disabled".to_string()));
    }

    // Create token
    let token = create_token(&user)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(TokenResponse {
        access_token: token,
        token_type: "bearer".to_string(),
        expires_in: 7 * 24 * 60 * 60, // 7 days in seconds
    }))
}

/// Get app status
async fn status(State(db): State<Database>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let first_user: Option<crate::models::User> = if count.0 > 0 {
        sqlx::query_as("SELECT * FROM users ORDER BY created_at LIMIT 1")
            .fetch_optional(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else {
        None
    };

    Ok(Json(AppStatus {
        has_users: count.0 > 0,
        user_count: count.0,
        first_user: first_user.map(UserResponse::from),
        local_mode: true,
    }))
}

/// Auto-login for local mode
async fn auto_login(State(db): State<Database>) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Get first user
    let user: Option<crate::models::User> =
        sqlx::query_as("SELECT * FROM users ORDER BY created_at LIMIT 1")
            .fetch_optional(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = user.ok_or((StatusCode::NOT_FOUND, "No user found".to_string()))?;

    if !user.is_active {
        return Err((StatusCode::FORBIDDEN, "Account is disabled".to_string()));
    }

    // Create token
    let token = create_token(&user)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(TokenResponse {
        access_token: token,
        token_type: "bearer".to_string(),
        expires_in: 7 * 24 * 60 * 60,
    }))
}

/// Get current user
async fn me(
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
