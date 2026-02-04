//! Integration test for compaction with latest_compacted_date field

use chrono::Utc;
use recap_core::db::Database;
use recap_core::services::compaction::run_compaction_cycle;
use sqlx::Row;
use tempfile::TempDir;

/// Helper to create a test database
async fn create_test_db() -> (Database, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");
    let db = Database::open(db_path).await.expect("Failed to create test database");
    (db, temp_dir)
}

/// Insert test snapshot data
async fn insert_test_snapshot(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    project_path: &str,
    hour_bucket: &str,
) {
    let id = uuid::Uuid::new_v4().to_string();
    let session_id = format!("sess-{}", uuid::Uuid::new_v4());

    sqlx::query(
        r#"
        INSERT INTO snapshot_raw_data (
            id, user_id, session_id, project_path, hour_bucket,
            user_messages, assistant_messages, tool_calls, files_modified, git_commits,
            message_count, raw_size_bytes, created_at
        ) VALUES (
            ?, ?, ?, ?, ?,
            '["test message"]', '["response"]', '[]', '["test.rs"]', '[]',
            1, 100, ?
        )
        "#,
    )
    .bind(&id)
    .bind(user_id)
    .bind(&session_id)
    .bind(project_path)
    .bind(hour_bucket)
    .bind(Utc::now())
    .execute(pool)
    .await
    .expect("Failed to insert test snapshot");
}

#[tokio::test]
async fn test_compaction_returns_latest_date() {
    let (db, _temp_dir) = create_test_db().await;
    let pool = &db.pool;
    let user_id = "test-user-1";
    let project_path = "/test/project";

    // Insert snapshots for multiple past dates
    // Use dates that are definitely in the past
    insert_test_snapshot(pool, user_id, project_path, "2024-01-10T10:00:00").await;
    insert_test_snapshot(pool, user_id, project_path, "2024-01-10T11:00:00").await;
    insert_test_snapshot(pool, user_id, project_path, "2024-01-15T09:00:00").await;
    insert_test_snapshot(pool, user_id, project_path, "2024-01-15T14:00:00").await;
    insert_test_snapshot(pool, user_id, project_path, "2024-01-20T08:00:00").await;

    // Run compaction cycle (without LLM - will use rule-based)
    let result = run_compaction_cycle(pool, None, user_id)
        .await
        .expect("Compaction should succeed");

    // Should have compacted some hourly summaries
    assert!(result.hourly_compacted > 0, "Should have compacted hourly summaries");

    // Should have latest_compacted_date set
    assert!(result.latest_compacted_date.is_some(), "Should have latest_compacted_date");

    // The latest date should be 2024-01-20 (the most recent date we inserted)
    let latest_date = result.latest_compacted_date.as_ref().unwrap();
    assert_eq!(latest_date, "2024-01-20", "Latest compacted date should be 2024-01-20");

    // Verify work_summaries table has the data
    let hourly_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM work_summaries WHERE scale = 'hourly'")
        .fetch_one(pool)
        .await
        .expect("Query should succeed")
        .get("count");

    assert!(hourly_count > 0, "Should have hourly summaries in database");

    // Verify daily summaries were created
    let daily_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM work_summaries WHERE scale = 'daily'")
        .fetch_one(pool)
        .await
        .expect("Query should succeed")
        .get("count");

    assert!(daily_count > 0, "Should have daily summaries in database");

    println!("Compaction result:");
    println!("  Hourly compacted: {}", result.hourly_compacted);
    println!("  Daily compacted: {}", result.daily_compacted);
    println!("  Weekly compacted: {}", result.weekly_compacted);
    println!("  Monthly compacted: {}", result.monthly_compacted);
    println!("  Latest compacted date: {:?}", result.latest_compacted_date);
    println!("  Errors: {:?}", result.errors);
}

#[tokio::test]
async fn test_compaction_no_data_returns_none() {
    let (db, _temp_dir) = create_test_db().await;
    let pool = &db.pool;
    let user_id = "test-user-empty";

    // Run compaction with no data
    let result = run_compaction_cycle(pool, None, user_id)
        .await
        .expect("Compaction should succeed even with no data");

    // Should have no compacted items
    assert_eq!(result.hourly_compacted, 0);
    assert_eq!(result.daily_compacted, 0);

    // latest_compacted_date should be None when nothing was compacted
    assert!(result.latest_compacted_date.is_none(), "Should have no latest_compacted_date when nothing compacted");
}
