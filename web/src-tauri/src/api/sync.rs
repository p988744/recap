//! Sync API routes
//!
//! Provides endpoints for:
//! - Getting sync status for all sources
//! - Triggering auto-sync
//! - Getting sync history

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    db::Database,
    models::SyncResult,
    services::{sync_claude_projects, SyncService},
};

/// Sync routes
pub fn routes() -> Router<Database> {
    Router::new()
        .route("/status", get(get_sync_status))
        .route("/auto", post(auto_sync))
        .route("/projects", get(list_available_projects))
}

/// Response for available projects
#[derive(Debug, Serialize)]
pub struct AvailableProject {
    pub path: String,
    pub name: String,
    pub source: String,
}

/// Get sync status for all sources
async fn get_sync_status(
    State(db): State<Database>,
    auth: AuthUser,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let sync_service = SyncService::new(db.pool.clone());

    let statuses = sync_service
        .get_sync_statuses(&auth.0.sub)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(statuses))
}

/// Request for auto-sync
#[derive(Debug, Deserialize)]
pub struct AutoSyncRequest {
    /// Specific project paths to sync (empty = all)
    pub project_paths: Option<Vec<String>>,
}

/// Response for auto-sync
#[derive(Debug, Serialize)]
pub struct AutoSyncResponse {
    pub success: bool,
    pub results: Vec<SyncResult>,
    pub total_items: i32,
}

/// Trigger auto-sync for Claude projects
async fn auto_sync(
    State(db): State<Database>,
    auth: AuthUser,
    Json(req): Json<AutoSyncRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let sync_service = SyncService::new(db.pool.clone());

    // Get projects to sync - use cwd paths (actual project directories)
    let project_paths: Vec<String> = if let Some(paths) = req.project_paths {
        paths
    } else {
        // Get all available Claude projects (these are the cwd paths from sessions)
        SyncService::list_claude_projects()
            .into_iter()
            .filter_map(|p| {
                // Read first session to get actual cwd
                if let Ok(files) = std::fs::read_dir(&p) {
                    for file in files.flatten() {
                        let path = file.path();
                        if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Some(first_line) = content.lines().next() {
                                    if let Ok(msg) = serde_json::from_str::<serde_json::Value>(first_line) {
                                        if let Some(cwd) = msg.get("cwd").and_then(|v| v.as_str()) {
                                            return Some(cwd.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                None
            })
            .collect()
    };

    if project_paths.is_empty() {
        return Ok(Json(AutoSyncResponse {
            success: true,
            results: vec![],
            total_items: 0,
        }));
    }

    // Get or create sync status for tracking
    let status = sync_service
        .get_or_create_status(&auth.0.sub, "claude", None)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // Mark as syncing
    sync_service
        .mark_syncing(&status.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // Call the actual sync logic
    let sync_result = match sync_claude_projects(&db.pool, &auth.0.sub, &project_paths).await {
        Ok(result) => result,
        Err(e) => {
            // Mark as error
            let _ = sync_service.mark_error(&status.id, &e).await;
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e));
        }
    };

    let item_count = (sync_result.work_items_created + sync_result.work_items_updated) as i32;

    // Mark as success
    sync_service
        .mark_success(&status.id, item_count)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let results = vec![SyncResult {
        success: true,
        source: "claude".to_string(),
        items_synced: item_count,
        message: Some(format!(
            "Processed {} sessions, created {} items, updated {} items",
            sync_result.sessions_processed,
            sync_result.work_items_created,
            sync_result.work_items_updated
        )),
    }];

    Ok(Json(AutoSyncResponse {
        success: true,
        results,
        total_items: item_count,
    }))
}

/// List available projects that can be synced
async fn list_available_projects(
    _auth: AuthUser,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut projects = Vec::new();

    // List Claude projects
    for path in SyncService::list_claude_projects() {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        projects.push(AvailableProject {
            path: path.to_string_lossy().to_string(),
            name,
            source: "claude".to_string(),
        });
    }

    Ok(Json(projects))
}
