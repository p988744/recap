//! Auth module
//!
//! Authentication operations using trait-based dependency injection for testability.
//!
//! ## Structure
//! - `types.rs` - Request/response data types
//! - `repository.rs` - UserRepository trait and SQLite implementation
//! - `service.rs` - Business logic (testable, framework-independent)
//! - `commands.rs` - Tauri command wrappers

pub mod commands;
pub mod repository;
pub mod service;
pub mod types;

#[cfg(test)]
mod tests;
