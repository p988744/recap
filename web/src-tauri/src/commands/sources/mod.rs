//! Sources module
//!
//! Tauri commands for managing data sources (Git repos, Claude, etc.)
//!
//! ## Structure
//! - `types.rs` - Response types
//! - `helpers.rs` - Utility functions for git validation and path operations
//! - `commands.rs` - Tauri commands

pub mod commands;
pub mod helpers;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export commands for registration
pub use commands::{add_git_repo, get_sources, remove_git_repo, set_source_mode};

// Re-export types for external use
pub use types::{AddGitRepoResponse, MessageResponse};

// Re-export helpers for use in other modules (pub(crate))
pub use helpers::{extract_repo_name, get_claude_projects_path, get_last_commit_info, is_valid_git_repo};
