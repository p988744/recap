//! Dashboard projects command
//!
//! Show project distribution.

use anyhow::Result;
use chrono::{Datelike, Duration};
use std::collections::HashMap;

use crate::commands::Context;
use crate::output::print_info;
use super::helpers::{clean_title, extract_project_name, get_default_user_id, parse_date, truncate};

pub async fn show_projects(ctx: &Context, start: Option<String>, end: Option<String>) -> Result<()> {
    let today = chrono::Local::now().date_naive();

    let (start_date, end_date) = if let (Some(s), Some(e)) = (start, end) {
        (parse_date(&s)?, parse_date(&e)?)
    } else {
        // Default: this week
        let weekday = today.weekday().num_days_from_monday();
        let start = today - Duration::days(weekday as i64);
        let end = start + Duration::days(6);
        (start, end)
    };

    let user_id = get_default_user_id(&ctx.db).await?;

    let items: Vec<recap_core::WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE user_id = ? AND date >= ? AND date <= ?"
    )
    .bind(&user_id)
    .bind(start_date.to_string())
    .bind(end_date.to_string())
    .fetch_all(&ctx.db.pool)
    .await?;

    if items.is_empty() {
        print_info(&format!("沒有 {} ~ {} 的工作記錄", start_date, end_date), ctx.quiet);
        return Ok(());
    }

    // Group by project
    let mut projects: HashMap<String, (f64, i64, Vec<String>)> = HashMap::new();
    for item in &items {
        let project = extract_project_name(&item.title);
        let entry = projects.entry(project).or_insert((0.0, 0, Vec::new()));
        entry.0 += item.hours;
        entry.1 += 1;

        let title = clean_title(&item.title);
        if !entry.2.contains(&title) && entry.2.len() < 5 {
            entry.2.push(title);
        }
    }

    let total_hours: f64 = items.iter().map(|i| i.hours).sum();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  專案分佈");
    println!("║  期間: {} ~ {}", start_date, end_date);
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Sort by hours
    let mut project_list: Vec<_> = projects.into_iter().collect();
    project_list.sort_by(|a, b| b.1.0.partial_cmp(&a.1.0).unwrap_or(std::cmp::Ordering::Equal));

    for (project, (hours, count, titles)) in &project_list {
        let pct = if total_hours > 0.0 { (hours / total_hours) * 100.0 } else { 0.0 };
        let bar_len = (pct / 5.0).min(20.0) as usize;

        println!("📁 {} ({:.1}h / {}項 / {:.1}%)", project, hours, count, pct);
        println!("   {}", "█".repeat(bar_len));
        for title in titles.iter().take(3) {
            println!("   • {}", truncate(title, 50));
        }
        println!();
    }

    println!("───────────────────────────────────────────────────────────────");
    println!("總計: {:.1} 小時 / {} 項工作 / {} 專案", total_hours, items.iter().count(), project_list.len());

    Ok(())
}
