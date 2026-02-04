//! Quota storage layer
//!
//! Handles persistence of quota snapshots to SQLite.

#[cfg(test)]
use chrono::{Datelike, Timelike};
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::{FromRow, SqlitePool};

use super::provider::QuotaError;
use super::types::{ExtraCredits, QuotaProviderType, QuotaSnapshot, QuotaWindowType};

// ============================================================================
// Database Row Types
// ============================================================================

/// Database row representation of a quota snapshot
///
/// This struct maps directly to the `quota_snapshots` table schema.
#[derive(Debug, Clone, FromRow)]
pub struct StoredQuotaSnapshot {
    /// Unique identifier (UUID)
    pub id: String,
    /// User who owns this quota
    pub user_id: String,
    /// Provider name (e.g., "claude", "antigravity")
    pub provider: String,
    /// Model this quota applies to (if model-specific)
    pub model: Option<String>,
    /// Type of quota window (e.g., "5_hour", "7_day")
    pub window_type: String,
    /// Percentage of quota used (0.0 - 100.0)
    pub used_percent: f64,
    /// When the quota resets (ISO 8601 format)
    pub resets_at: Option<String>,
    /// Extra credits used (if applicable)
    pub extra_credits_used: Option<f64>,
    /// Extra credits limit (if applicable)
    pub extra_credits_limit: Option<f64>,
    /// Raw API response for debugging
    pub raw_response: Option<String>,
    /// When this snapshot was taken (ISO 8601 format)
    pub created_at: String,
}

impl StoredQuotaSnapshot {
    /// Convert database row to QuotaSnapshot
    ///
    /// Returns `None` if parsing fails for required fields.
    pub fn to_quota_snapshot(&self) -> Option<QuotaSnapshot> {
        // Parse provider type
        let provider = self.provider.parse::<QuotaProviderType>().ok()?;

        // Parse window type
        let window_type = self.window_type.parse::<QuotaWindowType>().ok()?;

        // Parse resets_at datetime
        let resets_at = self.resets_at.as_ref().and_then(|s| parse_datetime(s));

        // Parse created_at datetime
        let created_at = parse_datetime(&self.created_at)?;

        // Build extra credits if both values are present
        let extra_credits = match (self.extra_credits_used, self.extra_credits_limit) {
            (Some(used), Some(limit)) => Some(ExtraCredits { used, limit }),
            _ => None,
        };

        Some(QuotaSnapshot {
            id: self.id.clone(),
            user_id: self.user_id.clone(),
            provider,
            model: self.model.clone(),
            window_type,
            used_percent: self.used_percent,
            resets_at,
            extra_credits,
            raw_response: self.raw_response.clone(),
            created_at,
        })
    }
}

/// Parse datetime string (supports both RFC3339 and NaiveDateTime formats)
fn parse_datetime(s: &str) -> Option<DateTime<Utc>> {
    // Try RFC3339 first (e.g., "2026-02-04T10:30:00Z")
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }

    // Try NaiveDateTime (e.g., "2026-02-04 10:30:00")
    if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(naive.and_utc());
    }

    // Try alternative formats
    if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(naive.and_utc());
    }

    log::warn!("[quota:store] Failed to parse datetime: {}", s);
    None
}

// ============================================================================
// QuotaStore
// ============================================================================

/// Storage layer for quota snapshots
///
/// Provides CRUD operations for quota data in SQLite.
pub struct QuotaStore {
    pool: SqlitePool,
}

impl QuotaStore {
    /// Create a new QuotaStore with the given database pool
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Save multiple quota snapshots to the database
    ///
    /// # Arguments
    /// * `user_id` - The user who owns these quotas
    /// * `snapshots` - List of quota snapshots to save
    /// * `raw_response` - Optional raw API response (applied to all snapshots)
    ///
    /// # Errors
    /// Returns `QuotaError` if the database operation fails.
    pub async fn save_snapshots(
        &self,
        user_id: &str,
        snapshots: &[QuotaSnapshot],
        raw_response: Option<&str>,
    ) -> Result<(), QuotaError> {
        if snapshots.is_empty() {
            log::debug!("[quota:store] No snapshots to save");
            return Ok(());
        }

        log::debug!(
            "[quota:store] Saving {} snapshots for user {}",
            snapshots.len(),
            user_id
        );

        for snapshot in snapshots {
            let id = uuid::Uuid::new_v4().to_string();
            let provider = snapshot.provider.to_string();
            let window_type = snapshot.window_type.to_string();
            let resets_at = snapshot.resets_at.map(|dt| dt.to_rfc3339());
            let (extra_used, extra_limit) = snapshot
                .extra_credits
                .as_ref()
                .map(|ec| (Some(ec.used), Some(ec.limit)))
                .unwrap_or((None, None));

            // Use raw_response from parameter if provided, otherwise from snapshot
            let raw = raw_response.or(snapshot.raw_response.as_deref());

            sqlx::query(
                r#"
                INSERT INTO quota_snapshots
                (id, user_id, provider, model, window_type, used_percent, resets_at,
                 extra_credits_used, extra_credits_limit, raw_response, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
                "#,
            )
            .bind(&id)
            .bind(user_id)
            .bind(&provider)
            .bind(&snapshot.model)
            .bind(&window_type)
            .bind(snapshot.used_percent)
            .bind(&resets_at)
            .bind(extra_used)
            .bind(extra_limit)
            .bind(raw)
            .execute(&self.pool)
            .await
            .map_err(|e| QuotaError::Other(format!("Failed to save quota snapshot: {}", e)))?;
        }

        log::info!(
            "[quota:store] Saved {} snapshots for user {}",
            snapshots.len(),
            user_id
        );

        Ok(())
    }

    /// Get the latest quota snapshots for a user
    ///
    /// # Arguments
    /// * `user_id` - The user to get quotas for
    /// * `provider` - Optional provider filter (e.g., "claude")
    ///
    /// # Returns
    /// List of the most recent snapshots, ordered by created_at DESC.
    /// If provider is specified, returns up to 10 snapshots for that provider.
    /// Otherwise, returns up to 20 snapshots across all providers.
    pub async fn get_latest(
        &self,
        user_id: &str,
        provider: Option<&str>,
    ) -> Result<Vec<QuotaSnapshot>, QuotaError> {
        let rows = if let Some(prov) = provider {
            log::debug!(
                "[quota:store] Getting latest snapshots for user {} provider {}",
                user_id,
                prov
            );

            sqlx::query_as::<_, StoredQuotaSnapshot>(
                r#"
                SELECT id, user_id, provider, model, window_type, used_percent,
                       resets_at, extra_credits_used, extra_credits_limit,
                       raw_response, created_at
                FROM quota_snapshots
                WHERE user_id = ? AND provider = ?
                ORDER BY created_at DESC
                LIMIT 10
                "#,
            )
            .bind(user_id)
            .bind(prov)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| QuotaError::Other(format!("Failed to fetch quota snapshots: {}", e)))?
        } else {
            log::debug!(
                "[quota:store] Getting latest snapshots for user {} (all providers)",
                user_id
            );

            sqlx::query_as::<_, StoredQuotaSnapshot>(
                r#"
                SELECT id, user_id, provider, model, window_type, used_percent,
                       resets_at, extra_credits_used, extra_credits_limit,
                       raw_response, created_at
                FROM quota_snapshots
                WHERE user_id = ?
                ORDER BY created_at DESC
                LIMIT 20
                "#,
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| QuotaError::Other(format!("Failed to fetch quota snapshots: {}", e)))?
        };

        let snapshots: Vec<QuotaSnapshot> = rows
            .iter()
            .filter_map(|row| row.to_quota_snapshot())
            .collect();

        log::debug!(
            "[quota:store] Found {} snapshots for user {}",
            snapshots.len(),
            user_id
        );

        Ok(snapshots)
    }

    /// Get historical quota snapshots for trend analysis
    ///
    /// # Arguments
    /// * `user_id` - The user to get history for
    /// * `provider` - Provider to filter by (e.g., "claude")
    /// * `window_type` - Window type to filter by (e.g., "5_hour")
    /// * `days` - Number of days of history to retrieve
    ///
    /// # Returns
    /// List of snapshots within the time range, ordered by created_at ASC.
    pub async fn get_history(
        &self,
        user_id: &str,
        provider: &str,
        window_type: &str,
        days: i32,
    ) -> Result<Vec<QuotaSnapshot>, QuotaError> {
        log::debug!(
            "[quota:store] Getting {} days of history for user {} provider {} window_type {}",
            days,
            user_id,
            provider,
            window_type
        );

        let rows = sqlx::query_as::<_, StoredQuotaSnapshot>(
            r#"
            SELECT id, user_id, provider, model, window_type, used_percent,
                   resets_at, extra_credits_used, extra_credits_limit,
                   raw_response, created_at
            FROM quota_snapshots
            WHERE user_id = ?
              AND provider = ?
              AND window_type = ?
              AND created_at >= datetime('now', '-' || ? || ' days')
            ORDER BY created_at ASC
            "#,
        )
        .bind(user_id)
        .bind(provider)
        .bind(window_type)
        .bind(days)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| QuotaError::Other(format!("Failed to fetch quota history: {}", e)))?;

        let snapshots: Vec<QuotaSnapshot> = rows
            .iter()
            .filter_map(|row| row.to_quota_snapshot())
            .collect();

        log::debug!(
            "[quota:store] Found {} history records for user {} provider {} window_type {}",
            snapshots.len(),
            user_id,
            provider,
            window_type
        );

        Ok(snapshots)
    }

    /// Delete old quota snapshots
    ///
    /// # Arguments
    /// * `days` - Delete snapshots older than this many days
    ///
    /// # Returns
    /// Number of rows deleted.
    pub async fn cleanup(&self, days: i32) -> Result<u64, QuotaError> {
        log::info!("[quota:store] Cleaning up snapshots older than {} days", days);

        let result = sqlx::query(
            r#"
            DELETE FROM quota_snapshots
            WHERE created_at < datetime('now', '-' || ? || ' days')
            "#,
        )
        .bind(days)
        .execute(&self.pool)
        .await
        .map_err(|e| QuotaError::Other(format!("Failed to cleanup quota snapshots: {}", e)))?;

        let deleted = result.rows_affected();
        log::info!("[quota:store] Deleted {} old quota snapshots", deleted);

        Ok(deleted)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_datetime_rfc3339() {
        let dt = parse_datetime("2026-02-04T10:30:00Z").unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 2);
        assert_eq!(dt.day(), 4);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
    }

    #[test]
    fn test_parse_datetime_naive() {
        let dt = parse_datetime("2026-02-04 10:30:00").unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 2);
        assert_eq!(dt.day(), 4);
    }

    #[test]
    fn test_parse_datetime_invalid() {
        assert!(parse_datetime("invalid").is_none());
        assert!(parse_datetime("").is_none());
    }

    #[test]
    fn test_stored_snapshot_to_quota_snapshot() {
        let stored = StoredQuotaSnapshot {
            id: "test-id".to_string(),
            user_id: "user-1".to_string(),
            provider: "claude".to_string(),
            model: Some("claude-sonnet-4".to_string()),
            window_type: "5_hour".to_string(),
            used_percent: 75.5,
            resets_at: Some("2026-02-04T15:30:00Z".to_string()),
            extra_credits_used: Some(10.0),
            extra_credits_limit: Some(100.0),
            raw_response: Some("{}".to_string()),
            created_at: "2026-02-04T10:30:00Z".to_string(),
        };

        let snapshot = stored.to_quota_snapshot().unwrap();
        assert_eq!(snapshot.id, "test-id");
        assert_eq!(snapshot.user_id, "user-1");
        assert_eq!(snapshot.provider, QuotaProviderType::Claude);
        assert_eq!(snapshot.model, Some("claude-sonnet-4".to_string()));
        assert_eq!(snapshot.window_type, QuotaWindowType::FiveHour);
        assert_eq!(snapshot.used_percent, 75.5);
        assert!(snapshot.resets_at.is_some());
        assert!(snapshot.extra_credits.is_some());
        let extra = snapshot.extra_credits.unwrap();
        assert_eq!(extra.used, 10.0);
        assert_eq!(extra.limit, 100.0);
    }

    #[test]
    fn test_stored_snapshot_invalid_provider() {
        let stored = StoredQuotaSnapshot {
            id: "test-id".to_string(),
            user_id: "user-1".to_string(),
            provider: "unknown_provider".to_string(),
            model: None,
            window_type: "5_hour".to_string(),
            used_percent: 50.0,
            resets_at: None,
            extra_credits_used: None,
            extra_credits_limit: None,
            raw_response: None,
            created_at: "2026-02-04T10:30:00Z".to_string(),
        };

        // Should return None because provider parsing fails
        assert!(stored.to_quota_snapshot().is_none());
    }

    #[test]
    fn test_stored_snapshot_invalid_window_type() {
        let stored = StoredQuotaSnapshot {
            id: "test-id".to_string(),
            user_id: "user-1".to_string(),
            provider: "claude".to_string(),
            model: None,
            window_type: "invalid_window".to_string(),
            used_percent: 50.0,
            resets_at: None,
            extra_credits_used: None,
            extra_credits_limit: None,
            raw_response: None,
            created_at: "2026-02-04T10:30:00Z".to_string(),
        };

        // Should return None because window_type parsing fails
        assert!(stored.to_quota_snapshot().is_none());
    }

    #[test]
    fn test_stored_snapshot_partial_extra_credits() {
        // Only extra_credits_used is set (not limit)
        let stored = StoredQuotaSnapshot {
            id: "test-id".to_string(),
            user_id: "user-1".to_string(),
            provider: "claude".to_string(),
            model: None,
            window_type: "7_day".to_string(),
            used_percent: 30.0,
            resets_at: None,
            extra_credits_used: Some(5.0),
            extra_credits_limit: None,
            raw_response: None,
            created_at: "2026-02-04T10:30:00Z".to_string(),
        };

        let snapshot = stored.to_quota_snapshot().unwrap();
        // extra_credits should be None because both fields are required
        assert!(snapshot.extra_credits.is_none());
    }
}
