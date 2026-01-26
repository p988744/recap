//! # recap-core
//!
//! Core business logic for Recap - shared between CLI and Tauri.
//!
//! This crate provides:
//! - Database operations (`db` module)
//! - Data models (`models` module)
//! - Business logic services (`services` module)
//! - Authentication utilities (`auth` module)
//! - Unified error handling (`error` module)

pub mod auth;
pub mod db;
pub mod error;
pub mod models;
pub mod services;

// Re-exports for convenience
pub use db::Database;
pub use error::{Error, Result};

// Re-export commonly used types from models
pub use models::{
    AppConfig, Claims, CreateWorkItem, GitLabProject, GitRepo, GitRepoInfo, HoursSource,
    PaginatedResponse, SourcesResponse, SyncResult, SyncStatus, SyncStatusResponse,
    SyncWorklogsRequest, SyncWorklogsResponse, UpdateWorkItem, User, UserResponse, WorkItem,
    WorkItemFilters, WorklogEntry, WorklogSyncResult,
};

// Re-export commonly used types from services
pub use services::{
    build_rule_based_outcome, calculate_session_hours, create_llm_service, create_sync_service,
    estimate_commit_hours, estimate_from_diff, extract_cwd, extract_tool_detail,
    generate_daily_hash, get_commits_for_date, get_commits_in_time_range, is_meaningful_message,
    parse_session_fast, parse_session_full, resolve_git_root, sync_claude_projects,
    sync_discovered_projects, ClaudeSyncResult, CommitRecord, DailyWorklog, DiscoveredProject,
    ExcelReportGenerator, ExcelWorkItem, FileChange, HoursEstimate, JiraAuthType, JiraClient,
    ParsedSession, ProjectSummary, ReportMetadata, SessionBrief, SessionMetadata,
    StandaloneSession, SyncService, TempoClient, TimelineCommit, ToolUsage,
    WorklogEntry as TempoWorklogEntry, WorklogUploader,
};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns the library version
pub fn version() -> &'static str {
    VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_exists() {
        assert!(!version().is_empty());
    }

    #[test]
    fn test_version_format() {
        let v = version();
        // Should be semver format: x.y.z
        let parts: Vec<&str> = v.split('.').collect();
        assert_eq!(parts.len(), 3, "Version should be in x.y.z format");
    }
}
