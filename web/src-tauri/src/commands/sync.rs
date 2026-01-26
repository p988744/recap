//! Sync commands
//!
//! Tauri commands for sync operations.
//! Uses trait-based dependency injection for testability.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;

use recap_core::auth::verify_token;
use recap_core::models::SyncResult;
use recap_core::services::SyncService;

use super::AppState;

// ============================================================================
// Request/Response types
// ============================================================================

#[derive(Debug, Clone, Serialize, Default)]
pub struct SyncStatus {
    pub id: String,
    pub source: String,
    pub source_path: Option<String>,
    pub last_sync_at: Option<String>,
    pub last_item_count: i32,
    pub status: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AutoSyncRequest {
    pub project_paths: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Default)]
pub struct AutoSyncResponse {
    pub success: bool,
    pub results: Vec<SyncResult>,
    pub total_items: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct AvailableProject {
    pub path: String,
    pub name: String,
    pub source: String,
}

// ============================================================================
// Core sync status model from recap_core
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct CoreSyncStatus {
    pub id: String,
    pub source: String,
    pub source_path: Option<String>,
    pub last_sync_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_item_count: i32,
    pub status: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ClaudeSyncResult {
    pub sessions_processed: i32,
    pub work_items_created: i32,
    pub work_items_updated: i32,
}

// ============================================================================
// Repository Trait
// ============================================================================

/// Sync repository trait - abstracts sync operations for testability
#[async_trait]
pub trait SyncRepository: Send + Sync {
    /// Get sync statuses for a user
    async fn get_sync_statuses(&self, user_id: &str) -> Result<Vec<CoreSyncStatus>, String>;

    /// Get or create sync status
    async fn get_or_create_status(
        &self,
        user_id: &str,
        source: &str,
        source_path: Option<&str>,
    ) -> Result<CoreSyncStatus, String>;

    /// Mark sync as in progress
    async fn mark_syncing(&self, status_id: &str) -> Result<(), String>;

    /// Mark sync as successful
    async fn mark_success(&self, status_id: &str, item_count: i32) -> Result<(), String>;

    /// Mark sync as error
    async fn mark_error(&self, status_id: &str, error: &str) -> Result<(), String>;

    /// Sync Claude projects
    async fn sync_claude_projects(
        &self,
        user_id: &str,
        project_paths: &[String],
    ) -> Result<ClaudeSyncResult, String>;

    /// List available Claude projects
    fn list_claude_projects(&self) -> Vec<PathBuf>;

    /// Discover Claude projects with git root resolution and multi-source discovery
    fn discover_projects(&self) -> Vec<recap_core::DiscoveredProject>;

    /// Sync Claude projects using discovered projects (with git root grouping)
    async fn sync_discovered_projects(
        &self,
        user_id: &str,
        projects: &[recap_core::DiscoveredProject],
    ) -> Result<ClaudeSyncResult, String>;
}

// ============================================================================
// SQLite Repository Implementation (Production)
// ============================================================================

/// SQLite implementation of SyncRepository
pub struct SqliteSyncRepository {
    pool: sqlx::SqlitePool,
}

impl SqliteSyncRepository {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SyncRepository for SqliteSyncRepository {
    async fn get_sync_statuses(&self, user_id: &str) -> Result<Vec<CoreSyncStatus>, String> {
        let sync_service = SyncService::new(self.pool.clone());
        let statuses = sync_service
            .get_sync_statuses(user_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(statuses
            .into_iter()
            .map(|s| CoreSyncStatus {
                id: s.id,
                source: s.source,
                source_path: s.source_path,
                last_sync_at: s.last_sync_at,
                last_item_count: s.last_item_count,
                status: s.status,
                error_message: s.error_message,
            })
            .collect())
    }

    async fn get_or_create_status(
        &self,
        user_id: &str,
        source: &str,
        source_path: Option<&str>,
    ) -> Result<CoreSyncStatus, String> {
        let sync_service = SyncService::new(self.pool.clone());
        let status = sync_service
            .get_or_create_status(user_id, source, source_path)
            .await
            .map_err(|e| e.to_string())?;

        Ok(CoreSyncStatus {
            id: status.id,
            source: status.source,
            source_path: status.source_path,
            last_sync_at: status.last_sync_at,
            last_item_count: status.last_item_count,
            status: status.status,
            error_message: status.error_message,
        })
    }

    async fn mark_syncing(&self, status_id: &str) -> Result<(), String> {
        let sync_service = SyncService::new(self.pool.clone());
        sync_service
            .mark_syncing(status_id)
            .await
            .map_err(|e| e.to_string())
    }

    async fn mark_success(&self, status_id: &str, item_count: i32) -> Result<(), String> {
        let sync_service = SyncService::new(self.pool.clone());
        sync_service
            .mark_success(status_id, item_count)
            .await
            .map_err(|e| e.to_string())
    }

    async fn mark_error(&self, status_id: &str, error: &str) -> Result<(), String> {
        let sync_service = SyncService::new(self.pool.clone());
        sync_service
            .mark_error(status_id, error)
            .await
            .map_err(|e| e.to_string())
    }

    async fn sync_claude_projects(
        &self,
        user_id: &str,
        project_paths: &[String],
    ) -> Result<ClaudeSyncResult, String> {
        let result =
            recap_core::services::sync_claude_projects(&self.pool, user_id, project_paths).await?;
        Ok(ClaudeSyncResult {
            sessions_processed: result.sessions_processed as i32,
            work_items_created: result.work_items_created as i32,
            work_items_updated: result.work_items_updated as i32,
        })
    }

    fn list_claude_projects(&self) -> Vec<PathBuf> {
        SyncService::list_claude_projects()
    }

    fn discover_projects(&self) -> Vec<recap_core::DiscoveredProject> {
        SyncService::discover_project_paths()
    }

    async fn sync_discovered_projects(
        &self,
        user_id: &str,
        projects: &[recap_core::DiscoveredProject],
    ) -> Result<ClaudeSyncResult, String> {
        let result =
            recap_core::services::sync_discovered_projects(&self.pool, user_id, projects).await?;
        Ok(ClaudeSyncResult {
            sessions_processed: result.sessions_processed as i32,
            work_items_created: result.work_items_created as i32,
            work_items_updated: result.work_items_updated as i32,
        })
    }
}

// ============================================================================
// Pure Business Logic (Testable without repository)
// ============================================================================

/// Convert CoreSyncStatus to SyncStatus for frontend
pub(crate) fn convert_sync_status(status: CoreSyncStatus) -> SyncStatus {
    SyncStatus {
        id: status.id,
        source: status.source,
        source_path: status.source_path,
        last_sync_at: status.last_sync_at.map(|d| d.to_string()),
        last_item_count: status.last_item_count,
        status: status.status,
        error_message: status.error_message,
    }
}

/// Extract project name from path
pub(crate) fn extract_project_name(path: &PathBuf) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

/// Build SyncResult from ClaudeSyncResult
pub(crate) fn build_sync_result(sync_result: &ClaudeSyncResult) -> SyncResult {
    let item_count =
        (sync_result.work_items_created + sync_result.work_items_updated) as i32;
    SyncResult {
        success: true,
        source: "claude".to_string(),
        items_synced: item_count,
        message: Some(format!(
            "Processed {} sessions, created {} items, updated {} items",
            sync_result.sessions_processed,
            sync_result.work_items_created,
            sync_result.work_items_updated
        )),
    }
}

/// Build empty sync response
pub(crate) fn build_empty_response() -> AutoSyncResponse {
    AutoSyncResponse {
        success: true,
        results: vec![],
        total_items: 0,
    }
}

/// Build successful sync response
pub(crate) fn build_success_response(sync_result: &ClaudeSyncResult) -> AutoSyncResponse {
    let item_count =
        (sync_result.work_items_created + sync_result.work_items_updated) as i32;
    AutoSyncResponse {
        success: true,
        results: vec![build_sync_result(sync_result)],
        total_items: item_count,
    }
}

/// Extract CWD from JSONL session file content.
/// Scans up to 100 lines to find the first line with a `cwd` field,
/// since many sessions now start with `type: "summary"` lines that lack `cwd`.
#[allow(dead_code)]
pub(crate) fn extract_cwd_from_session(content: &str) -> Option<String> {
    for line in content.lines().take(100) {
        if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(cwd) = msg.get("cwd").and_then(|v| v.as_str()) {
                if !cwd.is_empty() {
                    return Some(cwd.to_string());
                }
            }
        }
    }
    None
}

// ============================================================================
// Core Business Logic (Testable, uses trait)
// ============================================================================

/// Get sync status - testable business logic
pub async fn get_sync_status_impl<R: SyncRepository>(
    repo: &R,
    token: &str,
) -> Result<Vec<SyncStatus>, String> {
    let claims = verify_token(token).map_err(|e| e.to_string())?;
    let statuses = repo.get_sync_statuses(&claims.sub).await?;
    Ok(statuses.into_iter().map(convert_sync_status).collect())
}

/// Auto sync - testable business logic
pub async fn auto_sync_impl<R: SyncRepository>(
    repo: &R,
    token: &str,
    request: AutoSyncRequest,
) -> Result<AutoSyncResponse, String> {
    let claims = verify_token(token).map_err(|e| e.to_string())?;

    // Get or create sync status for tracking
    let status = repo
        .get_or_create_status(&claims.sub, "claude", None)
        .await?;

    // Mark as syncing
    repo.mark_syncing(&status.id).await?;

    // If explicit paths are provided, use legacy flow
    // Otherwise, use the new discovery-based flow
    let sync_result = if let Some(paths) = request.project_paths {
        if paths.is_empty() {
            let _ = repo.mark_success(&status.id, 0).await;
            return Ok(build_empty_response());
        }
        match repo.sync_claude_projects(&claims.sub, &paths).await {
            Ok(result) => result,
            Err(e) => {
                let _ = repo.mark_error(&status.id, &e).await;
                return Err(e);
            }
        }
    } else {
        // Use discovery-based sync with git root resolution
        let projects = repo.discover_projects();

        if projects.is_empty() {
            let _ = repo.mark_success(&status.id, 0).await;
            return Ok(build_empty_response());
        }

        match repo
            .sync_discovered_projects(&claims.sub, &projects)
            .await
        {
            Ok(result) => result,
            Err(e) => {
                let _ = repo.mark_error(&status.id, &e).await;
                return Err(e);
            }
        }
    };

    let item_count = (sync_result.work_items_created + sync_result.work_items_updated) as i32;

    // Mark as success
    repo.mark_success(&status.id, item_count).await?;

    Ok(build_success_response(&sync_result))
}

/// Extract CWDs from project directories (helper).
/// Uses sessions-index.json first, then extract_cwd() as fallback.
#[allow(dead_code)]
fn extract_project_cwds<R: SyncRepository>(repo: &R) -> Vec<String> {
    repo.list_claude_projects()
        .into_iter()
        .filter_map(|p| {
            // Strategy 1: Read sessions-index.json
            let index_path = p.join("sessions-index.json");
            if index_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&index_path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(project_path) = json
                            .get("entries")
                            .and_then(|e| e.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|entry| entry.get("projectPath"))
                            .and_then(|v| v.as_str())
                        {
                            if !project_path.is_empty() {
                                return Some(project_path.to_string());
                            }
                        }
                        if let Some(project_path) = json
                            .get("projectPath")
                            .and_then(|v| v.as_str())
                        {
                            if !project_path.is_empty() {
                                return Some(project_path.to_string());
                            }
                        }
                    }
                }
            }

            // Strategy 2: extract_cwd from JSONL files (scans multiple lines)
            match std::fs::read_dir(&p) {
                Ok(files) => {
                    for file in files.flatten() {
                        let path = file.path();
                        if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Some(cwd) = extract_cwd_from_session(&content) {
                                    return Some(cwd);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    log::debug!("Failed to read directory {:?}: {}", p, e);
                }
            }
            None
        })
        .collect()
}

/// List available projects - testable business logic
pub async fn list_available_projects_impl<R: SyncRepository>(
    repo: &R,
    token: &str,
) -> Result<Vec<AvailableProject>, String> {
    let _claims = verify_token(token).map_err(|e| e.to_string())?;

    let projects = repo
        .list_claude_projects()
        .into_iter()
        .map(|path| AvailableProject {
            name: extract_project_name(&path),
            path: path.to_string_lossy().to_string(),
            source: "claude".to_string(),
        })
        .collect();

    Ok(projects)
}

// ============================================================================
// Tauri Commands (Thin wrappers)
// ============================================================================

/// Get sync status for all sources
#[tauri::command]
pub async fn get_sync_status(
    state: State<'_, AppState>,
    token: String,
) -> Result<Vec<SyncStatus>, String> {
    let db = state.db.lock().await;
    let repo = SqliteSyncRepository::new(db.pool.clone());
    get_sync_status_impl(&repo, &token).await
}

/// Trigger auto-sync for Claude projects
#[tauri::command]
pub async fn auto_sync(
    state: State<'_, AppState>,
    token: String,
    request: AutoSyncRequest,
) -> Result<AutoSyncResponse, String> {
    let db = state.db.lock().await;
    let repo = SqliteSyncRepository::new(db.pool.clone());
    auto_sync_impl(&repo, &token, request).await
}

/// List available projects that can be synced
#[tauri::command]
pub async fn list_available_projects(
    state: State<'_, AppState>,
    token: String,
) -> Result<Vec<AvailableProject>, String> {
    let db = state.db.lock().await;
    let repo = SqliteSyncRepository::new(db.pool.clone());
    list_available_projects_impl(&repo, &token).await
}

// ============================================================================
// Tests with Mock Repository
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use recap_core::auth::create_token;
    use std::sync::Mutex;

    // ========================================================================
    // Mock Repository
    // ========================================================================

    pub struct MockSyncRepository {
        statuses: Mutex<Vec<CoreSyncStatus>>,
        projects: Vec<PathBuf>,
        sync_result: Option<ClaudeSyncResult>,
        should_fail: bool,
    }

    impl MockSyncRepository {
        pub fn new() -> Self {
            Self {
                statuses: Mutex::new(vec![]),
                projects: vec![],
                sync_result: None,
                should_fail: false,
            }
        }

        pub fn with_statuses(self, statuses: Vec<CoreSyncStatus>) -> Self {
            *self.statuses.lock().unwrap() = statuses;
            self
        }

        pub fn with_projects(mut self, projects: Vec<PathBuf>) -> Self {
            self.projects = projects;
            self
        }

        pub fn with_sync_result(mut self, result: ClaudeSyncResult) -> Self {
            self.sync_result = Some(result);
            self
        }

        pub fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }

        fn check_failure(&self) -> Result<(), String> {
            if self.should_fail {
                Err("Sync error".to_string())
            } else {
                Ok(())
            }
        }
    }

    #[async_trait]
    impl SyncRepository for MockSyncRepository {
        async fn get_sync_statuses(&self, _user_id: &str) -> Result<Vec<CoreSyncStatus>, String> {
            self.check_failure()?;
            Ok(self.statuses.lock().unwrap().clone())
        }

        async fn get_or_create_status(
            &self,
            _user_id: &str,
            source: &str,
            source_path: Option<&str>,
        ) -> Result<CoreSyncStatus, String> {
            self.check_failure()?;
            Ok(CoreSyncStatus {
                id: "status-1".to_string(),
                source: source.to_string(),
                source_path: source_path.map(|s| s.to_string()),
                last_sync_at: None,
                last_item_count: 0,
                status: "idle".to_string(),
                error_message: None,
            })
        }

        async fn mark_syncing(&self, _status_id: &str) -> Result<(), String> {
            self.check_failure()
        }

        async fn mark_success(&self, _status_id: &str, _item_count: i32) -> Result<(), String> {
            self.check_failure()
        }

        async fn mark_error(&self, _status_id: &str, _error: &str) -> Result<(), String> {
            Ok(()) // Never fail on error marking
        }

        async fn sync_claude_projects(
            &self,
            _user_id: &str,
            _project_paths: &[String],
        ) -> Result<ClaudeSyncResult, String> {
            self.check_failure()?;
            Ok(self.sync_result.clone().unwrap_or_default())
        }

        fn list_claude_projects(&self) -> Vec<PathBuf> {
            self.projects.clone()
        }

        fn discover_projects(&self) -> Vec<recap_core::DiscoveredProject> {
            // For mock: convert projects to DiscoveredProject (no git root resolution)
            self.projects
                .iter()
                .map(|p| recap_core::DiscoveredProject {
                    canonical_path: p.to_string_lossy().to_string(),
                    claude_dirs: vec![p.clone()],
                    name: p
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                })
                .collect()
        }

        async fn sync_discovered_projects(
            &self,
            _user_id: &str,
            _projects: &[recap_core::DiscoveredProject],
        ) -> Result<ClaudeSyncResult, String> {
            self.check_failure()?;
            Ok(self.sync_result.clone().unwrap_or_default())
        }
    }

    // Test user helper
    fn create_test_user() -> crate::models::User {
        crate::models::User {
            id: "user-1".to_string(),
            email: "test@test.com".to_string(),
            password_hash: "hash".to_string(),
            name: "Test User".to_string(),
            username: Some("testuser".to_string()),
            employee_id: None,
            department_id: None,
            title: None,
            gitlab_url: None,
            gitlab_pat: None,
            jira_url: None,
            jira_email: None,
            jira_pat: None,
            tempo_token: None,
            is_active: true,
            is_admin: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // ========================================================================
    // Pure Function Tests
    // ========================================================================

    #[test]
    fn test_convert_sync_status() {
        let core = CoreSyncStatus {
            id: "status-1".to_string(),
            source: "claude".to_string(),
            source_path: Some("/path/to/project".to_string()),
            last_sync_at: Some(Utc::now()),
            last_item_count: 5,
            status: "success".to_string(),
            error_message: None,
        };

        let result = convert_sync_status(core);

        assert_eq!(result.id, "status-1");
        assert_eq!(result.source, "claude");
        assert_eq!(result.source_path, Some("/path/to/project".to_string()));
        assert_eq!(result.last_item_count, 5);
        assert_eq!(result.status, "success");
        assert!(result.last_sync_at.is_some());
    }

    #[test]
    fn test_convert_sync_status_minimal() {
        let core = CoreSyncStatus {
            id: "status-2".to_string(),
            source: "gitlab".to_string(),
            source_path: None,
            last_sync_at: None,
            last_item_count: 0,
            status: "idle".to_string(),
            error_message: None,
        };

        let result = convert_sync_status(core);

        assert_eq!(result.id, "status-2");
        assert!(result.source_path.is_none());
        assert!(result.last_sync_at.is_none());
    }

    #[test]
    fn test_extract_project_name() {
        let path = PathBuf::from("/Users/test/projects/my-project");
        assert_eq!(extract_project_name(&path), "my-project");
    }

    #[test]
    fn test_extract_project_name_unknown() {
        let path = PathBuf::from("/");
        // Root path has no file name
        let name = extract_project_name(&path);
        assert_eq!(name, "Unknown");
    }

    #[test]
    fn test_build_sync_result() {
        let sync_result = ClaudeSyncResult {
            sessions_processed: 10,
            work_items_created: 5,
            work_items_updated: 3,
        };

        let result = build_sync_result(&sync_result);

        assert!(result.success);
        assert_eq!(result.source, "claude");
        assert_eq!(result.items_synced, 8); // 5 + 3
        assert!(result.message.is_some());
        assert!(result.message.unwrap().contains("10 sessions"));
    }

    #[test]
    fn test_build_empty_response() {
        let response = build_empty_response();

        assert!(response.success);
        assert!(response.results.is_empty());
        assert_eq!(response.total_items, 0);
    }

    #[test]
    fn test_build_success_response() {
        let sync_result = ClaudeSyncResult {
            sessions_processed: 5,
            work_items_created: 2,
            work_items_updated: 1,
        };

        let response = build_success_response(&sync_result);

        assert!(response.success);
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.total_items, 3);
    }

    #[test]
    fn test_extract_cwd_from_session_valid() {
        let content = r#"{"cwd": "/Users/test/project", "type": "init"}"#;
        let result = extract_cwd_from_session(content);
        assert_eq!(result, Some("/Users/test/project".to_string()));
    }

    #[test]
    fn test_extract_cwd_from_session_no_cwd() {
        let content = r#"{"type": "init"}"#;
        let result = extract_cwd_from_session(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_cwd_from_session_invalid_json() {
        let content = "not json";
        let result = extract_cwd_from_session(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_cwd_from_session_empty() {
        let content = "";
        let result = extract_cwd_from_session(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_cwd_from_session_summary_first_line() {
        // Simulates sessions that start with "type: summary" and no cwd
        let content = r#"{"type": "summary", "timestamp": "2026-01-01T00:00:00Z"}
{"type": "progress", "timestamp": "2026-01-01T00:01:00Z"}
{"cwd": "/Users/test/deep/project", "type": "human", "timestamp": "2026-01-01T00:02:00Z"}"#;
        let result = extract_cwd_from_session(content);
        assert_eq!(result, Some("/Users/test/deep/project".to_string()));
    }

    #[test]
    fn test_extract_cwd_from_session_empty_cwd_skipped() {
        let content = r#"{"cwd": "", "type": "init"}
{"cwd": "/Users/real/path", "type": "human"}"#;
        let result = extract_cwd_from_session(content);
        assert_eq!(result, Some("/Users/real/path".to_string()));
    }

    // ========================================================================
    // get_sync_status Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_sync_status_success() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let statuses = vec![
            CoreSyncStatus {
                id: "status-1".to_string(),
                source: "claude".to_string(),
                source_path: None,
                last_sync_at: Some(Utc::now()),
                last_item_count: 10,
                status: "success".to_string(),
                error_message: None,
            },
            CoreSyncStatus {
                id: "status-2".to_string(),
                source: "gitlab".to_string(),
                source_path: None,
                last_sync_at: None,
                last_item_count: 0,
                status: "idle".to_string(),
                error_message: None,
            },
        ];
        let repo = MockSyncRepository::new().with_statuses(statuses);

        let result = get_sync_status_impl(&repo, &token).await.unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].source, "claude");
        assert_eq!(result[1].source, "gitlab");
    }

    #[tokio::test]
    async fn test_get_sync_status_empty() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let repo = MockSyncRepository::new();

        let result = get_sync_status_impl(&repo, &token).await.unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_sync_status_invalid_token() {
        let repo = MockSyncRepository::new();

        let result = get_sync_status_impl(&repo, "invalid-token").await;

        assert!(result.is_err());
    }

    // ========================================================================
    // auto_sync Tests
    // ========================================================================

    #[tokio::test]
    async fn test_auto_sync_with_paths() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let sync_result = ClaudeSyncResult {
            sessions_processed: 5,
            work_items_created: 3,
            work_items_updated: 2,
        };
        let repo = MockSyncRepository::new().with_sync_result(sync_result);

        let request = AutoSyncRequest {
            project_paths: Some(vec!["/path/to/project".to_string()]),
        };

        let result = auto_sync_impl(&repo, &token, request).await.unwrap();

        assert!(result.success);
        assert_eq!(result.total_items, 5);
        assert_eq!(result.results.len(), 1);
    }

    #[tokio::test]
    async fn test_auto_sync_empty_paths() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let repo = MockSyncRepository::new(); // No projects

        let request = AutoSyncRequest {
            project_paths: Some(vec![]), // Empty paths
        };

        let result = auto_sync_impl(&repo, &token, request).await.unwrap();

        assert!(result.success);
        assert_eq!(result.total_items, 0);
        assert!(result.results.is_empty());
    }

    #[tokio::test]
    async fn test_auto_sync_invalid_token() {
        let repo = MockSyncRepository::new();
        let request = AutoSyncRequest::default();

        let result = auto_sync_impl(&repo, "invalid-token", request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_auto_sync_sync_failure() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let repo = MockSyncRepository::new().with_failure();

        let request = AutoSyncRequest {
            project_paths: Some(vec!["/path/to/project".to_string()]),
        };

        let result = auto_sync_impl(&repo, &token, request).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Sync error"));
    }

    // ========================================================================
    // list_available_projects Tests
    // ========================================================================

    #[tokio::test]
    async fn test_list_available_projects_success() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let projects = vec![
            PathBuf::from("/Users/test/.claude/projects/project-a"),
            PathBuf::from("/Users/test/.claude/projects/project-b"),
        ];
        let repo = MockSyncRepository::new().with_projects(projects);

        let result = list_available_projects_impl(&repo, &token).await.unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "project-a");
        assert_eq!(result[0].source, "claude");
        assert_eq!(result[1].name, "project-b");
    }

    #[tokio::test]
    async fn test_list_available_projects_empty() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let repo = MockSyncRepository::new();

        let result = list_available_projects_impl(&repo, &token).await.unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_list_available_projects_invalid_token() {
        let repo = MockSyncRepository::new();

        let result = list_available_projects_impl(&repo, "invalid-token").await;

        assert!(result.is_err());
    }
}
