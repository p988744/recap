//! Grouped work items
//!
//! Commands for getting work items grouped by project and date.

use std::collections::HashMap;
use tauri::State;

use recap_core::auth::verify_token;
use recap_core::models::WorkItem;

use crate::commands::AppState;
use super::query_builder::SafeQueryBuilder;
use super::types::{
    DateGroup, GroupedQuery, GroupedWorkItemsResponse, JiraIssueGroup, ProjectGroup, WorkLogItem,
};

/// Helper to extract project name from title or description
fn extract_project(title: &str, description: &Option<String>) -> String {
    if let Some(start) = title.find('[') {
        if let Some(end) = title.find(']') {
            return title[start + 1..end].to_string();
        }
    }
    if let Some(desc) = description {
        if let Some(line) = desc.lines().find(|l| l.starts_with("Project:")) {
            if let Some(name) = line.rsplit(|c| c == '/' || c == '\\').next() {
                return name.to_string();
            }
        }
    }
    "其他".to_string()
}

/// Get work items grouped by project and date
#[tauri::command]
pub async fn get_grouped_work_items(
    state: State<'_, AppState>,
    token: String,
    query: GroupedQuery,
) -> Result<GroupedWorkItemsResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Build parameterized query safely
    let mut builder = SafeQueryBuilder::new();
    builder.add_string_condition("user_id", "=", &claims.sub);
    builder.add_null_condition("parent_id", true);

    if let Some(start) = &query.start_date {
        builder.add_string_condition("date", ">=", start);
    }
    if let Some(end) = &query.end_date {
        builder.add_string_condition("date", "<=", end);
    }

    let items: Vec<WorkItem> = builder
        .fetch_all(
            &db.pool,
            "SELECT * FROM work_items",
            "ORDER BY date DESC, title",
            None,
            None,
        )
        .await?;

    let total_items = items.len() as i64;
    let total_hours: f64 = items.iter().map(|i| i.hours).sum();

    // Group by project
    let mut projects_map: HashMap<String, HashMap<Option<String>, Vec<&WorkItem>>> = HashMap::new();
    for item in &items {
        let project = extract_project(&item.title, &item.description);
        let jira_key = item.jira_issue_key.clone();
        projects_map
            .entry(project)
            .or_default()
            .entry(jira_key)
            .or_default()
            .push(item);
    }

    let mut by_project: Vec<ProjectGroup> = projects_map
        .into_iter()
        .map(|(project_name, issues_map)| {
            let mut issues: Vec<JiraIssueGroup> = issues_map
                .into_iter()
                .map(|(jira_key, items)| {
                    let total_hours: f64 = items.iter().map(|i| i.hours).sum();
                    let jira_title = items.first().and_then(|i| i.jira_issue_title.clone());
                    let logs: Vec<WorkLogItem> = items
                        .into_iter()
                        .map(|i| WorkLogItem {
                            id: i.id.clone(),
                            title: i.title.clone(),
                            description: i.description.clone(),
                            hours: i.hours,
                            date: i.date.to_string(),
                            source: i.source.clone(),
                            synced_to_tempo: i.synced_to_tempo,
                        })
                        .collect();
                    JiraIssueGroup {
                        jira_key,
                        jira_title,
                        total_hours,
                        logs,
                    }
                })
                .collect();
            issues.sort_by(|a, b| b.total_hours.partial_cmp(&a.total_hours).unwrap());
            let total_hours: f64 = issues.iter().map(|i| i.total_hours).sum();
            ProjectGroup {
                project_name,
                total_hours,
                issues,
            }
        })
        .collect();
    by_project.sort_by(|a, b| b.total_hours.partial_cmp(&a.total_hours).unwrap());

    // Group by date
    let mut dates_map: HashMap<String, HashMap<String, Vec<&WorkItem>>> = HashMap::new();
    for item in &items {
        let date = item.date.to_string();
        let project = extract_project(&item.title, &item.description);
        dates_map
            .entry(date)
            .or_default()
            .entry(project)
            .or_default()
            .push(item);
    }

    let mut by_date: Vec<DateGroup> = dates_map
        .into_iter()
        .map(|(date, projects_map)| {
            let mut projects: Vec<ProjectGroup> = projects_map
                .into_iter()
                .map(|(project_name, items)| {
                    let mut jira_map: HashMap<Option<String>, Vec<&WorkItem>> = HashMap::new();
                    for item in items {
                        jira_map.entry(item.jira_issue_key.clone()).or_default().push(item);
                    }
                    let issues: Vec<JiraIssueGroup> = jira_map
                        .into_iter()
                        .map(|(jira_key, items)| {
                            let total_hours: f64 = items.iter().map(|i| i.hours).sum();
                            let jira_title = items.first().and_then(|i| i.jira_issue_title.clone());
                            let logs: Vec<WorkLogItem> = items
                                .into_iter()
                                .map(|i| WorkLogItem {
                                    id: i.id.clone(),
                                    title: i.title.clone(),
                                    description: i.description.clone(),
                                    hours: i.hours,
                                    date: i.date.to_string(),
                                    source: i.source.clone(),
                                    synced_to_tempo: i.synced_to_tempo,
                                })
                                .collect();
                            JiraIssueGroup { jira_key, jira_title, total_hours, logs }
                        })
                        .collect();
                    let total_hours: f64 = issues.iter().map(|i| i.total_hours).sum();
                    ProjectGroup { project_name, total_hours, issues }
                })
                .collect();
            projects.sort_by(|a, b| b.total_hours.partial_cmp(&a.total_hours).unwrap());
            let total_hours: f64 = projects.iter().map(|p| p.total_hours).sum();
            DateGroup { date, total_hours, projects }
        })
        .collect();
    by_date.sort_by(|a, b| b.date.cmp(&a.date));

    Ok(GroupedWorkItemsResponse {
        by_project,
        by_date,
        total_hours,
        total_items,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_project_from_title_brackets() {
        let title = "[my-project] Fix bug in auth";
        let project = extract_project(title, &None);
        assert_eq!(project, "my-project");
    }

    #[test]
    fn test_extract_project_from_description() {
        let title = "Fix bug in auth";
        let description = Some("Project: /home/user/my-project\nSome other info".to_string());
        let project = extract_project(title, &description);
        assert_eq!(project, "my-project");
    }

    #[test]
    fn test_extract_project_fallback() {
        let title = "Fix bug in auth";
        let project = extract_project(title, &None);
        assert_eq!(project, "其他");
    }

    #[test]
    fn test_extract_project_malformed_brackets() {
        let title = "[unclosed bracket Fix bug";
        let project = extract_project(title, &None);
        assert_eq!(project, "其他");
    }
}
