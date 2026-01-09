//! Reports API routes

use axum::{
    body::Body,
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    db::Database,
    models::WorkItem,
    services::excel::{ExcelReportGenerator, ExcelWorkItem, ProjectSummary, ReportMetadata},
};

/// Reports routes
pub fn routes() -> Router<Database> {
    Router::new()
        .route("/personal", get(personal_report))
        .route("/summary", get(summary_report))
        .route("/by-category", get(by_category_report))
        .route("/by-source", get(by_source_report))
        .route("/export/excel", get(export_excel))
}

#[derive(Debug, Deserialize)]
pub struct ReportQuery {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct PersonalReport {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub total_hours: f64,
    pub total_items: i64,
    pub items_by_date: Vec<DailyItems>,
    pub work_items: Vec<WorkItem>,
}

#[derive(Debug, Serialize)]
pub struct DailyItems {
    pub date: NaiveDate,
    pub hours: f64,
    pub count: i64,
}

/// Get personal report for date range
async fn personal_report(
    State(db): State<Database>,
    auth: AuthUser,
    Query(query): Query<ReportQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Get all work items in date range
    let work_items: Vec<WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE user_id = ? AND date >= ? AND date <= ? ORDER BY date DESC, created_at DESC",
    )
    .bind(&auth.0.sub)
    .bind(&query.start_date)
    .bind(&query.end_date)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Calculate totals
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
                    date,
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

    // Add last group
    if let Some(date) = current_date {
        items_by_date.push(DailyItems {
            date,
            hours: current_hours,
            count: current_count,
        });
    }

    Ok(Json(PersonalReport {
        start_date: query.start_date,
        end_date: query.end_date,
        total_hours,
        total_items,
        items_by_date,
        work_items,
    }))
}

#[derive(Debug, Serialize)]
pub struct SummaryReport {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub total_hours: f64,
    pub total_items: i64,
    pub synced_to_tempo: i64,
    pub mapped_to_jira: i64,
    pub by_source: Vec<SourceSummary>,
}

#[derive(Debug, Serialize)]
pub struct SourceSummary {
    pub source: String,
    pub hours: f64,
    pub count: i64,
}

/// Get summary report
async fn summary_report(
    State(db): State<Database>,
    auth: AuthUser,
    Query(query): Query<ReportQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Get all work items in date range
    let work_items: Vec<WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE user_id = ? AND date >= ? AND date <= ?",
    )
    .bind(&auth.0.sub)
    .bind(&query.start_date)
    .bind(&query.end_date)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_hours: f64 = work_items.iter().map(|i| i.hours).sum();
    let total_items = work_items.len() as i64;
    let synced_to_tempo = work_items.iter().filter(|i| i.synced_to_tempo).count() as i64;
    let mapped_to_jira = work_items
        .iter()
        .filter(|i| i.jira_issue_key.is_some())
        .count() as i64;

    // Group by source
    let mut source_map: std::collections::HashMap<String, (f64, i64)> =
        std::collections::HashMap::new();

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

    Ok(Json(SummaryReport {
        start_date: query.start_date,
        end_date: query.end_date,
        total_hours,
        total_items,
        synced_to_tempo,
        mapped_to_jira,
        by_source,
    }))
}

#[derive(Debug, Serialize)]
pub struct CategoryReport {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub categories: Vec<CategorySummary>,
}

#[derive(Debug, Serialize)]
pub struct CategorySummary {
    pub category: String,
    pub hours: f64,
    pub count: i64,
    pub percentage: f64,
}

/// Get report grouped by category
async fn by_category_report(
    State(db): State<Database>,
    auth: AuthUser,
    Query(query): Query<ReportQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let work_items: Vec<WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE user_id = ? AND date >= ? AND date <= ?",
    )
    .bind(&auth.0.sub)
    .bind(&query.start_date)
    .bind(&query.end_date)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_hours: f64 = work_items.iter().map(|i| i.hours).sum();

    // Group by category
    let mut category_map: std::collections::HashMap<String, (f64, i64)> =
        std::collections::HashMap::new();

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

    Ok(Json(CategoryReport {
        start_date: query.start_date,
        end_date: query.end_date,
        categories,
    }))
}

/// Get report grouped by source
async fn by_source_report(
    State(db): State<Database>,
    auth: AuthUser,
    Query(query): Query<ReportQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let work_items: Vec<WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE user_id = ? AND date >= ? AND date <= ?",
    )
    .bind(&auth.0.sub)
    .bind(&query.start_date)
    .bind(&query.end_date)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_hours: f64 = work_items.iter().map(|i| i.hours).sum();

    // Group by source
    let mut source_map: std::collections::HashMap<String, (f64, i64)> =
        std::collections::HashMap::new();

    for item in &work_items {
        let entry = source_map.entry(item.source.clone()).or_insert((0.0, 0));
        entry.0 += item.hours;
        entry.1 += 1;
    }

    let sources: Vec<CategorySummary> = source_map
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

    Ok(Json(CategoryReport {
        start_date: query.start_date,
        end_date: query.end_date,
        categories: sources,
    }))
}

/// Export work items to Excel file
async fn export_excel(
    State(db): State<Database>,
    auth: AuthUser,
    Query(query): Query<ReportQuery>,
) -> Result<Response, (StatusCode, String)> {
    // Get user info
    let user_name: String = sqlx::query_scalar("SELECT name FROM users WHERE id = ?")
        .bind(&auth.0.sub)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Get work items
    let work_items: Vec<WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE user_id = ? AND date >= ? AND date <= ? AND parent_id IS NULL ORDER BY date DESC",
    )
    .bind(&auth.0.sub)
    .bind(&query.start_date)
    .bind(&query.end_date)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Convert to Excel format
    let excel_items: Vec<ExcelWorkItem> = work_items
        .iter()
        .map(|item| ExcelWorkItem {
            date: item.date.to_string(),
            title: item.title.clone(),
            description: item.description.clone(),
            hours: item.hours,
            project: item.category.clone(), // Using category as project for now
            jira_key: item.jira_issue_key.clone(),
            source: item.source.clone(),
            synced_to_tempo: item.synced_to_tempo,
        })
        .collect();

    // Group by project for summary
    let mut project_map: std::collections::HashMap<String, (f64, usize)> =
        std::collections::HashMap::new();

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
        start_date: query.start_date.to_string(),
        end_date: query.end_date.to_string(),
        generated_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    // Generate Excel
    let mut generator = ExcelReportGenerator::new()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    generator
        .create_personal_report(&metadata, &excel_items, &projects)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let buffer = generator
        .save_to_buffer()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Create filename
    let filename = format!(
        "work_report_{}_{}.xlsx",
        query.start_date.format("%Y%m%d"),
        query.end_date.format("%Y%m%d")
    );

    // Build response with Excel content
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename))
        .body(Body::from(buffer))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(response)
}
