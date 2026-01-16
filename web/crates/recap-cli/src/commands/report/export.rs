//! Report export commands
//!
//! Export work items to various formats.

use anyhow::Result;
use std::collections::HashMap;

use crate::commands::Context;
use crate::output::{print_info, print_success};
use super::helpers::{get_user_name, resolve_date_range};

pub async fn export_excel(
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
