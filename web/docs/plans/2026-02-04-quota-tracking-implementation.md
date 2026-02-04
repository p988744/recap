# Quota Tracking Implementation Plan - Phase 1: Claude

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement quota tracking for Claude Code with background polling, database storage, tray display, and UI pages.

**Architecture:** QuotaProvider trait abstracts provider-specific API calls. ClaudeQuotaProvider reads OAuth token from `~/.claude/credentials.json` and calls Anthropic's usage API. QuotaTimer polls at configurable intervals, stores snapshots, and triggers alerts.

**Tech Stack:** Rust (recap-core, Tauri), TypeScript/React, SQLite, Recharts for graphs

---

## Task 1: Database Schema

**Files:**
- Modify: `web/crates/recap-core/src/db/mod.rs`

**Step 1: Add quota_snapshots table migration**

Add this after the `llm_batch_requests` table creation (around line 680):

```rust
        // Create quota_snapshots table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS quota_snapshots (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                model TEXT,
                window_type TEXT NOT NULL,
                used_percent REAL NOT NULL,
                resets_at TEXT,
                extra_credits_used REAL,
                extra_credits_limit REAL,
                raw_response TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create index for quota queries
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_quota_provider_time
            ON quota_snapshots(user_id, provider, created_at)
            "#,
        )
        .execute(&self.pool)
        .await?;

        log::info!("[quota:db] quota_snapshots table created");
```

**Step 2: Run to verify migration works**

```bash
cd /Users/weifanliao/PycharmProjects/recap-worktrees/v2.2.0-dev/web && cargo build --package recap-core
```

Expected: Build succeeds

**Step 3: Commit**

```bash
git add web/crates/recap-core/src/db/mod.rs
git commit -m "feat(quota): add quota_snapshots table schema"
```

---

## Task 2: Core Types and Trait Definition

**Files:**
- Create: `web/crates/recap-core/src/services/quota/mod.rs`
- Create: `web/crates/recap-core/src/services/quota/types.rs`
- Create: `web/crates/recap-core/src/services/quota/provider.rs`
- Modify: `web/crates/recap-core/src/services/mod.rs`

**Step 1: Create quota module directory**

```bash
mkdir -p /Users/weifanliao/PycharmProjects/recap-worktrees/v2.2.0-dev/web/crates/recap-core/src/services/quota
```

**Step 2: Create types.rs**

```rust
//! Quota tracking types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Supported quota providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaProviderType {
    Claude,
    Antigravity,
}

impl std::fmt::Display for QuotaProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuotaProviderType::Claude => write!(f, "claude"),
            QuotaProviderType::Antigravity => write!(f, "antigravity"),
        }
    }
}

/// Quota window types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaWindowType {
    FiveHour,
    SevenDay,
    SevenDayOpus,
    SevenDaySonnet,
    Monthly,
}

impl std::fmt::Display for QuotaWindowType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuotaWindowType::FiveHour => write!(f, "five_hour"),
            QuotaWindowType::SevenDay => write!(f, "seven_day"),
            QuotaWindowType::SevenDayOpus => write!(f, "seven_day_opus"),
            QuotaWindowType::SevenDaySonnet => write!(f, "seven_day_sonnet"),
            QuotaWindowType::Monthly => write!(f, "monthly"),
        }
    }
}

impl QuotaWindowType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "five_hour" => Some(QuotaWindowType::FiveHour),
            "seven_day" => Some(QuotaWindowType::SevenDay),
            "seven_day_opus" => Some(QuotaWindowType::SevenDayOpus),
            "seven_day_sonnet" => Some(QuotaWindowType::SevenDaySonnet),
            "monthly" => Some(QuotaWindowType::Monthly),
            _ => None,
        }
    }
}

/// A single quota snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaSnapshot {
    pub provider: QuotaProviderType,
    pub model: Option<String>,
    pub window_type: QuotaWindowType,
    pub used_percent: f64,
    pub resets_at: Option<DateTime<Utc>>,
    pub extra_credits: Option<ExtraCredits>,
    pub fetched_at: DateTime<Utc>,
}

/// Extra credits usage (Claude specific)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraCredits {
    pub used: f64,
    pub limit: f64,
    pub currency: String,
}

/// Account info from provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub email: Option<String>,
    pub plan_name: Option<String>,
    pub organization: Option<String>,
}

/// Alert level for quota usage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertLevel {
    Normal,
    Warning,
    Critical,
}

/// Quota settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaSettings {
    pub interval_minutes: u32,
    pub warning_threshold: f64,
    pub critical_threshold: f64,
    pub notifications_enabled: bool,
}

impl Default for QuotaSettings {
    fn default() -> Self {
        Self {
            interval_minutes: 15,
            warning_threshold: 80.0,
            critical_threshold: 95.0,
            notifications_enabled: true,
        }
    }
}
```

**Step 3: Create provider.rs**

```rust
//! QuotaProvider trait definition

use async_trait::async_trait;
use thiserror::Error;

use super::types::{AccountInfo, QuotaSnapshot};

/// Errors that can occur when fetching quota
#[derive(Debug, Error)]
pub enum QuotaError {
    #[error("Provider not installed")]
    NotInstalled,

    #[error("Authentication required: {0}")]
    Unauthorized(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Token expired")]
    TokenExpired,

    #[error("IO error: {0}")]
    IoError(String),
}

impl From<std::io::Error> for QuotaError {
    fn from(e: std::io::Error) -> Self {
        QuotaError::IoError(e.to_string())
    }
}

impl From<reqwest::Error> for QuotaError {
    fn from(e: reqwest::Error) -> Self {
        QuotaError::NetworkError(e.to_string())
    }
}

/// Trait for quota providers
#[async_trait]
pub trait QuotaProvider: Send + Sync {
    /// Provider identifier
    fn provider_id(&self) -> &'static str;

    /// Fetch current quota usage
    async fn fetch_quota(&self) -> Result<Vec<QuotaSnapshot>, QuotaError>;

    /// Check if provider is available (installed and authenticated)
    async fn is_available(&self) -> bool;

    /// Get account information
    async fn get_account_info(&self) -> Result<Option<AccountInfo>, QuotaError>;
}
```

**Step 4: Create mod.rs**

```rust
//! Quota tracking module
//!
//! Tracks API quota usage for Claude Code and Antigravity.

pub mod types;
pub mod provider;
pub mod claude;
pub mod store;

pub use types::*;
pub use provider::{QuotaProvider, QuotaError};
```

**Step 5: Update services/mod.rs**

Add to the module declarations:

```rust
pub mod quota;
```

**Step 6: Build to verify**

```bash
cd /Users/weifanliao/PycharmProjects/recap-worktrees/v2.2.0-dev/web && cargo build --package recap-core
```

Expected: Build succeeds (will warn about unused modules until we implement claude.rs and store.rs)

**Step 7: Commit**

```bash
git add web/crates/recap-core/src/services/quota/ web/crates/recap-core/src/services/mod.rs
git commit -m "feat(quota): add QuotaProvider trait and core types"
```

---

## Task 3: Claude OAuth Provider Implementation

**Files:**
- Create: `web/crates/recap-core/src/services/quota/claude.rs`

**Step 1: Create claude.rs**

```rust
//! Claude Code quota provider
//!
//! Fetches quota usage from Anthropic's OAuth API.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::path::PathBuf;

use super::provider::{QuotaError, QuotaProvider};
use super::types::{
    AccountInfo, ExtraCredits, QuotaProviderType, QuotaSnapshot, QuotaWindowType,
};

const USAGE_API_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const OAUTH_BETA_HEADER: &str = "oauth-2025-04-20";

/// Claude OAuth credentials file structure
#[derive(Debug, Deserialize)]
struct ClaudeCredentials {
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
    #[serde(rename = "refreshToken")]
    refresh_token: Option<String>,
    #[serde(rename = "expiresAt")]
    expires_at: Option<String>,
}

/// OAuth usage API response
#[derive(Debug, Deserialize)]
struct OAuthUsageResponse {
    five_hour: Option<UsageWindow>,
    seven_day: Option<UsageWindow>,
    seven_day_opus: Option<UsageWindow>,
    seven_day_sonnet: Option<UsageWindow>,
    extra_usage: Option<ExtraUsage>,
}

#[derive(Debug, Deserialize)]
struct UsageWindow {
    utilization: Option<f64>,
    resets_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExtraUsage {
    is_enabled: Option<bool>,
    used_credits: Option<f64>,
    monthly_limit: Option<f64>,
    currency: Option<String>,
}

/// Claude quota provider
pub struct ClaudeQuotaProvider {
    credentials_path: PathBuf,
    client: Client,
}

impl ClaudeQuotaProvider {
    /// Create a new Claude quota provider
    pub fn new() -> Self {
        let credentials_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".claude")
            .join("credentials.json");

        log::info!("[quota:claude] Initialized with credentials path: {:?}", credentials_path);

        Self {
            credentials_path,
            client: Client::new(),
        }
    }

    /// Create with custom credentials path (for testing)
    pub fn with_credentials_path(path: PathBuf) -> Self {
        Self {
            credentials_path: path,
            client: Client::new(),
        }
    }

    /// Load OAuth token from credentials file
    fn load_oauth_token(&self) -> Result<String, QuotaError> {
        log::debug!("[quota:claude] Loading OAuth token from {:?}", self.credentials_path);

        if !self.credentials_path.exists() {
            log::warn!("[quota:claude] Credentials file not found");
            return Err(QuotaError::NotInstalled);
        }

        let content = std::fs::read_to_string(&self.credentials_path)?;
        let credentials: ClaudeCredentials = serde_json::from_str(&content)
            .map_err(|e| QuotaError::ParseError(format!("Failed to parse credentials: {}", e)))?;

        credentials.access_token.ok_or_else(|| {
            log::warn!("[quota:claude] No access token in credentials file");
            QuotaError::Unauthorized("No access token found".to_string())
        })
    }

    /// Call the OAuth usage API
    async fn call_usage_api(&self, token: &str) -> Result<OAuthUsageResponse, QuotaError> {
        log::debug!("[quota:claude] Calling usage API: {}", USAGE_API_URL);

        let response = self.client
            .get(USAGE_API_URL)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("anthropic-beta", OAUTH_BETA_HEADER)
            .header("User-Agent", "Recap")
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;

        let status = response.status();
        log::debug!("[quota:claude] API response status: {}", status);

        if status == reqwest::StatusCode::UNAUTHORIZED {
            log::warn!("[quota:claude] Unauthorized - token may be expired");
            return Err(QuotaError::TokenExpired);
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            log::error!("[quota:claude] API error {}: {}", status, body);
            return Err(QuotaError::ApiError(format!("HTTP {}: {}", status, body)));
        }

        let body = response.text().await?;
        log::debug!("[quota:claude] API response body: {}", body);

        serde_json::from_str(&body)
            .map_err(|e| QuotaError::ParseError(format!("Failed to parse response: {}", e)))
    }

    /// Convert API response to snapshots
    fn response_to_snapshots(&self, response: OAuthUsageResponse) -> Vec<QuotaSnapshot> {
        let now = Utc::now();
        let mut snapshots = Vec::new();

        // 5-hour window
        if let Some(window) = response.five_hour {
            if let Some(utilization) = window.utilization {
                snapshots.push(QuotaSnapshot {
                    provider: QuotaProviderType::Claude,
                    model: None,
                    window_type: QuotaWindowType::FiveHour,
                    used_percent: utilization * 100.0,
                    resets_at: window.resets_at.and_then(|s| parse_datetime(&s)),
                    extra_credits: None,
                    fetched_at: now,
                });
            }
        }

        // 7-day window
        if let Some(window) = response.seven_day {
            if let Some(utilization) = window.utilization {
                snapshots.push(QuotaSnapshot {
                    provider: QuotaProviderType::Claude,
                    model: None,
                    window_type: QuotaWindowType::SevenDay,
                    used_percent: utilization * 100.0,
                    resets_at: window.resets_at.and_then(|s| parse_datetime(&s)),
                    extra_credits: None,
                    fetched_at: now,
                });
            }
        }

        // 7-day Opus
        if let Some(window) = response.seven_day_opus {
            if let Some(utilization) = window.utilization {
                snapshots.push(QuotaSnapshot {
                    provider: QuotaProviderType::Claude,
                    model: Some("opus".to_string()),
                    window_type: QuotaWindowType::SevenDayOpus,
                    used_percent: utilization * 100.0,
                    resets_at: window.resets_at.and_then(|s| parse_datetime(&s)),
                    extra_credits: None,
                    fetched_at: now,
                });
            }
        }

        // 7-day Sonnet
        if let Some(window) = response.seven_day_sonnet {
            if let Some(utilization) = window.utilization {
                snapshots.push(QuotaSnapshot {
                    provider: QuotaProviderType::Claude,
                    model: Some("sonnet".to_string()),
                    window_type: QuotaWindowType::SevenDaySonnet,
                    used_percent: utilization * 100.0,
                    resets_at: window.resets_at.and_then(|s| parse_datetime(&s)),
                    extra_credits: None,
                    fetched_at: now,
                });
            }
        }

        // Extra usage (add to 5-hour snapshot if exists)
        if let Some(extra) = response.extra_usage {
            if extra.is_enabled.unwrap_or(false) {
                if let (Some(used), Some(limit)) = (extra.used_credits, extra.monthly_limit) {
                    let extra_credits = ExtraCredits {
                        used,
                        limit,
                        currency: extra.currency.unwrap_or_else(|| "USD".to_string()),
                    };

                    // Update first snapshot with extra credits info
                    if let Some(first) = snapshots.first_mut() {
                        first.extra_credits = Some(extra_credits);
                    }
                }
            }
        }

        log::info!(
            "[quota:claude] Parsed {} snapshots: {:?}",
            snapshots.len(),
            snapshots.iter().map(|s| format!("{}={:.1}%", s.window_type, s.used_percent)).collect::<Vec<_>>()
        );

        snapshots
    }
}

impl Default for ClaudeQuotaProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl QuotaProvider for ClaudeQuotaProvider {
    fn provider_id(&self) -> &'static str {
        "claude"
    }

    async fn fetch_quota(&self) -> Result<Vec<QuotaSnapshot>, QuotaError> {
        log::info!("[quota:claude] Starting quota fetch");

        let token = self.load_oauth_token()?;
        log::debug!("[quota:claude] OAuth token loaded successfully");

        let response = self.call_usage_api(&token).await?;
        let snapshots = self.response_to_snapshots(response);

        Ok(snapshots)
    }

    async fn is_available(&self) -> bool {
        self.credentials_path.exists() && self.load_oauth_token().is_ok()
    }

    async fn get_account_info(&self) -> Result<Option<AccountInfo>, QuotaError> {
        // OAuth API doesn't return account info directly
        // Would need to call a different endpoint
        Ok(None)
    }
}

/// Parse ISO8601 datetime string
fn parse_datetime(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_usage_response() {
        let json = r#"{
            "five_hour": {"utilization": 0.45, "resets_at": "2026-02-04T12:00:00Z"},
            "seven_day": {"utilization": 0.30, "resets_at": "2026-02-10T00:00:00Z"}
        }"#;

        let response: OAuthUsageResponse = serde_json::from_str(json).unwrap();
        let provider = ClaudeQuotaProvider::new();
        let snapshots = provider.response_to_snapshots(response);

        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].used_percent, 45.0);
        assert_eq!(snapshots[1].used_percent, 30.0);
    }

    #[test]
    fn test_parse_with_extra_usage() {
        let json = r#"{
            "five_hour": {"utilization": 0.50, "resets_at": "2026-02-04T12:00:00Z"},
            "extra_usage": {"is_enabled": true, "used_credits": 5.50, "monthly_limit": 100.0, "currency": "USD"}
        }"#;

        let response: OAuthUsageResponse = serde_json::from_str(json).unwrap();
        let provider = ClaudeQuotaProvider::new();
        let snapshots = provider.response_to_snapshots(response);

        assert_eq!(snapshots.len(), 1);
        assert!(snapshots[0].extra_credits.is_some());
        assert_eq!(snapshots[0].extra_credits.as_ref().unwrap().used, 5.50);
    }
}
```

**Step 2: Build to verify**

```bash
cd /Users/weifanliao/PycharmProjects/recap-worktrees/v2.2.0-dev/web && cargo build --package recap-core
```

Expected: Build succeeds

**Step 3: Run tests**

```bash
cd /Users/weifanliao/PycharmProjects/recap-worktrees/v2.2.0-dev/web && cargo test --package recap-core quota
```

Expected: Tests pass

**Step 4: Commit**

```bash
git add web/crates/recap-core/src/services/quota/claude.rs
git commit -m "feat(quota): implement ClaudeQuotaProvider with OAuth API"
```

---

## Task 4: Quota Store Implementation

**Files:**
- Create: `web/crates/recap-core/src/services/quota/store.rs`

**Step 1: Create store.rs**

```rust
//! Quota storage - save and query quota snapshots

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use super::provider::QuotaError;
use super::types::{ExtraCredits, QuotaProviderType, QuotaSnapshot, QuotaWindowType};

/// Stored quota snapshot (from database)
#[derive(Debug, Clone)]
pub struct StoredQuotaSnapshot {
    pub id: String,
    pub user_id: String,
    pub provider: String,
    pub model: Option<String>,
    pub window_type: String,
    pub used_percent: f64,
    pub resets_at: Option<String>,
    pub extra_credits_used: Option<f64>,
    pub extra_credits_limit: Option<f64>,
    pub raw_response: Option<String>,
    pub created_at: String,
}

impl StoredQuotaSnapshot {
    /// Convert to QuotaSnapshot
    pub fn to_quota_snapshot(&self) -> Option<QuotaSnapshot> {
        let provider = match self.provider.as_str() {
            "claude" => QuotaProviderType::Claude,
            "antigravity" => QuotaProviderType::Antigravity,
            _ => return None,
        };

        let window_type = QuotaWindowType::from_str(&self.window_type)?;

        let resets_at = self.resets_at.as_ref().and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        });

        let fetched_at = DateTime::parse_from_rfc3339(&self.created_at)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let extra_credits = if let (Some(used), Some(limit)) =
            (self.extra_credits_used, self.extra_credits_limit)
        {
            Some(ExtraCredits {
                used,
                limit,
                currency: "USD".to_string(),
            })
        } else {
            None
        };

        Some(QuotaSnapshot {
            provider,
            model: self.model.clone(),
            window_type,
            used_percent: self.used_percent,
            resets_at,
            extra_credits,
            fetched_at,
        })
    }
}

/// Quota store for database operations
pub struct QuotaStore {
    pool: SqlitePool,
}

impl QuotaStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Save quota snapshots to database
    pub async fn save_snapshots(
        &self,
        user_id: &str,
        snapshots: &[QuotaSnapshot],
        raw_response: Option<&str>,
    ) -> Result<(), QuotaError> {
        log::info!("[quota:store] Saving {} snapshots for user {}", snapshots.len(), user_id);

        for snapshot in snapshots {
            let id = Uuid::new_v4().to_string();
            let provider = snapshot.provider.to_string();
            let window_type = snapshot.window_type.to_string();
            let resets_at = snapshot.resets_at.map(|dt| dt.to_rfc3339());
            let (extra_used, extra_limit) = snapshot.extra_credits.as_ref()
                .map(|e| (Some(e.used), Some(e.limit)))
                .unwrap_or((None, None));

            sqlx::query(
                r#"
                INSERT INTO quota_snapshots
                (id, user_id, provider, model, window_type, used_percent, resets_at,
                 extra_credits_used, extra_credits_limit, raw_response, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
                "#
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
            .bind(raw_response)
            .execute(&self.pool)
            .await
            .map_err(|e| QuotaError::ApiError(format!("Database error: {}", e)))?;

            log::debug!(
                "[quota:store] Saved snapshot: provider={}, window={}, used={:.1}%",
                provider, window_type, snapshot.used_percent
            );
        }

        Ok(())
    }

    /// Get latest snapshots for a user/provider
    pub async fn get_latest(
        &self,
        user_id: &str,
        provider: Option<&str>,
    ) -> Result<Vec<QuotaSnapshot>, QuotaError> {
        log::debug!("[quota:store] Getting latest snapshots for user {}", user_id);

        let rows: Vec<StoredQuotaSnapshot> = if let Some(p) = provider {
            sqlx::query_as!(
                StoredQuotaSnapshot,
                r#"
                SELECT id, user_id, provider, model, window_type, used_percent,
                       resets_at, extra_credits_used, extra_credits_limit,
                       raw_response, created_at
                FROM quota_snapshots
                WHERE user_id = ? AND provider = ?
                ORDER BY created_at DESC
                LIMIT 10
                "#,
                user_id,
                p
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| QuotaError::ApiError(format!("Database error: {}", e)))?
        } else {
            sqlx::query_as!(
                StoredQuotaSnapshot,
                r#"
                SELECT id, user_id, provider, model, window_type, used_percent,
                       resets_at, extra_credits_used, extra_credits_limit,
                       raw_response, created_at
                FROM quota_snapshots
                WHERE user_id = ?
                ORDER BY created_at DESC
                LIMIT 20
                "#,
                user_id
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| QuotaError::ApiError(format!("Database error: {}", e)))?
        };

        let snapshots: Vec<QuotaSnapshot> = rows
            .iter()
            .filter_map(|r| r.to_quota_snapshot())
            .collect();

        log::debug!("[quota:store] Retrieved {} snapshots", snapshots.len());
        Ok(snapshots)
    }

    /// Get quota history for charts
    pub async fn get_history(
        &self,
        user_id: &str,
        provider: &str,
        window_type: &str,
        days: i32,
    ) -> Result<Vec<QuotaSnapshot>, QuotaError> {
        log::debug!(
            "[quota:store] Getting {} days history for {}/{}",
            days, provider, window_type
        );

        let rows: Vec<StoredQuotaSnapshot> = sqlx::query_as!(
            StoredQuotaSnapshot,
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
            user_id,
            provider,
            window_type,
            days
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| QuotaError::ApiError(format!("Database error: {}", e)))?;

        let snapshots: Vec<QuotaSnapshot> = rows
            .iter()
            .filter_map(|r| r.to_quota_snapshot())
            .collect();

        log::debug!("[quota:store] Retrieved {} history points", snapshots.len());
        Ok(snapshots)
    }

    /// Clean up old snapshots (keep last N days)
    pub async fn cleanup(&self, days: i32) -> Result<u64, QuotaError> {
        log::info!("[quota:store] Cleaning up snapshots older than {} days", days);

        let result = sqlx::query(
            r#"
            DELETE FROM quota_snapshots
            WHERE created_at < datetime('now', '-' || ? || ' days')
            "#
        )
        .bind(days)
        .execute(&self.pool)
        .await
        .map_err(|e| QuotaError::ApiError(format!("Database error: {}", e)))?;

        let deleted = result.rows_affected();
        log::info!("[quota:store] Deleted {} old snapshots", deleted);
        Ok(deleted)
    }
}
```

**Step 2: Update mod.rs to export store**

The mod.rs already has `pub mod store;` from Task 2.

**Step 3: Build to verify**

```bash
cd /Users/weifanliao/PycharmProjects/recap-worktrees/v2.2.0-dev/web && cargo build --package recap-core
```

Expected: Build succeeds

**Step 4: Commit**

```bash
git add web/crates/recap-core/src/services/quota/store.rs
git commit -m "feat(quota): implement QuotaStore for database operations"
```

---

## Task 5: Tauri Commands

**Files:**
- Create: `web/src-tauri/src/commands/quota.rs`
- Modify: `web/src-tauri/src/commands/mod.rs`
- Modify: `web/src-tauri/src/lib.rs`

**Step 1: Create quota.rs**

```rust
//! Quota tracking commands

use recap_core::services::quota::{
    claude::ClaudeQuotaProvider,
    store::QuotaStore,
    QuotaProvider,
    QuotaSnapshot,
    QuotaSettings,
};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::auth::verify_token;

use super::AppState;

/// Current quota response
#[derive(Debug, Serialize)]
pub struct CurrentQuotaResponse {
    pub snapshots: Vec<QuotaSnapshotDto>,
    pub provider_available: bool,
}

/// Quota snapshot DTO for frontend
#[derive(Debug, Serialize, Deserialize)]
pub struct QuotaSnapshotDto {
    pub provider: String,
    pub model: Option<String>,
    pub window_type: String,
    pub used_percent: f64,
    pub resets_at: Option<String>,
    pub extra_credits_used: Option<f64>,
    pub extra_credits_limit: Option<f64>,
    pub fetched_at: String,
}

impl From<QuotaSnapshot> for QuotaSnapshotDto {
    fn from(s: QuotaSnapshot) -> Self {
        Self {
            provider: s.provider.to_string(),
            model: s.model,
            window_type: s.window_type.to_string(),
            used_percent: s.used_percent,
            resets_at: s.resets_at.map(|dt| dt.to_rfc3339()),
            extra_credits_used: s.extra_credits.as_ref().map(|e| e.used),
            extra_credits_limit: s.extra_credits.as_ref().map(|e| e.limit),
            fetched_at: s.fetched_at.to_rfc3339(),
        }
    }
}

/// Fetch current quota from Claude API
#[tauri::command]
pub async fn get_current_quota(
    state: State<'_, AppState>,
    token: String,
) -> Result<CurrentQuotaResponse, String> {
    log::info!("[quota:cmd] get_current_quota called");

    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let user_id = &claims.sub;

    let provider = ClaudeQuotaProvider::new();

    if !provider.is_available().await {
        log::warn!("[quota:cmd] Claude provider not available");
        return Ok(CurrentQuotaResponse {
            snapshots: vec![],
            provider_available: false,
        });
    }

    match provider.fetch_quota().await {
        Ok(snapshots) => {
            log::info!("[quota:cmd] Fetched {} snapshots", snapshots.len());

            // Save to database
            let db = state.db.lock().await;
            let store = QuotaStore::new(db.pool.clone());
            if let Err(e) = store.save_snapshots(user_id, &snapshots, None).await {
                log::error!("[quota:cmd] Failed to save snapshots: {:?}", e);
            }

            let dtos: Vec<QuotaSnapshotDto> = snapshots.into_iter().map(Into::into).collect();
            Ok(CurrentQuotaResponse {
                snapshots: dtos,
                provider_available: true,
            })
        }
        Err(e) => {
            log::error!("[quota:cmd] Failed to fetch quota: {:?}", e);
            Err(format!("Failed to fetch quota: {}", e))
        }
    }
}

/// Get cached/stored quota snapshots
#[tauri::command]
pub async fn get_stored_quota(
    state: State<'_, AppState>,
    token: String,
    provider: Option<String>,
) -> Result<Vec<QuotaSnapshotDto>, String> {
    log::info!("[quota:cmd] get_stored_quota called, provider={:?}", provider);

    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let user_id = &claims.sub;

    let db = state.db.lock().await;
    let store = QuotaStore::new(db.pool.clone());

    let snapshots = store
        .get_latest(user_id, provider.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(snapshots.into_iter().map(Into::into).collect())
}

/// Get quota history for charts
#[tauri::command]
pub async fn get_quota_history(
    state: State<'_, AppState>,
    token: String,
    provider: String,
    window_type: String,
    days: Option<i32>,
) -> Result<Vec<QuotaSnapshotDto>, String> {
    let days = days.unwrap_or(7);
    log::info!(
        "[quota:cmd] get_quota_history: provider={}, window={}, days={}",
        provider, window_type, days
    );

    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let user_id = &claims.sub;

    let db = state.db.lock().await;
    let store = QuotaStore::new(db.pool.clone());

    let snapshots = store
        .get_history(user_id, &provider, &window_type, days)
        .await
        .map_err(|e| e.to_string())?;

    Ok(snapshots.into_iter().map(Into::into).collect())
}

/// Check if quota provider is available
#[tauri::command]
pub async fn check_quota_provider_available(
    _token: String,
    provider: String,
) -> Result<bool, String> {
    log::info!("[quota:cmd] check_quota_provider_available: {}", provider);

    match provider.as_str() {
        "claude" => {
            let provider = ClaudeQuotaProvider::new();
            Ok(provider.is_available().await)
        }
        _ => {
            log::warn!("[quota:cmd] Unknown provider: {}", provider);
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quota_snapshot_dto_conversion() {
        use recap_core::services::quota::types::{QuotaProviderType, QuotaWindowType};
        use chrono::Utc;

        let snapshot = QuotaSnapshot {
            provider: QuotaProviderType::Claude,
            model: None,
            window_type: QuotaWindowType::FiveHour,
            used_percent: 45.5,
            resets_at: None,
            extra_credits: None,
            fetched_at: Utc::now(),
        };

        let dto: QuotaSnapshotDto = snapshot.into();
        assert_eq!(dto.provider, "claude");
        assert_eq!(dto.window_type, "five_hour");
        assert_eq!(dto.used_percent, 45.5);
    }
}
```

**Step 2: Update commands/mod.rs**

Add to the module:

```rust
pub mod quota;
```

And add to the re-exports if there's a pub use section.

**Step 3: Update lib.rs to register commands**

Find the `invoke_handler` macro call and add the quota commands:

```rust
// In the generate_handler! or invoke_handler! macro
quota::get_current_quota,
quota::get_stored_quota,
quota::get_quota_history,
quota::check_quota_provider_available,
```

**Step 4: Build to verify**

```bash
cd /Users/weifanliao/PycharmProjects/recap-worktrees/v2.2.0-dev/web && cargo build
```

Expected: Build succeeds

**Step 5: Commit**

```bash
git add web/src-tauri/src/commands/quota.rs web/src-tauri/src/commands/mod.rs web/src-tauri/src/lib.rs
git commit -m "feat(quota): add Tauri commands for quota tracking"
```

---

## Task 6: Frontend Types and Service

**Files:**
- Create: `web/src/types/quota.ts`
- Create: `web/src/services/quota.ts`
- Modify: `web/src/types/index.ts`
- Modify: `web/src/services/index.ts`

**Step 1: Create types/quota.ts**

```typescript
// Quota tracking types

export type QuotaProvider = 'claude' | 'antigravity'

export type QuotaWindowType =
  | 'five_hour'
  | 'seven_day'
  | 'seven_day_opus'
  | 'seven_day_sonnet'
  | 'monthly'

export interface QuotaSnapshot {
  provider: QuotaProvider
  model: string | null
  window_type: QuotaWindowType
  used_percent: number
  resets_at: string | null
  extra_credits_used: number | null
  extra_credits_limit: number | null
  fetched_at: string
}

export interface CurrentQuotaResponse {
  snapshots: QuotaSnapshot[]
  provider_available: boolean
}

export interface QuotaSettings {
  interval_minutes: number
  warning_threshold: number
  critical_threshold: number
  notifications_enabled: boolean
}

export type AlertLevel = 'normal' | 'warning' | 'critical'

export function getAlertLevel(
  usedPercent: number,
  settings: QuotaSettings
): AlertLevel {
  if (usedPercent >= settings.critical_threshold) return 'critical'
  if (usedPercent >= settings.warning_threshold) return 'warning'
  return 'normal'
}

export function formatWindowType(windowType: QuotaWindowType): string {
  switch (windowType) {
    case 'five_hour': return '5hr'
    case 'seven_day': return '7day'
    case 'seven_day_opus': return 'Opus'
    case 'seven_day_sonnet': return 'Sonnet'
    case 'monthly': return 'Monthly'
    default: return windowType
  }
}

export function formatResetTime(resetsAt: string | null): string {
  if (!resetsAt) return '-'

  const resetDate = new Date(resetsAt)
  const now = new Date()
  const diffMs = resetDate.getTime() - now.getTime()

  if (diffMs <= 0) return 'Now'

  const diffMins = Math.floor(diffMs / 60000)
  const hours = Math.floor(diffMins / 60)
  const mins = diffMins % 60

  if (hours > 0) {
    return `${hours}h ${mins}m`
  }
  return `${mins}m`
}
```

**Step 2: Create services/quota.ts**

```typescript
// Quota tracking service

import { invoke } from '@tauri-apps/api/core'
import { getRequiredToken } from './client'
import type { CurrentQuotaResponse, QuotaSnapshot } from '@/types/quota'

const LOG_PREFIX = '[quota]'

/**
 * Fetch current quota from Claude API
 */
export async function getCurrentQuota(): Promise<CurrentQuotaResponse> {
  console.log(`${LOG_PREFIX} Fetching current quota...`)

  try {
    const result = await invoke<CurrentQuotaResponse>('get_current_quota', {
      token: getRequiredToken(),
    })
    console.log(`${LOG_PREFIX} Current quota fetched:`, result)
    return result
  } catch (error) {
    console.error(`${LOG_PREFIX} Failed to fetch current quota:`, error)
    throw error
  }
}

/**
 * Get stored quota snapshots
 */
export async function getStoredQuota(
  provider?: string
): Promise<QuotaSnapshot[]> {
  console.log(`${LOG_PREFIX} Getting stored quota, provider=${provider}`)

  try {
    const result = await invoke<QuotaSnapshot[]>('get_stored_quota', {
      token: getRequiredToken(),
      provider,
    })
    console.log(`${LOG_PREFIX} Stored quota retrieved:`, result.length, 'snapshots')
    return result
  } catch (error) {
    console.error(`${LOG_PREFIX} Failed to get stored quota:`, error)
    throw error
  }
}

/**
 * Get quota history for charts
 */
export async function getQuotaHistory(
  provider: string,
  windowType: string,
  days?: number
): Promise<QuotaSnapshot[]> {
  console.log(`${LOG_PREFIX} Getting quota history: ${provider}/${windowType}, ${days} days`)

  try {
    const result = await invoke<QuotaSnapshot[]>('get_quota_history', {
      token: getRequiredToken(),
      provider,
      windowType,
      days,
    })
    console.log(`${LOG_PREFIX} History retrieved:`, result.length, 'points')
    return result
  } catch (error) {
    console.error(`${LOG_PREFIX} Failed to get quota history:`, error)
    throw error
  }
}

/**
 * Check if a quota provider is available
 */
export async function checkProviderAvailable(
  provider: string
): Promise<boolean> {
  console.log(`${LOG_PREFIX} Checking provider availability: ${provider}`)

  try {
    const result = await invoke<boolean>('check_quota_provider_available', {
      token: getRequiredToken(),
      provider,
    })
    console.log(`${LOG_PREFIX} Provider ${provider} available: ${result}`)
    return result
  } catch (error) {
    console.error(`${LOG_PREFIX} Failed to check provider:`, error)
    return false
  }
}
```

**Step 3: Update types/index.ts**

Add:

```typescript
export * from './quota'
```

**Step 4: Update services/index.ts**

Add:

```typescript
export * as quota from './quota'
```

**Step 5: Verify TypeScript compiles**

```bash
cd /Users/weifanliao/PycharmProjects/recap-worktrees/v2.2.0-dev/web && npx tsc --noEmit
```

Expected: No errors

**Step 6: Commit**

```bash
git add web/src/types/quota.ts web/src/services/quota.ts web/src/types/index.ts web/src/services/index.ts
git commit -m "feat(quota): add frontend types and service layer"
```

---

## Task 7: Dashboard Quota Card Component

**Files:**
- Create: `web/src/pages/Dashboard/components/QuotaCard.tsx`
- Modify: `web/src/pages/Dashboard/index.tsx`

**Step 1: Create QuotaCard.tsx**

```tsx
// Dashboard quota card component

import { useState, useEffect } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import { RefreshCw, AlertTriangle, AlertCircle } from 'lucide-react'
import { quota } from '@/services'
import type { QuotaSnapshot, QuotaSettings, AlertLevel } from '@/types/quota'
import { getAlertLevel, formatWindowType, formatResetTime } from '@/types/quota'
import { cn } from '@/lib/utils'

const DEFAULT_SETTINGS: QuotaSettings = {
  interval_minutes: 15,
  warning_threshold: 80,
  critical_threshold: 95,
  notifications_enabled: true,
}

export function QuotaCard() {
  const [snapshots, setSnapshots] = useState<QuotaSnapshot[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [providerAvailable, setProviderAvailable] = useState(true)

  const fetchQuota = async () => {
    console.log('[QuotaCard] Fetching quota...')
    setLoading(true)
    setError(null)

    try {
      const result = await quota.getCurrentQuota()
      setSnapshots(result.snapshots)
      setProviderAvailable(result.provider_available)
      console.log('[QuotaCard] Quota fetched:', result.snapshots.length, 'snapshots')
    } catch (err) {
      console.error('[QuotaCard] Error fetching quota:', err)
      setError(err instanceof Error ? err.message : 'Failed to fetch quota')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchQuota()
  }, [])

  const getAlertColor = (level: AlertLevel) => {
    switch (level) {
      case 'critical': return 'text-red-500'
      case 'warning': return 'text-yellow-500'
      default: return 'text-green-500'
    }
  }

  const getProgressColor = (level: AlertLevel) => {
    switch (level) {
      case 'critical': return 'bg-red-500'
      case 'warning': return 'bg-yellow-500'
      default: return 'bg-green-500'
    }
  }

  const getAlertIcon = (level: AlertLevel) => {
    switch (level) {
      case 'critical': return <AlertCircle className="h-4 w-4 text-red-500" />
      case 'warning': return <AlertTriangle className="h-4 w-4 text-yellow-500" />
      default: return null
    }
  }

  // Group snapshots by provider
  const claudeSnapshots = snapshots.filter(s => s.provider === 'claude')
  const primarySnapshot = claudeSnapshots.find(s => s.window_type === 'five_hour')

  if (!providerAvailable) {
    return (
      <Card>
        <CardHeader className="flex flex-row items-center justify-between pb-2">
          <CardTitle className="text-sm font-medium">Quota Usage</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            Claude Code not configured. Run `claude` to authenticate.
          </p>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-sm font-medium">Quota Usage</CardTitle>
        <Button
          variant="ghost"
          size="sm"
          onClick={fetchQuota}
          disabled={loading}
        >
          <RefreshCw className={cn("h-4 w-4", loading && "animate-spin")} />
        </Button>
      </CardHeader>
      <CardContent>
        {error && (
          <p className="text-sm text-red-500 mb-2">{error}</p>
        )}

        {claudeSnapshots.length === 0 && !loading && !error && (
          <p className="text-sm text-muted-foreground">No quota data available</p>
        )}

        <div className="space-y-3">
          {claudeSnapshots.map((snapshot) => {
            const level = getAlertLevel(snapshot.used_percent, DEFAULT_SETTINGS)
            return (
              <div key={`${snapshot.provider}-${snapshot.window_type}`}>
                <div className="flex items-center justify-between mb-1">
                  <span className="text-sm font-medium flex items-center gap-1">
                    {formatWindowType(snapshot.window_type)}
                    {getAlertIcon(level)}
                  </span>
                  <span className={cn("text-sm font-bold", getAlertColor(level))}>
                    {snapshot.used_percent.toFixed(0)}%
                  </span>
                </div>
                <Progress
                  value={snapshot.used_percent}
                  className="h-2"
                  indicatorClassName={getProgressColor(level)}
                />
                <p className="text-xs text-muted-foreground mt-1">
                  Resets in {formatResetTime(snapshot.resets_at)}
                </p>
              </div>
            )
          })}
        </div>

        {primarySnapshot?.extra_credits_used != null && (
          <div className="mt-3 pt-3 border-t">
            <p className="text-xs text-muted-foreground">
              Extra credits: ${primarySnapshot.extra_credits_used?.toFixed(2)} / ${primarySnapshot.extra_credits_limit?.toFixed(2)}
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  )
}
```

**Step 2: Add QuotaCard to Dashboard**

Find where other cards are rendered in Dashboard/index.tsx and add:

```tsx
import { QuotaCard } from './components/QuotaCard'

// In the render, add alongside other cards:
<QuotaCard />
```

**Step 3: Verify build**

```bash
cd /Users/weifanliao/PycharmProjects/recap-worktrees/v2.2.0-dev/web && npm run build
```

Expected: Build succeeds

**Step 4: Commit**

```bash
git add web/src/pages/Dashboard/components/QuotaCard.tsx web/src/pages/Dashboard/index.tsx
git commit -m "feat(quota): add QuotaCard component to Dashboard"
```

---

## Task 8: Tray Title Update

**Files:**
- Modify: `web/src-tauri/src/commands/tray.rs`
- Modify: `web/src-tauri/src/commands/quota.rs`

**Step 1: Add tray title update function to tray.rs**

Add after existing functions:

```rust
/// Update tray title with quota percentage
#[tauri::command]
pub async fn update_tray_quota(
    app: AppHandle,
    claude_percent: Option<f64>,
    antigravity_percent: Option<f64>,
) -> Result<(), String> {
    let tray = app
        .tray_by_id("main-tray")
        .ok_or_else(|| "Tray icon not found".to_string())?;

    let title = match (claude_percent, antigravity_percent) {
        (Some(c), Some(a)) => format!("C:{:.0}% A:{:.0}%", c, a),
        (Some(c), None) => format!("{:.0}%", c),
        (None, Some(a)) => format!("{:.0}%", a),
        (None, None) => "—".to_string(),
    };

    tray.set_title(Some(&title)).map_err(|e| e.to_string())?;
    log::debug!("[tray] Updated quota title: {}", title);

    Ok(())
}
```

**Step 2: Register command in lib.rs**

Add to invoke_handler:

```rust
tray::update_tray_quota,
```

**Step 3: Add frontend service function**

Add to `web/src/services/tray.ts`:

```typescript
/**
 * Update tray title with quota percentages
 */
export async function updateTrayQuota(
  claudePercent?: number,
  antigravityPercent?: number
): Promise<void> {
  console.log('[tray] Updating quota:', { claudePercent, antigravityPercent })
  await invoke('update_tray_quota', {
    claudePercent,
    antigravityPercent,
  })
}
```

**Step 4: Update QuotaCard to sync tray**

In QuotaCard.tsx, after fetching quota:

```typescript
import { tray } from '@/services'

// In fetchQuota, after setSnapshots:
const fiveHour = result.snapshots.find(s =>
  s.provider === 'claude' && s.window_type === 'five_hour'
)
if (fiveHour) {
  tray.updateTrayQuota(fiveHour.used_percent)
}
```

**Step 5: Build and verify**

```bash
cd /Users/weifanliao/PycharmProjects/recap-worktrees/v2.2.0-dev/web && cargo build && npm run build
```

**Step 6: Commit**

```bash
git add web/src-tauri/src/commands/tray.rs web/src-tauri/src/lib.rs web/src/services/tray.ts web/src/pages/Dashboard/components/QuotaCard.tsx
git commit -m "feat(quota): update tray title with quota percentage"
```

---

## Task 9: Quota Page with History Chart

**Files:**
- Create: `web/src/pages/Quota/index.tsx`
- Create: `web/src/pages/Quota/components/QuotaChart.tsx`
- Create: `web/src/pages/Quota/hooks.ts`
- Modify: `web/src/App.tsx` (add route)

**Step 1: Create hooks.ts**

```typescript
// Quota page hooks

import { useState, useEffect, useCallback } from 'react'
import { quota } from '@/services'
import type { QuotaSnapshot } from '@/types/quota'

export function useQuotaData() {
  const [currentQuota, setCurrentQuota] = useState<QuotaSnapshot[]>([])
  const [history, setHistory] = useState<QuotaSnapshot[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const [provider, setProvider] = useState<string>('claude')
  const [windowType, setWindowType] = useState<string>('five_hour')
  const [days, setDays] = useState<number>(7)

  const fetchCurrent = useCallback(async () => {
    console.log('[useQuotaData] Fetching current quota')
    try {
      const result = await quota.getCurrentQuota()
      setCurrentQuota(result.snapshots)
    } catch (err) {
      console.error('[useQuotaData] Error:', err)
      setError(err instanceof Error ? err.message : 'Failed to fetch')
    }
  }, [])

  const fetchHistory = useCallback(async () => {
    console.log('[useQuotaData] Fetching history:', provider, windowType, days)
    try {
      const result = await quota.getQuotaHistory(provider, windowType, days)
      setHistory(result)
    } catch (err) {
      console.error('[useQuotaData] Error:', err)
      setError(err instanceof Error ? err.message : 'Failed to fetch')
    }
  }, [provider, windowType, days])

  const refresh = useCallback(async () => {
    setLoading(true)
    setError(null)
    await Promise.all([fetchCurrent(), fetchHistory()])
    setLoading(false)
  }, [fetchCurrent, fetchHistory])

  useEffect(() => {
    refresh()
  }, [refresh])

  return {
    currentQuota,
    history,
    loading,
    error,
    provider,
    setProvider,
    windowType,
    setWindowType,
    days,
    setDays,
    refresh,
  }
}
```

**Step 2: Create QuotaChart.tsx**

```tsx
// Quota history chart

import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  ReferenceLine,
} from 'recharts'
import type { QuotaSnapshot, QuotaSettings } from '@/types/quota'

interface QuotaChartProps {
  data: QuotaSnapshot[]
  settings: QuotaSettings
}

export function QuotaChart({ data, settings }: QuotaChartProps) {
  const chartData = data.map((snapshot) => ({
    time: new Date(snapshot.fetched_at).toLocaleString('zh-TW', {
      month: 'numeric',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    }),
    value: snapshot.used_percent,
    timestamp: snapshot.fetched_at,
  }))

  return (
    <ResponsiveContainer width="100%" height={300}>
      <LineChart data={chartData} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
        <CartesianGrid strokeDasharray="3 3" />
        <XAxis
          dataKey="time"
          fontSize={12}
          tickMargin={10}
        />
        <YAxis
          domain={[0, 100]}
          fontSize={12}
          tickFormatter={(v) => `${v}%`}
        />
        <Tooltip
          formatter={(value: number) => [`${value.toFixed(1)}%`, 'Usage']}
          labelFormatter={(label) => label}
        />
        <ReferenceLine
          y={settings.warning_threshold}
          stroke="#eab308"
          strokeDasharray="5 5"
          label={{ value: 'Warning', position: 'right', fontSize: 10 }}
        />
        <ReferenceLine
          y={settings.critical_threshold}
          stroke="#ef4444"
          strokeDasharray="5 5"
          label={{ value: 'Critical', position: 'right', fontSize: 10 }}
        />
        <Line
          type="monotone"
          dataKey="value"
          stroke="#3b82f6"
          strokeWidth={2}
          dot={false}
          activeDot={{ r: 4 }}
        />
      </LineChart>
    </ResponsiveContainer>
  )
}
```

**Step 3: Create Quota/index.tsx**

```tsx
// Quota tracking page

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { RefreshCw } from 'lucide-react'
import { useQuotaData } from './hooks'
import { QuotaChart } from './components/QuotaChart'
import type { QuotaSettings } from '@/types/quota'
import { formatWindowType } from '@/types/quota'
import { cn } from '@/lib/utils'

const DEFAULT_SETTINGS: QuotaSettings = {
  interval_minutes: 15,
  warning_threshold: 80,
  critical_threshold: 95,
  notifications_enabled: true,
}

export default function QuotaPage() {
  const {
    currentQuota,
    history,
    loading,
    error,
    provider,
    setProvider,
    windowType,
    setWindowType,
    days,
    setDays,
    refresh,
  } = useQuotaData()

  return (
    <div className="container mx-auto p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Quota Usage</h1>
        <Button onClick={refresh} disabled={loading}>
          <RefreshCw className={cn("h-4 w-4 mr-2", loading && "animate-spin")} />
          Refresh
        </Button>
      </div>

      {error && (
        <div className="bg-red-100 text-red-700 p-4 rounded-md">
          {error}
        </div>
      )}

      {/* Current Quota Summary */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {currentQuota.map((snapshot) => (
          <Card key={`${snapshot.provider}-${snapshot.window_type}`}>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">
                {snapshot.provider === 'claude' ? 'Claude' : 'Antigravity'} - {formatWindowType(snapshot.window_type)}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold">
                {snapshot.used_percent.toFixed(0)}%
              </div>
              {snapshot.resets_at && (
                <p className="text-sm text-muted-foreground">
                  Resets: {new Date(snapshot.resets_at).toLocaleString('zh-TW')}
                </p>
              )}
            </CardContent>
          </Card>
        ))}
      </div>

      {/* History Chart */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>History</CardTitle>
            <div className="flex gap-2">
              <Select value={provider} onValueChange={setProvider}>
                <SelectTrigger className="w-32">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="claude">Claude</SelectItem>
                  <SelectItem value="antigravity">Antigravity</SelectItem>
                </SelectContent>
              </Select>

              <Select value={windowType} onValueChange={setWindowType}>
                <SelectTrigger className="w-32">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="five_hour">5 Hour</SelectItem>
                  <SelectItem value="seven_day">7 Day</SelectItem>
                </SelectContent>
              </Select>

              <Select value={days.toString()} onValueChange={(v) => setDays(parseInt(v))}>
                <SelectTrigger className="w-32">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="1">1 Day</SelectItem>
                  <SelectItem value="7">7 Days</SelectItem>
                  <SelectItem value="14">14 Days</SelectItem>
                  <SelectItem value="30">30 Days</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          {history.length > 0 ? (
            <QuotaChart data={history} settings={DEFAULT_SETTINGS} />
          ) : (
            <p className="text-center text-muted-foreground py-8">
              No history data available
            </p>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
```

**Step 4: Add route in App.tsx**

Add the import and route:

```tsx
import QuotaPage from './pages/Quota'

// In routes:
<Route path="/quota" element={<QuotaPage />} />
```

**Step 5: Add navigation link**

Find the navigation component and add a link to /quota.

**Step 6: Build and verify**

```bash
cd /Users/weifanliao/PycharmProjects/recap-worktrees/v2.2.0-dev/web && npm run build
```

**Step 7: Commit**

```bash
git add web/src/pages/Quota/ web/src/App.tsx
git commit -m "feat(quota): add Quota page with history chart"
```

---

## Task 10: Background Polling Timer

**Files:**
- Create: `web/crates/recap-core/src/services/quota/timer.rs`
- Modify: `web/src-tauri/src/commands/quota.rs`
- Modify: `web/src-tauri/src/lib.rs`

**Step 1: Create timer.rs**

```rust
//! Quota polling timer

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;

use super::claude::ClaudeQuotaProvider;
use super::provider::QuotaProvider;
use super::store::QuotaStore;
use super::types::QuotaSnapshot;

/// Quota timer state
pub struct QuotaTimer {
    interval_minutes: u32,
    user_id: String,
    store: QuotaStore,
    last_snapshots: Arc<RwLock<Vec<QuotaSnapshot>>>,
}

impl QuotaTimer {
    pub fn new(interval_minutes: u32, user_id: String, store: QuotaStore) -> Self {
        Self {
            interval_minutes,
            user_id,
            store,
            last_snapshots: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Start the polling loop
    pub async fn start(self: Arc<Self>) {
        let interval = Duration::from_secs(self.interval_minutes as u64 * 60);
        log::info!(
            "[quota:timer] Starting polling every {} minutes",
            self.interval_minutes
        );

        let mut ticker = time::interval(interval);

        loop {
            ticker.tick().await;

            if let Err(e) = self.tick().await {
                log::error!("[quota:timer] Tick error: {:?}", e);
            }
        }
    }

    async fn tick(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::debug!("[quota:timer] Tick - fetching quota");

        let provider = ClaudeQuotaProvider::new();

        if !provider.is_available().await {
            log::debug!("[quota:timer] Claude provider not available, skipping");
            return Ok(());
        }

        match provider.fetch_quota().await {
            Ok(snapshots) => {
                log::info!(
                    "[quota:timer] Fetched {} snapshots",
                    snapshots.len()
                );

                // Save to database
                self.store
                    .save_snapshots(&self.user_id, &snapshots, None)
                    .await?;

                // Update cached snapshots
                *self.last_snapshots.write().await = snapshots;
            }
            Err(e) => {
                log::warn!("[quota:timer] Failed to fetch quota: {:?}", e);
            }
        }

        Ok(())
    }

    /// Get last fetched snapshots
    pub async fn get_last_snapshots(&self) -> Vec<QuotaSnapshot> {
        self.last_snapshots.read().await.clone()
    }
}
```

**Step 2: Update mod.rs**

Add:

```rust
pub mod timer;
```

**Step 3: Integrate timer in Tauri setup**

This is complex and depends on app lifecycle. For now, we can start the timer when first quota is fetched, or add a dedicated start command.

**Step 4: Build and verify**

```bash
cd /Users/weifanliao/PycharmProjects/recap-worktrees/v2.2.0-dev/web && cargo build --package recap-core
```

**Step 5: Commit**

```bash
git add web/crates/recap-core/src/services/quota/timer.rs web/crates/recap-core/src/services/quota/mod.rs
git commit -m "feat(quota): add background polling timer"
```

---

## Final Integration Test

After all tasks are complete:

**Step 1: Full build**

```bash
cd /Users/weifanliao/PycharmProjects/recap-worktrees/v2.2.0-dev/web && cargo build && npm run build
```

**Step 2: Run dev mode**

```bash
cd /Users/weifanliao/PycharmProjects/recap-worktrees/v2.2.0-dev/web && RUST_LOG=debug cargo tauri dev
```

**Step 3: Verify features**

1. Dashboard shows QuotaCard
2. Click refresh fetches from API
3. Tray shows percentage
4. /quota page shows history chart

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat(quota): complete Phase 1 - Claude quota tracking"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Database schema | db/mod.rs |
| 2 | Core types & trait | quota/types.rs, provider.rs, mod.rs |
| 3 | Claude provider | quota/claude.rs |
| 4 | Quota store | quota/store.rs |
| 5 | Tauri commands | commands/quota.rs |
| 6 | Frontend types/service | types/quota.ts, services/quota.ts |
| 7 | Dashboard card | Dashboard/components/QuotaCard.tsx |
| 8 | Tray update | commands/tray.rs |
| 9 | Quota page | pages/Quota/* |
| 10 | Background timer | quota/timer.rs |

Total: ~1500 lines of Rust, ~500 lines of TypeScript
