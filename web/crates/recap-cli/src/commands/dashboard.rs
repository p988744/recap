//! Dashboard commands
//!
//! CLI commands for displaying dashboard statistics and visualizations.

use anyhow::Result;
use chrono::{Datelike, Duration, NaiveDate};
use clap::Subcommand;
use serde::Serialize;
use std::collections::HashMap;
use tabled::Tabled;

use crate::output::{print_info, print_output};
use super::Context;

#[derive(Subcommand)]
pub enum DashboardAction {
    /// Show statistics summary
    Stats {
        /// Start date (YYYY-MM-DD), defaults to start of current week
        #[arg(short, long)]
        start: Option<String>,

        /// End date (YYYY-MM-DD), defaults to end of current week
        #[arg(short, long)]
        end: Option<String>,

        /// Show this week's stats (default)
        #[arg(long)]
        week: bool,

        /// Show this month's stats
        #[arg(long)]
        month: bool,
    },

    /// Show work timeline for a specific date
    Timeline {
        /// Date to show (YYYY-MM-DD), defaults to today
        #[arg(short, long)]
        date: Option<String>,
    },

    /// Show daily hours heatmap data
    Heatmap {
        /// Number of weeks to show (default: 12)
        #[arg(short, long, default_value = "12")]
        weeks: u32,
    },

    /// Show project distribution
    Projects {
        /// Start date (YYYY-MM-DD), defaults to start of current week
        #[arg(short, long)]
        start: Option<String>,

        /// End date (YYYY-MM-DD), defaults to end of current week
        #[arg(short, long)]
        end: Option<String>,
    },
}

// Output types

#[derive(Debug, Serialize, Tabled)]
pub struct StatsRow {
    #[tabled(rename = "指標")]
    pub metric: String,
    #[tabled(rename = "數值")]
    pub value: String,
}

#[derive(Debug, Serialize, Tabled)]
pub struct SourceRow {
    #[tabled(rename = "來源")]
    pub source: String,
    #[tabled(rename = "工時")]
    pub hours: String,
    #[tabled(rename = "佔比")]
    pub percentage: String,
}

#[derive(Debug, Serialize, Tabled)]
pub struct ProjectRow {
    #[tabled(rename = "專案")]
    pub project: String,
    #[tabled(rename = "工時")]
    pub hours: String,
    #[tabled(rename = "項目數")]
    pub items: String,
    #[tabled(rename = "佔比")]
    pub percentage: String,
}

#[derive(Debug, Serialize, Tabled)]
pub struct TimelineRow {
    #[tabled(rename = "時間")]
    pub time: String,
    #[tabled(rename = "專案")]
    pub project: String,
    #[tabled(rename = "工時")]
    pub hours: String,
    #[tabled(rename = "標題")]
    pub title: String,
    #[tabled(rename = "Commits")]
    pub commits: String,
}

#[derive(Debug, Serialize, Tabled)]
pub struct HeatmapRow {
    #[tabled(rename = "日期")]
    pub date: String,
    #[tabled(rename = "星期")]
    pub weekday: String,
    #[tabled(rename = "工時")]
    pub hours: String,
    #[tabled(rename = "項目")]
    pub items: String,
    #[tabled(rename = "視覺化")]
    pub visual: String,
}

pub async fn execute(ctx: &Context, action: DashboardAction) -> Result<()> {
    match action {
        DashboardAction::Stats { start, end, week, month } => {
            show_stats(ctx, start, end, week, month).await
        }
        DashboardAction::Timeline { date } => {
            show_timeline(ctx, date).await
        }
        DashboardAction::Heatmap { weeks } => {
            show_heatmap(ctx, weeks).await
        }
        DashboardAction::Projects { start, end } => {
            show_projects(ctx, start, end).await
        }
    }
}

async fn show_stats(
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

async fn show_timeline(ctx: &Context, date: Option<String>) -> Result<()> {
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
                let commits = recap_core::get_commits_in_time_range(project_path, start, end);
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

async fn show_heatmap(ctx: &Context, weeks: u32) -> Result<()> {
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

async fn show_projects(ctx: &Context, start: Option<String>, end: Option<String>) -> Result<()> {
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

// Helper functions

fn parse_date(s: &str) -> Result<NaiveDate> {
    if s == "today" {
        return Ok(chrono::Local::now().date_naive());
    }
    if s == "yesterday" {
        return Ok(chrono::Local::now().date_naive() - Duration::days(1));
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| anyhow::anyhow!("Invalid date format: {}. Use YYYY-MM-DD", s))
}

fn extract_project_name(title: &str) -> String {
    if let Some(start) = title.find('[') {
        if let Some(end) = title.find(']') {
            if end > start {
                return title[start + 1..end].to_string();
            }
        }
    }
    "其他".to_string()
}

fn clean_title(title: &str) -> String {
    if let Some(end) = title.find(']') {
        title[end + 1..].trim().to_string()
    } else {
        title.to_string()
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = chars[..max_chars - 3].iter().collect();
        format!("{}...", truncated)
    }
}

async fn get_default_user_id(db: &recap_core::Database) -> Result<String> {
    let user: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM users WHERE llm_api_key IS NOT NULL AND llm_api_key != '' LIMIT 1"
    )
        .fetch_optional(&db.pool)
        .await?;

    if let Some((id,)) = user {
        return Ok(id);
    }

    let user: Option<(String,)> = sqlx::query_as("SELECT id FROM users LIMIT 1")
        .fetch_optional(&db.pool)
        .await?;

    match user {
        Some((id,)) => Ok(id),
        None => Err(anyhow::anyhow!("No user found. Please run the app first to create a user.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

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
        let yesterday = chrono::Local::now().date_naive() - Duration::days(1);
        let parsed = parse_date("yesterday").unwrap();
        assert_eq!(parsed, yesterday);
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(parse_date("invalid").is_err());
        assert!(parse_date("2025/01/15").is_err());
    }

    #[test]
    fn test_extract_project_name_with_brackets() {
        assert_eq!(extract_project_name("[project] task"), "project");
        assert_eq!(extract_project_name("[my-app] feature"), "my-app");
    }

    #[test]
    fn test_extract_project_name_without_brackets() {
        assert_eq!(extract_project_name("plain task"), "其他");
        assert_eq!(extract_project_name("no brackets here"), "其他");
    }

    #[test]
    fn test_extract_project_name_malformed() {
        assert_eq!(extract_project_name("[unclosed"), "其他");
        assert_eq!(extract_project_name("no start]"), "其他");
    }

    #[test]
    fn test_clean_title_with_brackets() {
        assert_eq!(clean_title("[project] task description"), "task description");
        assert_eq!(clean_title("[app] feature work"), "feature work");
    }

    #[test]
    fn test_clean_title_without_brackets() {
        assert_eq!(clean_title("plain task"), "plain task");
    }

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("exact len!", 10), "exact len!");
    }

    #[test]
    fn test_truncate_long() {
        assert_eq!(truncate("this is very long text", 10), "this is...");
    }

    #[test]
    fn test_truncate_unicode() {
        assert_eq!(truncate("你好世界", 10), "你好世界");
        // 10 chars limit: 7 chars + "..." (3 chars) = 10
        assert_eq!(truncate("很長的中文字串需要被截斷", 10), "很長的中文字串...");
    }

    #[test]
    fn test_stats_row_serialization() {
        let row = StatsRow {
            metric: "總工時".to_string(),
            value: "40.5".to_string(),
        };
        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("總工時"));
        assert!(json.contains("40.5"));
    }

    #[test]
    fn test_source_row_serialization() {
        let row = SourceRow {
            source: "git".to_string(),
            hours: "20.0".to_string(),
            percentage: "50%".to_string(),
        };
        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("git"));
        assert!(json.contains("50%"));
    }

    #[test]
    fn test_project_row_serialization() {
        let row = ProjectRow {
            project: "recap".to_string(),
            hours: "15.5".to_string(),
            items: "10".to_string(),
            percentage: "38%".to_string(),
        };
        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("recap"));
        assert!(json.contains("15.5"));
    }

    #[test]
    fn test_timeline_row_serialization() {
        let row = TimelineRow {
            time: "09:00".to_string(),
            project: "test".to_string(),
            hours: "2.0".to_string(),
            title: "Task".to_string(),
            commits: "3".to_string(),
        };
        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("09:00"));
        assert!(json.contains("test"));
        assert!(json.contains("commits"));
    }

    #[test]
    fn test_heatmap_row_serialization() {
        let row = HeatmapRow {
            date: "2025-01-15".to_string(),
            weekday: "Wed".to_string(),
            hours: "8.0".to_string(),
            items: "5".to_string(),
            visual: "███".to_string(),
        };
        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("2025-01-15"));
        assert!(json.contains("Wed"));
        assert!(json.contains("visual"));
    }

    #[test]
    fn test_stats_row_debug() {
        let row = StatsRow {
            metric: "Test".to_string(),
            value: "123".to_string(),
        };
        let debug = format!("{:?}", row);
        assert!(debug.contains("Test"));
    }
}
