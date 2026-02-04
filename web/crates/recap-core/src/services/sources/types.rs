//! Common types for sync sources
//!
//! This module defines shared types used across all sync source implementations.

use serde::{Deserialize, Serialize};

/// A discovered project from a data source
#[derive(Debug, Clone)]
pub struct SourceProject {
    /// Human-readable project name
    pub name: String,
    /// Canonical project path (e.g., git root)
    pub path: String,
    /// Number of sessions/items in this project
    pub session_count: usize,
}

/// Result of a sync operation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceSyncResult {
    /// Source identifier (e.g., "claude_code", "antigravity")
    pub source: String,
    /// Number of projects scanned
    pub projects_scanned: usize,
    /// Number of sessions/items processed
    pub sessions_processed: usize,
    /// Number of sessions/items skipped (already exist or invalid)
    pub sessions_skipped: usize,
    /// Number of new work items created
    pub work_items_created: usize,
    /// Number of existing work items updated
    pub work_items_updated: usize,
    /// Error message if sync failed
    pub error: Option<String>,
}

impl SourceSyncResult {
    /// Create a new sync result for a source
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            ..Default::default()
        }
    }

    /// Create an error result
    pub fn with_error(source: &str, error: String) -> Self {
        Self {
            source: source.to_string(),
            error: Some(error),
            ..Default::default()
        }
    }
}

/// Parameters for creating/updating a work item
#[derive(Debug, Clone)]
pub struct WorkItemParams {
    /// User ID owning this work item
    pub user_id: String,
    /// Source identifier (e.g., "claude_code", "antigravity", "git")
    pub source: String,
    /// Source-specific item ID
    pub source_id: String,
    /// Work item title
    pub title: String,
    /// Optional description
    pub description: Option<String>,
    /// Hours spent
    pub hours: f64,
    /// Date in YYYY-MM-DD format
    pub date: String,
    /// Project path
    pub project_path: Option<String>,
    /// Session ID (for session-based sources)
    pub session_id: Option<String>,
    /// Start time (ISO 8601)
    pub start_time: Option<String>,
    /// End time (ISO 8601)
    pub end_time: Option<String>,
}

impl WorkItemParams {
    /// Create new work item params with required fields
    pub fn new(
        user_id: impl Into<String>,
        source: impl Into<String>,
        source_id: impl Into<String>,
        title: impl Into<String>,
        hours: f64,
        date: impl Into<String>,
    ) -> Self {
        Self {
            user_id: user_id.into(),
            source: source.into(),
            source_id: source_id.into(),
            title: title.into(),
            description: None,
            hours,
            date: date.into(),
            project_path: None,
            session_id: None,
            start_time: None,
            end_time: None,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set project path
    pub fn with_project_path(mut self, path: impl Into<String>) -> Self {
        self.project_path = Some(path.into());
        self
    }

    /// Set session ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set time range
    pub fn with_time_range(
        mut self,
        start_time: Option<String>,
        end_time: Option<String>,
    ) -> Self {
        self.start_time = start_time;
        self.end_time = end_time;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_sync_result_new() {
        let result = SourceSyncResult::new("claude_code");
        assert_eq!(result.source, "claude_code");
        assert_eq!(result.projects_scanned, 0);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_source_sync_result_with_error() {
        let result = SourceSyncResult::with_error("antigravity", "Connection failed".to_string());
        assert_eq!(result.source, "antigravity");
        assert_eq!(result.error, Some("Connection failed".to_string()));
    }

    #[test]
    fn test_work_item_params_builder() {
        let params = WorkItemParams::new(
            "user123",
            "claude_code",
            "session-abc",
            "Test work item",
            1.5,
            "2026-01-15",
        )
        .with_description("Test description")
        .with_project_path("/Users/test/project")
        .with_session_id("sess-123")
        .with_time_range(
            Some("2026-01-15T09:00:00Z".to_string()),
            Some("2026-01-15T10:30:00Z".to_string()),
        );

        assert_eq!(params.user_id, "user123");
        assert_eq!(params.source, "claude_code");
        assert_eq!(params.hours, 1.5);
        assert_eq!(params.description, Some("Test description".to_string()));
        assert_eq!(params.project_path, Some("/Users/test/project".to_string()));
        assert_eq!(params.session_id, Some("sess-123".to_string()));
        assert!(params.start_time.is_some());
        assert!(params.end_time.is_some());
    }
}
