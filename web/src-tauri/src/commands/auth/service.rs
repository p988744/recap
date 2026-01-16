//! Auth business logic
//!
//! Core authentication operations that are testable and independent of the framework.

use recap_core::{
    auth::{create_token, hash_password, verify_password},
    models::UserResponse,
};
use uuid::Uuid;

use super::repository::UserRepository;
use super::types::{AppStatus, LoginRequest, NewUser, RegisterRequest, TokenResponse};

/// Get app status - testable business logic
pub async fn get_app_status_impl<R: UserRepository>(repo: &R) -> Result<AppStatus, String> {
    let count = repo.get_user_count().await?;
    let first_user = if count > 0 {
        repo.get_first_user().await?
    } else {
        None
    };

    Ok(AppStatus {
        has_users: count > 0,
        user_count: count,
        first_user: first_user.map(UserResponse::from),
        local_mode: true,
    })
}

/// Register user - testable business logic
pub async fn register_user_impl<R: UserRepository>(
    repo: &R,
    request: RegisterRequest,
) -> Result<UserResponse, String> {
    // Check if username already exists
    if repo.username_exists(&request.username).await? {
        return Err("Username already exists".to_string());
    }

    // Generate email if not provided
    let email = request
        .email
        .clone()
        .unwrap_or_else(|| format!("{}@local", &request.username));

    // Check if email already exists
    if repo.email_exists(&email).await? {
        return Err("Email already registered".to_string());
    }

    // Check if this is the first user (will be admin)
    let is_first_user = repo.get_user_count().await? == 0;

    // Hash password
    let password_hash = hash_password(&request.password).map_err(|e| e.to_string())?;

    // Create user
    let new_user = NewUser {
        id: Uuid::new_v4().to_string(),
        username: request.username,
        email,
        password_hash,
        name: request.name,
        title: request.title,
        is_admin: is_first_user,
    };

    let user = repo.create_user(new_user).await?;
    Ok(UserResponse::from(user))
}

/// Login - testable business logic
pub async fn login_impl<R: UserRepository>(
    repo: &R,
    request: LoginRequest,
) -> Result<TokenResponse, String> {
    // Find user by username
    let user = repo
        .find_by_username(&request.username)
        .await?
        .ok_or_else(|| "Invalid credentials".to_string())?;

    // Verify password
    let valid = verify_password(&request.password, &user.password_hash).map_err(|e| e.to_string())?;

    if !valid {
        return Err("Invalid credentials".to_string());
    }

    if !user.is_active {
        return Err("Account is disabled".to_string());
    }

    // Create token
    let token = create_token(&user).map_err(|e| e.to_string())?;

    Ok(TokenResponse {
        access_token: token,
        token_type: "bearer".to_string(),
        expires_in: 7 * 24 * 60 * 60, // 7 days in seconds
    })
}

/// Auto-login - testable business logic
pub async fn auto_login_impl<R: UserRepository>(repo: &R) -> Result<TokenResponse, String> {
    // Get first user
    let user = repo
        .get_first_user()
        .await?
        .ok_or_else(|| "No user found".to_string())?;

    if !user.is_active {
        return Err("Account is disabled".to_string());
    }

    // Create token
    let token = create_token(&user).map_err(|e| e.to_string())?;

    Ok(TokenResponse {
        access_token: token,
        token_type: "bearer".to_string(),
        expires_in: 7 * 24 * 60 * 60,
    })
}

/// Get current user - testable business logic
pub async fn get_current_user_impl<R: UserRepository>(
    repo: &R,
    token: &str,
) -> Result<UserResponse, String> {
    // Verify token and get claims
    let claims = recap_core::auth::verify_token(token).map_err(|e| e.to_string())?;

    let user = repo
        .find_by_id(&claims.sub)
        .await?
        .ok_or_else(|| "User not found".to_string())?;

    Ok(UserResponse::from(user))
}
