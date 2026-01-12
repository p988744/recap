//! Sources commands
//!
//! Tauri commands for managing data sources (Git repos, Claude, etc.)

use serde::Serialize;
use std::path::Path;
use std::process::Command;
use tauri::State;
use uuid::Uuid;

use crate::auth::verify_token;
use crate::models::{GitRepo, GitRepoInfo, SourcesResponse};

use super::AppState;

// Types

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct AddGitRepoResponse {
    pub success: bool,
    pub message: String,
    pub repo: Option<GitRepoInfo>,
}

// Helper functions

/// Check if a path is a valid Git repository
fn is_valid_git_repo(path: &str) -> bool {
    let expanded = shellexpand::tilde(path);
    let path = Path::new(expanded.as_ref());
    path.join(".git").is_dir()
}

/// Get the last commit info from a Git repository
fn get_last_commit_info(path: &str) -> Option<(String, String)> {
    let expanded = shellexpand::tilde(path);
    let output = Command::new("git")
        .args(["log", "-1", "--format=%H|%ci"])
        .current_dir(expanded.as_ref())
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.trim().split('|').collect();
        if parts.len() >= 2 {
            return Some((
                parts[0][..7].to_string(), // Short hash
                parts[1].to_string(),      // Date
            ));
        }
    }
    None
}

/// Extract project name from path
fn extract_repo_name(path: &str) -> String {
    let expanded = shellexpand::tilde(path);
    Path::new(expanded.as_ref())
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Check if Claude projects directory exists
fn get_claude_projects_path() -> Option<String> {
    let home = dirs::home_dir()?;
    let claude_path = home.join(".claude").join("projects");
    if claude_path.exists() {
        Some(claude_path.to_string_lossy().to_string())
    } else {
        None
    }
}

// Commands

/// Get data sources configuration
#[tauri::command]
pub async fn get_sources(
    state: State<'_, AppState>,
    token: String,
) -> Result<SourcesResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Get user's source mode
    let mode: Option<String> = sqlx::query_scalar("SELECT source_mode FROM users WHERE id = ?")
        .bind(&claims.sub)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| e.to_string())?
        .flatten();

    let source_mode = mode.unwrap_or_else(|| "claude".to_string());

    // Get git repos from database
    let repos: Vec<GitRepo> = sqlx::query_as(
        "SELECT id, user_id, path, name, enabled, created_at FROM git_repos WHERE user_id = ? AND enabled = 1"
    )
    .bind(&claims.sub)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // Convert to GitRepoInfo with validation
    let git_repos: Vec<GitRepoInfo> = repos
        .into_iter()
        .map(|repo| {
            let valid = is_valid_git_repo(&repo.path);
            let (last_commit, last_commit_date) = if valid {
                get_last_commit_info(&repo.path)
                    .map(|(h, d)| (Some(h), Some(d)))
                    .unwrap_or((None, None))
            } else {
                (None, None)
            };

            GitRepoInfo {
                id: repo.id,
                path: repo.path,
                name: repo.name,
                valid,
                last_commit,
                last_commit_date,
            }
        })
        .collect();

    // Check Claude connection
    let claude_path = get_claude_projects_path();
    let claude_connected = claude_path.is_some();

    Ok(SourcesResponse {
        mode: source_mode,
        git_repos,
        claude_connected,
        claude_path,
    })
}

/// Add a local Git repository
#[tauri::command]
pub async fn add_git_repo(
    state: State<'_, AppState>,
    token: String,
    path: String,
) -> Result<AddGitRepoResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;

    // Expand ~ to home directory
    let expanded_path = shellexpand::tilde(&path).to_string();

    // Validate the path is a git repo
    if !is_valid_git_repo(&expanded_path) {
        return Ok(AddGitRepoResponse {
            success: false,
            message: format!("路徑 '{}' 不是有效的 Git 倉庫", path),
            repo: None,
        });
    }

    let db = state.db.lock().await;

    // Check if already exists
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT id FROM git_repos WHERE user_id = ? AND path = ?"
    )
    .bind(&claims.sub)
    .bind(&expanded_path)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    if existing.is_some() {
        return Ok(AddGitRepoResponse {
            success: false,
            message: "此 Git 倉庫已經新增過了".to_string(),
            repo: None,
        });
    }

    // Extract name from path
    let name = extract_repo_name(&expanded_path);
    let id = Uuid::new_v4().to_string();

    // Insert into database
    sqlx::query(
        "INSERT INTO git_repos (id, user_id, path, name, enabled) VALUES (?, ?, ?, ?, 1)"
    )
    .bind(&id)
    .bind(&claims.sub)
    .bind(&expanded_path)
    .bind(&name)
    .execute(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // Get last commit info
    let (last_commit, last_commit_date) = get_last_commit_info(&expanded_path)
        .map(|(h, d)| (Some(h), Some(d)))
        .unwrap_or((None, None));

    Ok(AddGitRepoResponse {
        success: true,
        message: format!("已新增 Git 倉庫: {}", name),
        repo: Some(GitRepoInfo {
            id,
            path: expanded_path,
            name,
            valid: true,
            last_commit,
            last_commit_date,
        }),
    })
}

/// Remove a local Git repository
#[tauri::command]
pub async fn remove_git_repo(
    state: State<'_, AppState>,
    token: String,
    repo_id: String,
) -> Result<MessageResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Delete (or disable) the repo
    let result = sqlx::query(
        "DELETE FROM git_repos WHERE id = ? AND user_id = ?"
    )
    .bind(&repo_id)
    .bind(&claims.sub)
    .execute(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Ok(MessageResponse {
            success: false,
            message: "找不到指定的 Git 倉庫".to_string(),
        });
    }

    Ok(MessageResponse {
        success: true,
        message: "已移除 Git 倉庫".to_string(),
    })
}

/// Set data source mode
#[tauri::command]
pub async fn set_source_mode(
    state: State<'_, AppState>,
    token: String,
    mode: String,
) -> Result<MessageResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;

    // Validate mode
    if mode != "git" && mode != "claude" {
        return Err("Invalid source mode. Must be 'git' or 'claude'".to_string());
    }

    let db = state.db.lock().await;

    sqlx::query("UPDATE users SET source_mode = ? WHERE id = ?")
        .bind(&mode)
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(MessageResponse {
        success: true,
        message: format!("已切換為 {} 模式", if mode == "git" { "Git" } else { "Claude" }),
    })
}
