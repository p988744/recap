//! Report summary commands
//!
//! Summary generation and display.

use anyhow::Result;
use std::collections::HashMap;

use crate::commands::Context;
use crate::output::{print_error, print_info, print_output};
use super::helpers::resolve_date_range;
use super::types::{DateSummaryRow, SummaryRow};

pub async fn show_summary(
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
