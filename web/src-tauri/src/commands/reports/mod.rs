//! Reports commands
//!
//! Tauri commands for report generation operations.
//!
//! This module is organized into:
//! - `types`: Type definitions for requests/responses
//! - `helpers`: Helper functions for report generation
//! - `queries`: Basic report query commands
//! - `export`: Excel export and Tempo report generation

// Declare all submodules as public so their #[tauri::command] items are accessible
pub mod export;
pub mod helpers;
pub mod queries;
pub mod types;

// Note: Commands are accessed via their submodule paths (e.g., reports::queries::get_personal_report)
// due to how tauri::generate_handler! macro works with #[tauri::command] attribute.
