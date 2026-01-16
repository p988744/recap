//! Sources helper functions
//!
//! Utilities for Git repository validation and path operations.

use std::path::Path;
use std::process::Command;

/// Check if a path is a valid Git repository
/// Supports both regular repos (.git is a directory) and worktrees (.git is a file)
pub fn is_valid_git_repo(path: &str) -> bool {
    let expanded = shellexpand::tilde(path);
    let path = Path::new(expanded.as_ref());
    let git_path = path.join(".git");
    // Regular git repo has .git directory, worktrees have .git file
    git_path.is_dir() || git_path.is_file()
}

/// Get the last commit info from a Git repository
pub fn get_last_commit_info(path: &str) -> Option<(String, String)> {
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
pub fn extract_repo_name(path: &str) -> String {
    let expanded = shellexpand::tilde(path);
    Path::new(expanded.as_ref())
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Check if Claude projects directory exists and return the path
pub fn get_claude_projects_path() -> Option<String> {
    let home = dirs::home_dir()?;
    let claude_path = home.join(".claude").join("projects");
    if claude_path.exists() {
        Some(claude_path.to_string_lossy().to_string())
    } else {
        None
    }
}
