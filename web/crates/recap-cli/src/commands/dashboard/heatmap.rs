//! Dashboard heatmap command
//!
//! Show daily hours heatmap data.

use anyhow::Result;
use chrono::{Datelike, Duration};
use std::collections::HashMap;

use crate::commands::Context;
use super::helpers::get_default_user_id;
use super::types::HeatmapRow;

pub async fn show_heatmap(ctx: &Context, weeks: u32) -> Result<()> {
    let today = chrono::Local::now().date_naive();
    let start_date = today - Duration::days((weeks * 7) as i64);

    let user_id = get_default_user_id(&ctx.db).await?;

    // Query daily hours
    let items: Vec<recap_core::WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE user_id = ? AND date >= ? AND date <= ?"
    )
    .bind(&user_id)
    .bind(start_date.to_string())
    .bind(today.to_string())
    .fetch_all(&ctx.db.pool)
    .await?;

    // Aggregate by date
    let mut daily_map: HashMap<String, (f64, i64)> = HashMap::new();
    for item in &items {
        let entry = daily_map.entry(item.date.to_string()).or_insert((0.0, 0));
        entry.0 += item.hours;
        entry.1 += 1;
    }

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  工作熱力圖 (過去 {} 週)", weeks);
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Show weekly summary
    let mut current = start_date;
    let weekdays = ["一", "二", "三", "四", "五", "六", "日"];

    let mut heatmap_rows: Vec<HeatmapRow> = Vec::new();
    let mut week_hours = 0.0;
    let mut week_start = current;

    while current <= today {
        let date_str = current.to_string();
        let (hours, items) = daily_map.get(&date_str).cloned().unwrap_or((0.0, 0));

        let weekday_idx = current.weekday().num_days_from_monday() as usize;
        let weekday = weekdays[weekday_idx];

        // Visual bar
        let bar_len = (hours * 2.0).min(10.0) as usize;
        let visual = if hours > 0.0 {
            format!("{} {:.1}h", "█".repeat(bar_len), hours)
        } else {
            "·".to_string()
        };

        week_hours += hours;

        // Only show days with work or today
        if hours > 0.0 || current == today {
            heatmap_rows.push(HeatmapRow {
                date: date_str,
                weekday: weekday.to_string(),
                hours: format!("{:.1}", hours),
                items: items.to_string(),
                visual,
            });
        }

        // End of week summary
        if weekday_idx == 6 || current == today {
            if week_hours > 0.0 {
                println!("📅 {} ~ {} (共 {:.1}h)", week_start, current, week_hours);
                let week_rows: Vec<_> = heatmap_rows.drain(..).collect();
                if !week_rows.is_empty() {
                    for row in &week_rows {
                        println!("   {} {} {}", row.date, row.weekday, row.visual);
                    }
                }
                println!();
            }
            week_hours = 0.0;
            week_start = current + Duration::days(1);
        }

        current += Duration::days(1);
    }

    // Summary
    let total_hours: f64 = daily_map.values().map(|(h, _)| h).sum();
    let total_days = daily_map.len();
    let avg_hours = if total_days > 0 { total_hours / total_days as f64 } else { 0.0 };

    println!("───────────────────────────────────────────────────────────────");
    println!("總計: {:.1} 小時 / {} 工作天 / 平均 {:.1} 小時/天", total_hours, total_days, avg_hours);

    Ok(())
}
