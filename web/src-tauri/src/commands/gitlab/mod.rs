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
