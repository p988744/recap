//! Dashboard timeline command
//!
//! Show work timeline for a specific date.

use anyhow::Result;

use crate::commands::Context;
use crate::output::{print_info, print_output};
use super::helpers::{clean_title, extract_project_name, get_default_user_id, parse_date, truncate};
use super::types::TimelineRow;

pub async fn show_timeline(ctx: &Context, date: Option<String>) -> Result<()> {
    let target_date = match date {
        Some(d) => parse_date(&d)?,
        None => chrono::Local::now().date_naive(),
    };

    let user_id = get_default_user_id(&ctx.db).await?;

    // Query work items for the date (claude_code source has timing info)
    let items: Vec<recap_core::WorkItem> = sqlx::query_as(
        r#"SELECT * FROM work_items
           WHERE user_id = ? AND date = ?
           ORDER BY start_time ASC, created_at ASC"#
    )
    .bind(&user_id)
    .bind(target_date.to_string())
    .fetch_all(&ctx.db.pool)
    .await?;

    if items.is_empty() {
        print_info(&format!("沒有 {} 的工作記錄", target_date), ctx.quiet);
        return Ok(());
    }

    let total_hours: f64 = items.iter().map(|i| i.hours).sum();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  {} 工作時間線", target_date);
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let mut timeline_rows: Vec<TimelineRow> = Vec::new();
    let mut total_commits = 0;

    for item in &items {
        let project = extract_project_name(&item.title);
        let title = clean_title(&item.title);

        // Get time range
        let time = if let Some(start) = &item.start_time {
            let start_short = start.split('T').nth(1)
                .and_then(|t| t.split('+').next())
                .and_then(|t| t.split(':').take(2).collect::<Vec<_>>().join(":").into())
                .unwrap_or_else(|| start.clone());

            if let Some(end) = &item.end_time {
                let end_short = end.split('T').nth(1)
                    .and_then(|t| t.split('+').next())
                    .and_then(|t| t.split(':').take(2).collect::<Vec<_>>().join(":").into())
                    .unwrap_or_else(|| end.clone());
                format!("{}-{}", start_short, end_short)
            } else {
                start_short
            }
        } else {
            "-".to_string()
        };

        // Get commits for this session
        let commit_count = if let Some(project_path) = &item.project_path {
            if let (Some(start), Some(end)) = (&item.start_time, &item.end_time) {
                let author = recap_core::get_git_user_email(project_path);
                let commits = recap_core::get_commits_in_time_range(project_path, start, end, author.as_deref());
                total_commits += commits.len();
                commits.len()
            } else {
                0
            }
        } else {
            0
        };

        timeline_rows.push(TimelineRow {
            time,
            project: truncate(&project, 15),
            hours: format!("{:.1}h", item.hours),
            title: truncate(&title, 35),
            commits: if commit_count > 0 { commit_count.to_string() } else { "-".to_string() },
        });
    }

    print_output(&timeline_rows, ctx.format)?;

    println!();
    println!("───────────────────────────────────────────────────────────────");
    println!("總計: {:.1} 小時 / {} 項工作 / {} commits", total_hours, items.len(), total_commits);

    Ok(())
}
