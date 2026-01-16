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

// Re-export Tauri commands for registration
pub use commands::{auto_login, get_app_status, get_current_user, login, register_user};

// Re-export types for external use
pub use types::{AppStatus, LoginRequest, NewUser, RegisterRequest, TokenResponse};

// Re-export repository trait for testing
pub use repository::UserRepository;
