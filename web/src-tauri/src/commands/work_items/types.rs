//! Work Items types
//!
//! Type definitions for work item commands.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use recap_core::models::WorkItem;

// Re-export TimelineCommit from recap_core
pub use recap_core::services::TimelineCommit;

// ==================== Core Types ====================

#[derive(Debug, Serialize)]
pub struct WorkItemWithChildren {
    #[serde(flatten)]
    pub item: WorkItem,
    pub child_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct WorkItemFilters {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub source: Option<String>,
    pub category: Option<String>,
    pub jira_mapped: Option<bool>,
    pub synced_to_tempo: Option<bool>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub parent_id: Option<String>,
    pub show_all: Option<bool>,
}

// ==================== Grouped View Types ====================

#[derive(Debug, Serialize)]
pub struct WorkLogItem {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub hours: f64,
    pub date: String,
    pub source: String,
    pub synced_to_tempo: bool,
}

#[derive(Debug, Serialize)]
pub struct JiraIssueGroup {
    pub jira_key: Option<String>,
    pub jira_title: Option<String>,
    pub total_hours: f64,
    pub logs: Vec<WorkLogItem>,
}

#[derive(Debug, Serialize)]
pub struct ProjectGroup {
    pub project_name: String,
    pub total_hours: f64,
    pub issues: Vec<JiraIssueGroup>,
}

#[derive(Debug, Serialize)]
pub struct DateGroup {
    pub date: String,
    pub total_hours: f64,
    pub projects: Vec<ProjectGroup>,
}

#[derive(Debug, Serialize)]
pub struct GroupedWorkItemsResponse {
    pub by_project: Vec<ProjectGroup>,
    pub by_date: Vec<DateGroup>,
    pub total_hours: f64,
    pub total_items: i64,
}

#[derive(Debug, Deserialize)]
pub struct GroupedQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

// ==================== Stats Types ====================

#[derive(Debug, Serialize)]
pub struct DailyHours {
    pub date: String,
    pub hours: f64,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct JiraMappingStats {
    pub mapped: i64,
    pub unmapped: i64,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct TempoSyncStats {
    pub synced: i64,
    pub not_synced: i64,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct WorkItemStatsResponse {
    pub total_items: i64,
    pub total_hours: f64,
    pub hours_by_source: HashMap<String, f64>,
    pub hours_by_project: HashMap<String, f64>,
    pub hours_by_category: HashMap<String, f64>,
    pub daily_hours: Vec<DailyHours>,
    pub jira_mapping: JiraMappingStats,
    pub tempo_sync: TempoSyncStats,
}

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

// ==================== Timeline Types ====================

#[derive(Debug, Deserialize)]
pub struct TimelineQuery {
    pub date: String,
    pub sources: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct TimelineSession {
    pub id: String,
    pub project: String,
    pub title: String,
    pub start_time: String,
    pub end_time: String,
    pub hours: f64,
    pub commits: Vec<TimelineCommit>,
}

#[derive(Debug, Serialize)]
pub struct TimelineResponse {
    pub date: String,
    pub sessions: Vec<TimelineSession>,
    pub total_hours: f64,
    pub total_commits: i32,
}

// ==================== Batch Sync Types ====================

#[derive(Debug, Deserialize)]
pub struct BatchSyncRequest {
    pub work_item_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BatchSyncResponse {
    pub synced: i64,
    pub failed: i64,
    pub errors: Vec<String>,
}

// ==================== Aggregate Types ====================

#[derive(Debug, Deserialize)]
pub struct AggregateRequest {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AggregateResponse {
    pub original_count: usize,
    pub aggregated_count: usize,
    pub deleted_count: usize,
}

// ==================== Commit-Centric Types ====================

#[derive(Debug, Serialize)]
pub struct CommitCentricWorklog {
    pub date: String,
    pub project: String,
    pub commits: Vec<recap_core::services::CommitRecord>,
    pub standalone_sessions: Vec<recap_core::services::StandaloneSession>,
    pub total_commits: i32,
    pub total_hours: f64,
}

#[derive(Debug, Deserialize)]
pub struct CommitCentricQuery {
    pub date: String,
    pub project_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_item_filters_default_values() {
        let json = r#"{}"#;
        let filters: WorkItemFilters = serde_json::from_str(json).unwrap();
        assert!(filters.page.is_none());
        assert!(filters.per_page.is_none());
        assert!(filters.source.is_none());
    }

    #[test]
    fn test_work_item_filters_with_values() {
        let json = r#"{"page": 2, "per_page": 50, "source": "git", "jira_mapped": true}"#;
        let filters: WorkItemFilters = serde_json::from_str(json).unwrap();
        assert_eq!(filters.page, Some(2));
        assert_eq!(filters.per_page, Some(50));
        assert_eq!(filters.source, Some("git".to_string()));
        assert_eq!(filters.jira_mapped, Some(true));
    }

    #[test]
    fn test_grouped_query_serialization() {
        let json = r#"{"start_date": "2024-01-01", "end_date": "2024-01-31"}"#;
        let query: GroupedQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.start_date, Some("2024-01-01".to_string()));
        assert_eq!(query.end_date, Some("2024-01-31".to_string()));
    }

    #[test]
    fn test_stats_query_empty() {
        let json = r#"{}"#;
        let query: StatsQuery = serde_json::from_str(json).unwrap();
        assert!(query.start_date.is_none());
        assert!(query.end_date.is_none());
    }

    #[test]
    fn test_batch_sync_request() {
        let json = r#"{"work_item_ids": ["id1", "id2", "id3"]}"#;
        let request: BatchSyncRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.work_item_ids.len(), 3);
    }

    #[test]
    fn test_aggregate_request() {
        let json = r#"{"start_date": "2024-01-01", "source": "claude_code"}"#;
        let request: AggregateRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.start_date, Some("2024-01-01".to_string()));
        assert_eq!(request.source, Some("claude_code".to_string()));
        assert!(request.end_date.is_none());
    }

    #[test]
    fn test_daily_hours_serialization() {
        let daily = DailyHours {
            date: "2024-01-15".to_string(),
            hours: 8.5,
            count: 5,
        };
        let json = serde_json::to_string(&daily).unwrap();
        assert!(json.contains("2024-01-15"));
        assert!(json.contains("8.5"));
    }

    #[test]
    fn test_jira_mapping_stats_serialization() {
        let stats = JiraMappingStats {
            mapped: 10,
            unmapped: 5,
            percentage: 66.67,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"mapped\":10"));
        assert!(json.contains("\"percentage\":66.67"));
    }

    #[test]
    fn test_tempo_sync_stats_serialization() {
        let stats = TempoSyncStats {
            synced: 8,
            not_synced: 2,
            percentage: 80.0,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"synced\":8"));
        assert!(json.contains("\"not_synced\":2"));
    }

    #[test]
    fn test_timeline_response_serialization() {
        let response = TimelineResponse {
            date: "2024-01-15".to_string(),
            sessions: vec![],
            total_hours: 6.5,
            total_commits: 3,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total_commits\":3"));
    }

    #[test]
    fn test_aggregate_response_serialization() {
        let response = AggregateResponse {
            original_count: 100,
            aggregated_count: 20,
            deleted_count: 80,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"original_count\":100"));
    }

    #[test]
    fn test_commit_centric_query() {
        let json = r#"{"date": "2024-01-15", "project_path": "/home/user/project"}"#;
        let query: CommitCentricQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.date, "2024-01-15");
        assert_eq!(query.project_path, Some("/home/user/project".to_string()));
    }

    #[test]
    fn test_timeline_query_with_sources() {
        let json = r#"{"date": "2024-01-15", "sources": ["claude_code"]}"#;
        let query: TimelineQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.date, "2024-01-15");
        assert_eq!(query.sources, Some(vec!["claude_code".to_string()]));
    }

    #[test]
    fn test_timeline_query_without_sources() {
        let json = r#"{"date": "2024-01-15"}"#;
        let query: TimelineQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.date, "2024-01-15");
        assert!(query.sources.is_none());
    }

    #[test]
    fn test_timeline_query_empty_sources() {
        let json = r#"{"date": "2024-01-15", "sources": []}"#;
        let query: TimelineQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.date, "2024-01-15");
        assert_eq!(query.sources, Some(vec![]));
    }
}
