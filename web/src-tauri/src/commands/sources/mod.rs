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
