//! Tauri commands for authentication
//!
//! Thin wrappers that connect Tauri's command system to the business logic.

use recap_core::models::UserResponse;
use tauri::State;

use crate::commands::AppState;
use super::repository::SqliteUserRepository;
use super::service;
use super::types::{AppStatus, LoginRequest, RegisterRequest, TokenResponse};

/// Get app status (has_users, local_mode, etc.)
#[tauri::command]
pub async fn get_app_status(state: State<'_, AppState>) -> Result<AppStatus, String> {
    let db = state.db.lock().await;
    let repo = SqliteUserRepository::new(&db.pool);
    service::get_app_status_impl(&repo).await
}

/// Register a new user
#[tauri::command]
pub async fn register_user(
    state: State<'_, AppState>,
    request: RegisterRequest,
) -> Result<UserResponse, String> {
    let db = state.db.lock().await;
    let repo = SqliteUserRepository::new(&db.pool);
    service::register_user_impl(&repo, request).await
}

/// Login and get token
#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    request: LoginRequest,
) -> Result<TokenResponse, String> {
    let db = state.db.lock().await;
    let repo = SqliteUserRepository::new(&db.pool);
    service::login_impl(&repo, request).await
}

/// Auto-login for local mode (uses first user)
#[tauri::command]
pub async fn auto_login(state: State<'_, AppState>) -> Result<TokenResponse, String> {
    let db = state.db.lock().await;
    let repo = SqliteUserRepository::new(&db.pool);
    service::auto_login_impl(&repo).await
}

/// Get current user by token
#[tauri::command]
pub async fn get_current_user(
    state: State<'_, AppState>,
    token: String,
) -> Result<UserResponse, String> {
    let db = state.db.lock().await;
    let repo = SqliteUserRepository::new(&db.pool);
    service::get_current_user_impl(&repo, &token).await
}
