//! Report generator
//!
//! Main logic for generating tempo reports.

use anyhow::Result;
use std::collections::HashMap;

use crate::commands::Context;
use crate::output::print_info;
use super::format::{print_markdown_report, print_text_report};
use super::helpers::{clean_title, extract_project_name, generate_smart_summary, get_default_user_id};
use super::period::resolve_period;
use super::types::{Period, ProjectSummary, TempoReport, WorkItemBrief};

pub async fn generate_tempo_report(
    ctx: &Context,
    period: Period,
    date: Option<String>,
    output_format: String,
) -> Result<()> {
    let (start_date, end_date, period_name) = resolve_period(&period, date)?;

    // Get user_id for LLM service
    let user_id = get_default_user_id(&ctx.db).await?;

    // Try to create LLM service
    let llm_service = recap_core::create_llm_service(&ctx.db.pool, &user_id).await.ok();
    let use_llm = llm_service.as_ref().map(|s| s.is_configured()).unwrap_or(false);

    if use_llm {
        print_info("Using LLM for smart summaries...", ctx.quiet);
    }

    // Fetch work items
    let items: Vec<recap_core::WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE date >= ? AND date <= ? ORDER BY date"
    )
    .bind(start_date.to_string())
    .bind(end_date.to_string())
    .fetch_all(&ctx.db.pool)
    .await?;

    if items.is_empty() {
        print_info(&format!("No work items found for {} ({} ~ {})",
            period_name, start_date, end_date), ctx.quiet);
        return Ok(());
    }

    // Group by project
    let mut projects_map: HashMap<String, Vec<&recap_core::WorkItem>> = HashMap::new();

    for item in &items {
        let project = extract_project_name(&item.title);
        projects_map.entry(project).or_default().push(item);
    }

    // Build report
    let mut projects: Vec<ProjectSummary> = Vec::new();
    let mut total_hours = 0.0;

    for (project, project_items) in &projects_map {
        let hours: f64 = project_items.iter().map(|i| i.hours).sum();
        total_hours += hours;

        let items_brief: Vec<WorkItemBrief> = project_items.iter().map(|i| {
            WorkItemBrief {
                date: i.date.to_string(),
                title: clean_title(&i.title),
                hours: i.hours,
            }
        }).collect();

        // Generate smart summary using LLM if available
        let summary = if use_llm {
            let work_items_text = project_items.iter()
                .map(|i| {
                    let title = clean_title(&i.title);
                    let desc = i.description.as_ref()
                        .map(|d| format!("\n  詳情: {}", d.chars().take(500).collect::<String>()))
                        .unwrap_or_default();
                    format!("- {} ({:.1}h): {}{}", i.date, i.hours, title, desc)
                })
                .collect::<Vec<_>>()
                .join("\n");

            match llm_service.as_ref().unwrap().summarize_project_work(project, &work_items_text).await {
                Ok((summaries, _usage)) => summaries,
                Err(e) => {
                    print_info(&format!("LLM error for {}: {}, using fallback", project, e), ctx.quiet);
                    generate_smart_summary(project_items)
                }
            }
        } else {
            generate_smart_summary(project_items)
        };

        projects.push(ProjectSummary {
            project: project.clone(),
            hours,
            items: items_brief,
            summary,
        });
    }

    // Sort by hours descending
    projects.sort_by(|a, b| b.hours.partial_cmp(&a.hours).unwrap_or(std::cmp::Ordering::Equal));

    let report = TempoReport {
        period: period_name.clone(),
        start_date: start_date.to_string(),
        end_date: end_date.to_string(),
        total_hours,
        total_items: items.len(),
        projects,
    };

    // Output
    match output_format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "markdown" => {
            print_markdown_report(&report);
        }
        _ => {
            print_text_report(&report);
        }
    }

    Ok(())
}
