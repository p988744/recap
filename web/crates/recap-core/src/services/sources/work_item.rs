//! Unified Work Item Upsert
//!
//! This module provides a unified function for creating and updating work items
//! from any data source. It handles deduplication via content hash and preserves
//! user modifications.

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use super::types::WorkItemParams;

/// Result of an upsert operation
#[derive(Debug, Clone, PartialEq)]
pub enum UpsertResult {
    /// A new work item was created
    Created(String),
    /// An existing work item was updated
    Updated(String),
    /// The work item already exists and was skipped (no changes needed)
    Skipped(String),
}

impl UpsertResult {
    /// Get the work item ID
    pub fn id(&self) -> &str {
        match self {
            UpsertResult::Created(id) => id,
            UpsertResult::Updated(id) => id,
            UpsertResult::Skipped(id) => id,
        }
    }

    /// Check if a new item was created
    pub fn is_created(&self) -> bool {
        matches!(self, UpsertResult::Created(_))
    }

    /// Check if an item was updated
    pub fn is_updated(&self) -> bool {
        matches!(self, UpsertResult::Updated(_))
    }
}

/// Generate a content hash for session-based deduplication.
///
/// Uses user_id + session_id for uniqueness, as session_id is a UUID
/// and already globally unique.
fn generate_session_hash(user_id: &str, session_id: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    format!("session:{}:{}", user_id, session_id).hash(&mut hasher);
    format!("sess_{:x}", hasher.finish())
}

/// Find an existing work item by content hash or session_id fallback.
///
/// This handles the transition from old hashes to new hashes by also
/// checking session_id for backward compatibility.
async fn find_existing_work_item(
    pool: &SqlitePool,
    user_id: &str,
    content_hash: &str,
    session_id: Option<&str>,
    source: &str,
) -> Result<Option<(String, Option<String>, Option<String>)>, String> {
    // First try: exact content_hash match
    let existing: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT id, hours_source, content_hash FROM work_items WHERE content_hash = ? AND user_id = ?",
    )
    .bind(content_hash)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    if existing.is_some() {
        return Ok(existing);
    }

    // Second try: session_id fallback for old hash migration
    if let Some(sid) = session_id {
        let fallback: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT id, hours_source, content_hash FROM work_items WHERE session_id = ? AND source = ? AND user_id = ?",
        )
        .bind(sid)
        .bind(source)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

        return Ok(fallback);
    }

    Ok(None)
}

/// Unified work item creation/update for all sources.
///
/// This function handles:
/// - Generating a content hash for deduplication
/// - Checking for existing work items
/// - Preserving user-modified hours
/// - Creating new or updating existing work items
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `params` - Work item parameters
///
/// # Returns
///
/// * `Ok(UpsertResult::Created(id))` - A new work item was created
/// * `Ok(UpsertResult::Updated(id))` - An existing work item was updated
/// * `Ok(UpsertResult::Skipped(id))` - The work item already exists unchanged
/// * `Err(String)` - An error occurred
pub async fn upsert_work_item(
    pool: &SqlitePool,
    params: WorkItemParams,
) -> Result<UpsertResult, String> {
    // Generate content hash based on session_id if available, otherwise use source_id
    let hash_key = params.session_id.as_deref().unwrap_or(&params.source_id);
    let content_hash = generate_session_hash(&params.user_id, hash_key);

    let now = Utc::now();

    // Check if work item already exists
    let existing = find_existing_work_item(
        pool,
        &params.user_id,
        &content_hash,
        params.session_id.as_deref(),
        &params.source,
    )
    .await?;

    if let Some((existing_id, existing_hours_source, old_hash)) = existing {
        // Check if hash needs migration
        let needs_hash_migration = old_hash.as_deref() != Some(&content_hash);

        // Preserve user-modified hours
        let user_modified = existing_hours_source.as_deref() == Some("user_modified");

        if user_modified {
            // Update without changing hours
            sqlx::query(
                r#"UPDATE work_items SET
                   title = ?, description = ?, hours_estimated = ?,
                   start_time = ?, end_time = ?, project_path = ?,
                   session_id = ?, content_hash = ?, updated_at = ?
                   WHERE id = ?"#,
            )
            .bind(&params.title)
            .bind(&params.description)
            .bind(params.hours)
            .bind(&params.start_time)
            .bind(&params.end_time)
            .bind(&params.project_path)
            .bind(&params.session_id)
            .bind(&content_hash)
            .bind(now)
            .bind(&existing_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        } else {
            // Update including hours
            sqlx::query(
                r#"UPDATE work_items SET
                   title = ?, description = ?, hours = ?, hours_source = 'session',
                   hours_estimated = ?, start_time = ?, end_time = ?, project_path = ?,
                   session_id = ?, content_hash = ?, updated_at = ?
                   WHERE id = ?"#,
            )
            .bind(&params.title)
            .bind(&params.description)
            .bind(params.hours)
            .bind(params.hours)
            .bind(&params.start_time)
            .bind(&params.end_time)
            .bind(&params.project_path)
            .bind(&params.session_id)
            .bind(&content_hash)
            .bind(now)
            .bind(&existing_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }

        if needs_hash_migration {
            log::info!(
                "Migrated hash for session {:?} from {:?} to {}",
                params.session_id,
                old_hash,
                content_hash
            );
        }

        return Ok(UpsertResult::Updated(existing_id));
    }

    // Create new work item
    let id = Uuid::new_v4().to_string();

    sqlx::query(
        r#"INSERT INTO work_items
        (id, user_id, source, source_id, title, description, hours, date,
         content_hash, hours_source, hours_estimated, session_id,
         start_time, end_time, project_path, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'session', ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(&params.user_id)
    .bind(&params.source)
    .bind(&params.source_id)
    .bind(&params.title)
    .bind(&params.description)
    .bind(params.hours)
    .bind(&params.date)
    .bind(&content_hash)
    .bind(params.hours)
    .bind(&params.session_id)
    .bind(&params.start_time)
    .bind(&params.end_time)
    .bind(&params.project_path)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(UpsertResult::Created(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_session_hash_consistent() {
        let hash1 = generate_session_hash("user1", "session-abc");
        let hash2 = generate_session_hash("user1", "session-abc");
        assert_eq!(hash1, hash2, "Same inputs should produce same hash");
    }

    #[test]
    fn test_generate_session_hash_different_sessions() {
        let hash1 = generate_session_hash("user1", "session-abc");
        let hash2 = generate_session_hash("user1", "session-def");
        assert_ne!(hash1, hash2, "Different sessions should produce different hashes");
    }

    #[test]
    fn test_generate_session_hash_different_users() {
        let hash1 = generate_session_hash("user1", "session-abc");
        let hash2 = generate_session_hash("user2", "session-abc");
        assert_ne!(hash1, hash2, "Different users should produce different hashes");
    }

    #[test]
    fn test_generate_session_hash_prefix() {
        let hash = generate_session_hash("user1", "session-abc");
        assert!(hash.starts_with("sess_"), "Hash should start with sess_ prefix");
    }

    #[test]
    fn test_upsert_result_id() {
        let created = UpsertResult::Created("id1".to_string());
        let updated = UpsertResult::Updated("id2".to_string());
        let skipped = UpsertResult::Skipped("id3".to_string());

        assert_eq!(created.id(), "id1");
        assert_eq!(updated.id(), "id2");
        assert_eq!(skipped.id(), "id3");
    }

    #[test]
    fn test_upsert_result_checks() {
        let created = UpsertResult::Created("id".to_string());
        let updated = UpsertResult::Updated("id".to_string());
        let skipped = UpsertResult::Skipped("id".to_string());

        assert!(created.is_created());
        assert!(!created.is_updated());

        assert!(!updated.is_created());
        assert!(updated.is_updated());

        assert!(!skipped.is_created());
        assert!(!skipped.is_updated());
    }
}
