//! Reports commands
//!
//! Tauri commands for report generation operations.

use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

use recap_core::auth::verify_token;
use recap_core::models::WorkItem;
use recap_core::services::excel::{ExcelReportGenerator, ExcelWorkItem, ProjectSummary, ReportMetadata};

use super::AppState;

// Types

#[derive(Debug, Deserialize)]
pub struct ReportQuery {
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Serialize)]
pub struct DailyItems {
    pub date: String,
    pub hours: f64,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct PersonalReport {
    pub start_date: String,
    pub end_date: String,
    pub total_hours: f64,
    pub total_items: i64,
    pub items_by_date: Vec<DailyItems>,
    pub work_items: Vec<WorkItem>,
}

#[derive(Debug, Serialize)]
pub struct SourceSummary {
    pub source: String,
    pub hours: f64,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct SummaryReport {
    pub start_date: String,
    pub end_date: String,
    pub total_hours: f64,
    pub total_items: i64,
    pub synced_to_tempo: i64,
    pub mapped_to_jira: i64,
    pub by_source: Vec<SourceSummary>,
}

#[derive(Debug, Serialize)]
pub struct CategorySummary {
    pub category: String,
    pub hours: f64,
    pub count: i64,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct CategoryReport {
    pub start_date: String,
    pub end_date: String,
    pub categories: Vec<CategorySummary>,
}

#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub success: bool,
    pub file_path: Option<String>,
    pub error: Option<String>,
}

// Commands

/// Get personal report for date range
#[tauri::command]
pub async fn get_personal_report(
    state: State<'_, AppState>,
    token: String,
    query: ReportQuery,
) -> Result<PersonalReport, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let start_date = NaiveDate::parse_from_str(&query.start_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid start_date: {}", e))?;
    let end_date = NaiveDate::parse_from_str(&query.end_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid end_date: {}", e))?;

    let work_items: Vec<WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE user_id = ? AND date >= ? AND date <= ? ORDER BY date DESC, created_at DESC",
    )
    .bind(&claims.sub)
    .bind(&start_date)
    .bind(&end_date)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let total_hours: f64 = work_items.iter().map(|i| i.hours).sum();
    let total_items = work_items.len() as i64;

    // Group by date
    let mut items_by_date: Vec<DailyItems> = Vec::new();
    let mut current_date: Option<NaiveDate> = None;
    let mut current_hours = 0.0;
    let mut current_count = 0i64;

    for item in &work_items {
        if current_date.is_none() || current_date != Some(item.date) {
            if let Some(date) = current_date {
                items_by_date.push(DailyItems {
                    date: date.to_string(),
                    hours: current_hours,
                    count: current_count,
                });
            }
            current_date = Some(item.date);
            current_hours = item.hours;
            current_count = 1;
        } else {
            current_hours += item.hours;
            current_count += 1;
        }
    }

    if let Some(date) = current_date {
        items_by_date.push(DailyItems {
            date: date.to_string(),
            hours: current_hours,
            count: current_count,
        });
    }

    Ok(PersonalReport {
        start_date: query.start_date,
        end_date: query.end_date,
        total_hours,
        total_items,
        items_by_date,
        work_items,
    })
}

/// Get summary report
#[tauri::command]
pub async fn get_summary_report(
    state: State<'_, AppState>,
    token: String,
    query: ReportQuery,
) -> Result<SummaryReport, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let start_date = NaiveDate::parse_from_str(&query.start_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid start_date: {}", e))?;
    let end_date = NaiveDate::parse_from_str(&query.end_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid end_date: {}", e))?;

    let work_items: Vec<WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE user_id = ? AND date >= ? AND date <= ?",
    )
    .bind(&claims.sub)
    .bind(&start_date)
    .bind(&end_date)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let total_hours: f64 = work_items.iter().map(|i| i.hours).sum();
    let total_items = work_items.len() as i64;
    let synced_to_tempo = work_items.iter().filter(|i| i.synced_to_tempo).count() as i64;
    let mapped_to_jira = work_items
        .iter()
        .filter(|i| i.jira_issue_key.is_some())
        .count() as i64;

    let mut source_map: HashMap<String, (f64, i64)> = HashMap::new();

    for item in &work_items {
        let entry = source_map.entry(item.source.clone()).or_insert((0.0, 0));
        entry.0 += item.hours;
        entry.1 += 1;
    }

    let by_source: Vec<SourceSummary> = source_map
        .into_iter()
        .map(|(source, (hours, count))| SourceSummary {
            source,
            hours,
            count,
        })
        .collect();

    Ok(SummaryReport {
        start_date: query.start_date,
        end_date: query.end_date,
        total_hours,
        total_items,
        synced_to_tempo,
        mapped_to_jira,
        by_source,
    })
}

/// Get report grouped by category
#[tauri::command]
pub async fn get_category_report(
    state: State<'_, AppState>,
    token: String,
    query: ReportQuery,
) -> Result<CategoryReport, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let start_date = NaiveDate::parse_from_str(&query.start_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid start_date: {}", e))?;
    let end_date = NaiveDate::parse_from_str(&query.end_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid end_date: {}", e))?;

    let work_items: Vec<WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE user_id = ? AND date >= ? AND date <= ?",
    )
    .bind(&claims.sub)
    .bind(&start_date)
    .bind(&end_date)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let total_hours: f64 = work_items.iter().map(|i| i.hours).sum();

    let mut category_map: HashMap<String, (f64, i64)> = HashMap::new();

    for item in &work_items {
        let category = item.category.clone().unwrap_or_else(|| "Uncategorized".to_string());
        let entry = category_map.entry(category).or_insert((0.0, 0));
        entry.0 += item.hours;
        entry.1 += 1;
    }

    let categories: Vec<CategorySummary> = category_map
        .into_iter()
        .map(|(category, (hours, count))| {
            let percentage = if total_hours > 0.0 {
                (hours / total_hours) * 100.0
            } else {
                0.0
            };
            CategorySummary {
                category,
                hours,
                count,
                percentage,
            }
        })
        .collect();

    Ok(CategoryReport {
        start_date: query.start_date,
        end_date: query.end_date,
        categories,
    })
}

/// Get report grouped by source
#[tauri::command]
pub async fn get_source_report(
    state: State<'_, AppState>,
    token: String,
    query: ReportQuery,
) -> Result<CategoryReport, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let start_date = NaiveDate::parse_from_str(&query.start_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid start_date: {}", e))?;
    let end_date = NaiveDate::parse_from_str(&query.end_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid end_date: {}", e))?;

    let work_items: Vec<WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE user_id = ? AND date >= ? AND date <= ?",
    )
    .bind(&claims.sub)
    .bind(&start_date)
    .bind(&end_date)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let total_hours: f64 = work_items.iter().map(|i| i.hours).sum();

    let mut source_map: HashMap<String, (f64, i64)> = HashMap::new();

    for item in &work_items {
        let entry = source_map.entry(item.source.clone()).or_insert((0.0, 0));
        entry.0 += item.hours;
        entry.1 += 1;
    }

    let categories: Vec<CategorySummary> = source_map
        .into_iter()
        .map(|(source, (hours, count))| {
            let percentage = if total_hours > 0.0 {
                (hours / total_hours) * 100.0
            } else {
                0.0
            };
            CategorySummary {
                category: source,
                hours,
                count,
                percentage,
            }
        })
        .collect();

    Ok(CategoryReport {
        start_date: query.start_date,
        end_date: query.end_date,
        categories,
    })
}

// Tempo Report Types

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TempoReportPeriod {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    SemiAnnual,
}

#[derive(Debug, Deserialize)]
pub struct TempoReportQuery {
    pub period: TempoReportPeriod,
    pub date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TempoProjectSummary {
    pub project: String,
    pub hours: f64,
    pub item_count: i64,
    pub summaries: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct TempoReport {
    pub period: String,
    pub start_date: String,
    pub end_date: String,
    pub total_hours: f64,
    pub total_items: i64,
    pub projects: Vec<TempoProjectSummary>,
    pub used_llm: bool,
}

/// Generate smart Tempo report with LLM summaries
#[tauri::command]
pub async fn generate_tempo_report(
    state: State<'_, AppState>,
    token: String,
    query: TempoReportQuery,
) -> Result<TempoReport, String> {
    use chrono::{Datelike, Duration};

    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let today = chrono::Local::now().date_naive();

    // Resolve period to date range
    let (start_date, end_date, period_name) = match query.period {
        TempoReportPeriod::Daily => {
            let target = match &query.date {
                Some(d) => NaiveDate::parse_from_str(d, "%Y-%m-%d")
                    .map_err(|_| "Invalid date format. Use YYYY-MM-DD".to_string())?,
                None => today,
            };
            (target, target, format!("Daily ({})", target))
        }
        TempoReportPeriod::Weekly => {
            let start = match &query.date {
                Some(d) => NaiveDate::parse_from_str(d, "%Y-%m-%d")
                    .map_err(|_| "Invalid date format. Use YYYY-MM-DD".to_string())?,
                None => {
                    let weekday = today.weekday().num_days_from_monday();
                    today - Duration::days(weekday as i64)
                }
            };
            let end = start + Duration::days(6);
            (start, end, format!("Weekly (W{})", start.iso_week().week()))
        }
        TempoReportPeriod::Monthly => {
            let (year, month) = match &query.date {
                Some(d) => {
                    let parts: Vec<&str> = d.split('-').collect();
                    if parts.len() >= 2 {
                        (parts[0].parse::<i32>().map_err(|_| "Invalid year")?,
                         parts[1].parse::<u32>().map_err(|_| "Invalid month")?)
                    } else {
                        return Err("Invalid month format. Use YYYY-MM".to_string());
                    }
                }
                None => (today.year(), today.month()),
            };
            let start = NaiveDate::from_ymd_opt(year, month, 1)
                .ok_or_else(|| "Invalid month".to_string())?;
            let end = if month == 12 {
                NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap() - Duration::days(1)
            } else {
                NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap() - Duration::days(1)
            };
            (start, end, format!("Monthly ({}-{:02})", year, month))
        }
        TempoReportPeriod::Quarterly => {
            let (year, quarter) = match &query.date {
                Some(d) => parse_quarter(d)?,
                None => {
                    let q = (today.month() - 1) / 3 + 1;
                    (today.year(), q)
                }
            };
            let start_month = (quarter - 1) * 3 + 1;
            let end_month = quarter * 3;
            let start = NaiveDate::from_ymd_opt(year, start_month, 1)
                .ok_or_else(|| "Invalid quarter".to_string())?;
            let end = if end_month == 12 {
                NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap() - Duration::days(1)
            } else {
                NaiveDate::from_ymd_opt(year, end_month + 1, 1).unwrap() - Duration::days(1)
            };
            (start, end, format!("Quarterly ({}-Q{})", year, quarter))
        }
        TempoReportPeriod::SemiAnnual => {
            let (year, half) = match &query.date {
                Some(d) => parse_half(d)?,
                None => {
                    let h = if today.month() <= 6 { 1 } else { 2 };
                    (today.year(), h)
                }
            };
            let (start_month, end_month) = if half == 1 { (1, 6) } else { (7, 12) };
            let start = NaiveDate::from_ymd_opt(year, start_month, 1)
                .ok_or_else(|| "Invalid half".to_string())?;
            let end = if end_month == 12 {
                NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap() - Duration::days(1)
            } else {
                NaiveDate::from_ymd_opt(year, end_month + 1, 1).unwrap() - Duration::days(1)
            };
            (start, end, format!("Semi-Annual ({}-H{})", year, half))
        }
    };

    // Try to create LLM service
    let llm_service = recap_core::create_llm_service(&db.pool, &claims.sub).await.ok();
    let use_llm = llm_service.as_ref().map(|s| s.is_configured()).unwrap_or(false);

    // Fetch work items
    let items: Vec<WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE user_id = ? AND date >= ? AND date <= ? ORDER BY date"
    )
    .bind(&claims.sub)
    .bind(start_date.to_string())
    .bind(end_date.to_string())
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let total_items = items.len() as i64;
    let total_hours: f64 = items.iter().map(|i| i.hours).sum();

    // Group by project
    let mut projects_map: HashMap<String, Vec<&WorkItem>> = HashMap::new();
    for item in &items {
        let project = extract_project_name(&item.title);
        projects_map.entry(project).or_default().push(item);
    }

    // Build report
    let mut projects: Vec<TempoProjectSummary> = Vec::new();

    for (project, project_items) in &projects_map {
        let hours: f64 = project_items.iter().map(|i| i.hours).sum();
        let item_count = project_items.len() as i64;

        // Generate smart summary using LLM if available
        let summaries = if use_llm {
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
                Ok(s) => s,
                Err(_) => generate_fallback_summary(project_items),
            }
        } else {
            generate_fallback_summary(project_items)
        };

        projects.push(TempoProjectSummary {
            project: project.clone(),
            hours,
            item_count,
            summaries,
        });
    }

    // Sort by hours descending
    projects.sort_by(|a, b| b.hours.partial_cmp(&a.hours).unwrap_or(std::cmp::Ordering::Equal));

    Ok(TempoReport {
        period: period_name,
        start_date: start_date.to_string(),
        end_date: end_date.to_string(),
        total_hours,
        total_items,
        projects,
        used_llm: use_llm,
    })
}

// Helper functions for Tempo report (pub(crate) for testing)

pub(crate) fn parse_quarter(s: &str) -> Result<(i32, u32), String> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        return Err("Invalid quarter format. Use YYYY-Q1/Q2/Q3/Q4".to_string());
    }
    let year = parts[0].parse::<i32>().map_err(|_| "Invalid year")?;
    let q = parts[1].trim_start_matches('Q').trim_start_matches('q')
        .parse::<u32>().map_err(|_| "Invalid quarter")?;
    if q < 1 || q > 4 {
        return Err("Quarter must be Q1, Q2, Q3, or Q4".to_string());
    }
    Ok((year, q))
}

pub(crate) fn parse_half(s: &str) -> Result<(i32, u32), String> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        return Err("Invalid half format. Use YYYY-H1/H2".to_string());
    }
    let year = parts[0].parse::<i32>().map_err(|_| "Invalid year")?;
    let h = parts[1].trim_start_matches('H').trim_start_matches('h')
        .parse::<u32>().map_err(|_| "Invalid half")?;
    if h < 1 || h > 2 {
        return Err("Half must be H1 or H2".to_string());
    }
    Ok((year, h))
}

pub(crate) fn extract_project_name(title: &str) -> String {
    if let Some(start) = title.find('[') {
        if let Some(end) = title.find(']') {
            if end > start {
                return title[start + 1..end].to_string();
            }
        }
    }
    "其他".to_string()
}

pub(crate) fn clean_title(title: &str) -> String {
    if let Some(end) = title.find(']') {
        title[end + 1..].trim().to_string()
    } else {
        title.to_string()
    }
}

pub(crate) fn generate_fallback_summary(items: &[&WorkItem]) -> Vec<String> {
    items.iter()
        .take(5)
        .map(|i| {
            let title = clean_title(&i.title);
            if title.len() > 50 {
                format!("{}...", title.chars().take(47).collect::<String>())
            } else {
                title
            }
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// Export work items to Excel file and return the file path
#[tauri::command]
pub async fn export_excel_report(
    state: State<'_, AppState>,
    token: String,
    query: ReportQuery,
) -> Result<ExportResult, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let start_date = NaiveDate::parse_from_str(&query.start_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid start_date: {}", e))?;
    let end_date = NaiveDate::parse_from_str(&query.end_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid end_date: {}", e))?;

    // Get user info
    let user_name: String = sqlx::query_scalar("SELECT name FROM users WHERE id = ?")
        .bind(&claims.sub)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    // Get work items
    let work_items: Vec<WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE user_id = ? AND date >= ? AND date <= ? AND parent_id IS NULL ORDER BY date DESC",
    )
    .bind(&claims.sub)
    .bind(&start_date)
    .bind(&end_date)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // Convert to Excel format
    let excel_items: Vec<ExcelWorkItem> = work_items
        .iter()
        .map(|item| ExcelWorkItem {
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

    // Group by project for summary
    let mut project_map: HashMap<String, (f64, usize)> = HashMap::new();

    for item in &work_items {
        let project = item.category.clone().unwrap_or_else(|| "No Category".to_string());
        let entry = project_map.entry(project).or_insert((0.0, 0));
        entry.0 += item.hours;
        entry.1 += 1;
    }

    let projects: Vec<ProjectSummary> = project_map
        .into_iter()
        .map(|(name, (hours, count))| ProjectSummary {
            project_name: name,
            total_hours: hours,
            item_count: count,
        })
        .collect();

    // Create metadata
    let metadata = ReportMetadata {
        user_name,
        start_date: query.start_date.clone(),
        end_date: query.end_date.clone(),
        generated_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    // Generate Excel
    let mut generator = match ExcelReportGenerator::new() {
        Ok(g) => g,
        Err(e) => return Ok(ExportResult {
            success: false,
            file_path: None,
            error: Some(e.to_string()),
        }),
    };

    if let Err(e) = generator.create_personal_report(&metadata, &excel_items, &projects) {
        return Ok(ExportResult {
            success: false,
            file_path: None,
            error: Some(e.to_string()),
        });
    }

    // Get downloads directory
    let downloads_dir = dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    let filename = format!(
        "work_report_{}_{}.xlsx",
        query.start_date.replace('-', ""),
        query.end_date.replace('-', "")
    );
    let file_path = downloads_dir.join(&filename);

    if let Err(e) = generator.save(&file_path) {
        return Ok(ExportResult {
            success: false,
            file_path: None,
            error: Some(e.to_string()),
        });
    }

    Ok(ExportResult {
        success: true,
        file_path: Some(file_path.to_string_lossy().to_string()),
        error: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    // ==================== parse_quarter Tests ====================

    #[test]
    fn test_parse_quarter_valid() {
        assert_eq!(parse_quarter("2024-Q1").unwrap(), (2024, 1));
        assert_eq!(parse_quarter("2024-Q2").unwrap(), (2024, 2));
        assert_eq!(parse_quarter("2024-Q3").unwrap(), (2024, 3));
        assert_eq!(parse_quarter("2024-Q4").unwrap(), (2024, 4));
    }

    #[test]
    fn test_parse_quarter_lowercase() {
        assert_eq!(parse_quarter("2024-q1").unwrap(), (2024, 1));
        assert_eq!(parse_quarter("2024-q4").unwrap(), (2024, 4));
    }

    #[test]
    fn test_parse_quarter_invalid_format() {
        assert!(parse_quarter("2024").is_err());
        assert!(parse_quarter("2024-").is_err());
        assert!(parse_quarter("2024-Q1-extra").is_err());
        assert!(parse_quarter("-Q1").is_err());
    }

    #[test]
    fn test_parse_quarter_invalid_quarter() {
        assert!(parse_quarter("2024-Q0").is_err());
        assert!(parse_quarter("2024-Q5").is_err());
        assert!(parse_quarter("2024-Qx").is_err());
    }

    #[test]
    fn test_parse_quarter_invalid_year() {
        assert!(parse_quarter("abc-Q1").is_err());
    }

    // ==================== parse_half Tests ====================

    #[test]
    fn test_parse_half_valid() {
        assert_eq!(parse_half("2024-H1").unwrap(), (2024, 1));
        assert_eq!(parse_half("2024-H2").unwrap(), (2024, 2));
    }

    #[test]
    fn test_parse_half_lowercase() {
        assert_eq!(parse_half("2024-h1").unwrap(), (2024, 1));
        assert_eq!(parse_half("2024-h2").unwrap(), (2024, 2));
    }

    #[test]
    fn test_parse_half_invalid_format() {
        assert!(parse_half("2024").is_err());
        assert!(parse_half("2024-").is_err());
        assert!(parse_half("2024-H1-extra").is_err());
    }

    #[test]
    fn test_parse_half_invalid_half() {
        assert!(parse_half("2024-H0").is_err());
        assert!(parse_half("2024-H3").is_err());
        assert!(parse_half("2024-Hx").is_err());
    }

    // ==================== extract_project_name Tests ====================

    #[test]
    fn test_extract_project_name_with_brackets() {
        assert_eq!(extract_project_name("[Project A] Task description"), "Project A");
        assert_eq!(extract_project_name("[My-Project] Feature work"), "My-Project");
        assert_eq!(extract_project_name("[recap] Fix authentication bug"), "recap");
    }

    #[test]
    fn test_extract_project_name_no_brackets() {
        assert_eq!(extract_project_name("Task without project"), "其他");
        assert_eq!(extract_project_name("Simple task"), "其他");
    }

    #[test]
    fn test_extract_project_name_malformed_brackets() {
        // Only opening bracket
        assert_eq!(extract_project_name("[Incomplete task"), "其他");
        // Only closing bracket
        assert_eq!(extract_project_name("Incomplete] task"), "其他");
        // Wrong order
        assert_eq!(extract_project_name("]Wrong[ order"), "其他");
    }

    #[test]
    fn test_extract_project_name_empty_brackets() {
        assert_eq!(extract_project_name("[] Empty project"), "");
    }

    #[test]
    fn test_extract_project_name_nested_brackets() {
        // Should extract up to first ]
        assert_eq!(extract_project_name("[Outer [Inner]] Task"), "Outer [Inner");
    }

    // ==================== clean_title Tests ====================

    #[test]
    fn test_clean_title_with_project() {
        assert_eq!(clean_title("[Project A] Task description"), "Task description");
        assert_eq!(clean_title("[recap] Fix bug"), "Fix bug");
    }

    #[test]
    fn test_clean_title_no_project() {
        assert_eq!(clean_title("Task without project"), "Task without project");
        assert_eq!(clean_title("Simple task"), "Simple task");
    }

    #[test]
    fn test_clean_title_trims_whitespace() {
        assert_eq!(clean_title("[Project]   Extra spaces   "), "Extra spaces");
        assert_eq!(clean_title("[P]  \t\n Task"), "Task");
    }

    #[test]
    fn test_clean_title_empty_after_bracket() {
        assert_eq!(clean_title("[Project]"), "");
        assert_eq!(clean_title("[Project]  "), "");
    }

    // ==================== generate_fallback_summary Tests ====================

    fn create_test_work_item(title: &str) -> WorkItem {
        WorkItem {
            id: "test-id".to_string(),
            user_id: "user-1".to_string(),
            source: "test".to_string(),
            source_id: None,
            source_url: None,
            title: title.to_string(),
            description: None,
            hours: 1.0,
            date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            jira_issue_key: None,
            jira_issue_suggested: None,
            jira_issue_title: None,
            category: None,
            tags: None,
            yearly_goal_id: None,
            synced_to_tempo: false,
            tempo_worklog_id: None,
            synced_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            parent_id: None,
            hours_source: None,
            hours_estimated: None,
            commit_hash: None,
            session_id: None,
            start_time: None,
            end_time: None,
            project_path: None,
        }
    }

    #[test]
    fn test_generate_fallback_summary_basic() {
        let item1 = create_test_work_item("[Project] Task 1");
        let item2 = create_test_work_item("[Project] Task 2");
        let items: Vec<&WorkItem> = vec![&item1, &item2];

        let summaries = generate_fallback_summary(&items);
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0], "Task 1");
        assert_eq!(summaries[1], "Task 2");
    }

    #[test]
    fn test_generate_fallback_summary_max_five() {
        let items: Vec<WorkItem> = (1..=10)
            .map(|i| create_test_work_item(&format!("[P] Task {}", i)))
            .collect();
        let item_refs: Vec<&WorkItem> = items.iter().collect();

        let summaries = generate_fallback_summary(&item_refs);
        assert_eq!(summaries.len(), 5); // Max 5 items
    }

    #[test]
    fn test_generate_fallback_summary_truncates_long_titles() {
        let long_title = format!("[Project] {}", "A".repeat(60));
        let item = create_test_work_item(&long_title);
        let items: Vec<&WorkItem> = vec![&item];

        let summaries = generate_fallback_summary(&items);
        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].len() <= 50, "Title should be truncated to 50 chars");
        assert!(summaries[0].ends_with("..."), "Truncated title should end with ...");
    }

    #[test]
    fn test_generate_fallback_summary_filters_empty() {
        let item = create_test_work_item("[Project]"); // Empty after cleaning
        let items: Vec<&WorkItem> = vec![&item];

        let summaries = generate_fallback_summary(&items);
        assert_eq!(summaries.len(), 0); // Empty strings should be filtered
    }

    #[test]
    fn test_generate_fallback_summary_empty_input() {
        let items: Vec<&WorkItem> = vec![];
        let summaries = generate_fallback_summary(&items);
        assert!(summaries.is_empty());
    }
}
