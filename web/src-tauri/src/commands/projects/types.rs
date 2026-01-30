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
            source: "git".to_string(),
            work_item_count: 10,
            total_hours: 24.5,
            latest_date: Some("2024-01-15".to_string()),
            hidden: false,
            display_name: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"project_name\":\"recap\""));
        assert!(json.contains("\"work_item_count\":10"));
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
}
