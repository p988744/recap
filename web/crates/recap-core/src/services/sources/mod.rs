//! Data Source Abstraction
//!
//! This module provides a unified trait for pluggable data sources
//! that can sync work items from various external systems.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │ BackgroundSyncService                               │
//! │   for source in get_enabled_sources() {             │
//! │       source.sync(&pool, user_id).await             │
//! │   }                                                 │
//! └─────────────────────────────────────────────────────┘
//!          │
//!          ▼
//! ┌─────────────────────────────────────────────────────┐
//! │ trait SyncSource                                    │
//! │   fn source_name() -> &str                          │
//! │   fn discover_projects() -> Vec<SourceProject>      │
//! │   fn sync_sessions() -> SourceSyncResult            │
//! └─────────────────────────────────────────────────────┘
//!          │
//!     ┌────┴────┐
//!     ▼         ▼
//! ┌──────┐  ┌──────┐
//! │Claude│  │ Git  │
//! └──────┘  └──────┘
//! ```
//!
//! # Adding a New Source
//!
//! 1. Create a new module (e.g., `git.rs`)
//! 2. Implement the `SyncSource` trait
//! 3. Add it to the registry in `registry.rs`

pub mod types;
pub mod work_item;
pub mod claude;
pub mod registry;

pub use types::{SourceProject, SourceSyncResult, WorkItemParams};
pub use work_item::{upsert_work_item, UpsertResult};
pub use claude::ClaudeSource;
pub use registry::{get_enabled_sources, SyncConfig};

use async_trait::async_trait;
use sqlx::SqlitePool;

/// Trait for pluggable data sources
///
/// Implement this trait to add a new work item data source to Recap.
/// Sources can discover projects and sync sessions/items to work items.
#[async_trait]
pub trait SyncSource: Send + Sync {
    /// Unique identifier for this source (e.g., "claude_code")
    ///
    /// This is stored in the `source` column of work_items.
    fn source_name(&self) -> &'static str;

    /// Human-readable display name
    ///
    /// Used in UI and logs (e.g., "Claude Code")
    fn display_name(&self) -> &'static str;

    /// Check if this source is currently available
    ///
    /// Returns true if the source can be used (e.g., service is running).
    /// Default implementation returns true.
    async fn is_available(&self) -> bool {
        true
    }

    /// Discover projects/sessions from this source
    ///
    /// Returns a list of discovered projects that can be synced.
    async fn discover_projects(&self) -> Result<Vec<SourceProject>, String>;

    /// Sync all discovered sessions to work items
    ///
    /// This is the main sync method that processes all discovered items
    /// and creates/updates work items in the database.
    async fn sync_sessions(
        &self,
        pool: &SqlitePool,
        user_id: &str,
    ) -> Result<SourceSyncResult, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test that types are properly re-exported
    #[test]
    fn test_types_exported() {
        let _project = SourceProject {
            name: "test".to_string(),
            path: "/test".to_string(),
            session_count: 0,
        };

        let _result = SourceSyncResult::new("test");

        let _params = WorkItemParams::new(
            "user",
            "source",
            "id",
            "title",
            1.0,
            "2026-01-01",
        );
    }
}
