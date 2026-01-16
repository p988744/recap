//! Work item helper functions
//!
//! Shared utilities for work item commands.

use anyhow::Result;
use chrono::NaiveDate;

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

/// Parse date string supporting common formats
pub fn parse_date(s: &str) -> Result<NaiveDate> {
    if s == "today" {
        return Ok(chrono::Local::now().date_naive());
    }
    if s == "yesterday" {
        return Ok(chrono::Local::now().date_naive() - chrono::Duration::days(1));
    }

    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| anyhow::anyhow!("Invalid date format: {}. Use YYYY-MM-DD", s))
}

/// Resolve a short ID to full ID
pub async fn resolve_work_item_id(db: &recap_core::Database, id: &str) -> Result<String> {
    let pattern = format!("{}%", id);
    let item: Option<(String,)> = sqlx::query_as("SELECT id FROM work_items WHERE id LIKE ? LIMIT 1")
        .bind(&pattern)
        .fetch_optional(&db.pool)
        .await?;

    match item {
        Some((full_id,)) => Ok(full_id),
        None => Err(anyhow::anyhow!("Work item not found: {}", id)),
    }
}

/// Get or create a default user for CLI usage
pub async fn get_or_create_default_user(db: &recap_core::Database) -> Result<String> {
    // Try to find existing user
    let user: Option<(String,)> = sqlx::query_as("SELECT id FROM users LIMIT 1")
        .fetch_optional(&db.pool)
        .await?;

    if let Some((id,)) = user {
        return Ok(id);
    }

    // Create default user for CLI
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();
    let password_hash = recap_core::auth::hash_password("cli_user")?;

    sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, name, username, created_at, updated_at)
        VALUES (?, 'cli@localhost', ?, 'CLI User', 'cli', ?, ?)
        "#
    )
    .bind(&id)
    .bind(&password_hash)
    .bind(now)
    .bind(now)
    .execute(&db.pool)
    .await?;

    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_date_valid_format() {
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
        let yesterday = chrono::Local::now().date_naive() - chrono::Duration::days(1);
        let parsed = parse_date("yesterday").unwrap();
        assert_eq!(parsed, yesterday);
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(parse_date("invalid").is_err());
        assert!(parse_date("2025/01/15").is_err()); // wrong separator
        assert!(parse_date("01-15-2025").is_err()); // wrong order
        assert!(parse_date("").is_err());
    }

    #[test]
    fn test_parse_date_error_message() {
        let err = parse_date("bad-date").unwrap_err();
        assert!(err.to_string().contains("bad-date"));
        assert!(err.to_string().contains("YYYY-MM-DD"));
    }

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("exact len!", 10), "exact len!");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("this is a long string", 10), "this is...");
        assert_eq!(truncate("abcdefghijklmnop", 10), "abcdefg...");
    }

    #[test]
    fn test_truncate_empty_string() {
        assert_eq!(truncate("", 10), "");
    }

    #[test]
    fn test_truncate_unicode() {
        assert_eq!(truncate("你好世界", 10), "你好世界");
        assert_eq!(truncate("你好世界這是一個很長的字串", 10), "你好世界這是一...");
    }

    #[test]
    fn test_truncate_exact_boundary() {
        assert_eq!(truncate("1234567890", 10), "1234567890");
        assert_eq!(truncate("12345678901", 10), "1234567...");
    }
}
