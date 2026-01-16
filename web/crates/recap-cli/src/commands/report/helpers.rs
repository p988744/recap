//! Report helper functions
//!
//! Shared utilities for report commands.

use anyhow::Result;
use chrono::{Datelike, NaiveDate};

/// Resolve date range from optional start and end dates
pub fn resolve_date_range(start: Option<String>, end: Option<String>) -> Result<(NaiveDate, NaiveDate)> {
    let today = chrono::Local::now().date_naive();

    let end_date = match end {
        Some(e) => parse_date(&e)?,
        None => today,
    };

    let start_date = match start {
        Some(s) => parse_date(&s)?,
        None => {
            // Default to start of current month
            NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
                .unwrap_or(today)
        }
    };

    Ok((start_date, end_date))
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

/// Get user name from database
pub async fn get_user_name(db: &recap_core::Database) -> Result<String> {
    let user: Option<(String,)> = sqlx::query_as("SELECT name FROM users LIMIT 1")
        .fetch_optional(&db.pool)
        .await?;

    Ok(user.map(|(name,)| name).unwrap_or_else(|| "CLI User".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let yesterday = chrono::Local::now().date_naive() - chrono::Duration::days(1);
        let parsed = parse_date("yesterday").unwrap();
        assert_eq!(parsed, yesterday);
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(parse_date("invalid").is_err());
        assert!(parse_date("2025/01/15").is_err());
        assert!(parse_date("").is_err());
    }

    #[test]
    fn test_parse_date_error_message() {
        let err = parse_date("bad").unwrap_err();
        assert!(err.to_string().contains("bad"));
        assert!(err.to_string().contains("YYYY-MM-DD"));
    }

    #[test]
    fn test_resolve_date_range_both_specified() {
        let (start, end) = resolve_date_range(
            Some("2025-01-01".to_string()),
            Some("2025-01-31".to_string()),
        ).unwrap();

        assert_eq!(start.to_string(), "2025-01-01");
        assert_eq!(end.to_string(), "2025-01-31");
    }

    #[test]
    fn test_resolve_date_range_only_start() {
        let today = chrono::Local::now().date_naive();
        let (start, end) = resolve_date_range(
            Some("2025-01-01".to_string()),
            None,
        ).unwrap();

        assert_eq!(start.to_string(), "2025-01-01");
        assert_eq!(end, today);
    }

    #[test]
    fn test_resolve_date_range_only_end() {
        let today = chrono::Local::now().date_naive();
        let (start, end) = resolve_date_range(
            None,
            Some("2025-01-31".to_string()),
        ).unwrap();

        assert_eq!(start.day(), 1);
        assert_eq!(start.month(), today.month());
        assert_eq!(end.to_string(), "2025-01-31");
    }

    #[test]
    fn test_resolve_date_range_defaults() {
        let result = resolve_date_range(None, None);
        assert!(result.is_ok());

        let (start, end) = result.unwrap();
        let today = chrono::Local::now().date_naive();

        assert_eq!(end, today);
        assert_eq!(start.day(), 1);
        assert_eq!(start.month(), today.month());
    }

    #[test]
    fn test_resolve_date_range_with_today_keyword() {
        let today = chrono::Local::now().date_naive();
        let (start, end) = resolve_date_range(
            Some("today".to_string()),
            Some("today".to_string()),
        ).unwrap();

        assert_eq!(start, today);
        assert_eq!(end, today);
    }
}
