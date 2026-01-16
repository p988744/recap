//! Reports export commands
//!
//! Commands for exporting reports to Excel and generating Tempo reports.

use chrono::{Datelike, Duration, NaiveDate, Utc};
use std::collections::HashMap;
use tauri::State;

use recap_core::auth::verify_token;
use recap_core::models::WorkItem;
use recap_core::services::excel::{ExcelReportGenerator, ExcelWorkItem, ProjectSummary, ReportMetadata};

use crate::commands::AppState;
use super::helpers::{clean_title, extract_project_name, generate_fallback_summary, parse_half, parse_quarter};
use super::types::{ExportResult, ReportQuery, TempoProjectSummary, TempoReport, TempoReportPeriod, TempoReportQuery};

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

/// Generate smart Tempo report with LLM summaries
#[tauri::command]
pub async fn generate_tempo_report(
    state: State<'_, AppState>,
    token: String,
    query: TempoReportQuery,
) -> Result<TempoReport, String> {
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
