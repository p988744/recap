//! Work item types
//!
//! Types for work item commands.

use clap::Subcommand;
use serde::Serialize;
use tabled::Tabled;

use super::helpers::truncate;

#[derive(Subcommand)]
pub enum WorkAction {
    /// List work items
    List {
        /// Filter by date (YYYY-MM-DD), defaults to today
        #[arg(short, long)]
        date: Option<String>,

        /// Filter by date range start
        #[arg(long)]
        start: Option<String>,

        /// Filter by date range end
        #[arg(long)]
        end: Option<String>,

        /// Filter by source (git, claude, gitlab, manual)
        #[arg(short, long)]
        source: Option<String>,

        /// Maximum number of items to show
        #[arg(short, long, default_value = "50")]
        limit: i64,
    },

    /// Add a new work item
    Add {
        /// Work item title
        #[arg(short, long)]
        title: String,

        /// Hours spent
        #[arg(short = 'H', long, default_value = "1.0")]
        hours: f64,

        /// Date (YYYY-MM-DD), defaults to today
        #[arg(short, long)]
        date: Option<String>,

        /// Description
        #[arg(short = 'D', long)]
        description: Option<String>,

        /// Category
        #[arg(short, long)]
        category: Option<String>,

        /// Jira issue key
        #[arg(short, long)]
        jira: Option<String>,
    },

    /// Update an existing work item
    Update {
        /// Work item ID
        id: String,

        /// New title
        #[arg(short, long)]
        title: Option<String>,

        /// New hours
        #[arg(short = 'H', long)]
        hours: Option<f64>,

        /// New description
        #[arg(short = 'D', long)]
        description: Option<String>,

        /// New Jira issue key
        #[arg(short, long)]
        jira: Option<String>,
    },

    /// Delete a work item
    Delete {
        /// Work item ID
        id: String,

        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Show work item details
    Show {
        /// Work item ID
        id: String,
    },
}

/// Work item row for table display
#[derive(Debug, Serialize, Tabled)]
pub struct WorkItemRow {
    #[tabled(rename = "ID")]
    pub id: String,
    #[tabled(rename = "Date")]
    pub date: String,
    #[tabled(rename = "Title")]
    pub title: String,
    #[tabled(rename = "Hours")]
    pub hours: String,
    #[tabled(rename = "Source")]
    pub source: String,
    #[tabled(rename = "Jira")]
    pub jira: String,
}

impl From<recap_core::WorkItem> for WorkItemRow {
    fn from(item: recap_core::WorkItem) -> Self {
        Self {
            id: item.id[..8].to_string(), // Short ID
            date: item.date.to_string(),
            title: truncate(&item.title, 40),
            hours: format!("{:.1}", item.hours),
            source: item.source,
            jira: item.jira_issue_key.unwrap_or_else(|| "-".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_item_row_serialization() {
        let row = WorkItemRow {
            id: "abc12345".to_string(),
            date: "2025-01-15".to_string(),
            title: "Test work item".to_string(),
            hours: "2.5".to_string(),
            source: "manual".to_string(),
            jira: "PROJ-123".to_string(),
        };

        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("abc12345"));
        assert!(json.contains("Test work item"));
        assert!(json.contains("PROJ-123"));
    }

    #[test]
    fn test_work_item_row_hours_formatting() {
        let row = WorkItemRow {
            id: "12345678".to_string(),
            date: "2025-01-15".to_string(),
            title: "Test".to_string(),
            hours: format!("{:.1}", 2.5),
            source: "manual".to_string(),
            jira: "-".to_string(),
        };
        assert_eq!(row.hours, "2.5");
    }
}
