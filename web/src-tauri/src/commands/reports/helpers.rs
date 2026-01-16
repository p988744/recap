//! Reports helpers
//!
//! Helper functions for report generation.

use recap_core::models::WorkItem;

/// Parse quarter string (e.g., "2024-Q1") to (year, quarter)
pub fn parse_quarter(s: &str) -> Result<(i32, u32), String> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        return Err("Invalid quarter format. Use YYYY-Q1/Q2/Q3/Q4".to_string());
    }
    let year = parts[0].parse::<i32>().map_err(|_| "Invalid year")?;
    let q = parts[1].trim_start_matches('Q').trim_start_matches('q')
        .parse::<u32>().map_err(|_| "Invalid quarter")?;
    if q < 1 || q > 4 {
        return Err("Quarter must be Q1, Q2, Q3, or Q4".to_string());
    }
    Ok((year, q))
}

/// Parse half string (e.g., "2024-H1") to (year, half)
pub fn parse_half(s: &str) -> Result<(i32, u32), String> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        return Err("Invalid half format. Use YYYY-H1/H2".to_string());
    }
    let year = parts[0].parse::<i32>().map_err(|_| "Invalid year")?;
    let h = parts[1].trim_start_matches('H').trim_start_matches('h')
        .parse::<u32>().map_err(|_| "Invalid half")?;
    if h < 1 || h > 2 {
        return Err("Half must be H1 or H2".to_string());
    }
    Ok((year, h))
}

/// Extract project name from title (e.g., "[Project A] Task" -> "Project A")
pub fn extract_project_name(title: &str) -> String {
    if let Some(start) = title.find('[') {
        if let Some(end) = title.find(']') {
            if end > start {
                return title[start + 1..end].to_string();
            }
        }
    }
    "其他".to_string()
}

/// Clean title by removing project prefix (e.g., "[Project] Task" -> "Task")
pub fn clean_title(title: &str) -> String {
    if let Some(end) = title.find(']') {
        title[end + 1..].trim().to_string()
    } else {
        title.to_string()
    }
}

/// Generate fallback summary when LLM is not available
pub fn generate_fallback_summary(items: &[&WorkItem]) -> Vec<String> {
    items.iter()
        .take(5)
        .map(|i| {
            let title = clean_title(&i.title);
            if title.len() > 50 {
                format!("{}...", title.chars().take(47).collect::<String>())
            } else {
                title
            }
        })
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    // ==================== parse_quarter Tests ====================

    #[test]
    fn test_parse_quarter_valid() {
        assert_eq!(parse_quarter("2024-Q1").unwrap(), (2024, 1));
        assert_eq!(parse_quarter("2024-Q2").unwrap(), (2024, 2));
        assert_eq!(parse_quarter("2024-Q3").unwrap(), (2024, 3));
        assert_eq!(parse_quarter("2024-Q4").unwrap(), (2024, 4));
    }

    #[test]
    fn test_parse_quarter_lowercase() {
        assert_eq!(parse_quarter("2024-q1").unwrap(), (2024, 1));
        assert_eq!(parse_quarter("2024-q4").unwrap(), (2024, 4));
    }

    #[test]
    fn test_parse_quarter_invalid_format() {
        assert!(parse_quarter("2024").is_err());
        assert!(parse_quarter("2024-").is_err());
        assert!(parse_quarter("2024-Q1-extra").is_err());
        assert!(parse_quarter("-Q1").is_err());
    }

    #[test]
    fn test_parse_quarter_invalid_quarter() {
        assert!(parse_quarter("2024-Q0").is_err());
        assert!(parse_quarter("2024-Q5").is_err());
        assert!(parse_quarter("2024-Qx").is_err());
    }

    #[test]
    fn test_parse_quarter_invalid_year() {
        assert!(parse_quarter("abc-Q1").is_err());
    }

    // ==================== parse_half Tests ====================

    #[test]
    fn test_parse_half_valid() {
        assert_eq!(parse_half("2024-H1").unwrap(), (2024, 1));
        assert_eq!(parse_half("2024-H2").unwrap(), (2024, 2));
    }

    #[test]
    fn test_parse_half_lowercase() {
        assert_eq!(parse_half("2024-h1").unwrap(), (2024, 1));
        assert_eq!(parse_half("2024-h2").unwrap(), (2024, 2));
    }

    #[test]
    fn test_parse_half_invalid_format() {
        assert!(parse_half("2024").is_err());
        assert!(parse_half("2024-").is_err());
        assert!(parse_half("2024-H1-extra").is_err());
    }

    #[test]
    fn test_parse_half_invalid_half() {
        assert!(parse_half("2024-H0").is_err());
        assert!(parse_half("2024-H3").is_err());
        assert!(parse_half("2024-Hx").is_err());
    }

    // ==================== extract_project_name Tests ====================

    #[test]
    fn test_extract_project_name_with_brackets() {
        assert_eq!(extract_project_name("[Project A] Task description"), "Project A");
        assert_eq!(extract_project_name("[My-Project] Feature work"), "My-Project");
        assert_eq!(extract_project_name("[recap] Fix authentication bug"), "recap");
    }

    #[test]
    fn test_extract_project_name_no_brackets() {
        assert_eq!(extract_project_name("Task without project"), "其他");
        assert_eq!(extract_project_name("Simple task"), "其他");
    }

    #[test]
    fn test_extract_project_name_malformed_brackets() {
        assert_eq!(extract_project_name("[Incomplete task"), "其他");
        assert_eq!(extract_project_name("Incomplete] task"), "其他");
        assert_eq!(extract_project_name("]Wrong[ order"), "其他");
    }

    #[test]
    fn test_extract_project_name_empty_brackets() {
        assert_eq!(extract_project_name("[] Empty project"), "");
    }

    #[test]
    fn test_extract_project_name_nested_brackets() {
        assert_eq!(extract_project_name("[Outer [Inner]] Task"), "Outer [Inner");
    }

    // ==================== clean_title Tests ====================

    #[test]
    fn test_clean_title_with_project() {
        assert_eq!(clean_title("[Project A] Task description"), "Task description");
        assert_eq!(clean_title("[recap] Fix bug"), "Fix bug");
    }

    #[test]
    fn test_clean_title_no_project() {
        assert_eq!(clean_title("Task without project"), "Task without project");
        assert_eq!(clean_title("Simple task"), "Simple task");
    }

    #[test]
    fn test_clean_title_trims_whitespace() {
        assert_eq!(clean_title("[Project]   Extra spaces   "), "Extra spaces");
        assert_eq!(clean_title("[P]  \t\n Task"), "Task");
    }

    #[test]
    fn test_clean_title_empty_after_bracket() {
        assert_eq!(clean_title("[Project]"), "");
        assert_eq!(clean_title("[Project]  "), "");
    }

    // ==================== generate_fallback_summary Tests ====================

    fn create_test_work_item(title: &str) -> WorkItem {
        WorkItem {
            id: "test-id".to_string(),
            user_id: "user-1".to_string(),
            source: "test".to_string(),
            source_id: None,
            source_url: None,
            title: title.to_string(),
            description: None,
            hours: 1.0,
            date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            jira_issue_key: None,
            jira_issue_suggested: None,
            jira_issue_title: None,
            category: None,
            tags: None,
            yearly_goal_id: None,
            synced_to_tempo: false,
            tempo_worklog_id: None,
            synced_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            parent_id: None,
            hours_source: None,
            hours_estimated: None,
            commit_hash: None,
            session_id: None,
            start_time: None,
            end_time: None,
            project_path: None,
        }
    }

    #[test]
    fn test_generate_fallback_summary_basic() {
        let item1 = create_test_work_item("[Project] Task 1");
        let item2 = create_test_work_item("[Project] Task 2");
        let items: Vec<&WorkItem> = vec![&item1, &item2];

        let summaries = generate_fallback_summary(&items);
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0], "Task 1");
        assert_eq!(summaries[1], "Task 2");
    }

    #[test]
    fn test_generate_fallback_summary_max_five() {
        let items: Vec<WorkItem> = (1..=10)
            .map(|i| create_test_work_item(&format!("[P] Task {}", i)))
            .collect();
        let item_refs: Vec<&WorkItem> = items.iter().collect();

        let summaries = generate_fallback_summary(&item_refs);
        assert_eq!(summaries.len(), 5);
    }

    #[test]
    fn test_generate_fallback_summary_truncates_long_titles() {
        let long_title = format!("[Project] {}", "A".repeat(60));
        let item = create_test_work_item(&long_title);
        let items: Vec<&WorkItem> = vec![&item];

        let summaries = generate_fallback_summary(&items);
        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].len() <= 50);
        assert!(summaries[0].ends_with("..."));
    }

    #[test]
    fn test_generate_fallback_summary_filters_empty() {
        let item = create_test_work_item("[Project]");
        let items: Vec<&WorkItem> = vec![&item];

        let summaries = generate_fallback_summary(&items);
        assert_eq!(summaries.len(), 0);
    }

    #[test]
    fn test_generate_fallback_summary_empty_input() {
        let items: Vec<&WorkItem> = vec![];
        let summaries = generate_fallback_summary(&items);
        assert!(summaries.is_empty());
    }
}
