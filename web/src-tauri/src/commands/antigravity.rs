//! Antigravity (Gemini Code) session commands
//!
//! Tauri commands for Antigravity session operations.
//! Uses the local Antigravity HTTP API when the app is running.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use tauri::State;

use recap_core::auth::verify_token;
use recap_core::services::{calculate_session_hours, generate_daily_hash};

use super::AppState;

// ==================== Public Types ====================

#[derive(Debug, Serialize)]
pub struct AntigravityApiStatus {
    pub running: bool,
    pub api_url: Option<String>,
    pub healthy: bool,
    pub session_count: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct AntigravityProject {
    pub path: String,
    pub name: String,
    pub sessions: Vec<AntigravitySession>,
}

#[derive(Debug, Serialize, Clone)]
pub struct AntigravitySession {
    pub session_id: String,
    pub summary: Option<String>,
    pub cwd: String,
    pub git_branch: Option<String>,
    pub git_repo: Option<String>,
    pub step_count: usize,
    pub first_timestamp: Option<String>,
    pub last_timestamp: Option<String>,
    pub status: String,
}


#[derive(Debug, Serialize)]
pub struct AntigravitySyncResult {
    pub sessions_processed: usize,
    pub sessions_skipped: usize,
    pub work_items_created: usize,
    pub work_items_updated: usize,
}

// ==================== API Response Types ====================

#[derive(Debug, Deserialize)]
struct ApiResponse {
    #[serde(rename = "trajectorySummaries")]
    trajectory_summaries: Option<HashMap<String, TrajectorySummary>>,
}

#[derive(Debug, Deserialize, Clone)]
struct TrajectorySummary {
    summary: Option<String>,
    #[serde(rename = "stepCount")]
    step_count: Option<usize>,
    #[serde(rename = "createdTime")]
    created_time: Option<String>,
    #[serde(rename = "lastModifiedTime")]
    last_modified_time: Option<String>,
    status: Option<String>,
    workspaces: Option<Vec<Workspace>>,
}

#[derive(Debug, Deserialize, Clone)]
struct Workspace {
    #[serde(rename = "workspaceFolderAbsoluteUri")]
    workspace_folder_absolute_uri: Option<String>,
    #[serde(rename = "branchName")]
    branch_name: Option<String>,
    repository: Option<Repository>,
}

#[derive(Debug, Deserialize, Clone)]
struct Repository {
    #[serde(rename = "computedName")]
    computed_name: Option<String>,
    #[serde(rename = "gitOriginUrl")]
    git_origin_url: Option<String>,
}

// ==================== Process Discovery ====================

/// Information about a running Antigravity process
struct AntigravityProcess {
    csrf_token: String,
    port: u16,
}

/// Find the running Antigravity language server process and extract connection info
fn find_antigravity_process() -> Option<AntigravityProcess> {
    // Run ps command to find the process
    let output = Command::new("ps")
        .args(["aux"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Find the language_server process line
    for line in stdout.lines() {
        if line.contains("language_server_macos") || line.contains("language_server_linux") {
            // Extract CSRF token
            let csrf_token = extract_csrf_token(line)?;

            // Extract server_port directly from command line (more reliable than lsof)
            let port = extract_server_port(line)?;

            return Some(AntigravityProcess { csrf_token, port });
        }
    }

    None
}

/// Extract server_port from process command line
/// First tries --server_port, then falls back to --extension_server_port + 1
fn extract_server_port(line: &str) -> Option<u16> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    // First try explicit --server_port
    for (i, part) in parts.iter().enumerate() {
        if *part == "--server_port" {
            if let Some(port) = parts.get(i + 1).and_then(|p| p.parse().ok()) {
                return Some(port);
            }
        }
    }

    // Fall back to --extension_server_port + 1
    // When --random_port is used, server_port = extension_server_port + 1
    for (i, part) in parts.iter().enumerate() {
        if *part == "--extension_server_port" {
            if let Some(ext_port) = parts.get(i + 1).and_then(|p| p.parse::<u16>().ok()) {
                return Some(ext_port + 1);
            }
        }
    }

    None
}

/// Extract CSRF token from process command line
fn extract_csrf_token(line: &str) -> Option<String> {
    // Look for --csrf_token argument
    let parts: Vec<&str> = line.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == "--csrf_token" {
            return parts.get(i + 1).map(|s| s.to_string());
        }
    }
    None
}


// ==================== API Client ====================

/// Call the Antigravity API to get all sessions
async fn fetch_all_trajectories(process: &AntigravityProcess) -> Result<ApiResponse, String> {
    let url = format!(
        "https://localhost:{}/exa.language_server_pb.LanguageServerService/GetAllCascadeTrajectories",
        process.port
    );

    // Create a client that accepts self-signed certificates
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Connect-Protocol-Version", "1")
        .header("X-Codeium-Csrf-Token", &process.csrf_token)
        .body("{}")
        .send()
        .await
        .map_err(|e| format!("API request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API returned error status: {}", response.status()));
    }

    let api_response: ApiResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    Ok(api_response)
}

/// Convert API response to our project/session structure
fn convert_to_projects(api_response: ApiResponse) -> Vec<AntigravityProject> {
    let mut projects_map: HashMap<String, Vec<AntigravitySession>> = HashMap::new();

    if let Some(trajectories) = api_response.trajectory_summaries {
        for (session_id, trajectory) in trajectories {
            // Extract workspace info
            let (cwd, git_branch, git_repo) = if let Some(workspaces) = &trajectory.workspaces {
                if let Some(ws) = workspaces.first() {
                    let path = ws
                        .workspace_folder_absolute_uri
                        .as_ref()
                        .map(|u| u.trim_start_matches("file://").to_string())
                        .unwrap_or_default();
                    let branch = ws.branch_name.clone();
                    let repo = ws.repository.as_ref().and_then(|r| r.computed_name.clone());
                    (path, branch, repo)
                } else {
                    (String::new(), None, None)
                }
            } else {
                (String::new(), None, None)
            };

            // Skip sessions without a workspace
            if cwd.is_empty() {
                continue;
            }

            let session = AntigravitySession {
                session_id,
                summary: trajectory.summary,
                cwd: cwd.clone(),
                git_branch,
                git_repo,
                step_count: trajectory.step_count.unwrap_or(0),
                first_timestamp: trajectory.created_time,
                last_timestamp: trajectory.last_modified_time,
                status: trajectory.status.unwrap_or_else(|| "UNKNOWN".to_string()),
            };

            projects_map.entry(cwd).or_default().push(session);
        }
    }

    // Convert to Vec<AntigravityProject>
    let mut projects: Vec<AntigravityProject> = projects_map
        .into_iter()
        .map(|(path, mut sessions)| {
            // Sort sessions by last modified time (newest first)
            sessions.sort_by(|a, b| {
                b.last_timestamp
                    .as_ref()
                    .unwrap_or(&String::new())
                    .cmp(a.last_timestamp.as_ref().unwrap_or(&String::new()))
            });

            let name = std::path::Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            AntigravityProject {
                path,
                name,
                sessions,
            }
        })
        .collect();

    // Sort projects by latest session timestamp
    projects.sort_by(|a, b| {
        let a_latest = a
            .sessions
            .first()
            .and_then(|s| s.last_timestamp.as_ref());
        let b_latest = b
            .sessions
            .first()
            .and_then(|s| s.last_timestamp.as_ref());
        b_latest.cmp(&a_latest)
    });

    projects
}

// ==================== Helper Functions ====================

fn session_hours_from_timestamps(first: &Option<String>, last: &Option<String>) -> f64 {
    match (first, last) {
        (Some(start), Some(end)) => calculate_session_hours(start, end),
        _ => 0.5,
    }
}

fn build_session_description(session: &AntigravitySession, hours: f64) -> String {
    let mut parts = vec![
        format!("📁 Project: {}", session.cwd),
        format!(
            "🌿 Branch: {}",
            session.git_branch.as_deref().unwrap_or("N/A")
        ),
        format!("💬 Steps: {} | ⏱️ Duration: {:.1}h", session.step_count, hours),
    ];

    if let Some(repo) = &session.git_repo {
        parts.push(format!("🔗 Repository: {}", repo));
    }

    if let Some(summary) = &session.summary {
        parts.push(format!("📋 Summary: {}", summary));
    }

    parts.join("\n\n")
}

// ==================== Public Request/Response Types ====================

#[derive(Debug, Deserialize)]
pub struct AntigravitySyncProjectsRequest {
    pub project_paths: Vec<String>,
}

// ==================== Internal Functions (for background sync) ====================

/// List all Antigravity sessions - internal version without Tauri state
pub async fn list_antigravity_sessions_internal() -> Result<Vec<AntigravityProject>, String> {
    let process = find_antigravity_process()
        .ok_or_else(|| "No Antigravity process found. The app may not be running.".to_string())?;

    let api_response = fetch_all_trajectories(&process).await?;
    let projects = convert_to_projects(api_response);

    Ok(projects)
}

/// Sync Antigravity projects - internal version for background sync
pub async fn sync_antigravity_projects_internal(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    request: AntigravitySyncProjectsRequest,
) -> Result<AntigravitySyncResult, String> {
    let process = find_antigravity_process()
        .ok_or_else(|| "Antigravity is not running. Please start the Antigravity app.".to_string())?;

    let api_response = fetch_all_trajectories(&process).await?;
    let projects = convert_to_projects(api_response);

    let mut sessions_processed = 0;
    let mut sessions_skipped = 0;
    let mut work_items_created = 0;
    let mut work_items_updated = 0;

    // Filter to requested projects
    let requested_paths: std::collections::HashSet<_> = request.project_paths.iter().collect();

    for project in projects {
        if !requested_paths.contains(&project.path) {
            continue;
        }

        for session in &project.sessions {
            match process_session(pool, user_id, session).await {
                Ok(ProcessResult::Created) => {
                    sessions_processed += 1;
                    work_items_created += 1;
                }
                Ok(ProcessResult::Updated) => {
                    sessions_processed += 1;
                    work_items_updated += 1;
                }
                Ok(ProcessResult::Skipped) => {
                    sessions_skipped += 1;
                }
                Err(e) => {
                    log::error!("Failed to process session: {}", e);
                    sessions_skipped += 1;
                }
            }
        }
    }

    Ok(AntigravitySyncResult {
        sessions_processed,
        sessions_skipped,
        work_items_created,
        work_items_updated,
    })
}

// ==================== Tauri Commands ====================

/// Check if Antigravity is running (process exists)
#[tauri::command]
pub async fn check_antigravity_installed(
    _state: State<'_, AppState>,
    token: String,
) -> Result<bool, String> {
    let _claims = verify_token(&token).map_err(|e| e.to_string())?;
    Ok(find_antigravity_process().is_some())
}

/// Check Antigravity API status - returns URL and health check result
#[tauri::command]
pub async fn check_antigravity_api_status(
    _state: State<'_, AppState>,
    token: String,
) -> Result<AntigravityApiStatus, String> {
    let _claims = verify_token(&token).map_err(|e| e.to_string())?;

    let process = match find_antigravity_process() {
        Some(p) => p,
        None => {
            return Ok(AntigravityApiStatus {
                running: false,
                api_url: None,
                healthy: false,
                session_count: None,
            });
        }
    };

    let api_url = format!("https://localhost:{}", process.port);

    // Try to fetch sessions to verify API is healthy
    match fetch_all_trajectories(&process).await {
        Ok(response) => {
            let session_count = response
                .trajectory_summaries
                .as_ref()
                .map(|t| t.len());
            Ok(AntigravityApiStatus {
                running: true,
                api_url: Some(api_url),
                healthy: true,
                session_count,
            })
        }
        Err(_) => Ok(AntigravityApiStatus {
            running: true,
            api_url: Some(api_url),
            healthy: false,
            session_count: None,
        }),
    }
}

/// List all Antigravity sessions from the running app
#[tauri::command]
pub async fn list_antigravity_sessions(
    _state: State<'_, AppState>,
    token: String,
) -> Result<Vec<AntigravityProject>, String> {
    let _claims = verify_token(&token).map_err(|e| e.to_string())?;

    let process = find_antigravity_process()
        .ok_or_else(|| "Antigravity is not running. Please start the Antigravity app.".to_string())?;

    let api_response = fetch_all_trajectories(&process).await?;
    let projects = convert_to_projects(api_response);

    Ok(projects)
}

/// Sync selected Antigravity projects - create work items from sessions
#[tauri::command]
pub async fn sync_antigravity_projects(
    state: State<'_, AppState>,
    token: String,
    request: AntigravitySyncProjectsRequest,
) -> Result<AntigravitySyncResult, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let process = find_antigravity_process()
        .ok_or_else(|| "Antigravity is not running. Please start the Antigravity app.".to_string())?;

    let api_response = fetch_all_trajectories(&process).await?;
    let projects = convert_to_projects(api_response);

    let mut sessions_processed = 0;
    let mut sessions_skipped = 0;
    let mut work_items_created = 0;
    let mut work_items_updated = 0;

    // Filter to requested projects
    let requested_paths: std::collections::HashSet<_> = request.project_paths.iter().collect();

    for project in projects {
        if !requested_paths.contains(&project.path) {
            continue;
        }

        for session in &project.sessions {
            match process_session(&db.pool, &claims.sub, session).await {
                Ok(ProcessResult::Created) => {
                    sessions_processed += 1;
                    work_items_created += 1;
                }
                Ok(ProcessResult::Updated) => {
                    sessions_processed += 1;
                    work_items_updated += 1;
                }
                Ok(ProcessResult::Skipped) => {
                    sessions_skipped += 1;
                }
                Err(e) => {
                    log::error!("Failed to process session: {}", e);
                    sessions_skipped += 1;
                }
            }
        }
    }

    Ok(AntigravitySyncResult {
        sessions_processed,
        sessions_skipped,
        work_items_created,
        work_items_updated,
    })
}

enum ProcessResult {
    Created,
    Updated,
    Skipped,
}

async fn process_session(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    session: &AntigravitySession,
) -> Result<ProcessResult, String> {
    if session.step_count == 0 {
        return Ok(ProcessResult::Skipped);
    }

    let hours = session_hours_from_timestamps(&session.first_timestamp, &session.last_timestamp);

    let date = session
        .first_timestamp
        .as_ref()
        .and_then(|ts| ts.split('T').next())
        .unwrap_or("2026-01-01");

    let content_hash = generate_daily_hash(user_id, &session.cwd, date);

    // Check if already exists
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT id FROM work_items WHERE content_hash = ? AND user_id = ?")
            .bind(&content_hash)
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;

    if existing.is_some() {
        return Ok(ProcessResult::Skipped);
    }

    let project_name = std::path::Path::new(&session.cwd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown");

    let title = if let Some(ref summary) = session.summary {
        let truncated: String = summary.chars().take(80).collect();
        format!("[{}] {}", project_name, truncated)
    } else {
        format!("[{}] Gemini Code session", project_name)
    };

    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();
    let description = build_session_description(session, hours);

    sqlx::query(
        r#"INSERT INTO work_items
        (id, user_id, source, source_id, title, description, hours, date, content_hash, hours_source, hours_estimated, project_path, created_at, updated_at)
        VALUES (?, ?, 'antigravity', ?, ?, ?, ?, ?, ?, 'session', ?, ?, ?, ?)"#
    )
    .bind(&id)
    .bind(user_id)
    .bind(&session.session_id)
    .bind(&title)
    .bind(&description)
    .bind(hours)
    .bind(date)
    .bind(&content_hash)
    .bind(hours)
    .bind(&session.cwd) // project_path
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(ProcessResult::Created)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_csrf_token() {
        let line = "/Applications/Antigravity.app/Contents/Resources/app/extensions/antigravity/bin/language_server_macos_arm --enable_lsp --extension_server_port 52500 --csrf_token abc123-def456 --random_port";
        let token = extract_csrf_token(line);
        assert_eq!(token, Some("abc123-def456".to_string()));
    }

    #[test]
    fn test_extract_csrf_token_not_found() {
        let line = "/some/other/process --flag value";
        let token = extract_csrf_token(line);
        assert_eq!(token, None);
    }

    #[test]
    fn test_extract_server_port() {
        let line = "weifanliao 38824 /Applications/Antigravity.app/Contents/Resources/app/extensions/antigravity/bin/language_server_macos_arm --enable_lsp --extension_server_port 52500 --csrf_token abc123 --server_port 52501 --lsp_port 52507";
        let port = extract_server_port(line);
        assert_eq!(port, Some(52501));
    }

    #[test]
    fn test_extract_server_port_from_extension_port() {
        // When --random_port is used, server_port is not in command line
        // We fall back to extension_server_port + 1
        let line = "weifanliao 44334 /Applications/Antigravity.app/Contents/Resources/app/extensions/antigravity/bin/language_server_macos_arm --enable_lsp --extension_server_port 64115 --csrf_token abc123 --random_port";
        let port = extract_server_port(line);
        assert_eq!(port, Some(64116));
    }

    #[test]
    fn test_session_hours_from_timestamps() {
        let first = Some("2024-01-15T09:00:00Z".to_string());
        let last = Some("2024-01-15T11:00:00Z".to_string());
        let hours = session_hours_from_timestamps(&first, &last);
        assert!((hours - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_session_hours_from_timestamps_none() {
        let hours = session_hours_from_timestamps(&None, &None);
        assert!((hours - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_convert_to_projects() {
        let mut trajectories = HashMap::new();
        trajectories.insert(
            "session-1".to_string(),
            TrajectorySummary {
                summary: Some("Test summary".to_string()),
                step_count: Some(10),
                created_time: Some("2024-01-15T09:00:00Z".to_string()),
                last_modified_time: Some("2024-01-15T10:00:00Z".to_string()),
                status: Some("CASCADE_RUN_STATUS_IDLE".to_string()),
                workspaces: Some(vec![Workspace {
                    workspace_folder_absolute_uri: Some("file:///Users/test/project".to_string()),
                    branch_name: Some("main".to_string()),
                    repository: Some(Repository {
                        computed_name: Some("test/project".to_string()),
                        git_origin_url: Some("https://github.com/test/project.git".to_string()),
                    }),
                }]),
            },
        );

        let api_response = ApiResponse {
            trajectory_summaries: Some(trajectories),
        };

        let projects = convert_to_projects(api_response);

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "project");
        assert_eq!(projects[0].path, "/Users/test/project");
        assert_eq!(projects[0].sessions.len(), 1);
        assert_eq!(projects[0].sessions[0].session_id, "session-1");
        assert_eq!(projects[0].sessions[0].summary, Some("Test summary".to_string()));
        assert_eq!(projects[0].sessions[0].git_branch, Some("main".to_string()));
    }

    #[test]
    fn test_convert_to_projects_groups_by_path() {
        let mut trajectories = HashMap::new();

        // Two sessions for the same project
        trajectories.insert(
            "session-1".to_string(),
            TrajectorySummary {
                summary: Some("First session".to_string()),
                step_count: Some(10),
                created_time: Some("2024-01-15T09:00:00Z".to_string()),
                last_modified_time: Some("2024-01-15T10:00:00Z".to_string()),
                status: None,
                workspaces: Some(vec![Workspace {
                    workspace_folder_absolute_uri: Some("file:///Users/test/project-a".to_string()),
                    branch_name: None,
                    repository: None,
                }]),
            },
        );

        trajectories.insert(
            "session-2".to_string(),
            TrajectorySummary {
                summary: Some("Second session".to_string()),
                step_count: Some(20),
                created_time: Some("2024-01-16T09:00:00Z".to_string()),
                last_modified_time: Some("2024-01-16T10:00:00Z".to_string()),
                status: None,
                workspaces: Some(vec![Workspace {
                    workspace_folder_absolute_uri: Some("file:///Users/test/project-a".to_string()),
                    branch_name: None,
                    repository: None,
                }]),
            },
        );

        let api_response = ApiResponse {
            trajectory_summaries: Some(trajectories),
        };

        let projects = convert_to_projects(api_response);

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].sessions.len(), 2);
    }

    #[test]
    fn test_convert_to_projects_skips_empty_workspace() {
        let mut trajectories = HashMap::new();

        trajectories.insert(
            "session-no-workspace".to_string(),
            TrajectorySummary {
                summary: Some("No workspace".to_string()),
                step_count: Some(5),
                created_time: None,
                last_modified_time: None,
                status: None,
                workspaces: None,
            },
        );

        let api_response = ApiResponse {
            trajectory_summaries: Some(trajectories),
        };

        let projects = convert_to_projects(api_response);

        assert_eq!(projects.len(), 0);
    }

    #[test]
    fn test_build_session_description() {
        let session = AntigravitySession {
            session_id: "test-123".to_string(),
            summary: Some("Implement feature X".to_string()),
            cwd: "/Users/test/project".to_string(),
            git_branch: Some("feature/x".to_string()),
            git_repo: Some("test/project".to_string()),
            step_count: 50,
            first_timestamp: Some("2024-01-15T09:00:00Z".to_string()),
            last_timestamp: Some("2024-01-15T11:00:00Z".to_string()),
            status: "IDLE".to_string(),
        };

        let desc = build_session_description(&session, 2.0);

        assert!(desc.contains("📁 Project: /Users/test/project"));
        assert!(desc.contains("🌿 Branch: feature/x"));
        assert!(desc.contains("💬 Steps: 50"));
        assert!(desc.contains("⏱️ Duration: 2.0h"));
        assert!(desc.contains("🔗 Repository: test/project"));
        assert!(desc.contains("📋 Summary: Implement feature X"));
    }
}
