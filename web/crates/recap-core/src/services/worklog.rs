//! Commit-centric worklog service
//!
//! Provides functionality for generating work logs based on git commits
//! with session data as supplementary information.

use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use crate::models::HoursSource;

/// A single commit record with hours estimation
#[derive(Debug, Clone, Serialize)]
pub struct CommitRecord {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub time: String,
    pub date: String,
    pub files_changed: Vec<FileChange>,
    pub total_additions: i32,
    pub total_deletions: i32,
    // Hours
    pub hours: f64,
    pub hours_source: String,
    pub hours_estimated: f64,
    // Related session (if any)
    pub related_session: Option<SessionBrief>,
}

/// File change in a commit
#[derive(Debug, Clone, Serialize)]
pub struct FileChange {
    pub path: String,
    pub additions: i32,
    pub deletions: i32,
}

/// Brief session info for linking
#[derive(Debug, Clone, Serialize)]
pub struct SessionBrief {
    pub session_id: String,
    pub hours: f64,
    pub first_message: Option<String>,
    pub tools_used: HashMap<String, usize>,
}

/// Session without commits (standalone)
#[derive(Debug, Clone, Serialize)]
pub struct StandaloneSession {
    pub session_id: String,
    pub project: String,
    pub start_time: String,
    pub end_time: String,
    pub hours: f64,
    pub outcome: String,
    pub outcome_source: String, // "llm" | "rule" | "first_message"
    pub tools_used: HashMap<String, usize>,
    pub files_modified: Vec<String>,
}

/// Daily worklog combining commits and sessions
#[derive(Debug, Clone, Serialize)]
pub struct DailyWorklog {
    pub date: String,
    pub commits: Vec<CommitRecord>,
    pub standalone_sessions: Vec<StandaloneSession>,
    pub total_commits: i32,
    pub total_session_hours: f64,
    pub total_estimated_hours: f64,
}

/// Hours estimation result
#[derive(Debug, Clone)]
pub struct HoursEstimate {
    pub hours: f64,
    pub source: HoursSource,
}

/// Estimate hours for a commit based on available data
pub fn estimate_commit_hours(
    commit_time: &DateTime<FixedOffset>,
    prev_commit_time: Option<&DateTime<FixedOffset>>,
    related_session: Option<&SessionBrief>,
    additions: i32,
    deletions: i32,
    files_count: usize,
    user_override: Option<f64>,
) -> HoursEstimate {
    // Priority 1: User manually set hours
    if let Some(hours) = user_override {
        return HoursEstimate {
            hours,
            source: HoursSource::UserModified,
        };
    }

    // Priority 2: Related session hours
    if let Some(session) = related_session {
        return HoursEstimate {
            hours: session.hours,
            source: HoursSource::Session,
        };
    }

    // Priority 3: Commit interval (if previous commit exists and gap is reasonable)
    if let Some(prev_time) = prev_commit_time {
        let gap = commit_time.signed_duration_since(*prev_time);
        let gap_minutes = gap.num_minutes();

        // Only use interval if gap is between 5 minutes and 4 hours
        if gap_minutes > 5 && gap_minutes < 240 {
            let raw_hours = (gap_minutes as f64 / 60.0).min(4.0).max(0.25);
            // Round to nearest 0.25
            let hours = (raw_hours * 4.0).round() / 4.0;
            return HoursEstimate {
                hours,
                source: HoursSource::CommitInterval,
            };
        }
    }

    // Priority 4: Heuristic based on lines and files
    let hours = estimate_from_diff(additions, deletions, files_count);
    HoursEstimate {
        hours,
        source: HoursSource::Heuristic,
    }
}

/// Estimate hours from diff statistics using logarithmic scaling
pub fn estimate_from_diff(additions: i32, deletions: i32, files_count: usize) -> f64 {
    let total_lines = (additions + deletions) as f64;
    let files = files_count as f64;

    if total_lines == 0.0 {
        return 0.25; // Minimum 15 minutes for empty commits
    }

    // Logarithmic scaling: more lines = diminishing returns
    // ln(100) ≈ 4.6, ln(1000) ≈ 6.9
    let line_factor = (total_lines + 1.0).ln() * 0.2;

    // File bonus: each file adds some overhead
    let file_factor = files * 0.15;

    // Combine and clamp
    let hours = (line_factor + file_factor).max(0.25).min(4.0);

    // Round to nearest 0.25
    (hours * 4.0).round() / 4.0
}

/// Get commits for a specific date from a git repository
pub fn get_commits_for_date(repo_path: &str, date: &NaiveDate) -> Vec<CommitRecord> {
    let repo_dir = PathBuf::from(repo_path);

    if !repo_dir.exists() || !repo_dir.join(".git").exists() {
        return Vec::new();
    }

    let since = format!("{} 00:00:00", date);
    let until = format!("{} 23:59:59", date);

    // Get commit list with metadata
    let output = Command::new("git")
        .arg("log")
        .arg("--since")
        .arg(&since)
        .arg("--until")
        .arg(&until)
        .arg("--format=%H|%h|%an|%aI|%s")
        .arg("--all")
        .current_dir(&repo_dir)
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();
    let mut prev_time: Option<DateTime<FixedOffset>> = None;

    for line in stdout.lines().rev() {
        // Parse in reverse order (oldest first) for interval calculation
        let parts: Vec<&str> = line.splitn(5, '|').collect();
        if parts.len() < 5 {
            continue;
        }

        let hash = parts[0].to_string();
        let short_hash = parts[1].to_string();
        let author = parts[2].to_string();
        let time_str = parts[3].to_string();
        let message = parts[4].to_string();

        // Parse commit time
        let commit_time = match DateTime::parse_from_rfc3339(&time_str) {
            Ok(t) => t,
            Err(_) => continue,
        };

        // Get file changes for this commit
        let (files_changed, additions, deletions) = get_commit_file_changes(&repo_dir, &hash);

        // Estimate hours
        let estimate = estimate_commit_hours(
            &commit_time,
            prev_time.as_ref(),
            None, // No session linking in this basic function
            additions,
            deletions,
            files_changed.len(),
            None, // No user override
        );

        commits.push(CommitRecord {
            hash,
            short_hash,
            message,
            author,
            time: time_str.clone(),
            date: date.to_string(),
            files_changed,
            total_additions: additions,
            total_deletions: deletions,
            hours: estimate.hours,
            hours_source: estimate.source.as_str().to_string(),
            hours_estimated: estimate.hours,
            related_session: None,
        });

        prev_time = Some(commit_time);
    }

    // Reverse back to newest first
    commits.reverse();
    commits
}

/// Get file changes for a specific commit
fn get_commit_file_changes(repo_dir: &PathBuf, hash: &str) -> (Vec<FileChange>, i32, i32) {
    let output = Command::new("git")
        .arg("show")
        .arg("--numstat")
        .arg("--format=")
        .arg(hash)
        .current_dir(repo_dir)
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return (Vec::new(), 0, 0),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();
    let mut total_add = 0;
    let mut total_del = 0;

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let additions = parts[0].parse::<i32>().unwrap_or(0);
            let deletions = parts[1].parse::<i32>().unwrap_or(0);
            let path = parts[2..].join(" ");

            // Skip binary files (shown as "-")
            if parts[0] != "-" && parts[1] != "-" {
                total_add += additions;
                total_del += deletions;
                files.push(FileChange {
                    path,
                    additions,
                    deletions,
                });
            }
        }
    }

    (files, total_add, total_del)
}

/// Calculate session hours from start and end timestamps
/// Returns hours capped between 0.25 and 8.0, rounded to nearest 0.25h
pub fn calculate_session_hours(start: &str, end: &str) -> f64 {
    if let (Ok(start_dt), Ok(end_dt)) = (
        DateTime::parse_from_rfc3339(start),
        DateTime::parse_from_rfc3339(end),
    ) {
        let duration = end_dt.signed_duration_since(start_dt);
        let hours = duration.num_minutes() as f64 / 60.0;
        let capped = hours.min(8.0).max(0.25);
        // Round to nearest 0.25h for consistency with commit hours
        (capped * 4.0).round() / 4.0
    } else {
        0.5 // Default fallback
    }
}

/// Commit info for timeline display (simplified version of CommitRecord)
#[derive(Debug, Clone, Serialize)]
pub struct TimelineCommit {
    pub hash: String,
    pub author: String,
    pub time: String,
    pub message: String,
}

/// Get commits within a specific time range (for session-based timeline)
pub fn get_commits_in_time_range(repo_path: &str, start: &str, end: &str) -> Vec<TimelineCommit> {
    if repo_path.is_empty() {
        return Vec::new();
    }

    let repo_dir = PathBuf::from(repo_path);
    if !repo_dir.exists() || !repo_dir.join(".git").exists() {
        return Vec::new();
    }

    let output = Command::new("git")
        .arg("log")
        .arg("--since")
        .arg(start)
        .arg("--until")
        .arg(end)
        .arg("--format=%H|%an|%aI|%s")
        .arg("--all")
        .current_dir(&repo_dir)
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() >= 4 {
            commits.push(TimelineCommit {
                hash: parts[0].chars().take(8).collect(),
                author: parts[1].to_string(),
                time: parts[2].to_string(),
                message: parts[3].to_string(),
            });
        }
    }

    commits
}

/// Build a rule-based outcome summary for a session without commits
pub fn build_rule_based_outcome(
    files_modified: &[String],
    tools_used: &HashMap<String, usize>,
    first_message: Option<&str>,
) -> String {
    let mut parts = Vec::new();

    // Summarize file modifications
    if !files_modified.is_empty() {
        let file_names: Vec<&str> = files_modified
            .iter()
            .filter_map(|f| f.split('/').last())
            .take(3)
            .collect();

        if !file_names.is_empty() {
            let more = if files_modified.len() > 3 {
                format!(" (+{})", files_modified.len() - 3)
            } else {
                String::new()
            };
            parts.push(format!("修改: {}{}", file_names.join(", "), more));
        }
    }

    // Summarize significant tool usage
    let significant_tools: Vec<String> = tools_used
        .iter()
        .filter(|(_, count)| **count >= 3)
        .map(|(tool, count)| format!("{}({})", tool, count))
        .collect();

    if !significant_tools.is_empty() {
        parts.push(significant_tools.join(", "));
    }

    // Fallback to first message if nothing else
    if parts.is_empty() {
        if let Some(msg) = first_message {
            let truncated: String = msg.chars().take(50).collect();
            if msg.len() > 50 {
                return format!("{}... (進行中)", truncated);
            }
            return truncated;
        }
        return "工作 session".to_string();
    }

    parts.join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_from_diff_small_change() {
        // Small change: ~10 lines, 1 file
        let hours = estimate_from_diff(8, 2, 1);
        assert!(hours >= 0.25 && hours <= 1.0, "Small change should be 0.25-1h, got {}", hours);
    }

    #[test]
    fn test_estimate_from_diff_medium_change() {
        // Medium change: ~100 lines, 3 files
        let hours = estimate_from_diff(80, 20, 3);
        assert!(hours >= 1.0 && hours <= 2.0, "Medium change should be 1-2h, got {}", hours);
    }

    #[test]
    fn test_estimate_from_diff_large_change() {
        // Large change: ~1000 lines, 5 files
        let hours = estimate_from_diff(800, 200, 5);
        assert!(hours >= 2.0 && hours <= 4.0, "Large change should be 2-4h, got {}", hours);
    }

    #[test]
    fn test_estimate_from_diff_empty() {
        // Empty commit
        let hours = estimate_from_diff(0, 0, 0);
        assert_eq!(hours, 0.25, "Empty commit should be 0.25h");
    }

    #[test]
    fn test_estimate_commit_hours_user_override() {
        let time = DateTime::parse_from_rfc3339("2026-01-11T10:00:00+08:00").unwrap();
        let estimate = estimate_commit_hours(&time, None, None, 100, 10, 2, Some(3.5));
        assert_eq!(estimate.hours, 3.5);
        assert_eq!(estimate.source, HoursSource::UserModified);
    }

    #[test]
    fn test_estimate_commit_hours_session() {
        let time = DateTime::parse_from_rfc3339("2026-01-11T10:00:00+08:00").unwrap();
        let session = SessionBrief {
            session_id: "test".to_string(),
            hours: 2.5,
            first_message: None,
            tools_used: HashMap::new(),
        };
        let estimate = estimate_commit_hours(&time, None, Some(&session), 100, 10, 2, None);
        assert_eq!(estimate.hours, 2.5);
        assert_eq!(estimate.source, HoursSource::Session);
    }

    #[test]
    fn test_estimate_commit_hours_interval() {
        let prev_time = DateTime::parse_from_rfc3339("2026-01-11T09:00:00+08:00").unwrap();
        let time = DateTime::parse_from_rfc3339("2026-01-11T10:30:00+08:00").unwrap();
        let estimate = estimate_commit_hours(&time, Some(&prev_time), None, 100, 10, 2, None);
        assert_eq!(estimate.hours, 1.5);
        assert_eq!(estimate.source, HoursSource::CommitInterval);
    }

    #[test]
    fn test_estimate_commit_hours_heuristic_fallback() {
        let time = DateTime::parse_from_rfc3339("2026-01-11T10:00:00+08:00").unwrap();
        let estimate = estimate_commit_hours(&time, None, None, 100, 10, 2, None);
        assert_eq!(estimate.source, HoursSource::Heuristic);
        assert!(estimate.hours > 0.0);
    }

    #[test]
    fn test_build_rule_based_outcome_files() {
        let files = vec![
            "/home/user/project/src/main.rs".to_string(),
            "/home/user/project/src/lib.rs".to_string(),
        ];
        let tools = HashMap::new();
        let outcome = build_rule_based_outcome(&files, &tools, None);
        assert!(outcome.contains("main.rs"));
        assert!(outcome.contains("lib.rs"));
    }

    #[test]
    fn test_build_rule_based_outcome_tools() {
        let files = Vec::new();
        let mut tools = HashMap::new();
        tools.insert("Edit".to_string(), 10);
        tools.insert("Bash".to_string(), 5);
        let outcome = build_rule_based_outcome(&files, &tools, None);
        assert!(outcome.contains("Edit(10)"));
        assert!(outcome.contains("Bash(5)"));
    }

    #[test]
    fn test_build_rule_based_outcome_fallback() {
        let outcome = build_rule_based_outcome(&[], &HashMap::new(), Some("幫我實作登入功能"));
        assert!(outcome.contains("幫我實作登入功能"));
    }

    // Tests for shared functions (used by Timeline and commit-centric worklog)

    #[test]
    fn test_calculate_session_hours_valid() {
        let hours = calculate_session_hours(
            "2026-01-11T09:00:00+08:00",
            "2026-01-11T11:30:00+08:00",
        );
        assert!((hours - 2.5).abs() < 0.01, "Expected 2.5h, got {}", hours);
    }

    #[test]
    fn test_calculate_session_hours_max_cap() {
        // Should cap at 8 hours
        let hours = calculate_session_hours(
            "2026-01-11T00:00:00+08:00",
            "2026-01-11T12:00:00+08:00",
        );
        assert_eq!(hours, 8.0, "Should cap at 8 hours");
    }

    #[test]
    fn test_calculate_session_hours_min_cap() {
        // Short session should return minimum 0.25h (15 minutes)
        let hours = calculate_session_hours(
            "2026-01-11T09:00:00+08:00",
            "2026-01-11T09:03:00+08:00", // 3 minutes
        );
        assert_eq!(hours, 0.25, "Should cap at minimum 0.25 hours");
    }

    #[test]
    fn test_calculate_session_hours_invalid() {
        // Invalid timestamps should return default 0.5h
        let hours = calculate_session_hours("invalid", "also-invalid");
        assert_eq!(hours, 0.5, "Invalid timestamps should return 0.5h");
    }

    #[test]
    fn test_get_commits_in_time_range_empty_path() {
        let commits = get_commits_in_time_range("", "2026-01-11T00:00:00+08:00", "2026-01-11T23:59:59+08:00");
        assert!(commits.is_empty(), "Empty path should return no commits");
    }

    #[test]
    fn test_get_commits_in_time_range_nonexistent_path() {
        let commits = get_commits_in_time_range("/nonexistent/path", "2026-01-11T00:00:00+08:00", "2026-01-11T23:59:59+08:00");
        assert!(commits.is_empty(), "Nonexistent path should return no commits");
    }
}
