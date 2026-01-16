//! Tempo report commands
//!
//! Generate smart work summaries for Tempo time logging.

mod format;
mod generator;
mod helpers;
mod period;
mod types;

use anyhow::Result;

use crate::commands::Context;

// Re-export public types
pub use types::{Period, ProjectSummary, TempoReport, TempoReportAction, WorkItemBrief};

pub async fn execute(ctx: &Context, action: TempoReportAction) -> Result<()> {
    match action {
        TempoReportAction::Generate { period, date, output } => {
            generator::generate_tempo_report(ctx, period, date, output).await
        }
    }
}
