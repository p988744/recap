//! Dashboard stats command
//!
//! Show statistics summary for work items.

use anyhow::Result;
use chrono::{Datelike, Duration, NaiveDate};
use std::collections::HashMap;

use crate::commands::Context;
use crate::output::print_output;
use super::helpers::{extract_project_name, get_default_user_id, parse_date, truncate};
use super::types::{ProjectRow, SourceRow, StatsRow};

pub async fn show_stats(
    ctx: &Context,
    start: Option<String>,
    end: Option<String>,
    _week: bool,
    month: bool,
) -> Result<()> {
    let today = chrono::Local::now().date_naive();

    let (start_date, end_date) = if month {
        // This month
        let start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
        let end = if today.month() == 12 {
            NaiveDate::from_ymd_opt(today.year() + 1, 1, 1).unwrap() - Duration::days(1)
        } else {
            NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1).unwrap() - Duration::days(1)
        };
        (start, end)
    } else if let (Some(s), Some(e)) = (start, end) {
        (parse_date(&s)?, parse_date(&e)?)
    } else {
        // Default: this week (Monday to Sunday)
        let weekday = today.weekday().num_days_from_monday();
        let start = today - Duration::days(weekday as i64);
        let end = start + Duration::days(6);
        (start, end)
    };

    // Get user_id
    let user_id = get_default_user_id(&ctx.db).await?;

    // Query work items
    let items: Vec<recap_core::WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE user_id = ? AND date >= ? AND date <= ?"
    )
    .bind(&user_id)
    .bind(start_date.to_string())
    .bind(end_date.to_string())
    .fetch_all(&ctx.db.pool)
    .await?;

    let total_items = items.len() as i64;
    let total_hours: f64 = items.iter().map(|i| i.hours).sum();

    // Hours by source
    let mut hours_by_source: HashMap<String, f64> = HashMap::new();
    for item in &items {
        *hours_by_source.entry(item.source.clone()).or_insert(0.0) += item.hours;
    }

    // Hours by project
    let mut hours_by_project: HashMap<String, (f64, i64)> = HashMap::new();
    for item in &items {
        let project = extract_project_name(&item.title);
        let entry = hours_by_project.entry(project).or_insert((0.0, 0));
        entry.0 += item.hours;
        entry.1 += 1;
    }

    // Jira mapping stats
    let jira_mapped = items.iter().filter(|i| i.jira_issue_key.is_some()).count() as i64;
    let jira_percentage = if total_items > 0 {
        (jira_mapped as f64 / total_items as f64) * 100.0
    } else {
        0.0
    };

    // Tempo sync stats
    let tempo_synced = items.iter().filter(|i| i.synced_to_tempo).count() as i64;
    let tempo_percentage = if total_items > 0 {
        (tempo_synced as f64 / total_items as f64) * 100.0
    } else {
        0.0
    };

    // Count unique work days
    let work_days: std::collections::HashSet<_> = items.iter().map(|i| i.date.to_string()).collect();
    let work_day_count = work_days.len();

    // Print header
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Dashboard 統計摘要");
    println!("║  期間: {} ~ {}", start_date, end_date);
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Main stats
    let stats = vec![
        StatsRow { metric: "總工時".to_string(), value: format!("{:.1} 小時", total_hours) },
        StatsRow { metric: "工作項目".to_string(), value: format!("{} 項", total_items) },
        StatsRow { metric: "專案數".to_string(), value: format!("{} 個", hours_by_project.len()) },
        StatsRow { metric: "工作天數".to_string(), value: format!("{} 天", work_day_count) },
    ];
    print_output(&stats, ctx.format)?;
    println!();

    // Jira & Tempo stats
    println!("📊 同步狀態");
    println!("───────────────────────────────────────────────────────────────");
    println!("  Jira 對應: {}/{} ({:.1}%)", jira_mapped, total_items, jira_percentage);
    println!("  Tempo 同步: {}/{} ({:.1}%)", tempo_synced, total_items, tempo_percentage);
    println!();

    // Hours by source
    if !hours_by_source.is_empty() {
        println!("📁 按來源分類");
        println!("───────────────────────────────────────────────────────────────");
        let mut source_rows: Vec<SourceRow> = hours_by_source
            .iter()
            .map(|(source, hours)| {
                let pct = if total_hours > 0.0 { (hours / total_hours) * 100.0 } else { 0.0 };
                SourceRow {
                    source: source.clone(),
                    hours: format!("{:.1}h", hours),
                    percentage: format!("{:.1}%", pct),
                }
            })
            .collect();
        source_rows.sort_by(|a, b| b.hours.partial_cmp(&a.hours).unwrap_or(std::cmp::Ordering::Equal));
        print_output(&source_rows, ctx.format)?;
        println!();
    }

    // Top projects
    if !hours_by_project.is_empty() {
        println!("🏆 專案排行");
        println!("───────────────────────────────────────────────────────────────");
        let mut project_rows: Vec<ProjectRow> = hours_by_project
            .iter()
            .map(|(project, (hours, count))| {
                let pct = if total_hours > 0.0 { (hours / total_hours) * 100.0 } else { 0.0 };
                ProjectRow {
                    project: truncate(project, 20),
                    hours: format!("{:.1}h", hours),
                    items: count.to_string(),
                    percentage: format!("{:.1}%", pct),
                }
            })
            .collect();
        project_rows.sort_by(|a, b| {
            let a_h: f64 = a.hours.trim_end_matches('h').parse().unwrap_or(0.0);
            let b_h: f64 = b.hours.trim_end_matches('h').parse().unwrap_or(0.0);
            b_h.partial_cmp(&a_h).unwrap_or(std::cmp::Ordering::Equal)
        });
        print_output(&project_rows.into_iter().take(10).collect::<Vec<_>>(), ctx.format)?;
    }

    Ok(())
}
