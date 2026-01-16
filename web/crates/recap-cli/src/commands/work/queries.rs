//! Work item query commands
//!
//! Read operations for work items.

use anyhow::Result;

use crate::commands::Context;
use crate::output::{print_output, print_single};
use super::helpers::{parse_date, resolve_work_item_id};
use super::types::WorkItemRow;

pub async fn list_work_items(
    ctx: &Context,
    date: Option<String>,
    start: Option<String>,
    end: Option<String>,
    source: Option<String>,
    limit: i64,
) -> Result<()> {
    let mut query = String::from(
        "SELECT * FROM work_items WHERE 1=1"
    );
    let mut bindings: Vec<String> = Vec::new();

    // Handle date filtering
    if let Some(d) = date {
        let parsed_date = parse_date(&d)?;
        query.push_str(" AND date = ?");
        bindings.push(parsed_date.to_string());
    } else if let (Some(s), Some(e)) = (start, end) {
        let start_date = parse_date(&s)?;
        let end_date = parse_date(&e)?;
        query.push_str(" AND date >= ? AND date <= ?");
        bindings.push(start_date.to_string());
        bindings.push(end_date.to_string());
    }

    if let Some(src) = source {
        query.push_str(" AND source = ?");
        bindings.push(src);
    }

    query.push_str(" ORDER BY date DESC, created_at DESC LIMIT ?");
    bindings.push(limit.to_string());

    // Build the query with bindings
    let mut sqlx_query = sqlx::query_as::<_, recap_core::WorkItem>(&query);
    for binding in &bindings {
        sqlx_query = sqlx_query.bind(binding);
    }

    let items: Vec<recap_core::WorkItem> = sqlx_query
        .fetch_all(&ctx.db.pool)
        .await?;

    let rows: Vec<WorkItemRow> = items.into_iter().map(WorkItemRow::from).collect();
    print_output(&rows, ctx.format)?;

    Ok(())
}

pub async fn show_work_item(ctx: &Context, id: String) -> Result<()> {
    let full_id = resolve_work_item_id(&ctx.db, &id).await?;

    let item: recap_core::WorkItem = sqlx::query_as("SELECT * FROM work_items WHERE id = ?")
        .bind(&full_id)
        .fetch_one(&ctx.db.pool)
        .await?;

    print_single(&WorkItemRow::from(item), ctx.format)?;

    Ok(())
}
