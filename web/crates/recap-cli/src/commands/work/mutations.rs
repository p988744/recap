//! Work item mutation commands
//!
//! Create, update, and delete operations for work items.

use anyhow::Result;

use crate::commands::Context;
use crate::output::{print_error, print_single, print_success};
use super::helpers::{get_or_create_default_user, parse_date, resolve_work_item_id};
use super::types::WorkItemRow;

pub async fn add_work_item(
    ctx: &Context,
    title: String,
    hours: f64,
    date: Option<String>,
    description: Option<String>,
    category: Option<String>,
    jira: Option<String>,
) -> Result<()> {
    let date = match date {
        Some(d) => parse_date(&d)?,
        None => chrono::Local::now().date_naive(),
    };

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    // For CLI, we use a default user_id (simplified auth)
    let user_id = get_or_create_default_user(&ctx.db).await?;

    sqlx::query(
        r#"
        INSERT INTO work_items (id, user_id, source, title, description, hours, date, category, jira_issue_key, created_at, updated_at)
        VALUES (?, ?, 'manual', ?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&id)
    .bind(&user_id)
    .bind(&title)
    .bind(&description)
    .bind(hours)
    .bind(date)
    .bind(&category)
    .bind(&jira)
    .bind(now)
    .bind(now)
    .execute(&ctx.db.pool)
    .await?;

    print_success(&format!("Created work item: {}", &id[..8]), ctx.quiet);

    // Show the created item
    if !ctx.quiet {
        let item: recap_core::WorkItem = sqlx::query_as("SELECT * FROM work_items WHERE id = ?")
            .bind(&id)
            .fetch_one(&ctx.db.pool)
            .await?;
        print_single(&WorkItemRow::from(item), ctx.format)?;
    }

    Ok(())
}

pub async fn update_work_item(
    ctx: &Context,
    id: String,
    title: Option<String>,
    hours: Option<f64>,
    description: Option<String>,
    jira: Option<String>,
) -> Result<()> {
    // Find the item (support short ID)
    let full_id = resolve_work_item_id(&ctx.db, &id).await?;

    let now = chrono::Utc::now();

    // Build dynamic update query
    let mut updates = vec!["updated_at = ?".to_string()];
    let mut bindings: Vec<String> = vec![now.to_rfc3339()];

    if let Some(t) = title {
        updates.push("title = ?".to_string());
        bindings.push(t);
    }
    if let Some(h) = hours {
        updates.push("hours = ?".to_string());
        updates.push("hours_source = ?".to_string());
        bindings.push(h.to_string());
        bindings.push("user_modified".to_string());
    }
    if let Some(d) = description {
        updates.push("description = ?".to_string());
        bindings.push(d);
    }
    if let Some(j) = jira {
        updates.push("jira_issue_key = ?".to_string());
        bindings.push(j);
    }

    let query = format!(
        "UPDATE work_items SET {} WHERE id = ?",
        updates.join(", ")
    );
    bindings.push(full_id.clone());

    let mut sqlx_query = sqlx::query(&query);
    for binding in &bindings {
        sqlx_query = sqlx_query.bind(binding);
    }

    sqlx_query.execute(&ctx.db.pool).await?;

    print_success(&format!("Updated work item: {}", &full_id[..8]), ctx.quiet);

    Ok(())
}

pub async fn delete_work_item(ctx: &Context, id: String, force: bool) -> Result<()> {
    let full_id = resolve_work_item_id(&ctx.db, &id).await?;

    if !force {
        // Show item before deletion
        let item: recap_core::WorkItem = sqlx::query_as("SELECT * FROM work_items WHERE id = ?")
            .bind(&full_id)
            .fetch_one(&ctx.db.pool)
            .await?;

        print_single(&WorkItemRow::from(item), ctx.format)?;
        print_error("Use --force to confirm deletion");
        return Ok(());
    }

    sqlx::query("DELETE FROM work_items WHERE id = ?")
        .bind(&full_id)
        .execute(&ctx.db.pool)
        .await?;

    print_success(&format!("Deleted work item: {}", &full_id[..8]), ctx.quiet);

    Ok(())
}
