//! Data models for the Recap application

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// User model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub name: String,
    pub username: Option<String>,
    pub employee_id: Option<String>,
    pub department_id: Option<String>,
    pub title: Option<String>,
    pub gitlab_url: Option<String>,
    pub gitlab_pat: Option<String>,
    pub jira_url: Option<String>,
    pub jira_email: Option<String>,
    pub jira_pat: Option<String>,
    pub tempo_token: Option<String>,
    pub is_active: bool,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User response (without sensitive fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
    pub username: Option<String>,
    pub employee_id: Option<String>,
    pub department_id: Option<String>,
    pub title: Option<String>,
    pub gitlab_url: Option<String>,
    pub jira_email: Option<String>,
    pub is_active: bool,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
            username: user.username,
            employee_id: user.employee_id,
            department_id: user.department_id,
            title: user.title,
            gitlab_url: user.gitlab_url,
            jira_email: user.jira_email,
            is_active: user.is_active,
            is_admin: user.is_admin,
            created_at: user.created_at,
        }
    }
}

/// Work item model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkItem {
    pub id: String,
    pub user_id: String,
    pub source: String,           // "gitlab", "claude_code", "manual", "commit"
    pub source_id: Option<String>,
    pub source_url: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub hours: f64,
    pub date: NaiveDate,
    pub jira_issue_key: Option<String>,
    pub jira_issue_suggested: Option<String>,
    pub jira_issue_title: Option<String>,
    pub category: Option<String>,
    pub tags: Option<String>,     // JSON array
    pub yearly_goal_id: Option<String>,
    pub synced_to_tempo: bool,
    pub tempo_worklog_id: Option<String>,
    pub synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub parent_id: Option<String>, // For grouping: child items link to parent
    // Commit-centric fields
    pub hours_source: Option<String>,    // 'user_modified' | 'session' | 'commit_interval' | 'heuristic' | 'manual'
    pub hours_estimated: Option<f64>,    // System-calculated hours (preserved even if user overrides)
    pub commit_hash: Option<String>,     // Git commit hash for commit-based items
    pub session_id: Option<String>,      // Claude session ID for linking
    // Timeline support fields
    pub start_time: Option<String>,      // ISO 8601 timestamp for session start
    pub end_time: Option<String>,        // ISO 8601 timestamp for session end
    pub project_path: Option<String>,    // Project path for filtering
}

/// Hours source enum for clarity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HoursSource {
    UserModified,    // User manually changed the hours
    Session,         // Calculated from linked Claude session
    CommitInterval,  // Estimated from time between commits
    Heuristic,       // Estimated from lines/files changed
    Manual,          // Default for manually created items
}

impl HoursSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            HoursSource::UserModified => "user_modified",
            HoursSource::Session => "session",
            HoursSource::CommitInterval => "commit_interval",
            HoursSource::Heuristic => "heuristic",
            HoursSource::Manual => "manual",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "user_modified" => HoursSource::UserModified,
            "session" => HoursSource::Session,
            "commit_interval" => HoursSource::CommitInterval,
            "heuristic" => HoursSource::Heuristic,
            _ => HoursSource::Manual,
        }
    }
}

/// GitLab project model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GitLabProject {
    pub id: String,
    pub user_id: String,
    pub gitlab_project_id: i64,
    pub name: String,
    pub path_with_namespace: String,
    pub gitlab_url: String,
    pub default_branch: String,
    pub enabled: bool,
    pub last_synced: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// App configuration (stored in config file, not DB)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub llm_provider: String,
    pub llm_model: String,
    pub llm_api_key: Option<String>,
    pub llm_base_url: Option<String>,
    pub daily_work_hours: f64,
    pub normalize_hours: bool,
}

/// JWT Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // user id
    pub email: String,
    pub exp: i64,
}

/// Create work item request
#[derive(Debug, Deserialize)]
pub struct CreateWorkItem {
    pub title: String,
    pub description: Option<String>,
    pub hours: Option<f64>,
    pub date: NaiveDate,
    pub source: Option<String>,
    pub source_id: Option<String>,
    pub jira_issue_key: Option<String>,
    pub jira_issue_title: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Update work item request
#[derive(Debug, Deserialize)]
pub struct UpdateWorkItem {
    pub title: Option<String>,
    pub description: Option<String>,
    pub hours: Option<f64>,
    pub date: Option<NaiveDate>,
    pub jira_issue_key: Option<String>,
    pub jira_issue_title: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub synced_to_tempo: Option<bool>,
    pub tempo_worklog_id: Option<String>,
}

/// Work item filters
#[derive(Debug, Deserialize, Default)]
pub struct WorkItemFilters {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub source: Option<String>,
    pub category: Option<String>,
    pub jira_mapped: Option<bool>,
    pub synced_to_tempo: Option<bool>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub search: Option<String>,
    pub parent_id: Option<String>,  // Filter by parent (get children)
    pub show_all: Option<bool>,     // Show all items including children
}

/// Paginated response
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub pages: i64,
}

/// Sync status model for tracking auto-sync state
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SyncStatus {
    pub id: String,
    pub user_id: String,
    pub source: String,              // "claude", "gitlab", "local_git"
    pub source_path: Option<String>, // Project path or GitLab project_id
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_item_count: i32,
    pub status: String,              // "idle", "syncing", "error", "success"
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Sync status response for API
#[derive(Debug, Serialize)]
pub struct SyncStatusResponse {
    pub id: String,
    pub source: String,
    pub source_path: Option<String>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_item_count: i32,
    pub status: String,
    pub error_message: Option<String>,
}

impl From<SyncStatus> for SyncStatusResponse {
    fn from(s: SyncStatus) -> Self {
        Self {
            id: s.id,
            source: s.source,
            source_path: s.source_path,
            last_sync_at: s.last_sync_at,
            last_item_count: s.last_item_count,
            status: s.status,
            error_message: s.error_message,
        }
    }
}

/// Sync result for API response
#[derive(Debug, Serialize)]
pub struct SyncResult {
    pub success: bool,
    pub source: String,
    pub items_synced: i32,
    pub message: Option<String>,
}

/// Local Git repository model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GitRepo {
    pub id: String,
    pub user_id: String,
    pub path: String,
    pub name: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

/// Git repo info for API response (includes runtime validation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRepoInfo {
    pub id: String,
    pub path: String,
    pub name: String,
    pub valid: bool,
    pub last_commit: Option<String>,
    pub last_commit_date: Option<String>,
}

/// Sources response for API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesResponse {
    pub mode: String,
    pub git_repos: Vec<GitRepoInfo>,
    pub claude_connected: bool,
    pub claude_path: Option<String>,
}

/// Worklog entry for Tempo sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorklogEntry {
    pub issue_key: String,
    pub date: String,
    pub minutes: i32,
    pub description: String,
}

/// Sync worklogs request
#[derive(Debug, Clone, Deserialize)]
pub struct SyncWorklogsRequest {
    pub entries: Vec<WorklogEntry>,
    pub dry_run: bool,
}

/// Individual worklog sync result
#[derive(Debug, Clone, Serialize)]
pub struct WorklogSyncResult {
    pub id: Option<String>,
    pub issue_key: String,
    pub date: String,
    pub minutes: i32,
    pub hours: f64,
    pub description: String,
    pub status: String,
    pub error_message: Option<String>,
}

/// Sync worklogs response
#[derive(Debug, Clone, Serialize)]
pub struct SyncWorklogsResponse {
    pub success: bool,
    pub total_entries: i32,
    pub successful: i32,
    pub failed: i32,
    pub results: Vec<WorklogSyncResult>,
    pub dry_run: bool,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // ========================================================================
    // HoursSource Tests
    // ========================================================================

    #[test]
    fn test_hours_source_as_str() {
        assert_eq!(HoursSource::UserModified.as_str(), "user_modified");
        assert_eq!(HoursSource::Session.as_str(), "session");
        assert_eq!(HoursSource::CommitInterval.as_str(), "commit_interval");
        assert_eq!(HoursSource::Heuristic.as_str(), "heuristic");
        assert_eq!(HoursSource::Manual.as_str(), "manual");
    }

    #[test]
    fn test_hours_source_from_str() {
        assert_eq!(HoursSource::from_str("user_modified"), HoursSource::UserModified);
        assert_eq!(HoursSource::from_str("session"), HoursSource::Session);
        assert_eq!(HoursSource::from_str("commit_interval"), HoursSource::CommitInterval);
        assert_eq!(HoursSource::from_str("heuristic"), HoursSource::Heuristic);
        assert_eq!(HoursSource::from_str("manual"), HoursSource::Manual);
    }

    #[test]
    fn test_hours_source_from_str_unknown() {
        // Unknown strings should default to Manual
        assert_eq!(HoursSource::from_str("unknown"), HoursSource::Manual);
        assert_eq!(HoursSource::from_str(""), HoursSource::Manual);
        assert_eq!(HoursSource::from_str("MANUAL"), HoursSource::Manual); // Case sensitive
    }

    #[test]
    fn test_hours_source_roundtrip() {
        let sources = [
            HoursSource::UserModified,
            HoursSource::Session,
            HoursSource::CommitInterval,
            HoursSource::Heuristic,
            HoursSource::Manual,
        ];

        for source in sources {
            let str_val = source.as_str();
            let parsed = HoursSource::from_str(str_val);
            assert_eq!(parsed, source);
        }
    }

    // ========================================================================
    // User to UserResponse Tests
    // ========================================================================

    fn create_test_user() -> User {
        User {
            id: "user-123".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "secret_hash".to_string(),
            name: "Test User".to_string(),
            username: Some("testuser".to_string()),
            employee_id: Some("EMP001".to_string()),
            department_id: Some("DEPT001".to_string()),
            title: Some("Developer".to_string()),
            gitlab_url: Some("https://gitlab.com".to_string()),
            gitlab_pat: Some("secret_pat".to_string()),
            jira_url: Some("https://jira.com".to_string()),
            jira_email: Some("test@jira.com".to_string()),
            jira_pat: Some("jira_secret".to_string()),
            tempo_token: Some("tempo_secret".to_string()),
            is_active: true,
            is_admin: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_user_to_user_response_conversion() {
        let user = create_test_user();
        let created_at = user.created_at;

        let response: UserResponse = user.into();

        assert_eq!(response.id, "user-123");
        assert_eq!(response.email, "test@example.com");
        assert_eq!(response.name, "Test User");
        assert_eq!(response.username, Some("testuser".to_string()));
        assert_eq!(response.employee_id, Some("EMP001".to_string()));
        assert_eq!(response.department_id, Some("DEPT001".to_string()));
        assert_eq!(response.title, Some("Developer".to_string()));
        assert_eq!(response.gitlab_url, Some("https://gitlab.com".to_string()));
        assert_eq!(response.jira_email, Some("test@jira.com".to_string()));
        assert!(response.is_active);
        assert!(!response.is_admin);
        assert_eq!(response.created_at, created_at);
    }

    #[test]
    fn test_user_to_user_response_excludes_sensitive_fields() {
        let user = create_test_user();
        let response: UserResponse = user.into();

        // UserResponse should not contain these sensitive fields
        // We verify by checking the struct doesn't have password_hash, gitlab_pat, etc.
        // Since they're not in UserResponse, we just verify the conversion compiles
        // and the response contains expected non-sensitive fields
        assert!(!response.id.is_empty());
        assert!(!response.email.is_empty());
    }

    #[test]
    fn test_user_to_user_response_with_none_fields() {
        let user = User {
            id: "user-456".to_string(),
            email: "minimal@example.com".to_string(),
            password_hash: "hash".to_string(),
            name: "Minimal User".to_string(),
            username: None,
            employee_id: None,
            department_id: None,
            title: None,
            gitlab_url: None,
            gitlab_pat: None,
            jira_url: None,
            jira_email: None,
            jira_pat: None,
            tempo_token: None,
            is_active: false,
            is_admin: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response: UserResponse = user.into();

        assert_eq!(response.id, "user-456");
        assert!(response.username.is_none());
        assert!(response.employee_id.is_none());
        assert!(response.department_id.is_none());
        assert!(response.title.is_none());
        assert!(response.gitlab_url.is_none());
        assert!(response.jira_email.is_none());
        assert!(!response.is_active);
        assert!(response.is_admin);
    }

    // ========================================================================
    // SyncStatus to SyncStatusResponse Tests
    // ========================================================================

    fn create_test_sync_status() -> SyncStatus {
        SyncStatus {
            id: "sync-123".to_string(),
            user_id: "user-123".to_string(),
            source: "claude".to_string(),
            source_path: Some("/path/to/project".to_string()),
            last_sync_at: Some(Utc::now()),
            last_item_count: 42,
            status: "success".to_string(),
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_sync_status_to_response_conversion() {
        let status = create_test_sync_status();
        let last_sync_at = status.last_sync_at;

        let response: SyncStatusResponse = status.into();

        assert_eq!(response.id, "sync-123");
        assert_eq!(response.source, "claude");
        assert_eq!(response.source_path, Some("/path/to/project".to_string()));
        assert_eq!(response.last_sync_at, last_sync_at);
        assert_eq!(response.last_item_count, 42);
        assert_eq!(response.status, "success");
        assert!(response.error_message.is_none());
    }

    #[test]
    fn test_sync_status_to_response_with_error() {
        let status = SyncStatus {
            id: "sync-456".to_string(),
            user_id: "user-123".to_string(),
            source: "gitlab".to_string(),
            source_path: None,
            last_sync_at: None,
            last_item_count: 0,
            status: "error".to_string(),
            error_message: Some("Connection failed".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response: SyncStatusResponse = status.into();

        assert_eq!(response.id, "sync-456");
        assert_eq!(response.source, "gitlab");
        assert!(response.source_path.is_none());
        assert!(response.last_sync_at.is_none());
        assert_eq!(response.last_item_count, 0);
        assert_eq!(response.status, "error");
        assert_eq!(response.error_message, Some("Connection failed".to_string()));
    }

    #[test]
    fn test_sync_status_excludes_internal_fields() {
        let status = create_test_sync_status();
        let response: SyncStatusResponse = status.into();

        // SyncStatusResponse should not contain user_id, created_at, updated_at
        // We verify by checking the response contains expected fields only
        assert!(!response.id.is_empty());
        assert!(!response.source.is_empty());
    }

    // ========================================================================
    // Serialization Tests
    // ========================================================================

    #[test]
    fn test_hours_source_serialization() {
        let source = HoursSource::Session;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"Session\"");

        let deserialized: HoursSource = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, HoursSource::Session);
    }

    #[test]
    fn test_sync_result_serialization() {
        let result = SyncResult {
            success: true,
            source: "claude".to_string(),
            items_synced: 10,
            message: Some("Synced successfully".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"source\":\"claude\""));
        assert!(json.contains("\"items_synced\":10"));
    }

    #[test]
    fn test_worklog_entry_deserialization() {
        let json = r#"{
            "issue_key": "PROJ-123",
            "date": "2024-01-15",
            "minutes": 60,
            "description": "Working on feature"
        }"#;

        let entry: WorklogEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.issue_key, "PROJ-123");
        assert_eq!(entry.date, "2024-01-15");
        assert_eq!(entry.minutes, 60);
        assert_eq!(entry.description, "Working on feature");
    }

    #[test]
    fn test_paginated_response_serialization() {
        let response = PaginatedResponse {
            items: vec!["item1".to_string(), "item2".to_string()],
            total: 100,
            page: 1,
            per_page: 10,
            pages: 10,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":100"));
        assert!(json.contains("\"page\":1"));
        assert!(json.contains("\"pages\":10"));
    }
}
