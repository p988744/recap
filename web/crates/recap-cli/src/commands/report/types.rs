//! Report types
//!
//! Types for report commands.

use clap::Subcommand;
use serde::Serialize;
use tabled::Tabled;

#[derive(Subcommand)]
pub enum ReportAction {
    /// Show work summary for a date range
    Summary {
        /// Start date (YYYY-MM-DD), defaults to start of current month
        #[arg(short, long)]
        start: Option<String>,

        /// End date (YYYY-MM-DD), defaults to today
        #[arg(short, long)]
        end: Option<String>,

        /// Group by: date, project, source
        #[arg(short, long, default_value = "date")]
        group_by: String,
    },

    /// Export work items to Excel
    Export {
        /// Start date (YYYY-MM-DD), defaults to start of current month
        #[arg(short, long)]
        start: Option<String>,

        /// End date (YYYY-MM-DD), defaults to today
        #[arg(short, long)]
        end: Option<String>,

        /// Output file path (default: work_report.xlsx)
        #[arg(short, long, default_value = "work_report.xlsx")]
        output: String,
    },
}

/// Summary row for table display
#[derive(Debug, Serialize, Tabled)]
pub struct SummaryRow {
    #[tabled(rename = "Group")]
    pub group: String,
    #[tabled(rename = "Hours")]
    pub hours: String,
    #[tabled(rename = "Items")]
    pub items: String,
}

/// Date summary row
#[derive(Debug, Serialize, Tabled)]
pub struct DateSummaryRow {
    #[tabled(rename = "Date")]
    pub date: String,
    #[tabled(rename = "Hours")]
    pub hours: String,
    #[tabled(rename = "Items")]
    pub items: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_row_serialization() {
        let row = SummaryRow {
            group: "Project A".to_string(),
            hours: "10.5".to_string(),
            items: "5".to_string(),
        };

        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("Project A"));
        assert!(json.contains("10.5"));
        assert!(json.contains("5"));
    }

    #[test]
    fn test_summary_row_debug() {
        let row = SummaryRow {
            group: "Test".to_string(),
            hours: "8.0".to_string(),
            items: "3".to_string(),
        };

        let debug = format!("{:?}", row);
        assert!(debug.contains("Test"));
        assert!(debug.contains("8.0"));
    }

    #[test]
    fn test_date_summary_row_serialization() {
        let row = DateSummaryRow {
            date: "2025-01-15".to_string(),
            hours: "6.5".to_string(),
            items: "4".to_string(),
        };

        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("2025-01-15"));
        assert!(json.contains("6.5"));
    }

    #[test]
    fn test_date_summary_row_debug() {
        let row = DateSummaryRow {
            date: "2025-01-15".to_string(),
            hours: "4.0".to_string(),
            items: "2".to_string(),
        };

        let debug = format!("{:?}", row);
        assert!(debug.contains("2025-01-15"));
    }

    #[test]
    fn test_hours_formatting() {
        let hours = 2.5;
        let formatted = format!("{:.1}", hours);
        assert_eq!(formatted, "2.5");

        let hours2 = 10.0;
        let formatted2 = format!("{:.1}", hours2);
        assert_eq!(formatted2, "10.0");
    }
}
