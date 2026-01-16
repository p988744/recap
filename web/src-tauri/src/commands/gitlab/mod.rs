//! GitLab module
//!
//! Tauri commands for GitLab integration operations.
//!
//! ## Structure
//! - `types.rs` - Request/response data types
//! - `config.rs` - Configuration commands (status, configure, remove)
//! - `projects.rs` - Project management (list, add, remove, search)
//! - `sync.rs` - Sync GitLab data to work items

pub mod config;
pub mod projects;
pub mod sync;
pub mod types;

// Re-export commands for registration
pub use config::{configure_gitlab, get_gitlab_status, remove_gitlab_config};
pub use projects::{add_gitlab_project, list_gitlab_projects, remove_gitlab_project, search_gitlab_projects};
pub use sync::sync_gitlab;

// Re-export types for external use
pub use types::{
    AddProjectRequest, ConfigureGitLabRequest, GitLabConfigStatus, GitLabProjectInfo,
    SearchProjectsRequest, SyncGitLabRequest, SyncGitLabResponse,
};
