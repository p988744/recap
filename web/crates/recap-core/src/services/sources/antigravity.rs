//! Antigravity (Gemini Code) Source Implementation
//!
//! This module implements the SyncSource trait for Antigravity sessions.
//! Antigravity is Gemini Code's local language server that provides an HTTP API
//! for accessing code session data.
//!
//! ## Data Flow
//!
//! 1. **API Sync**: Fetch session metadata from Antigravity HTTP API
//! 2. **Local Files**: Read detailed session data from ~/.gemini/tmp/*/chats/
//! 3. **Snapshot Capture**: Store hourly buckets in snapshot_raw_data
//! 4. **Compaction**: LLM generates summaries (handled by compaction service)

use async_trait::async_trait;
use chrono::{DateTime, Local, Timelike};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::process::Command;

use super::{SyncSource, SourceProject, SourceSyncResult, WorkItemParams, upsert_work_item, UpsertResult};
use crate::services::snapshot::save_hourly_snapshots;
use crate::services::worklog::calculate_session_hours;

/// Antigravity (Gemini Code) data source
///
/// Syncs work items from Antigravity's local HTTP API when the app is running.
pub struct AntigravitySource {
    /// Cached connection info (port and CSRF token)
    connection: Option<AntigravityConnection>,
}

/// Connection info for Antigravity API
#[derive(Debug, Clone)]
pub struct AntigravityConnection {
    pub port: u16,
    pub csrf_token: String,
}

impl AntigravitySource {
    /// Create a new Antigravity source
    pub fn new() -> Self {
        Self { connection: None }
    }

    /// Create a source with pre-configured connection
    pub fn with_connection(connection: AntigravityConnection) -> Self {
        Self {
            connection: Some(connection),
        }
    }

    /// Detect running Antigravity process and extract connection info
    pub fn detect_connection() -> Option<AntigravityConnection> {
        find_antigravity_process()
    }

    /// Get or detect connection
    fn get_connection(&self) -> Option<AntigravityConnection> {
        self.connection.clone().or_else(find_antigravity_process)
    }
}

impl Default for AntigravitySource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SyncSource for AntigravitySource {
    fn source_name(&self) -> &'static str {
        "antigravity"
    }

    fn display_name(&self) -> &'static str {
        "Antigravity (Gemini Code)"
    }

    async fn is_available(&self) -> bool {
        self.get_connection().is_some()
    }

    async fn discover_projects(&self) -> Result<Vec<SourceProject>, String> {
        let connection = self.get_connection()
            .ok_or_else(|| "Antigravity is not running".to_string())?;

        let api_response = fetch_all_trajectories(&connection).await?;
        let projects = convert_to_projects(api_response);

        Ok(projects
            .into_iter()
            .map(|p| SourceProject {
                name: p.name,
                path: p.path,
                session_count: p.sessions.len(),
            })
            .collect())
    }

    async fn sync_sessions(
        &self,
        pool: &SqlitePool,
        user_id: &str,
    ) -> Result<SourceSyncResult, String> {
        let mut result = SourceSyncResult::new(self.source_name());

        // Get connection (needed for both phases)
        let connection = match self.get_connection() {
            Some(conn) => conn,
            None => {
                log::debug!("Antigravity not running, skipping sync");
                return Ok(result);
            }
        };

        // Phase 1: Sync from API - get session metadata (timestamps, summaries, etc.)
        let api_projects = match fetch_all_trajectories(&connection).await {
            Ok(api_response) => {
                let projects = convert_to_projects(api_response);
                result.projects_scanned = projects.len();

                for project in &projects {
                    for session in &project.sessions {
                        match process_session(pool, user_id, session, self.source_name()).await {
                            Ok(ProcessResult::Created) => {
                                result.sessions_processed += 1;
                                result.work_items_created += 1;
                            }
                            Ok(ProcessResult::Updated) => {
                                result.sessions_processed += 1;
                                result.work_items_updated += 1;
                            }
                            Ok(ProcessResult::Skipped) => {
                                result.sessions_skipped += 1;
                            }
                            Err(e) => {
                                log::error!("Failed to process Antigravity session: {}", e);
                                result.sessions_skipped += 1;
                            }
                        }
                    }
                }
                projects
            }
            Err(e) => {
                log::warn!("Failed to fetch from Antigravity API: {}", e);
                Vec::new()
            }
        };

        // Phase 2: Capture detailed snapshots from API (GetCascadeTrajectorySteps)
        // This enables LLM-powered summary generation via compaction
        let snapshots_captured = capture_api_snapshots(pool, user_id, &connection, &api_projects).await;
        log::info!(
            "Antigravity sync: {} work items, {} snapshots captured",
            result.work_items_created + result.work_items_updated,
            snapshots_captured
        );

        Ok(result)
    }
}

// ==================== API Types ====================

#[derive(Debug, Deserialize)]
struct ApiResponse {
    #[serde(rename = "trajectorySummaries")]
    trajectory_summaries: Option<HashMap<String, TrajectorySummary>>,
}

#[derive(Debug, Deserialize)]
struct StepsResponse {
    steps: Option<Vec<CascadeStep>>,
}

#[derive(Debug, Deserialize, Clone)]
struct CascadeStep {
    #[serde(rename = "type")]
    step_type: Option<String>,
    status: Option<String>,
    metadata: Option<StepMetadata>,
    #[serde(rename = "userInput")]
    user_input: Option<UserInput>,
    #[serde(rename = "plannerResponse")]
    planner_response: Option<PlannerResponse>,
    #[serde(rename = "codeAction")]
    code_action: Option<CodeAction>,
    #[serde(rename = "runCommand")]
    run_command: Option<RunCommand>,
    #[serde(rename = "notifyUser")]
    notify_user: Option<NotifyUser>,
}

#[derive(Debug, Deserialize, Clone)]
struct StepMetadata {
    #[serde(rename = "createdAt")]
    created_at: Option<String>,
    #[serde(rename = "completedAt")]
    completed_at: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct UserInput {
    #[serde(rename = "userResponse")]
    user_response: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct PlannerResponse {
    #[serde(rename = "modelResponseText")]
    model_response_text: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct CodeAction {
    #[serde(rename = "filePath")]
    file_path: Option<String>,
    #[serde(rename = "actionType")]
    action_type: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct RunCommand {
    command: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct NotifyUser {
    message: Option<String>,
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

// ==================== Internal Types ====================

/// A discovered Antigravity project
#[derive(Debug)]
struct AntigravityProject {
    path: String,
    name: String,
    sessions: Vec<AntigravitySession>,
}

/// An Antigravity session
#[derive(Debug, Clone)]
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

enum ProcessResult {
    Created,
    Updated,
    Skipped,
}

// ==================== Process Discovery ====================

/// Find the running Antigravity language server process
fn find_antigravity_process() -> Option<AntigravityConnection> {
    let output = Command::new("ps")
        .args(["aux"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        if line.contains("language_server_macos") || line.contains("language_server_linux") {
            let csrf_token = extract_csrf_token(line)?;
            let port = extract_server_port(line)?;
            return Some(AntigravityConnection { port, csrf_token });
        }
    }

    None
}

/// Extract server_port from process command line
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
    let parts: Vec<&str> = line.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == "--csrf_token" {
            return parts.get(i + 1).map(|s| s.to_string());
        }
    }
    None
}

// ==================== API Client ====================

/// Fetch all trajectories from Antigravity API
async fn fetch_all_trajectories(connection: &AntigravityConnection) -> Result<ApiResponse, String> {
    let url = format!(
        "https://localhost:{}/exa.language_server_pb.LanguageServerService/GetAllCascadeTrajectories",
        connection.port
    );

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Connect-Protocol-Version", "1")
        .header("X-Codeium-Csrf-Token", &connection.csrf_token)
        .body("{}")
        .send()
        .await
        .map_err(|e| format!("API request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API returned error status: {}", response.status()));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))
}

/// Fetch detailed steps for a specific trajectory
async fn fetch_trajectory_steps(
    connection: &AntigravityConnection,
    cascade_id: &str,
    start_index: usize,
    end_index: usize,
) -> Result<StepsResponse, String> {
    let url = format!(
        "https://localhost:{}/exa.language_server_pb.LanguageServerService/GetCascadeTrajectorySteps",
        connection.port
    );

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let body = serde_json::json!({
        "cascadeId": cascade_id,
        "startIndex": start_index,
        "endIndex": end_index
    });

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Connect-Protocol-Version", "1")
        .header("X-Codeium-Csrf-Token", &connection.csrf_token)
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| format!("API request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API returned error status: {}", response.status()));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse steps response: {}", e))
}

/// Convert API response to internal project/session structure
fn convert_to_projects(api_response: ApiResponse) -> Vec<AntigravityProject> {
    let mut projects_map: HashMap<String, Vec<AntigravitySession>> = HashMap::new();

    if let Some(trajectories) = api_response.trajectory_summaries {
        for (session_id, trajectory) in trajectories {
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
        let a_latest = a.sessions.first().and_then(|s| s.last_timestamp.as_ref());
        let b_latest = b.sessions.first().and_then(|s| s.last_timestamp.as_ref());
        b_latest.cmp(&a_latest)
    });

    projects
}

// ==================== Session Processing ====================

/// Process a single Antigravity session into a work item
async fn process_session(
    pool: &SqlitePool,
    user_id: &str,
    session: &AntigravitySession,
    source_name: &str,
) -> Result<ProcessResult, String> {
    if session.step_count == 0 {
        return Ok(ProcessResult::Skipped);
    }

    let hours = session_hours_from_timestamps(&session.first_timestamp, &session.last_timestamp);

    let date = session
        .first_timestamp
        .as_ref()
        .and_then(|ts| ts.split('T').next())
        .unwrap_or("2026-01-01")
        .to_string();

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

    let description = build_session_description(session, hours);

    let params = WorkItemParams::new(
        user_id,
        source_name,
        &session.session_id,
        title,
        hours,
        &date,
    )
    .with_description(description)
    .with_project_path(&session.cwd)
    .with_session_id(&session.session_id)
    .with_time_range(session.first_timestamp.clone(), session.last_timestamp.clone());

    match upsert_work_item(pool, params).await {
        Ok(UpsertResult::Created(_)) => Ok(ProcessResult::Created),
        Ok(UpsertResult::Updated(_)) => Ok(ProcessResult::Updated),
        Ok(UpsertResult::Skipped(_)) => Ok(ProcessResult::Skipped),
        Err(e) => Err(e),
    }
}

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

// ==================== Snapshot Capture ====================

/// Capture snapshots from Antigravity API
///
/// This fetches detailed step data using GetCascadeTrajectorySteps API
/// and saves them to snapshot_raw_data for LLM-powered summary generation.
async fn capture_api_snapshots(
    pool: &SqlitePool,
    user_id: &str,
    connection: &AntigravityConnection,
    api_projects: &[AntigravityProject],
) -> usize {
    let mut total_saved = 0;

    for project in api_projects {
        for session in &project.sessions {
            // Skip sessions with no steps
            if session.step_count == 0 {
                continue;
            }

            // Fetch detailed steps from API (batch of 100 at a time)
            let mut all_steps = Vec::new();
            let batch_size = 100;
            let mut start = 0;

            while start < session.step_count {
                let end = std::cmp::min(start + batch_size, session.step_count);
                match fetch_trajectory_steps(connection, &session.session_id, start, end).await {
                    Ok(response) => {
                        if let Some(steps) = response.steps {
                            all_steps.extend(steps);
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to fetch steps for session {}: {}",
                            session.session_id,
                            e
                        );
                        break;
                    }
                }
                start = end;
            }

            if all_steps.is_empty() {
                continue;
            }

            // Convert steps to hourly buckets
            let buckets = convert_steps_to_hourly_buckets(&all_steps);

            if buckets.is_empty() {
                continue;
            }

            // Save to snapshot_raw_data
            match save_hourly_snapshots(
                pool,
                user_id,
                &session.session_id,
                &session.cwd,
                &buckets,
            )
            .await
            {
                Ok(saved) => {
                    total_saved += saved;
                    log::debug!(
                        "Saved {} snapshots for Antigravity session {} ({} steps)",
                        saved,
                        session.session_id,
                        all_steps.len()
                    );
                }
                Err(e) => {
                    log::warn!(
                        "Failed to save snapshots for session {}: {}",
                        session.session_id,
                        e
                    );
                }
            }
        }
    }

    total_saved
}

/// Convert Cascade steps to hourly buckets for snapshot storage
fn convert_steps_to_hourly_buckets(steps: &[CascadeStep]) -> Vec<crate::services::snapshot::HourlyBucket> {
    use crate::services::snapshot::{HourlyBucket, ToolCallRecord};
    use std::collections::HashMap as StdHashMap;

    let mut buckets_map: StdHashMap<String, HourlyBucket> = StdHashMap::new();

    for step in steps {
        // Get timestamp from metadata
        let timestamp = step
            .metadata
            .as_ref()
            .and_then(|m| m.created_at.as_ref())
            .cloned()
            .unwrap_or_default();

        // Truncate to hour bucket
        let hour_bucket = match truncate_to_hour(&timestamp) {
            Some(h) => h,
            None => continue,
        };

        let bucket = buckets_map.entry(hour_bucket.clone()).or_insert_with(|| HourlyBucket {
            hour_bucket,
            user_messages: Vec::new(),
            assistant_summaries: Vec::new(),
            tool_calls: Vec::new(),
            files_modified: Vec::new(),
            git_commits: Vec::new(),
            message_count: 0,
        });

        let step_type = step.step_type.as_deref().unwrap_or("");

        match step_type {
            "CORTEX_STEP_TYPE_USER_INPUT" => {
                if let Some(ui) = &step.user_input {
                    if let Some(msg) = &ui.user_response {
                        let truncated: String = msg.chars().take(500).collect();
                        if !truncated.trim().is_empty() {
                            bucket.user_messages.push(truncated);
                            bucket.message_count += 1;
                        }
                    }
                }
            }
            "CORTEX_STEP_TYPE_PLANNER_RESPONSE" | "CORTEX_STEP_TYPE_NOTIFY_USER" => {
                let msg = step
                    .planner_response
                    .as_ref()
                    .and_then(|pr| pr.model_response_text.as_ref())
                    .or_else(|| step.notify_user.as_ref().and_then(|nu| nu.message.as_ref()));

                if let Some(text) = msg {
                    let truncated: String = text.chars().take(200).collect();
                    bucket.assistant_summaries.push(truncated);
                }
            }
            "CORTEX_STEP_TYPE_CODE_ACTION" => {
                if let Some(ca) = &step.code_action {
                    if let Some(path) = &ca.file_path {
                        if !bucket.files_modified.contains(path) {
                            bucket.files_modified.push(path.clone());
                        }
                        bucket.tool_calls.push(ToolCallRecord {
                            tool: ca.action_type.clone().unwrap_or_else(|| "CodeAction".to_string()),
                            input_summary: path.clone(),
                            timestamp: timestamp.clone(),
                        });
                    }
                }
            }
            "CORTEX_STEP_TYPE_RUN_COMMAND" => {
                if let Some(rc) = &step.run_command {
                    if let Some(cmd) = &rc.command {
                        bucket.tool_calls.push(ToolCallRecord {
                            tool: "RunCommand".to_string(),
                            input_summary: cmd.chars().take(200).collect(),
                            timestamp: timestamp.clone(),
                        });
                    }
                }
            }
            "CORTEX_STEP_TYPE_VIEW_FILE" | "CORTEX_STEP_TYPE_GREP_SEARCH" |
            "CORTEX_STEP_TYPE_FIND" | "CORTEX_STEP_TYPE_LIST_DIRECTORY" => {
                bucket.tool_calls.push(ToolCallRecord {
                    tool: step_type.replace("CORTEX_STEP_TYPE_", ""),
                    input_summary: String::new(),
                    timestamp: timestamp.clone(),
                });
            }
            _ => {}
        }
    }

    // Convert to sorted vec
    let mut buckets: Vec<HourlyBucket> = buckets_map.into_values().collect();
    buckets.sort_by(|a, b| a.hour_bucket.cmp(&b.hour_bucket));
    buckets
}

/// Truncate ISO timestamp to hour boundary in local timezone
fn truncate_to_hour(timestamp: &str) -> Option<String> {
    let dt = DateTime::parse_from_rfc3339(timestamp).ok()?;
    let local_dt: DateTime<Local> = dt.with_timezone(&Local);
    let truncated = local_dt
        .with_minute(0)?
        .with_second(0)?
        .with_nanosecond(0)?;
    Some(truncated.format("%Y-%m-%dT%H:%M:%S").to_string())
}

// ==================== Public API for Commands ====================

/// Sync Antigravity projects - public API for Tauri commands
///
/// This function is used by the Tauri command layer for explicit sync requests.
pub async fn sync_antigravity_projects(
    pool: &SqlitePool,
    user_id: &str,
    project_paths: &[String],
) -> Result<SourceSyncResult, String> {
    let connection = find_antigravity_process()
        .ok_or_else(|| "Antigravity is not running. Please start the Antigravity app.".to_string())?;

    let api_response = fetch_all_trajectories(&connection).await?;
    let projects = convert_to_projects(api_response);

    let mut result = SourceSyncResult::new("antigravity");

    // Filter to requested projects
    let requested_paths: std::collections::HashSet<_> = project_paths.iter().collect();

    for project in projects {
        if !requested_paths.contains(&project.path) {
            continue;
        }

        result.projects_scanned += 1;

        for session in &project.sessions {
            match process_session(pool, user_id, session, "antigravity").await {
                Ok(ProcessResult::Created) => {
                    result.sessions_processed += 1;
                    result.work_items_created += 1;
                }
                Ok(ProcessResult::Updated) => {
                    result.sessions_processed += 1;
                    result.work_items_updated += 1;
                }
                Ok(ProcessResult::Skipped) => {
                    result.sessions_skipped += 1;
                }
                Err(e) => {
                    log::error!("Failed to process session: {}", e);
                    result.sessions_skipped += 1;
                }
            }
        }
    }

    Ok(result)
}

/// List all Antigravity sessions - public API for Tauri commands
pub async fn list_antigravity_sessions() -> Result<Vec<SourceProject>, String> {
    let source = AntigravitySource::new();
    source.discover_projects().await
}

/// Check if Antigravity is available
pub fn is_antigravity_available() -> bool {
    find_antigravity_process().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_antigravity_source_name() {
        let source = AntigravitySource::new();
        assert_eq!(source.source_name(), "antigravity");
        assert_eq!(source.display_name(), "Antigravity (Gemini Code)");
    }

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
