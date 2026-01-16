//! Sources types
//!
//! Response types for source management operations.

use recap_core::models::GitRepoInfo;
use serde::Serialize;

/// Generic message response
#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub success: bool,
    pub message: String,
}

/// Response for adding a Git repository
#[derive(Debug, Serialize)]
pub struct AddGitRepoResponse {
    pub success: bool,
    pub message: String,
    pub repo: Option<GitRepoInfo>,
}
