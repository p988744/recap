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

/// Raw snapshot data for hourly session buckets
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SnapshotRawData {
    pub id: String,
    pub user_id: String,
    pub session_id: String,
    pub project_path: String,
    pub hour_bucket: String,
    pub user_messages: Option<String>,
    pub assistant_messages: Option<String>,
    pub tool_calls: Option<String>,
    pub files_modified: Option<String>,
    pub git_commits: Option<String>,
    pub message_count: i32,
    pub raw_size_bytes: i32,
    pub created_at: DateTime<Utc>,
}

/// Work summary at various time scales (hourly, daily, weekly, monthly)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkSummary {
    pub id: String,
    pub user_id: String,
    pub project_path: Option<String>,
    pub scale: String,
    pub period_start: String,
    pub period_end: String,
    pub summary: String,
    pub key_activities: Option<String>,
    pub git_commits_summary: Option<String>,
    pub previous_context: Option<String>,
    pub source_snapshot_ids: Option<String>,
    pub llm_model: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
