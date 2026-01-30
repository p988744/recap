//! Reports queries
//!
//! Commands for basic report generation.

use chrono::NaiveDate;
use std::collections::HashMap;
use tauri::State;

use recap_core::auth::verify_token;
use recap_core::models::WorkItem;

use crate::commands::AppState;
use super::helpers::extract_project_name;
use super::types::{
    AnalyzeDailyEntry, AnalyzeProjectSummary, AnalyzeQuery, AnalyzeResponse,
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

/// Analyze work items for a date range, grouped by project
///
/// Returns an AnalyzeResponse with projects and daily entries,
/// matching the format previously provided by the HTTP /analyze endpoint.
#[tauri::command]
pub async fn analyze_work_items(
    state: State<'_, AppState>,
    token: String,
    query: AnalyzeQuery,
) -> Result<AnalyzeResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let start_date = NaiveDate::parse_from_str(&query.start_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid start_date: {}", e))?;
    let end_date = NaiveDate::parse_from_str(&query.end_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid end_date: {}", e))?;

    let work_items: Vec<WorkItem> = sqlx::query_as(
        r#"SELECT * FROM work_items WHERE user_id = ? AND date >= ? AND date <= ?
           AND NOT EXISTS (
               SELECT 1 FROM project_preferences pp
               WHERE pp.user_id = work_items.user_id
               AND pp.hidden = 1
               AND work_items.title LIKE '[' || pp.project_name || ']%'
           )
           ORDER BY date ASC"#,
    )
    .bind(&claims.sub)
    .bind(&start_date)
    .bind(&end_date)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // Collect unique dates
    let mut dates_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    for item in &work_items {
        dates_set.insert(item.date.to_string());
    }
    let mut dates_covered: Vec<String> = dates_set.into_iter().collect();
    dates_covered.sort();

    // Group by project
    let mut project_map: HashMap<String, Vec<&WorkItem>> = HashMap::new();
    for item in &work_items {
        let project_name = extract_project_name(&item.title);
        project_map.entry(project_name).or_default().push(item);
    }

    let total_minutes: f64 = work_items.iter().map(|i| i.hours * 60.0).sum();
    let total_hours: f64 = work_items.iter().map(|i| i.hours).sum();

    // Build project summaries
    let mut projects: Vec<AnalyzeProjectSummary> = Vec::new();
    for (project_name, items) in &project_map {
        let proj_total_hours: f64 = items.iter().map(|i| i.hours).sum();
        let proj_total_minutes = proj_total_hours * 60.0;

        // Get project_path from first item that has one
        let project_path = items
            .iter()
            .find_map(|i| i.project_path.clone())
            .unwrap_or_default();

        // Collect jira IDs
        let jira_id = items.iter().find_map(|i| i.jira_issue_key.clone());
        let mut jira_suggestions: Vec<String> = items
            .iter()
            .filter_map(|i| i.jira_issue_suggested.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        jira_suggestions.sort();

        // Group items by date for daily entries
        let mut daily_map: HashMap<String, Vec<&WorkItem>> = HashMap::new();
        for item in items {
            daily_map
                .entry(item.date.to_string())
                .or_default()
                .push(item);
        }

        let mut daily_entries: Vec<AnalyzeDailyEntry> = Vec::new();
        let mut sorted_dates: Vec<String> = daily_map.keys().cloned().collect();
        sorted_dates.sort();

        for date in sorted_dates {
            let day_items = &daily_map[&date];
            let day_hours: f64 = day_items.iter().map(|i| i.hours).sum();
            let day_minutes = day_hours * 60.0;

            let descriptions: Vec<String> = day_items
                .iter()
                .filter_map(|i| i.description.clone())
                .filter(|d| !d.is_empty())
                .collect();

            let titles: Vec<String> = day_items.iter().map(|i| i.title.clone()).collect();

            let description = if !descriptions.is_empty() {
                descriptions.join("; ")
            } else {
                titles.join("; ")
            };

            daily_entries.push(AnalyzeDailyEntry {
                date,
                minutes: day_minutes,
                hours: day_hours,
                todos: titles,
                summaries: descriptions,
                description,
            });
        }

        projects.push(AnalyzeProjectSummary {
            project_name: project_name.clone(),
            project_path,
            total_minutes: proj_total_minutes,
            total_hours: proj_total_hours,
            daily_entries,
            jira_id,
            jira_id_suggestions: jira_suggestions,
        });
    }

    // Sort projects by total_hours descending
    projects.sort_by(|a, b| {
        b.total_hours
            .partial_cmp(&a.total_hours)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(AnalyzeResponse {
        start_date: query.start_date,
        end_date: query.end_date,
        total_minutes,
        total_hours,
        dates_covered,
        projects,
        mode: "tauri".to_string(),
    })
}
