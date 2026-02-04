//! Projects commands
//!
//! Tauri commands for project management and visibility.
//!
//! This module is organized into:
//! - `types`: Type definitions for requests/responses
//! - `queries`: List, detail, visibility, and hidden project queries
//! - `descriptions`: Project description CRUD
//! - `timeline`: Project timeline with sessions and commits
//! - `summaries`: AI-powered project summary generation with caching
//! - `git_diff`: Git commit diff viewing

pub mod descriptions;
pub mod git_diff;
pub mod queries;
pub mod summaries;
pub mod timeline;
pub mod types;
