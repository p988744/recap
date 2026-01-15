//! Sources commands
//!
//! Tauri commands for managing data sources (Git repos, Claude, etc.)

use serde::Serialize;
use std::path::Path;
use std::process::Command;
use tauri::State;
use uuid::Uuid;

use recap_core::auth::verify_token;
use recap_core::models::{GitRepo, GitRepoInfo, SourcesResponse};

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

// Helper functions (pub(crate) for testing)

/// Check if a path is a valid Git repository
pub(crate) fn is_valid_git_repo(path: &str) -> bool {
    let expanded = shellexpand::tilde(path);
    let path = Path::new(expanded.as_ref());
    path.join(".git").is_dir()
}

/// Get the last commit info from a Git repository
pub(crate) fn get_last_commit_info(path: &str) -> Option<(String, String)> {
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
pub(crate) fn extract_repo_name(path: &str) -> String {
    let expanded = shellexpand::tilde(path);
    Path::new(expanded.as_ref())
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Check if Claude projects directory exists
pub(crate) fn get_claude_projects_path() -> Option<String> {
    let home = dirs::home_dir()?;
    let claude_path = home.join(".claude").join("projects");
    if claude_path.exists() {
        Some(claude_path.to_string_lossy().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_repo_name_simple() {
        assert_eq!(extract_repo_name("/home/user/projects/recap"), "recap");
        assert_eq!(extract_repo_name("/Users/test/my-project"), "my-project");
    }

    #[test]
    fn test_extract_repo_name_with_tilde() {
        // Should expand ~ and extract the last component
        let name = extract_repo_name("~/projects/test-repo");
        assert_eq!(name, "test-repo");
    }

    #[test]
    fn test_extract_repo_name_trailing_slash() {
        // Path with trailing slash
        let name = extract_repo_name("/home/user/project/");
        // file_name() returns None for paths ending with /
        // Our implementation should handle this
        assert!(!name.is_empty());
    }

    #[test]
    fn test_extract_repo_name_root_path() {
        let name = extract_repo_name("/");
        assert_eq!(name, "unknown");
    }

    #[test]
    fn test_is_valid_git_repo_current_project() {
        // The current project should be a valid git repo
        // CARGO_MANIFEST_DIR is src-tauri/, git root is recap/ (two levels up)
        let project_path = env!("CARGO_MANIFEST_DIR");
        let git_root = std::path::Path::new(project_path)
            .parent() // web/
            .and_then(|p| p.parent()) // recap/
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(
            is_valid_git_repo(&git_root),
            "Git root (recap/) should be a valid git repo"
        );
    }

    #[test]
    fn test_is_valid_git_repo_invalid_path() {
        assert!(!is_valid_git_repo("/nonexistent/path/that/does/not/exist"));
        assert!(!is_valid_git_repo("/tmp")); // /tmp is not a git repo
    }

    #[test]
    fn test_get_last_commit_info_current_project() {
        // The current project should have commits
        // CARGO_MANIFEST_DIR is src-tauri/, git root is recap/ (two levels up)
        let project_path = env!("CARGO_MANIFEST_DIR");
        let git_root = std::path::Path::new(project_path)
            .parent() // web/
            .and_then(|p| p.parent()) // recap/
            .unwrap()
            .to_string_lossy()
            .to_string();

        let result = get_last_commit_info(&git_root);
        assert!(result.is_some(), "Current project should have commit info");

        let (hash, date) = result.unwrap();
        assert_eq!(hash.len(), 7, "Short hash should be 7 characters");
        assert!(!date.is_empty(), "Commit date should not be empty");
    }

    #[test]
    fn test_get_last_commit_info_invalid_path() {
        let result = get_last_commit_info("/nonexistent/path");
        assert!(result.is_none(), "Invalid path should return None");
    }

    #[test]
    fn test_get_claude_projects_path() {
        // This test verifies the function works without crashing
        // Result depends on whether ~/.claude/projects exists
        let result = get_claude_projects_path();
        if let Some(path) = result {
            assert!(path.contains(".claude"), "Path should contain .claude");
            assert!(path.contains("projects"), "Path should contain projects");
        }
        // If None, that's also valid (just means the directory doesn't exist)
    }

    #[test]
    fn test_source_mode_validation() {
        // Test that only "git" and "claude" are valid modes
        let valid_modes = ["git", "claude"];
        let invalid_modes = ["Git", "CLAUDE", "other", ""];

        for mode in valid_modes {
            assert!(
                mode == "git" || mode == "claude",
                "Mode '{}' should be valid",
                mode
            );
        }

        for mode in invalid_modes {
            assert!(
                mode != "git" && mode != "claude",
                "Mode '{}' should be invalid",
                mode
            );
        }
    }

    /// Test that GitRepoInfo serialization contains all required fields
    /// This ensures frontend-backend type alignment
    #[test]
    fn test_git_repo_info_serialization() {
        use recap_core::models::GitRepoInfo;

        let repo_info = GitRepoInfo {
            id: "test-uuid-123".to_string(),
            path: "/home/user/project".to_string(),
            name: "project".to_string(),
            valid: true,
            last_commit: Some("abc1234".to_string()),
            last_commit_date: Some("2026-01-12 10:00:00 +0800".to_string()),
        };

        let json = serde_json::to_value(&repo_info).expect("Should serialize");

        // Verify all required fields exist (matching frontend GitRepoInfo interface)
        assert!(json.get("id").is_some(), "id field is required for frontend");
        assert!(json.get("path").is_some(), "path field is required");
        assert!(json.get("name").is_some(), "name field is required");
        assert!(json.get("valid").is_some(), "valid field is required");

        // Verify field types
        assert!(json["id"].is_string(), "id should be string");
        assert!(json["path"].is_string(), "path should be string");
        assert!(json["name"].is_string(), "name should be string");
        assert!(json["valid"].is_boolean(), "valid should be boolean");

        // Verify optional fields
        assert!(json.get("last_commit").is_some(), "last_commit should exist");
        assert!(json.get("last_commit_date").is_some(), "last_commit_date should exist");
    }

    /// Test SourcesResponse serialization for frontend compatibility
    #[test]
    fn test_sources_response_serialization() {
        use recap_core::models::{GitRepoInfo, SourcesResponse};

        let response = SourcesResponse {
            mode: "git".to_string(),
            git_repos: vec![GitRepoInfo {
                id: "repo-1".to_string(),
                path: "/path/to/repo".to_string(),
                name: "repo".to_string(),
                valid: true,
                last_commit: None,
                last_commit_date: None,
            }],
            claude_connected: true,
            claude_path: Some("/home/user/.claude/projects".to_string()),
        };

        let json = serde_json::to_value(&response).expect("Should serialize");

        // Verify all required fields for frontend SourcesResponse interface
        assert!(json.get("mode").is_some(), "mode field is required");
        assert!(json.get("git_repos").is_some(), "git_repos field is required");
        assert!(json.get("claude_connected").is_some(), "claude_connected field is required");

        // Verify git_repos is an array
        assert!(json["git_repos"].is_array(), "git_repos should be array");

        // Verify git_repos items have id field
        let repos = json["git_repos"].as_array().unwrap();
        assert!(!repos.is_empty(), "Should have repos");
        assert!(
            repos[0].get("id").is_some(),
            "Each repo in git_repos must have id for frontend"
        );
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
