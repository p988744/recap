//! GitLab types
//!
//! Request/response types for GitLab integration.

use serde::{Deserialize, Serialize};

/// Request to add a GitLab project to tracking
#[derive(Debug, Deserialize)]
pub struct AddProjectRequest {
    pub gitlab_project_id: i64,
    /// Optional fields - if not provided, will be fetched from GitLab API
    pub name: Option<String>,
    pub path_with_namespace: Option<String>,
    pub gitlab_url: Option<String>,
    pub default_branch: Option<String>,
}

/// Request to sync GitLab data
#[derive(Debug, Deserialize)]
pub struct SyncGitLabRequest {
    pub project_id: Option<String>,
}

/// Response from GitLab sync operation
#[derive(Debug, Serialize)]
pub struct SyncGitLabResponse {
    pub synced_commits: i64,
    pub synced_merge_requests: i64,
    pub work_items_created: i64,
}

/// Request to search GitLab projects
#[derive(Debug, Deserialize)]
pub struct SearchProjectsRequest {
    pub search: Option<String>,
}

/// GitLab project information from API
#[derive(Debug, Serialize, Deserialize)]
pub struct GitLabProjectInfo {
    pub id: i64,
    pub name: String,
    pub path_with_namespace: String,
    pub web_url: String,
    pub default_branch: Option<String>,
}

/// GitLab commit from API
#[derive(Debug, Deserialize)]
pub struct GitLabCommit {
    pub id: String,
    pub title: String,
    pub message: Option<String>,
    pub committed_date: String,
    pub stats: Option<CommitStats>,
}

/// Commit statistics from GitLab API
#[derive(Debug, Deserialize)]
pub struct CommitStats {
    pub additions: i32,
    pub deletions: i32,
}

/// GitLab configuration status
#[derive(Debug, Serialize)]
pub struct GitLabConfigStatus {
    pub configured: bool,
    pub gitlab_url: Option<String>,
}

/// Request to configure GitLab
#[derive(Debug, Deserialize)]
pub struct ConfigureGitLabRequest {
    pub gitlab_url: String,
    pub gitlab_pat: String,
}
