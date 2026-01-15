//! Tempo report commands
//!
//! Generate smart work summaries for Tempo time logging.

use anyhow::Result;
use chrono::{Datelike, NaiveDate, Duration};
use clap::{Subcommand, ValueEnum};
use serde::Serialize;
use std::collections::HashMap;

use crate::output::print_info;
use super::Context;

#[derive(Clone, ValueEnum, Debug)]
pub enum Period {
    /// Daily report
    Daily,
    /// Weekly report
    Weekly,
    /// Monthly report
    Monthly,
    /// Quarterly report
    Quarterly,
    /// Semi-annual report
    SemiAnnual,
}

#[derive(Subcommand)]
pub enum TempoReportAction {
    /// Generate smart work summary for Tempo
    Generate {
        /// Report period granularity
        #[arg(short, long, value_enum, default_value = "weekly")]
        period: Period,

        /// Start date (YYYY-MM-DD) or period identifier
        /// For daily: specific date (default: today)
        /// For weekly: week start date (default: this week)
        /// For monthly: YYYY-MM (default: this month)
        /// For quarterly: YYYY-Q1/Q2/Q3/Q4 (default: this quarter)
        /// For semi-annual: YYYY-H1/H2 (default: this half)
        #[arg(short, long)]
        date: Option<String>,

        /// Output format: text, json, or markdown
        #[arg(short, long, default_value = "text")]
        output: String,
    },
}

/// Project summary for Tempo
#[derive(Debug, Serialize)]
pub struct ProjectSummary {
    pub project: String,
    pub hours: f64,
    pub items: Vec<WorkItemBrief>,
    pub summary: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct WorkItemBrief {
    pub date: String,
    pub title: String,
    pub hours: f64,
}

#[derive(Debug, Serialize)]
pub struct TempoReport {
    pub period: String,
    pub start_date: String,
    pub end_date: String,
    pub total_hours: f64,
    pub total_items: usize,
    pub projects: Vec<ProjectSummary>,
}

pub async fn execute(ctx: &Context, action: TempoReportAction) -> Result<()> {
    match action {
        TempoReportAction::Generate { period, date, output } => {
            generate_tempo_report(ctx, period, date, output).await
        }
    }
}

async fn generate_tempo_report(
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
                Ok(summaries) => summaries,
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

fn resolve_period(period: &Period, date: Option<String>) -> Result<(NaiveDate, NaiveDate, String)> {
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

fn parse_quarter(s: &str) -> Result<(i32, u32)> {
    // Format: YYYY-Q1, YYYY-Q2, YYYY-Q3, YYYY-Q4
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

fn parse_half(s: &str) -> Result<(i32, u32)> {
    // Format: YYYY-H1, YYYY-H2
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

fn extract_project_name(title: &str) -> String {
    // Extract [project] from title
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
    // Remove [project] prefix and clean up
    let cleaned = if let Some(end) = title.find(']') {
        title[end + 1..].trim().to_string()
    } else {
        title.to_string()
    };

    // Truncate long titles
    let chars: Vec<char> = cleaned.chars().collect();
    if chars.len() > 60 {
        format!("{}...", chars[..57].iter().collect::<String>())
    } else {
        cleaned
    }
}

fn generate_smart_summary(items: &[&recap_core::WorkItem]) -> Vec<String> {
    // Group similar work items and generate summary
    let mut summaries: Vec<String> = Vec::new();
    let mut seen_keywords: HashMap<String, f64> = HashMap::new();

    for item in items {
        let title = clean_title(&item.title).to_lowercase();

        // Extract keywords and group
        let keywords = extract_keywords(&title);
        for keyword in keywords {
            *seen_keywords.entry(keyword).or_insert(0.0) += item.hours;
        }
    }

    // Sort by hours and generate summaries
    let mut keyword_list: Vec<_> = seen_keywords.into_iter().collect();
    keyword_list.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    for (keyword, _hours) in keyword_list.iter().take(5) {
        if !keyword.is_empty() {
            summaries.push(format_keyword_summary(keyword, items));
        }
    }

    // If no good summaries, use titles directly
    if summaries.is_empty() {
        for item in items.iter().take(5) {
            let title = clean_title(&item.title);
            if !title.is_empty() && title.len() > 3 {
                summaries.push(title);
            }
        }
    }

    summaries.into_iter().take(5).collect()
}

fn extract_keywords(title: &str) -> Vec<String> {
    let mut keywords = Vec::new();

    // Common work-related keywords
    let patterns = [
        ("實作", "實作"),
        ("implement", "實作"),
        ("設計", "設計"),
        ("design", "設計"),
        ("測試", "測試"),
        ("test", "測試"),
        ("修復", "修復"),
        ("fix", "修復"),
        ("bug", "修復"),
        ("研究", "研究"),
        ("調查", "研究"),
        ("文件", "文件撰寫"),
        ("doc", "文件撰寫"),
        ("review", "程式碼審查"),
        ("審查", "程式碼審查"),
        ("部署", "部署"),
        ("deploy", "部署"),
        ("優化", "效能優化"),
        ("refactor", "程式碼重構"),
        ("重構", "程式碼重構"),
        ("cli", "CLI 開發"),
        ("api", "API 開發"),
        ("sync", "資料同步"),
        ("同步", "資料同步"),
        ("gitlab", "GitLab 整合"),
        ("git", "版本控制"),
    ];

    for (pattern, keyword) in patterns {
        if title.contains(pattern) {
            keywords.push(keyword.to_string());
        }
    }

    keywords
}

fn format_keyword_summary(keyword: &str, items: &[&recap_core::WorkItem]) -> String {
    // Find related items and create a meaningful summary
    let related: Vec<_> = items.iter()
        .filter(|i| clean_title(&i.title).to_lowercase().contains(&keyword.to_lowercase())
            || extract_keywords(&clean_title(&i.title).to_lowercase()).contains(&keyword.to_string()))
        .collect();

    if related.is_empty() {
        return keyword.to_string();
    }

    // Use the most descriptive title
    let best = related.iter()
        .max_by_key(|i| i.hours as i64)
        .map(|i| clean_title(&i.title))
        .unwrap_or_else(|| keyword.to_string());

    if best.len() > 5 {
        best
    } else {
        keyword.to_string()
    }
}

fn print_text_report(report: &TempoReport) {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  {} 工作報告", report.period);
    println!("║  期間: {} ~ {}", report.start_date, report.end_date);
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    for project in &report.projects {
        println!("📁 {} ({:.1} 小時)", project.project, project.hours);
        for summary in &project.summary {
            println!("   • {}", summary);
        }
        println!();
    }

    println!("───────────────────────────────────────────────────────────────");
    println!("總計: {:.1} 小時 / {} 項工作", report.total_hours, report.total_items);
}

fn print_markdown_report(report: &TempoReport) {
    println!("# {} 工作報告", report.period);
    println!();
    println!("**期間:** {} ~ {}", report.start_date, report.end_date);
    println!();

    for project in &report.projects {
        println!("## {} ({:.1} 小時)", project.project, project.hours);
        println!();
        for summary in &project.summary {
            println!("- {}", summary);
        }
        println!();
    }

    println!("---");
    println!("**總計:** {:.1} 小時 / {} 項工作", report.total_hours, report.total_items);
}

/// Get or find default user for CLI operations
/// Prefers user with LLM configured
async fn get_default_user_id(db: &recap_core::Database) -> Result<String> {
    // First try to find a user with LLM API key configured
    let user_with_llm: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM users WHERE llm_api_key IS NOT NULL AND llm_api_key != '' LIMIT 1"
    )
        .fetch_optional(&db.pool)
        .await?;

    if let Some((id,)) = user_with_llm {
        return Ok(id);
    }

    // Fall back to any user
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

    #[test]
    fn test_extract_project_name() {
        assert_eq!(extract_project_name("[recap] some work"), "recap");
        assert_eq!(extract_project_name("[funtime-website] task"), "funtime-website");
        assert_eq!(extract_project_name("no project tag"), "其他");
    }

    #[test]
    fn test_clean_title() {
        assert_eq!(clean_title("[recap] some work"), "some work");
        assert_eq!(clean_title("no tag"), "no tag");
    }

    #[test]
    fn test_parse_quarter() {
        assert_eq!(parse_quarter("2026-Q1").unwrap(), (2026, 1));
        assert_eq!(parse_quarter("2026-Q4").unwrap(), (2026, 4));
        assert!(parse_quarter("2026-Q5").is_err());
    }

    #[test]
    fn test_parse_half() {
        assert_eq!(parse_half("2026-H1").unwrap(), (2026, 1));
        assert_eq!(parse_half("2026-H2").unwrap(), (2026, 2));
        assert!(parse_half("2026-H3").is_err());
    }
}
