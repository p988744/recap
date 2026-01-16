//! Dashboard commands
//!
//! CLI commands for displaying dashboard statistics and visualizations.

mod helpers;
mod heatmap;
mod projects;
mod stats;
mod timeline;
mod types;

use anyhow::Result;

use crate::commands::Context;

// Re-export public types
pub use types::{DashboardAction, HeatmapRow, ProjectRow, SourceRow, StatsRow, TimelineRow};

pub async fn execute(ctx: &Context, action: DashboardAction) -> Result<()> {
    match action {
        DashboardAction::Stats { start, end, week, month } => {
            stats::show_stats(ctx, start, end, week, month).await
        }
        DashboardAction::Timeline { date } => {
            timeline::show_timeline(ctx, date).await
        }
        DashboardAction::Heatmap { weeks } => {
            heatmap::show_heatmap(ctx, weeks).await
        }
        DashboardAction::Projects { start, end } => {
            projects::show_projects(ctx, start, end).await
        }
    }
}
