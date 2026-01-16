//! Tempo report types
//!
//! Types for tempo report commands.

use clap::{Subcommand, ValueEnum};
use serde::Serialize;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_summary_serialization() {
        let summary = ProjectSummary {
            project: "test-project".to_string(),
            hours: 10.5,
            items: vec![],
            summary: vec!["Did some work".to_string()],
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("test-project"));
        assert!(json.contains("10.5"));
    }

    #[test]
    fn test_work_item_brief_serialization() {
        let brief = WorkItemBrief {
            date: "2025-01-15".to_string(),
            title: "Test task".to_string(),
            hours: 2.0,
        };

        let json = serde_json::to_string(&brief).unwrap();
        assert!(json.contains("2025-01-15"));
        assert!(json.contains("Test task"));
    }

    #[test]
    fn test_tempo_report_serialization() {
        let report = TempoReport {
            period: "Weekly".to_string(),
            start_date: "2025-01-13".to_string(),
            end_date: "2025-01-19".to_string(),
            total_hours: 40.0,
            total_items: 10,
            projects: vec![],
        };

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("Weekly"));
        assert!(json.contains("40"));
    }

    #[test]
    fn test_period_enum_debug() {
        assert_eq!(format!("{:?}", Period::Daily), "Daily");
        assert_eq!(format!("{:?}", Period::Weekly), "Weekly");
        assert_eq!(format!("{:?}", Period::Monthly), "Monthly");
        assert_eq!(format!("{:?}", Period::Quarterly), "Quarterly");
        assert_eq!(format!("{:?}", Period::SemiAnnual), "SemiAnnual");
    }
}
