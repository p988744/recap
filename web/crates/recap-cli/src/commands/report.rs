//! Report commands
//!
//! Commands for generating work reports: summary, export.

use anyhow::Result;
use chrono::{Datelike, NaiveDate};
use clap::Subcommand;
use serde::Serialize;
use std::collections::HashMap;
use tabled::Tabled;

use crate::output::{print_output, print_success, print_info, print_error};
use super::Context;

#[derive(Subcommand)]
pub enum ReportAction {
    /// Show work summary for a date range
    Summary {
        /// Start date (YYYY-MM-DD), defaults to start of current month
        #[arg(short, long)]
        start: Option<String>,

        /// End date (YYYY-MM-DD), defaults to today
        #[arg(short, long)]
        end: Option<String>,

        /// Group by: date, project, source
        #[arg(short, long, default_value = "date")]
        group_by: String,
    },

    /// Export work items to Excel
    Export {
        /// Start date (YYYY-MM-DD), defaults to start of current month
        #[arg(short, long)]
        start: Option<String>,

        /// End date (YYYY-MM-DD), defaults to today
        #[arg(short, long)]
        end: Option<String>,

        /// Output file path (default: work_report.xlsx)
        #[arg(short, long, default_value = "work_report.xlsx")]
        output: String,
    },
}

/// Summary row for table display
#[derive(Debug, Serialize, Tabled)]
pub struct SummaryRow {
    #[tabled(rename = "Group")]
    pub group: String,
    #[tabled(rename = "Hours")]
    pub hours: String,
    #[tabled(rename = "Items")]
    pub items: String,
}

/// Date summary row
#[derive(Debug, Serialize, Tabled)]
pub struct DateSummaryRow {
    #[tabled(rename = "Date")]
    pub date: String,
    #[tabled(rename = "Hours")]
    pub hours: String,
    #[tabled(rename = "Items")]
    pub items: String,
}

pub async fn execute(ctx: &Context, action: ReportAction) -> Result<()> {
    match action {
        ReportAction::Summary { start, end, group_by } => {
            show_summary(ctx, start, end, group_by).await
        }
        ReportAction::Export { start, end, output } => {
            export_excel(ctx, start, end, output).await
        }
    }
}

async fn show_summary(
    ctx: &Context,
    start: Option<String>,
    end: Option<String>,
    group_by: String,
) -> Result<()> {
    let (start_date, end_date) = resolve_date_range(start, end)?;

    print_info(&format!("Work summary from {} to {}", start_date, end_date), ctx.quiet);

    // Fetch work items in date range
    let items: Vec<recap_core::WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE date >= ? AND date <= ? ORDER BY date"
    )
    .bind(start_date.to_string())
    .bind(end_date.to_string())
    .fetch_all(&ctx.db.pool)
    .await?;

    if items.is_empty() {
        print_info("No work items found in this date range.", ctx.quiet);
        return Ok(());
    }

    match group_by.as_str() {
        "date" => show_by_date(ctx, &items).await?,
        "project" | "category" => show_by_project(ctx, &items).await?,
        "source" => show_by_source(ctx, &items).await?,
        _ => {
            print_error(&format!("Unknown group_by option: {}. Use: date, project, source", group_by));
            return Ok(());
        }
    }

    // Show totals
    let total_hours: f64 = items.iter().map(|i| i.hours).sum();
    print_info(&format!("\nTotal: {:.1} hours across {} items", total_hours, items.len()), ctx.quiet);

    Ok(())
}

async fn show_by_date(ctx: &Context, items: &[recap_core::WorkItem]) -> Result<()> {
    let mut by_date: HashMap<String, (f64, usize)> = HashMap::new();

    for item in items {
        let entry = by_date.entry(item.date.to_string()).or_insert((0.0, 0));
        entry.0 += item.hours;
        entry.1 += 1;
    }

    let mut rows: Vec<DateSummaryRow> = by_date
        .into_iter()
        .map(|(date, (hours, count))| DateSummaryRow {
            date,
            hours: format!("{:.1}", hours),
            items: count.to_string(),
        })
        .collect();

    rows.sort_by(|a, b| a.date.cmp(&b.date));
    print_output(&rows, ctx.format)?;

    Ok(())
}

async fn show_by_project(ctx: &Context, items: &[recap_core::WorkItem]) -> Result<()> {
    let mut by_project: HashMap<String, (f64, usize)> = HashMap::new();

    for item in items {
        let project = item.category.clone().unwrap_or_else(|| "Uncategorized".to_string());
        let entry = by_project.entry(project).or_insert((0.0, 0));
        entry.0 += item.hours;
        entry.1 += 1;
    }

    let mut rows: Vec<SummaryRow> = by_project
        .into_iter()
        .map(|(group, (hours, count))| SummaryRow {
            group,
            hours: format!("{:.1}", hours),
            items: count.to_string(),
        })
        .collect();

    rows.sort_by(|a, b| b.hours.partial_cmp(&a.hours).unwrap_or(std::cmp::Ordering::Equal));
    print_output(&rows, ctx.format)?;

    Ok(())
}

async fn show_by_source(ctx: &Context, items: &[recap_core::WorkItem]) -> Result<()> {
    let mut by_source: HashMap<String, (f64, usize)> = HashMap::new();

    for item in items {
        let entry = by_source.entry(item.source.clone()).or_insert((0.0, 0));
        entry.0 += item.hours;
        entry.1 += 1;
    }

    let mut rows: Vec<SummaryRow> = by_source
        .into_iter()
        .map(|(group, (hours, count))| SummaryRow {
            group,
            hours: format!("{:.1}", hours),
            items: count.to_string(),
        })
        .collect();

    rows.sort_by(|a, b| b.hours.partial_cmp(&a.hours).unwrap_or(std::cmp::Ordering::Equal));
    print_output(&rows, ctx.format)?;

    Ok(())
}

async fn export_excel(
    ctx: &Context,
    start: Option<String>,
    end: Option<String>,
    output: String,
) -> Result<()> {
    let (start_date, end_date) = resolve_date_range(start, end)?;

    print_info(&format!("Exporting work items from {} to {}", start_date, end_date), ctx.quiet);

    // Fetch work items
    let items: Vec<recap_core::WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE date >= ? AND date <= ? ORDER BY date"
    )
    .bind(start_date.to_string())
    .bind(end_date.to_string())
    .fetch_all(&ctx.db.pool)
    .await?;

    if items.is_empty() {
        print_info("No work items found in this date range.", ctx.quiet);
        return Ok(());
    }

    // Convert to Excel format
    let excel_items: Vec<recap_core::ExcelWorkItem> = items
        .iter()
        .map(|item| recap_core::ExcelWorkItem {
            date: item.date.to_string(),
            title: item.title.clone(),
            description: item.description.clone(),
            hours: item.hours,
            project: item.category.clone(),
            jira_key: item.jira_issue_key.clone(),
            source: item.source.clone(),
            synced_to_tempo: item.synced_to_tempo,
        })
        .collect();

    // Build project summaries
    let mut project_map: HashMap<String, (f64, usize)> = HashMap::new();
    for item in &excel_items {
        let project = item.project.clone().unwrap_or_else(|| "Uncategorized".to_string());
        let entry = project_map.entry(project).or_insert((0.0, 0));
        entry.0 += item.hours;
        entry.1 += 1;
    }

    let projects: Vec<recap_core::ProjectSummary> = project_map
        .into_iter()
        .map(|(name, (hours, count))| recap_core::ProjectSummary {
            project_name: name,
            total_hours: hours,
            item_count: count,
        })
        .collect();

    // Get user name
    let user_name = get_user_name(&ctx.db).await.unwrap_or_else(|_| "CLI User".to_string());

    let metadata = recap_core::ReportMetadata {
        user_name,
        start_date: start_date.to_string(),
        end_date: end_date.to_string(),
        generated_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    // Generate report
    let mut generator = recap_core::ExcelReportGenerator::new()?;
    generator.create_personal_report(&metadata, &excel_items, &projects)?;
    generator.save(&output)?;

    print_success(&format!("Exported {} items to {}", excel_items.len(), output), ctx.quiet);
    Ok(())
}

fn resolve_date_range(start: Option<String>, end: Option<String>) -> Result<(NaiveDate, NaiveDate)> {
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

fn parse_date(s: &str) -> Result<NaiveDate> {
    if s == "today" {
        return Ok(chrono::Local::now().date_naive());
    }
    if s == "yesterday" {
        return Ok(chrono::Local::now().date_naive() - chrono::Duration::days(1));
    }

    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| anyhow::anyhow!("Invalid date format: {}. Use YYYY-MM-DD", s))
}

async fn get_user_name(db: &recap_core::Database) -> Result<String> {
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

        // Start should be first of current month
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

    #[test]
    fn test_summary_row_serialization() {
        let row = SummaryRow {
            group: "Project A".to_string(),
            hours: "10.5".to_string(),
            items: "5".to_string(),
        };

        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("Project A"));
        assert!(json.contains("10.5"));
        assert!(json.contains("5"));
    }

    #[test]
    fn test_summary_row_debug() {
        let row = SummaryRow {
            group: "Test".to_string(),
            hours: "8.0".to_string(),
            items: "3".to_string(),
        };

        let debug = format!("{:?}", row);
        assert!(debug.contains("Test"));
        assert!(debug.contains("8.0"));
    }

    #[test]
    fn test_date_summary_row_serialization() {
        let row = DateSummaryRow {
            date: "2025-01-15".to_string(),
            hours: "6.5".to_string(),
            items: "4".to_string(),
        };

        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("2025-01-15"));
        assert!(json.contains("6.5"));
    }

    #[test]
    fn test_date_summary_row_debug() {
        let row = DateSummaryRow {
            date: "2025-01-15".to_string(),
            hours: "4.0".to_string(),
            items: "2".to_string(),
        };

        let debug = format!("{:?}", row);
        assert!(debug.contains("2025-01-15"));
    }

    #[test]
    fn test_hours_formatting() {
        // Test that hours format to one decimal place
        let hours = 2.5;
        let formatted = format!("{:.1}", hours);
        assert_eq!(formatted, "2.5");

        let hours2 = 10.0;
        let formatted2 = format!("{:.1}", hours2);
        assert_eq!(formatted2, "10.0");
    }
}
