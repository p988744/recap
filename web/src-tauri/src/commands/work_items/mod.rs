//! Work Items commands
//!
//! Tauri commands for work item operations.
//!
//! This module is organized into:
//! - `types`: Type definitions for requests/responses
//! - `query_builder`: Safe SQL query builder
//! - `queries`: List, get, stats, and timeline queries
//! - `mutations`: Create, update, delete operations
//! - `grouped`: Grouped work items by project/date
//! - `sync`: Batch sync and aggregation
//! - `commit_centric`: Commit-centric worklog generation
//! - `helpers`: Session parsing helpers (used for tests)

// Declare all submodules as public so their #[tauri::command] items are accessible
pub mod commit_centric;
pub mod grouped;
pub mod helpers;
pub mod mutations;
pub mod queries;
pub mod query_builder;
pub mod sync;
pub mod types;

// Note: Commands are accessed via their submodule paths (e.g., work_items::queries::list_work_items)
// due to how tauri::generate_handler! macro works with #[tauri::command] attribute.
