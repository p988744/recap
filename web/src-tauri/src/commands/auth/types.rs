//! Auth request/response types
//!
//! Data types for authentication operations.

use recap_core::models::UserResponse;
use serde::{Deserialize, Serialize};

/// Request for user registration
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub name: String,
    pub email: Option<String>,
    pub title: Option<String>,
}

/// Request for user login
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Response containing access token
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// Application status information
#[derive(Debug, Clone, Serialize)]
pub struct AppStatus {
    pub has_users: bool,
    pub user_count: i64,
    pub first_user: Option<UserResponse>,
    pub local_mode: bool,
}

/// Data for creating a new user
#[derive(Debug, Clone)]
pub struct NewUser {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub name: String,
    pub title: Option<String>,
    pub is_admin: bool,
}
