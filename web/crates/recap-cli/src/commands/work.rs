//! Work item commands
//!
//! Commands for managing work items: list, add, update, delete.

use anyhow::Result;
use chrono::NaiveDate;
use clap::Subcommand;
use serde::Serialize;
use tabled::Tabled;

use crate::output::{print_error, print_output, print_single, print_success};
use super::Context;

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

fn truncate(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = chars[..max_chars - 3].iter().collect();
        format!("{}...", truncated)
    }
}

pub async fn execute(ctx: &Context, action: WorkAction) -> Result<()> {
    match action {
        WorkAction::List { date, start, end, source, limit } => {
            list_work_items(ctx, date, start, end, source, limit).await
        }
        WorkAction::Add { title, hours, date, description, category, jira } => {
            add_work_item(ctx, title, hours, date, description, category, jira).await
        }
        WorkAction::Update { id, title, hours, description, jira } => {
            update_work_item(ctx, id, title, hours, description, jira).await
        }
        WorkAction::Delete { id, force } => {
            delete_work_item(ctx, id, force).await
        }
        WorkAction::Show { id } => {
            show_work_item(ctx, id).await
        }
    }
}

async fn list_work_items(
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

async fn add_work_item(
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

async fn update_work_item(
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

async fn delete_work_item(ctx: &Context, id: String, force: bool) -> Result<()> {
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

async fn show_work_item(ctx: &Context, id: String) -> Result<()> {
    let full_id = resolve_work_item_id(&ctx.db, &id).await?;

    let item: recap_core::WorkItem = sqlx::query_as("SELECT * FROM work_items WHERE id = ?")
        .bind(&full_id)
        .fetch_one(&ctx.db.pool)
        .await?;

    print_single(&WorkItemRow::from(item), ctx.format)?;

    Ok(())
}

/// Resolve a short ID to full ID
async fn resolve_work_item_id(db: &recap_core::Database, id: &str) -> Result<String> {
    let pattern = format!("{}%", id);
    let item: Option<(String,)> = sqlx::query_as("SELECT id FROM work_items WHERE id LIKE ? LIMIT 1")
        .bind(&pattern)
        .fetch_optional(&db.pool)
        .await?;

    match item {
        Some((full_id,)) => Ok(full_id),
        None => Err(anyhow::anyhow!("Work item not found: {}", id)),
    }
}

/// Get or create a default user for CLI usage
async fn get_or_create_default_user(db: &recap_core::Database) -> Result<String> {
    // Try to find existing user
    let user: Option<(String,)> = sqlx::query_as("SELECT id FROM users LIMIT 1")
        .fetch_optional(&db.pool)
        .await?;

    if let Some((id,)) = user {
        return Ok(id);
    }

    // Create default user for CLI
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();
    let password_hash = recap_core::auth::hash_password("cli_user")?;

    sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, name, username, created_at, updated_at)
        VALUES (?, 'cli@localhost', ?, 'CLI User', 'cli', ?, ?)
        "#
    )
    .bind(&id)
    .bind(&password_hash)
    .bind(now)
    .bind(now)
    .execute(&db.pool)
    .await?;

    Ok(id)
}

fn parse_date(s: &str) -> Result<NaiveDate> {
    // Support common formats
    if s == "today" {
        return Ok(chrono::Local::now().date_naive());
    }
    if s == "yesterday" {
        return Ok(chrono::Local::now().date_naive() - chrono::Duration::days(1));
    }

    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| anyhow::anyhow!("Invalid date format: {}. Use YYYY-MM-DD", s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_date_valid_format() {
        let date = parse_date("2025-01-15").unwrap();
        assert_eq!(date.year(), 2025);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 15);
    }

    #[test]
    fn test_parse_date_today() {
        let today = chrono::Local::now().date_naive();
        let parsed = parse_date("today").unwrap();
        assert_eq!(parsed, today);
    }

    #[test]
    fn test_parse_date_yesterday() {
        let yesterday = chrono::Local::now().date_naive() - chrono::Duration::days(1);
        let parsed = parse_date("yesterday").unwrap();
        assert_eq!(parsed, yesterday);
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(parse_date("invalid").is_err());
        assert!(parse_date("2025/01/15").is_err()); // wrong separator
        assert!(parse_date("01-15-2025").is_err()); // wrong order
        assert!(parse_date("").is_err());
    }

    #[test]
    fn test_parse_date_error_message() {
        let err = parse_date("bad-date").unwrap_err();
        assert!(err.to_string().contains("bad-date"));
        assert!(err.to_string().contains("YYYY-MM-DD"));
    }

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("exact len!", 10), "exact len!");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("this is a long string", 10), "this is...");
        assert_eq!(truncate("abcdefghijklmnop", 10), "abcdefg...");
    }

    #[test]
    fn test_truncate_empty_string() {
        assert_eq!(truncate("", 10), "");
    }

    #[test]
    fn test_truncate_unicode() {
        // Unicode characters should be handled correctly
        assert_eq!(truncate("你好世界", 10), "你好世界");
        assert_eq!(truncate("你好世界這是一個很長的字串", 10), "你好世界這是一...");
    }

    #[test]
    fn test_truncate_exact_boundary() {
        assert_eq!(truncate("1234567890", 10), "1234567890");
        assert_eq!(truncate("12345678901", 10), "1234567...");
    }

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
