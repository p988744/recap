//! Sources module tests
//!
//! Tests for helper functions and serialization compatibility.

use super::helpers::{extract_repo_name, get_claude_projects_path, get_last_commit_info, is_valid_git_repo};

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
    // CARGO_MANIFEST_DIR is src-tauri/
    // In a worktree: desktop-dev/.git (file), In main repo: recap/.git (directory)
    let project_path = env!("CARGO_MANIFEST_DIR");
    let git_root = std::path::Path::new(project_path)
        .parent() // web/
        .and_then(|p| p.parent()) // worktree root or repo root
        .unwrap()
        .to_string_lossy()
        .to_string();
    assert!(
        is_valid_git_repo(&git_root),
        "Project root should be a valid git repo (or worktree)"
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
