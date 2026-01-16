//! Period resolution
//!
//! Functions for resolving report periods to date ranges.

use anyhow::Result;
use chrono::{Datelike, Duration, NaiveDate};

use super::types::Period;

/// Resolve a period specification to a date range
pub fn resolve_period(period: &Period, date: Option<String>) -> Result<(NaiveDate, NaiveDate, String)> {
    let today = chrono::Local::now().date_naive();

    match period {
        Period::Daily => {
            let target = match date {
                Some(d) => NaiveDate::parse_from_str(&d, "%Y-%m-%d")
                    .map_err(|_| anyhow::anyhow!("Invalid date format. Use YYYY-MM-DD"))?,
                None => today,
            };
            Ok((target, target, format!("Daily ({})", target)))
        }
        Period::Weekly => {
            let start = match date {
                Some(d) => NaiveDate::parse_from_str(&d, "%Y-%m-%d")
                    .map_err(|_| anyhow::anyhow!("Invalid date format. Use YYYY-MM-DD"))?,
                None => {
                    // Get Monday of current week
                    let weekday = today.weekday().num_days_from_monday();
                    today - Duration::days(weekday as i64)
                }
            };
            let end = start + Duration::days(6);
            Ok((start, end, format!("Weekly (W{})", start.iso_week().week())))
        }
        Period::Monthly => {
            let (year, month) = match date {
                Some(d) => {
                    let parts: Vec<&str> = d.split('-').collect();
                    if parts.len() >= 2 {
                        (parts[0].parse::<i32>()?, parts[1].parse::<u32>()?)
                    } else {
                        return Err(anyhow::anyhow!("Invalid month format. Use YYYY-MM"));
                    }
                }
                None => (today.year(), today.month()),
            };
            let start = NaiveDate::from_ymd_opt(year, month, 1)
                .ok_or_else(|| anyhow::anyhow!("Invalid month"))?;
            let end = if month == 12 {
                NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap() - Duration::days(1)
            } else {
                NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap() - Duration::days(1)
            };
            Ok((start, end, format!("Monthly ({}-{:02})", year, month)))
        }
        Period::Quarterly => {
            let (year, quarter) = match date {
                Some(d) => parse_quarter(&d)?,
                None => {
                    let q = (today.month() - 1) / 3 + 1;
                    (today.year(), q)
                }
            };
            let start_month = (quarter - 1) * 3 + 1;
            let end_month = quarter * 3;
            let start = NaiveDate::from_ymd_opt(year, start_month, 1)
                .ok_or_else(|| anyhow::anyhow!("Invalid quarter"))?;
            let end = if end_month == 12 {
                NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap() - Duration::days(1)
            } else {
                NaiveDate::from_ymd_opt(year, end_month + 1, 1).unwrap() - Duration::days(1)
            };
            Ok((start, end, format!("Quarterly ({}-Q{})", year, quarter)))
        }
        Period::SemiAnnual => {
            let (year, half) = match date {
                Some(d) => parse_half(&d)?,
                None => {
                    let h = if today.month() <= 6 { 1 } else { 2 };
                    (today.year(), h)
                }
            };
            let (start_month, end_month) = if half == 1 { (1, 6) } else { (7, 12) };
            let start = NaiveDate::from_ymd_opt(year, start_month, 1)
                .ok_or_else(|| anyhow::anyhow!("Invalid half"))?;
            let end = if end_month == 12 {
                NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap() - Duration::days(1)
            } else {
                NaiveDate::from_ymd_opt(year, end_month + 1, 1).unwrap() - Duration::days(1)
            };
            Ok((start, end, format!("Semi-Annual ({}-H{})", year, half)))
        }
    }
}

/// Parse quarter string (YYYY-Q1/Q2/Q3/Q4)
pub fn parse_quarter(s: &str) -> Result<(i32, u32)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!("Invalid quarter format. Use YYYY-Q1/Q2/Q3/Q4"));
    }
    let year = parts[0].parse::<i32>()?;
    let q = parts[1].trim_start_matches('Q').trim_start_matches('q').parse::<u32>()?;
    if q < 1 || q > 4 {
        return Err(anyhow::anyhow!("Quarter must be Q1, Q2, Q3, or Q4"));
    }
    Ok((year, q))
}

/// Parse half string (YYYY-H1/H2)
pub fn parse_half(s: &str) -> Result<(i32, u32)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!("Invalid half format. Use YYYY-H1/H2"));
    }
    let year = parts[0].parse::<i32>()?;
    let h = parts[1].trim_start_matches('H').trim_start_matches('h').parse::<u32>()?;
    if h < 1 || h > 2 {
        return Err(anyhow::anyhow!("Half must be H1 or H2"));
    }
    Ok((year, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_quarter_valid() {
        assert_eq!(parse_quarter("2026-Q1").unwrap(), (2026, 1));
        assert_eq!(parse_quarter("2026-Q2").unwrap(), (2026, 2));
        assert_eq!(parse_quarter("2026-Q3").unwrap(), (2026, 3));
        assert_eq!(parse_quarter("2026-Q4").unwrap(), (2026, 4));
    }

    #[test]
    fn test_parse_quarter_lowercase() {
        assert_eq!(parse_quarter("2026-q1").unwrap(), (2026, 1));
        assert_eq!(parse_quarter("2026-q4").unwrap(), (2026, 4));
    }

    #[test]
    fn test_parse_quarter_invalid() {
        assert!(parse_quarter("2026-Q5").is_err());
        assert!(parse_quarter("2026-Q0").is_err());
        assert!(parse_quarter("2026").is_err());
        assert!(parse_quarter("invalid").is_err());
    }

    #[test]
    fn test_parse_half_valid() {
        assert_eq!(parse_half("2026-H1").unwrap(), (2026, 1));
        assert_eq!(parse_half("2026-H2").unwrap(), (2026, 2));
    }

    #[test]
    fn test_parse_half_lowercase() {
        assert_eq!(parse_half("2026-h1").unwrap(), (2026, 1));
        assert_eq!(parse_half("2026-h2").unwrap(), (2026, 2));
    }

    #[test]
    fn test_parse_half_invalid() {
        assert!(parse_half("2026-H3").is_err());
        assert!(parse_half("2026-H0").is_err());
        assert!(parse_half("2026").is_err());
        assert!(parse_half("invalid").is_err());
    }

    #[test]
    fn test_resolve_period_daily_default() {
        let today = chrono::Local::now().date_naive();
        let (start, end, name) = resolve_period(&Period::Daily, None).unwrap();
        assert_eq!(start, today);
        assert_eq!(end, today);
        assert!(name.contains("Daily"));
    }

    #[test]
    fn test_resolve_period_daily_specific() {
        let (start, end, _) = resolve_period(&Period::Daily, Some("2025-06-15".to_string())).unwrap();
        assert_eq!(start.to_string(), "2025-06-15");
        assert_eq!(end.to_string(), "2025-06-15");
    }

    #[test]
    fn test_resolve_period_weekly_default() {
        let (start, end, name) = resolve_period(&Period::Weekly, None).unwrap();
        // Should be 7 days span
        let days = (end - start).num_days();
        assert_eq!(days, 6);
        assert!(name.contains("Weekly"));
    }

    #[test]
    fn test_resolve_period_monthly_default() {
        let today = chrono::Local::now().date_naive();
        let (start, _end, name) = resolve_period(&Period::Monthly, None).unwrap();
        assert_eq!(start.day(), 1);
        assert_eq!(start.month(), today.month());
        assert!(name.contains("Monthly"));
    }

    #[test]
    fn test_resolve_period_monthly_specific() {
        let (start, end, _) = resolve_period(&Period::Monthly, Some("2025-02".to_string())).unwrap();
        assert_eq!(start.to_string(), "2025-02-01");
        assert_eq!(end.to_string(), "2025-02-28");
    }

    #[test]
    fn test_resolve_period_quarterly_default() {
        let (start, _end, name) = resolve_period(&Period::Quarterly, None).unwrap();
        assert_eq!(start.day(), 1);
        assert!(name.contains("Quarterly"));
        assert!(name.contains("-Q"));
    }

    #[test]
    fn test_resolve_period_quarterly_specific() {
        let (start, end, _) = resolve_period(&Period::Quarterly, Some("2025-Q1".to_string())).unwrap();
        assert_eq!(start.to_string(), "2025-01-01");
        assert_eq!(end.to_string(), "2025-03-31");
    }

    #[test]
    fn test_resolve_period_semiannual_default() {
        let (start, _end, name) = resolve_period(&Period::SemiAnnual, None).unwrap();
        assert_eq!(start.day(), 1);
        assert!(name.contains("Semi-Annual"));
        assert!(name.contains("-H"));
    }

    #[test]
    fn test_resolve_period_semiannual_h1() {
        let (start, end, _) = resolve_period(&Period::SemiAnnual, Some("2025-H1".to_string())).unwrap();
        assert_eq!(start.to_string(), "2025-01-01");
        assert_eq!(end.to_string(), "2025-06-30");
    }

    #[test]
    fn test_resolve_period_semiannual_h2() {
        let (start, end, _) = resolve_period(&Period::SemiAnnual, Some("2025-H2".to_string())).unwrap();
        assert_eq!(start.to_string(), "2025-07-01");
        assert_eq!(end.to_string(), "2025-12-31");
    }
}
