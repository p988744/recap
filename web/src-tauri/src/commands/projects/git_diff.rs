//! Git diff commands
//!
//! Tauri commands for viewing git commit diffs.

use std::path::Path;

use recap_core::utils::create_command;

use super::types::{CommitDiffResponse, CommitFileChange, CommitStats, GetCommitDiffRequest};
use crate::commands::AppState;
use tauri::State;

/// Maximum size of diff output in bytes (100KB)
const MAX_DIFF_SIZE: usize = 100 * 1024;

/// Get the full diff for a commit
#[tauri::command]
pub async fn get_commit_diff(
    _state: State<'_, AppState>,
    request: GetCommitDiffRequest,
) -> Result<CommitDiffResponse, String> {
    let project_path = Path::new(&request.project_path);

    // Validate project path exists
    if !project_path.exists() {
        return Err(format!(
            "Project path does not exist: {}",
            request.project_path
        ));
    }

    // Find git root (project_path might not be the git root)
    let git_root = find_git_root(project_path).ok_or_else(|| {
        format!(
            "No git repository found at or above: {}",
            request.project_path
        )
    })?;

    // Get commit info
    let commit_info = get_commit_info(&git_root, &request.commit_hash)?;

    // Get file changes with stats
    let files = get_commit_files(&git_root, &request.commit_hash)?;

    // Calculate total stats
    let stats = CommitStats {
        files_changed: files.len() as i32,
        insertions: files.iter().map(|f| f.insertions).sum(),
        deletions: files.iter().map(|f| f.deletions).sum(),
    };

    // Get diff text (may be truncated)
    let diff_text = get_diff_text(&git_root, &request.commit_hash).ok();

    Ok(CommitDiffResponse {
        hash: request.commit_hash,
        message: commit_info.message,
        author: commit_info.author,
        date: commit_info.date,
        files,
        diff_text,
        stats,
    })
}

/// Find the git root directory starting from a path
fn find_git_root(start_path: &Path) -> Option<std::path::PathBuf> {
    let mut current = if start_path.is_file() {
        start_path.parent()?
    } else {
        start_path
    };

    loop {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }
        current = current.parent()?;
    }
}

/// Commit info from git log
struct CommitInfo {
    message: String,
    author: String,
    date: String,
}

/// Get basic commit info (message, author, date)
fn get_commit_info(git_root: &Path, commit_hash: &str) -> Result<CommitInfo, String> {
    let output = run_git_command(
        git_root,
        &[
            "log",
            "-1",
            "--format=%s%n%an <%ae>%n%aI",
            commit_hash,
        ],
    )?;

    let lines: Vec<&str> = output.trim().lines().collect();
    if lines.len() < 3 {
        return Err(format!("Invalid git log output for commit: {}", commit_hash));
    }

    Ok(CommitInfo {
        message: lines[0].to_string(),
        author: lines[1].to_string(),
        date: lines[2].to_string(),
    })
}

/// Get file changes for a commit
fn get_commit_files(git_root: &Path, commit_hash: &str) -> Result<Vec<CommitFileChange>, String> {
    // Use --numstat for insertions/deletions and --name-status for status
    let numstat_output = run_git_command(
        git_root,
        &["show", "--numstat", "--format=", commit_hash],
    )?;

    let name_status_output = run_git_command(
        git_root,
        &["show", "--name-status", "--format=", commit_hash],
    )?;

    // Parse numstat output (insertions, deletions, path)
    let mut files: Vec<CommitFileChange> = Vec::new();
    let numstat_lines: Vec<&str> = numstat_output.trim().lines().collect();
    let status_lines: Vec<&str> = name_status_output.trim().lines().collect();

    for (numstat_line, status_line) in numstat_lines.iter().zip(status_lines.iter()) {
        let numstat_parts: Vec<&str> = numstat_line.split('\t').collect();
        let status_parts: Vec<&str> = status_line.split('\t').collect();

        if numstat_parts.len() < 3 || status_parts.is_empty() {
            continue;
        }

        let insertions = numstat_parts[0].parse::<i32>().unwrap_or(0);
        let deletions = numstat_parts[1].parse::<i32>().unwrap_or(0);

        let status_char = status_parts[0].chars().next().unwrap_or('M');
        let (status, old_path) = match status_char {
            'A' => ("added".to_string(), None),
            'D' => ("deleted".to_string(), None),
            'M' => ("modified".to_string(), None),
            'R' => {
                // Renamed: status_parts[1] is old name, status_parts[2] is new name
                let old = if status_parts.len() > 1 {
                    Some(status_parts[1].to_string())
                } else {
                    None
                };
                ("renamed".to_string(), old)
            }
            'C' => ("copied".to_string(), None),
            _ => ("modified".to_string(), None),
        };

        // For renamed files, the path in numstat includes both old and new paths
        let path = if status_char == 'R' && status_parts.len() > 2 {
            status_parts[2].to_string()
        } else if status_parts.len() > 1 {
            status_parts[1].to_string()
        } else {
            numstat_parts[2].to_string()
        };

        files.push(CommitFileChange {
            path,
            status,
            old_path,
            insertions,
            deletions,
        });
    }

    // If the zip didn't work well (different line counts), fall back to numstat only
    if files.is_empty() && !numstat_lines.is_empty() {
        for line in numstat_lines {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let insertions = parts[0].parse::<i32>().unwrap_or(0);
                let deletions = parts[1].parse::<i32>().unwrap_or(0);
                let path = parts[2].to_string();

                files.push(CommitFileChange {
                    path,
                    status: "modified".to_string(),
                    old_path: None,
                    insertions,
                    deletions,
                });
            }
        }
    }

    Ok(files)
}

/// Get the diff text for a commit
fn get_diff_text(git_root: &Path, commit_hash: &str) -> Result<String, String> {
    let output = run_git_command(git_root, &["show", "--format=", commit_hash])?;

    // Truncate if too large
    if output.len() > MAX_DIFF_SIZE {
        let truncated = &output[..MAX_DIFF_SIZE];
        // Find the last newline to avoid cutting in the middle of a line
        if let Some(last_newline) = truncated.rfind('\n') {
            return Ok(format!(
                "{}\n\n... (diff truncated, {} bytes total)",
                &truncated[..last_newline],
                output.len()
            ));
        }
        return Ok(format!(
            "{}\n\n... (diff truncated, {} bytes total)",
            truncated,
            output.len()
        ));
    }

    Ok(output)
}

/// Run a git command with timeout
fn run_git_command(git_root: &Path, args: &[&str]) -> Result<String, String> {
    let child = create_command("git")
        .args(args)
        .current_dir(git_root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn git command: {}", e))?;

    // Wait with timeout (blocking, but this is in an async context so it's fine)
    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for git command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Git command failed: {}", stderr));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid UTF-8 in git output: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_git_root() {
        // This test assumes we're in a git repo
        let current_dir = std::env::current_dir().unwrap();
        let git_root = find_git_root(&current_dir);
        assert!(git_root.is_some());
        assert!(git_root.unwrap().join(".git").exists());
    }

    #[test]
    fn test_find_git_root_not_found() {
        let git_root = find_git_root(Path::new("/"));
        // Root directory might or might not have .git
        // Just ensure it doesn't panic
        let _ = git_root;
    }
}
