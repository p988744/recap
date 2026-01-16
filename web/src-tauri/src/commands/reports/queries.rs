//! Reports queries
//!
//! Commands for basic report generation.

use chrono::NaiveDate;
use std::collections::HashMap;
use tauri::State;

use recap_core::auth::verify_token;
use recap_core::models::WorkItem;

use crate::commands::AppState;
use super::types::{
    CategoryReport, CategorySummary, DailyItems, PersonalReport, ReportQuery,
    SourceSummary, SummaryReport,
};

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
