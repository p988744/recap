//! Sync commands
//!
//! Tauri commands for sync operations.

use serde::{Deserialize, Serialize};
use tauri::State;

use recap_core::auth::verify_token;
use recap_core::models::SyncResult;
use recap_core::services::{sync_claude_projects, SyncService};

use super::AppState;

// Types

#[derive(Debug, Serialize)]
pub struct SyncStatus {
    pub id: String,
    pub source: String,
    pub source_path: Option<String>,
    pub last_sync_at: Option<String>,
    pub last_item_count: i32,
    pub status: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AutoSyncRequest {
    pub project_paths: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct AutoSyncResponse {
    pub success: bool,
    pub results: Vec<SyncResult>,
    pub total_items: i32,
}

#[derive(Debug, Serialize)]
pub struct AvailableProject {
    pub path: String,
    pub name: String,
    pub source: String,
}

// Commands

/// Get sync status for all sources
#[tauri::command]
pub async fn get_sync_status(
    state: State<'_, AppState>,
    token: String,
) -> Result<Vec<SyncStatus>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let sync_service = SyncService::new(db.pool.clone());

    let statuses = sync_service
        .get_sync_statuses(&claims.sub)
        .await
        .map_err(|e| e.to_string())?;

    Ok(statuses
        .into_iter()
        .map(|s| SyncStatus {
            id: s.id,
            source: s.source,
            source_path: s.source_path,
            last_sync_at: s.last_sync_at.map(|d| d.to_string()),
            last_item_count: s.last_item_count,
            status: s.status,
            error_message: s.error_message,
        })
        .collect())
}

/// Trigger auto-sync for Claude projects
#[tauri::command]
pub async fn auto_sync(
    state: State<'_, AppState>,
    token: String,
    request: AutoSyncRequest,
) -> Result<AutoSyncResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let sync_service = SyncService::new(db.pool.clone());

    // Get projects to sync - use cwd paths (actual project directories)
    let project_paths: Vec<String> = if let Some(paths) = request.project_paths {
        paths
    } else {
        // Get all available Claude projects (these are the cwd paths from sessions)
        SyncService::list_claude_projects()
            .into_iter()
            .filter_map(|p| {
                // Read first session to get actual cwd
                match std::fs::read_dir(&p) {
                    Ok(files) => {
                        for file in files.flatten() {
                            let path = file.path();
                            if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                                match std::fs::read_to_string(&path) {
                                    Ok(content) => {
                                        if let Some(first_line) = content.lines().next() {
                                            match serde_json::from_str::<serde_json::Value>(first_line) {
                                                Ok(msg) => {
                                                    if let Some(cwd) = msg.get("cwd").and_then(|v| v.as_str()) {
                                                        return Some(cwd.to_string());
                                                    }
                                                }
                                                Err(e) => {
                                                    log::debug!("Failed to parse session JSON in {:?}: {}", path, e);
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::debug!("Failed to read session file {:?}: {}", path, e);
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
    };

    if project_paths.is_empty() {
        return Ok(AutoSyncResponse {
            success: true,
            results: vec![],
            total_items: 0,
        });
    }

    // Get or create sync status for tracking
    let status = sync_service
        .get_or_create_status(&claims.sub, "claude", None)
        .await
        .map_err(|e| e.to_string())?;

    // Mark as syncing
    sync_service
        .mark_syncing(&status.id)
        .await
        .map_err(|e| e.to_string())?;

    // Call the actual sync logic
    let sync_result = match sync_claude_projects(&db.pool, &claims.sub, &project_paths).await {
        Ok(result) => result,
        Err(e) => {
            // Mark as error
            let _ = sync_service.mark_error(&status.id, &e).await;
            return Err(e);
        }
    };

    let item_count = (sync_result.work_items_created + sync_result.work_items_updated) as i32;

    // Mark as success
    sync_service
        .mark_success(&status.id, item_count)
        .await
        .map_err(|e| e.to_string())?;

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

    Ok(AutoSyncResponse {
        success: true,
        results,
        total_items: item_count,
    })
}

/// List available projects that can be synced
#[tauri::command]
pub async fn list_available_projects(
    token: String,
) -> Result<Vec<AvailableProject>, String> {
    let _claims = verify_token(&token).map_err(|e| e.to_string())?;

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

    Ok(projects)
}
