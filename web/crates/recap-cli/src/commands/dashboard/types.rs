//! Dashboard types
//!
//! Types for dashboard commands and display.

use clap::Subcommand;
use serde::Serialize;
use tabled::Tabled;

#[derive(Subcommand)]
pub enum DashboardAction {
    /// Show statistics summary
    Stats {
        /// Start date (YYYY-MM-DD), defaults to start of current week
        #[arg(short, long)]
        start: Option<String>,

        /// End date (YYYY-MM-DD), defaults to end of current week
        #[arg(short, long)]
        end: Option<String>,

        /// Show this week's stats (default)
        #[arg(long)]
        week: bool,

        /// Show this month's stats
        #[arg(long)]
        month: bool,
    },

    /// Show work timeline for a specific date
    Timeline {
        /// Date to show (YYYY-MM-DD), defaults to today
        #[arg(short, long)]
        date: Option<String>,
    },

    /// Show daily hours heatmap data
    Heatmap {
        /// Number of weeks to show (default: 12)
        #[arg(short, long, default_value = "12")]
        weeks: u32,
    },

    /// Show project distribution
    Projects {
        /// Start date (YYYY-MM-DD), defaults to start of current week
        #[arg(short, long)]
        start: Option<String>,

        /// End date (YYYY-MM-DD), defaults to end of current week
        #[arg(short, long)]
        end: Option<String>,
    },
}

#[derive(Debug, Serialize, Tabled)]
pub struct StatsRow {
    #[tabled(rename = "指標")]
    pub metric: String,
    #[tabled(rename = "數值")]
    pub value: String,
}

#[derive(Debug, Serialize, Tabled)]
pub struct SourceRow {
    #[tabled(rename = "來源")]
    pub source: String,
    #[tabled(rename = "工時")]
    pub hours: String,
    #[tabled(rename = "佔比")]
    pub percentage: String,
}

#[derive(Debug, Serialize, Tabled)]
pub struct ProjectRow {
    #[tabled(rename = "專案")]
    pub project: String,
    #[tabled(rename = "工時")]
    pub hours: String,
    #[tabled(rename = "項目數")]
    pub items: String,
    #[tabled(rename = "佔比")]
    pub percentage: String,
}

#[derive(Debug, Serialize, Tabled)]
pub struct TimelineRow {
    #[tabled(rename = "時間")]
    pub time: String,
    #[tabled(rename = "專案")]
    pub project: String,
    #[tabled(rename = "工時")]
    pub hours: String,
    #[tabled(rename = "標題")]
    pub title: String,
    #[tabled(rename = "Commits")]
    pub commits: String,
}

#[derive(Debug, Serialize, Tabled)]
pub struct HeatmapRow {
    #[tabled(rename = "日期")]
    pub date: String,
    #[tabled(rename = "星期")]
    pub weekday: String,
    #[tabled(rename = "工時")]
    pub hours: String,
    #[tabled(rename = "項目")]
    pub items: String,
    #[tabled(rename = "視覺化")]
    pub visual: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_row_serialization() {
        let row = StatsRow {
            metric: "總工時".to_string(),
            value: "40.5".to_string(),
        };
        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("總工時"));
        assert!(json.contains("40.5"));
    }

    #[test]
    fn test_source_row_serialization() {
        let row = SourceRow {
            source: "git".to_string(),
            hours: "20.0".to_string(),
            percentage: "50%".to_string(),
        };
        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("git"));
        assert!(json.contains("50%"));
    }

    #[test]
    fn test_project_row_serialization() {
        let row = ProjectRow {
            project: "recap".to_string(),
            hours: "15.5".to_string(),
            items: "10".to_string(),
            percentage: "38%".to_string(),
        };
        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("recap"));
        assert!(json.contains("15.5"));
    }

    #[test]
    fn test_timeline_row_serialization() {
        let row = TimelineRow {
            time: "09:00".to_string(),
            project: "test".to_string(),
            hours: "2.0".to_string(),
            title: "Task".to_string(),
            commits: "3".to_string(),
        };
        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("09:00"));
        assert!(json.contains("test"));
        assert!(json.contains("commits"));
    }

    #[test]
    fn test_heatmap_row_serialization() {
        let row = HeatmapRow {
            date: "2025-01-15".to_string(),
            weekday: "Wed".to_string(),
            hours: "8.0".to_string(),
            items: "5".to_string(),
            visual: "███".to_string(),
        };
        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("2025-01-15"));
        assert!(json.contains("Wed"));
        assert!(json.contains("visual"));
    }

    #[test]
    fn test_stats_row_debug() {
        let row = StatsRow {
            metric: "Test".to_string(),
            value: "123".to_string(),
        };
        let debug = format!("{:?}", row);
        assert!(debug.contains("Test"));
    }
}
