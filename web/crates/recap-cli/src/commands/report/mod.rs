//! Report commands
//!
//! Commands for generating work reports: summary, export.

mod export;
mod helpers;
mod summary;
mod types;

use anyhow::Result;

use crate::commands::Context;

// Re-export public types
pub use types::{DateSummaryRow, ReportAction, SummaryRow};

pub async fn execute(ctx: &Context, action: ReportAction) -> Result<()> {
    match action {
        ReportAction::Summary { start, end, group_by } => {
            summary::show_summary(ctx, start, end, group_by).await
        }
        ReportAction::Export { start, end, output } => {
            export::export_excel(ctx, start, end, output).await
        }
    }
}
