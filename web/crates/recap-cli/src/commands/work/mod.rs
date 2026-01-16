//! Work item commands
//!
//! Commands for managing work items: list, add, update, delete.

pub mod helpers;
mod mutations;
mod queries;
mod types;

use anyhow::Result;

use crate::commands::Context;

// Re-export public types
pub use types::{WorkAction, WorkItemRow};

pub async fn execute(ctx: &Context, action: WorkAction) -> Result<()> {
    match action {
        WorkAction::List { date, start, end, source, limit } => {
            queries::list_work_items(ctx, date, start, end, source, limit).await
        }
        WorkAction::Add { title, hours, date, description, category, jira } => {
            mutations::add_work_item(ctx, title, hours, date, description, category, jira).await
        }
        WorkAction::Update { id, title, hours, description, jira } => {
            mutations::update_work_item(ctx, id, title, hours, description, jira).await
        }
        WorkAction::Delete { id, force } => {
            mutations::delete_work_item(ctx, id, force).await
        }
        WorkAction::Show { id } => {
            queries::show_work_item(ctx, id).await
        }
    }
}
