//! Quota Tracking Tauri Commands
//!
//! Provides commands for fetching and managing API quota usage data
//! for Claude Code and other AI coding assistants.

use recap_core::auth::verify_token;
use recap_core::services::quota::{ClaudeQuotaProvider, QuotaProvider, QuotaStore};
use serde::{Deserialize, Serialize};
use tauri::State;

use super::AppState;

// ============================================================================
// DTOs (Data Transfer Objects)
// ============================================================================

/// Response containing current quota information
#[derive(Debug, Serialize)]
pub struct CurrentQuotaResponse {
    /// List of quota snapshots for all window types
    pub snapshots: Vec<QuotaSnapshotDto>,
    /// Whether the quota provider is available (authenticated)
    pub provider_available: bool,
}

/// DTO for quota snapshot data
#[derive(Debug, Serialize, Deserialize)]
pub struct QuotaSnapshotDto {
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
    /// When this snapshot was taken (ISO 8601 format)
    pub fetched_at: String,
}

// ============================================================================
// Commands
// ============================================================================

/// Get current quota from the provider and save to database.
///
/// This fetches fresh quota data from the provider's API, saves it to the
/// database, and returns the snapshots.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_current_quota(
    state: State<'_, AppState>,
    token: String,
) -> Result<CurrentQuotaResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    log::info!("[quota:cmd] Fetching current quota for user {}", claims.sub);

    // Get manual token from database (if configured)
    let db = state.db.lock().await;
    let manual_token: Option<String> = sqlx::query_scalar(
        "SELECT claude_oauth_token FROM users WHERE id = ?"
    )
    .bind(&claims.sub)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?
    .flatten();
    drop(db);

    // Create provider with manual token if available
    let provider = ClaudeQuotaProvider::new()
        .with_manual_token(manual_token)
        .with_user_id(&claims.sub);

    // Check availability
    let provider_available = provider.is_available().await;
    if !provider_available {
        log::warn!("[quota:cmd] Claude quota provider not available");
        return Ok(CurrentQuotaResponse {
            snapshots: vec![],
            provider_available: false,
        });
    }

    // Fetch quota from provider
    let quota_snapshots = provider.fetch_quota().await.map_err(|e| {
        log::error!("[quota:cmd] Failed to fetch quota: {}", e);
        e.to_string()
    })?;

    log::debug!(
        "[quota:cmd] Fetched {} snapshots from provider",
        quota_snapshots.len()
    );

    // Save to database
    let db = state.db.lock().await;
    let store = QuotaStore::new(db.pool.clone());
    store
        .save_snapshots(&claims.sub, &quota_snapshots, None)
        .await
        .map_err(|e| {
            log::error!("[quota:cmd] Failed to save snapshots: {}", e);
            e.to_string()
        })?;

    // Convert to DTOs
    let snapshots: Vec<QuotaSnapshotDto> = quota_snapshots
        .into_iter()
        .map(|s| QuotaSnapshotDto {
            provider: s.provider.to_string(),
            model: s.model,
            window_type: s.window_type.to_string(),
            used_percent: s.used_percent,
            resets_at: s.resets_at.map(|dt| dt.to_rfc3339()),
            extra_credits_used: s.extra_credits.as_ref().map(|ec| ec.used),
            extra_credits_limit: s.extra_credits.as_ref().map(|ec| ec.limit),
            fetched_at: s.created_at.to_rfc3339(),
        })
        .collect();

    log::info!(
        "[quota:cmd] Successfully fetched and saved {} quota snapshots",
        snapshots.len()
    );

    Ok(CurrentQuotaResponse {
        snapshots,
        provider_available: true,
    })
}

/// Get cached/stored quota snapshots from database.
///
/// Returns the most recent snapshots without fetching from the provider.
/// Useful for quick UI updates without making API calls.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_stored_quota(
    state: State<'_, AppState>,
    token: String,
    provider: Option<String>,
) -> Result<Vec<QuotaSnapshotDto>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    log::debug!(
        "[quota:cmd] Getting stored quota for user {}, provider: {:?}",
        claims.sub,
        provider
    );

    let db = state.db.lock().await;
    let store = QuotaStore::new(db.pool.clone());

    let snapshots = store
        .get_latest(&claims.sub, provider.as_deref())
        .await
        .map_err(|e| {
            log::error!("[quota:cmd] Failed to get stored quota: {}", e);
            e.to_string()
        })?;

    let dtos: Vec<QuotaSnapshotDto> = snapshots
        .into_iter()
        .map(|s| QuotaSnapshotDto {
            provider: s.provider.to_string(),
            model: s.model,
            window_type: s.window_type.to_string(),
            used_percent: s.used_percent,
            resets_at: s.resets_at.map(|dt| dt.to_rfc3339()),
            extra_credits_used: s.extra_credits.as_ref().map(|ec| ec.used),
            extra_credits_limit: s.extra_credits.as_ref().map(|ec| ec.limit),
            fetched_at: s.created_at.to_rfc3339(),
        })
        .collect();

    log::debug!("[quota:cmd] Found {} stored quota snapshots", dtos.len());

    Ok(dtos)
}

/// Get quota history for trend analysis and charts.
///
/// Returns snapshots over a time period for a specific provider and window type.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_quota_history(
    state: State<'_, AppState>,
    token: String,
    provider: String,
    window_type: String,
    days: Option<i32>,
) -> Result<Vec<QuotaSnapshotDto>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let days = days.unwrap_or(7);

    log::debug!(
        "[quota:cmd] Getting {} days of quota history for user {}, provider: {}, window: {}",
        days,
        claims.sub,
        provider,
        window_type
    );

    let db = state.db.lock().await;
    let store = QuotaStore::new(db.pool.clone());

    let snapshots = store
        .get_history(&claims.sub, &provider, &window_type, days)
        .await
        .map_err(|e| {
            log::error!("[quota:cmd] Failed to get quota history: {}", e);
            e.to_string()
        })?;

    let dtos: Vec<QuotaSnapshotDto> = snapshots
        .into_iter()
        .map(|s| QuotaSnapshotDto {
            provider: s.provider.to_string(),
            model: s.model,
            window_type: s.window_type.to_string(),
            used_percent: s.used_percent,
            resets_at: s.resets_at.map(|dt| dt.to_rfc3339()),
            extra_credits_used: s.extra_credits.as_ref().map(|ec| ec.used),
            extra_credits_limit: s.extra_credits.as_ref().map(|ec| ec.limit),
            fetched_at: s.created_at.to_rfc3339(),
        })
        .collect();

    log::debug!("[quota:cmd] Found {} history records", dtos.len());

    Ok(dtos)
}

/// Check if a quota provider is available/configured.
///
/// This is a quick check that doesn't make network requests.
#[tauri::command(rename_all = "snake_case")]
pub async fn check_quota_provider_available(
    token: String,
    provider: String,
) -> Result<bool, String> {
    let _claims = verify_token(&token).map_err(|e| e.to_string())?;
    log::debug!("[quota:cmd] Checking if provider {} is available", provider);

    let is_available = match provider.to_lowercase().as_str() {
        "claude" | "claude_code" => {
            let provider = ClaudeQuotaProvider::new();
            provider.is_available().await
        }
        "antigravity" | "gemini" => {
            // TODO: Implement Antigravity provider
            log::debug!("[quota:cmd] Antigravity provider not yet implemented");
            false
        }
        _ => {
            log::warn!("[quota:cmd] Unknown provider: {}", provider);
            false
        }
    };

    log::debug!("[quota:cmd] Provider {} available: {}", provider, is_available);

    Ok(is_available)
}

// ============================================================================
// Claude OAuth Token Management (Fallback)
// ============================================================================

/// Get the manually configured Claude OAuth token.
///
/// Returns the token if set, or None if not configured.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_claude_oauth_token(
    state: State<'_, AppState>,
    token: String,
) -> Result<Option<String>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    log::debug!("[quota:cmd] Getting Claude OAuth token for user {}", claims.sub);

    let db = state.db.lock().await;

    let result: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT claude_oauth_token FROM users WHERE id = ?"
    )
    .bind(&claims.sub)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(result.and_then(|r| r.0))
}

/// Set the Claude OAuth token manually.
///
/// This is used as a fallback when automatic credential discovery fails.
/// Pass None or empty string to clear the token.
#[tauri::command(rename_all = "snake_case")]
pub async fn set_claude_oauth_token(
    state: State<'_, AppState>,
    token: String,
    oauth_token: Option<String>,
) -> Result<(), String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    log::info!("[quota:cmd] Setting Claude OAuth token for user {}", claims.sub);

    let db = state.db.lock().await;

    // Clean the token (None or empty string means clear)
    let clean_token = oauth_token.filter(|t| !t.trim().is_empty());

    sqlx::query("UPDATE users SET claude_oauth_token = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&clean_token)
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    log::info!(
        "[quota:cmd] Claude OAuth token {} for user {}",
        if clean_token.is_some() { "set" } else { "cleared" },
        claims.sub
    );

    Ok(())
}

/// Check if Claude OAuth is available (either automatic or manual).
#[tauri::command(rename_all = "snake_case")]
pub async fn check_claude_auth_status(
    state: State<'_, AppState>,
    token: String,
) -> Result<ClaudeAuthStatus, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    log::debug!("[quota:cmd] Checking Claude auth status for user {}", claims.sub);

    // Check if manual token is set
    let db = state.db.lock().await;
    let result: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT claude_oauth_token FROM users WHERE id = ?"
    )
    .bind(&claims.sub)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let manual_token = result.and_then(|r| r.0).filter(|t| !t.is_empty());
    drop(db);

    // Check automatic credential availability
    let provider = ClaudeQuotaProvider::new();
    let auto_available = provider.is_available().await;

    // If manual token is set, check if it works
    let manual_valid = if manual_token.is_some() {
        let provider_with_token = ClaudeQuotaProvider::new()
            .with_manual_token(manual_token.clone())
            .with_user_id(&claims.sub);
        provider_with_token.is_available().await
    } else {
        false
    };

    Ok(ClaudeAuthStatus {
        auto_available,
        manual_configured: manual_token.is_some(),
        manual_valid,
        active_source: if manual_token.is_some() {
            "manual".to_string()
        } else if auto_available {
            "auto".to_string()
        } else {
            "none".to_string()
        },
    })
}

/// Response for Claude auth status check
#[derive(Debug, Serialize)]
pub struct ClaudeAuthStatus {
    /// Whether automatic credential discovery works (Keychain/file)
    pub auto_available: bool,
    /// Whether a manual token is configured
    pub manual_configured: bool,
    /// Whether the manual token is valid (if configured)
    pub manual_valid: bool,
    /// Which auth source is active: "auto", "manual", or "none"
    pub active_source: String,
}
