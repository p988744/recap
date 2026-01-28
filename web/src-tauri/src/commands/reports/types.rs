//! Reports types
//!
//! Type definitions for report commands.

use serde::{Deserialize, Serialize};

use recap_core::models::WorkItem;

// ==================== Query Types ====================

#[derive(Debug, Deserialize)]
pub struct ReportQuery {
    pub start_date: String,
    pub end_date: String,
}

// ==================== Basic Report Types ====================

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

// ==================== Analyze Types ====================

#[derive(Debug, Deserialize)]
pub struct AnalyzeQuery {
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeDailyEntry {
    pub date: String,
    pub minutes: f64,
    pub hours: f64,
    pub todos: Vec<String>,
    pub summaries: Vec<String>,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeProjectSummary {
    pub project_name: String,
    pub project_path: String,
    pub total_minutes: f64,
    pub total_hours: f64,
    pub daily_entries: Vec<AnalyzeDailyEntry>,
    pub jira_id: Option<String>,
    pub jira_id_suggestions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeResponse {
    pub start_date: String,
    pub end_date: String,
    pub total_minutes: f64,
    pub total_hours: f64,
    pub dates_covered: Vec<String>,
    pub projects: Vec<AnalyzeProjectSummary>,
    pub mode: String,
}

// ==================== Tempo Report Types ====================

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_query_deserialize() {
        let json = r#"{"start_date": "2024-01-01", "end_date": "2024-01-31"}"#;
        let query: ReportQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.start_date, "2024-01-01");
        assert_eq!(query.end_date, "2024-01-31");
    }

    #[test]
    fn test_tempo_report_period_deserialize() {
        let json = r#""daily""#;
        let period: TempoReportPeriod = serde_json::from_str(json).unwrap();
        assert!(matches!(period, TempoReportPeriod::Daily));

        let json = r#""quarterly""#;
        let period: TempoReportPeriod = serde_json::from_str(json).unwrap();
        assert!(matches!(period, TempoReportPeriod::Quarterly));
    }

    #[test]
    fn test_tempo_report_query_deserialize() {
        let json = r#"{"period": "weekly", "date": "2024-01-15"}"#;
        let query: TempoReportQuery = serde_json::from_str(json).unwrap();
        assert!(matches!(query.period, TempoReportPeriod::Weekly));
        assert_eq!(query.date, Some("2024-01-15".to_string()));
    }

    #[test]
    fn test_export_result_serialize() {
        let result = ExportResult {
            success: true,
            file_path: Some("/path/to/file.xlsx".to_string()),
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("file.xlsx"));
    }
}
