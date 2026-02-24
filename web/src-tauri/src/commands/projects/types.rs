//! Project types
//!
//! Type definitions for project management commands.

use serde::{Deserialize, Serialize};

/// Project info for the project list view
#[derive(Debug, Serialize)]
pub struct ProjectInfo {
    pub project_name: String,
    pub project_path: Option<String>,
    pub source: String,
    /// All sources that contributed to this project (for showing multiple badges)
    pub sources: Vec<String>,
    pub work_item_count: i64,
    pub total_hours: f64,
    pub latest_date: Option<String>,
    pub hidden: bool,
    pub display_name: Option<String>,
}

/// Source breakdown for a project
#[derive(Debug, Serialize)]
pub struct ProjectSourceInfo {
    pub source: String,
    pub item_count: i64,
    pub latest_date: Option<String>,
    pub project_path: Option<String>,
}

/// Summary of a work item (lightweight)
#[derive(Debug, Serialize)]
pub struct WorkItemSummary {
    pub id: String,
    pub title: String,
    pub date: String,
    pub hours: f64,
    pub source: String,
}

/// Aggregated stats for a project
#[derive(Debug, Serialize)]
pub struct ProjectStats {
    pub total_items: i64,
    pub total_hours: f64,
    pub date_range: Option<(String, String)>,
}

/// Full project detail view
#[derive(Debug, Serialize)]
pub struct ProjectDetail {
    pub project_name: String,
    pub project_path: Option<String>,
    pub hidden: bool,
    pub display_name: Option<String>,
    pub sources: Vec<ProjectSourceInfo>,
    pub recent_items: Vec<WorkItemSummary>,
    pub stats: ProjectStats,
}

/// Request to set project visibility
#[derive(Debug, Deserialize)]
pub struct SetProjectVisibilityRequest {
    pub project_name: String,
    pub hidden: bool,
}

/// A single Claude Code project directory entry
#[derive(Debug, Serialize)]
pub struct ClaudeCodeDirEntry {
    pub path: String,
    pub session_count: i64,
}

/// Directory info for a project (Claude Code + Git repo)
#[derive(Debug, Serialize)]
pub struct ProjectDirectories {
    pub claude_code_dirs: Vec<ClaudeCodeDirEntry>,
    pub git_repo_path: Option<String>,
}

/// Request to add a manual project
#[derive(Debug, Deserialize)]
pub struct AddManualProjectRequest {
    pub project_name: String,
    pub git_repo_path: String,
    pub display_name: Option<String>,
}

/// Claude session path response
#[derive(Debug, Serialize)]
pub struct ClaudeSessionPathResponse {
    pub path: String,
    pub is_default: bool,
}

/// Project description for AI context
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectDescription {
    pub project_name: String,
    pub goal: Option<String>,
    pub tech_stack: Option<String>,
    pub key_features: Option<Vec<String>>,
    pub notes: Option<String>,
}

/// Request to update project description
#[derive(Debug, Deserialize)]
pub struct UpdateProjectDescriptionRequest {
    pub project_name: String,
    pub goal: Option<String>,
    pub tech_stack: Option<String>,
    pub key_features: Option<Vec<String>>,
    pub notes: Option<String>,
}

/// Project summary from cache
#[derive(Debug, Serialize)]
pub struct ProjectSummaryResponse {
    pub summary: Option<String>,
    pub period_type: String,
    pub period_start: String,
    pub period_end: String,
    pub is_stale: bool,
    pub generated_at: Option<String>,
}

/// Request to generate project summary
#[derive(Debug, Deserialize)]
pub struct GenerateSummaryRequest {
    pub project_name: String,
    pub period_type: String, // "week" | "month"
    pub period_start: String,
    pub period_end: String,
    #[serde(default)]
    pub force_regenerate: bool,
}

/// Summary freshness status
#[derive(Debug, Serialize)]
pub struct SummaryFreshness {
    pub project_name: String,
    pub has_new_activity: bool,
    pub last_activity_date: Option<String>,
    pub last_summary_date: Option<String>,
}

// ============ Timeline Types ============

/// Request for project timeline
#[derive(Debug, Deserialize)]
pub struct ProjectTimelineRequest {
    pub project_name: String,
    pub time_unit: String, // "day" | "week" | "month" | "quarter" | "year"
    pub range_start: String,
    pub range_end: String,
    pub sources: Option<Vec<String>>,
    pub cursor: Option<String>,
    pub limit: Option<i32>,
}

/// Response for project timeline
#[derive(Debug, Serialize)]
pub struct ProjectTimelineResponse {
    pub groups: Vec<TimelineGroup>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

/// A group of sessions within a time period
#[derive(Debug, Serialize)]
pub struct TimelineGroup {
    pub period_label: String, // "2026-01-30" or "2026 W05" or "2026-01"
    pub period_start: String,
    pub period_end: String,
    pub total_hours: f64,
    pub summary: Option<String>,
    pub sessions: Vec<TimelineSession>,
    pub standalone_commits: Vec<TimelineCommit>,
}

/// A session within a timeline group
#[derive(Debug, Serialize)]
pub struct TimelineSession {
    pub id: String,
    pub source: String, // "claude_code"
    pub title: String,
    pub start_time: String,
    pub end_time: String,
    pub hours: f64,
    pub summary: Option<String>,
    pub commits: Vec<TimelineCommit>,
}

/// A commit within a timeline session
#[derive(Debug, Clone, Serialize)]
pub struct TimelineCommit {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub time: String,
    pub files_changed: i32,
    pub insertions: i32,
    pub deletions: i32,
}

// ============ Git Diff Types ============

/// Request for commit diff
#[derive(Debug, Deserialize)]
pub struct GetCommitDiffRequest {
    pub project_path: String,
    pub commit_hash: String,
}

/// Response for commit diff
#[derive(Debug, Serialize)]
pub struct CommitDiffResponse {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
    pub files: Vec<CommitFileChange>,
    pub diff_text: Option<String>, // Full diff output, null if repo unavailable
    pub stats: CommitStats,
}

/// File change in a commit
#[derive(Debug, Serialize)]
pub struct CommitFileChange {
    pub path: String,
    pub status: String, // "added" | "modified" | "deleted" | "renamed"
    pub old_path: Option<String>, // For renamed files
    pub insertions: i32,
    pub deletions: i32,
}

/// Commit statistics
#[derive(Debug, Serialize)]
pub struct CommitStats {
    pub files_changed: i32,
    pub insertions: i32,
    pub deletions: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_project_visibility_request_deserialize() {
        let json = r#"{"project_name": "recap", "hidden": true}"#;
        let req: SetProjectVisibilityRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.project_name, "recap");
        assert!(req.hidden);
    }

    #[test]
    fn test_project_info_serialize() {
        let info = ProjectInfo {
            project_name: "recap".to_string(),
            project_path: Some("/home/user/recap".to_string()),
            source: "claude_code".to_string(),
            sources: vec!["claude_code".to_string()],
            work_item_count: 10,
            total_hours: 24.5,
            latest_date: Some("2024-01-15".to_string()),
            hidden: false,
            display_name: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"project_name\":\"recap\""));
        assert!(json.contains("\"work_item_count\":10"));
        assert!(json.contains("\"sources\":[\"claude_code\"]"));
    }

    #[test]
    fn test_project_detail_serialize() {
        let detail = ProjectDetail {
            project_name: "recap".to_string(),
            project_path: None,
            hidden: false,
            display_name: None,
            sources: vec![],
            recent_items: vec![],
            stats: ProjectStats {
                total_items: 0,
                total_hours: 0.0,
                date_range: None,
            },
        };
        let json = serde_json::to_string(&detail).unwrap();
        assert!(json.contains("\"project_name\":\"recap\""));
        assert!(json.contains("\"total_items\":0"));
    }

    #[test]
    fn test_claude_session_path_response_serialize() {
        let response = ClaudeSessionPathResponse {
            path: "/home/user/.claude".to_string(),
            is_default: true,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"path\":\"/home/user/.claude\""));
        assert!(json.contains("\"is_default\":true"));
    }

    #[test]
    fn test_project_timeline_request_deserialize() {
        let json = r#"{
            "project_name": "recap",
            "time_unit": "week",
            "range_start": "2026-01-01",
            "range_end": "2026-01-31",
            "sources": ["claude_code"],
            "limit": 10
        }"#;
        let req: ProjectTimelineRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.project_name, "recap");
        assert_eq!(req.time_unit, "week");
        assert_eq!(req.sources, Some(vec!["claude_code".to_string()]));
        assert_eq!(req.limit, Some(10));
        assert!(req.cursor.is_none());
    }

    #[test]
    fn test_timeline_group_serialize() {
        let group = TimelineGroup {
            period_label: "2026-01-30".to_string(),
            period_start: "2026-01-30".to_string(),
            period_end: "2026-01-30".to_string(),
            total_hours: 4.5,
            summary: Some("Worked on feature X".to_string()),
            sessions: vec![],
            standalone_commits: vec![],
        };
        let json = serde_json::to_string(&group).unwrap();
        assert!(json.contains("\"period_label\":\"2026-01-30\""));
        assert!(json.contains("\"total_hours\":4.5"));
        assert!(json.contains("\"summary\":\"Worked on feature X\""));
    }

    #[test]
    fn test_timeline_session_serialize() {
        let session = TimelineSession {
            id: "session-1".to_string(),
            source: "claude_code".to_string(),
            title: "Working on feature X".to_string(),
            start_time: "2026-01-30T09:00:00".to_string(),
            end_time: "2026-01-30T10:00:00".to_string(),
            hours: 1.0,
            summary: Some("Implemented feature X".to_string()),
            commits: vec![],
        };
        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("\"source\":\"claude_code\""));
        assert!(json.contains("\"hours\":1.0"));
    }

    #[test]
    fn test_timeline_commit_serialize() {
        let commit = TimelineCommit {
            hash: "abc123def456".to_string(),
            short_hash: "abc123d".to_string(),
            message: "Add new feature".to_string(),
            author: "developer".to_string(),
            time: "2026-01-30T10:30:00".to_string(),
            files_changed: 5,
            insertions: 100,
            deletions: 20,
        };
        let json = serde_json::to_string(&commit).unwrap();
        assert!(json.contains("\"short_hash\":\"abc123d\""));
        assert!(json.contains("\"files_changed\":5"));
    }

    #[test]
    fn test_get_commit_diff_request_deserialize() {
        let json = r#"{
            "project_path": "/home/user/projects/recap",
            "commit_hash": "abc123def456"
        }"#;
        let req: GetCommitDiffRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.project_path, "/home/user/projects/recap");
        assert_eq!(req.commit_hash, "abc123def456");
    }

    #[test]
    fn test_commit_diff_response_serialize() {
        let response = CommitDiffResponse {
            hash: "abc123def456".to_string(),
            message: "Add new feature".to_string(),
            author: "developer <dev@example.com>".to_string(),
            date: "2026-01-30T10:30:00".to_string(),
            files: vec![CommitFileChange {
                path: "src/main.rs".to_string(),
                status: "modified".to_string(),
                old_path: None,
                insertions: 50,
                deletions: 10,
            }],
            diff_text: Some("+added line\n-deleted line".to_string()),
            stats: CommitStats {
                files_changed: 1,
                insertions: 50,
                deletions: 10,
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"hash\":\"abc123def456\""));
        assert!(json.contains("\"files_changed\":1"));
        assert!(json.contains("\"status\":\"modified\""));
    }

    #[test]
    fn test_commit_file_change_serialize() {
        let file = CommitFileChange {
            path: "src/lib.rs".to_string(),
            status: "renamed".to_string(),
            old_path: Some("src/old_lib.rs".to_string()),
            insertions: 100,
            deletions: 5,
        };
        let json = serde_json::to_string(&file).unwrap();
        assert!(json.contains("\"path\":\"src/lib.rs\""));
        assert!(json.contains("\"old_path\":\"src/old_lib.rs\""));
        assert!(json.contains("\"status\":\"renamed\""));
    }

    #[test]
    fn test_commit_stats_serialize() {
        let stats = CommitStats {
            files_changed: 10,
            insertions: 500,
            deletions: 200,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"files_changed\":10"));
        assert!(json.contains("\"insertions\":500"));
        assert!(json.contains("\"deletions\":200"));
    }
}
