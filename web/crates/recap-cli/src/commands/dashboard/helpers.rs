//! Dashboard helper functions
//!
//! Shared utilities for dashboard commands.

use anyhow::Result;
use chrono::{Duration, NaiveDate};

/// Parse a date string into NaiveDate
pub fn parse_date(s: &str) -> Result<NaiveDate> {
    if s == "today" {
        return Ok(chrono::Local::now().date_naive());
    }
    if s == "yesterday" {
        return Ok(chrono::Local::now().date_naive() - Duration::days(1));
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| anyhow::anyhow!("Invalid date format: {}. Use YYYY-MM-DD", s))
}

/// Extract project name from title with [project] format
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

/// Clean title by removing [project] prefix
pub fn clean_title(title: &str) -> String {
    if let Some(end) = title.find(']') {
        title[end + 1..].trim().to_string()
    } else {
        title.to_string()
    }
}

/// Truncate string to max characters with ellipsis
pub fn truncate(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = chars[..max_chars - 3].iter().collect();
        format!("{}...", truncated)
    }
}

/// Get the default user ID from database
pub async fn get_default_user_id(db: &recap_core::Database) -> Result<String> {
    let user: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM users WHERE llm_api_key IS NOT NULL AND llm_api_key != '' LIMIT 1"
    )
        .fetch_optional(&db.pool)
        .await?;

    if let Some((id,)) = user {
        return Ok(id);
    }

    let user: Option<(String,)> = sqlx::query_as("SELECT id FROM users LIMIT 1")
        .fetch_optional(&db.pool)
        .await?;

    match user {
        Some((id,)) => Ok(id),
        None => Err(anyhow::anyhow!("No user found. Please run the app first to create a user.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_date_valid() {
        let date = parse_date("2025-01-15").unwrap();
        assert_eq!(date.year(), 2025);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 15);
    }

    #[test]
    fn test_parse_date_today() {
        let today = chrono::Local::now().date_naive();
        let parsed = parse_date("today").unwrap();
        assert_eq!(parsed, today);
    }

    #[test]
    fn test_parse_date_yesterday() {
        let yesterday = chrono::Local::now().date_naive() - Duration::days(1);
        let parsed = parse_date("yesterday").unwrap();
        assert_eq!(parsed, yesterday);
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(parse_date("invalid").is_err());
        assert!(parse_date("2025/01/15").is_err());
    }

    #[test]
    fn test_extract_project_name_with_brackets() {
        assert_eq!(extract_project_name("[project] task"), "project");
        assert_eq!(extract_project_name("[my-app] feature"), "my-app");
    }

    #[test]
    fn test_extract_project_name_without_brackets() {
        assert_eq!(extract_project_name("plain task"), "其他");
        assert_eq!(extract_project_name("no brackets here"), "其他");
    }

    #[test]
    fn test_extract_project_name_malformed() {
        assert_eq!(extract_project_name("[unclosed"), "其他");
        assert_eq!(extract_project_name("no start]"), "其他");
    }

    #[test]
    fn test_clean_title_with_brackets() {
        assert_eq!(clean_title("[project] task description"), "task description");
        assert_eq!(clean_title("[app] feature work"), "feature work");
    }

    #[test]
    fn test_clean_title_without_brackets() {
        assert_eq!(clean_title("plain task"), "plain task");
    }

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("exact len!", 10), "exact len!");
    }

    #[test]
    fn test_truncate_long() {
        assert_eq!(truncate("this is very long text", 10), "this is...");
    }

    #[test]
    fn test_truncate_unicode() {
        assert_eq!(truncate("你好世界", 10), "你好世界");
        // 10 chars limit: 7 chars + "..." (3 chars) = 10
        assert_eq!(truncate("很長的中文字串需要被截斷", 10), "很長的中文字串...");
    }
}
