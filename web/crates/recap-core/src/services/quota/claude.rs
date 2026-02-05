//! Claude Code quota provider
//!
//! Implements the QuotaProvider trait for Claude Code, using OAuth
//! to access Anthropic's usage API.
//!
//! # Overview
//!
//! This provider reads the OAuth access token with the following priority:
//!
//! 1. **Manual token** - Configured via Settings UI (stored in database)
//! 2. **System credential store**:
//!    - **macOS**: Keychain under service "Claude Code-credentials"
//!    - **Windows**: Credential Manager under service "Claude Code-credentials"
//! 3. **File fallback**: `~/.claude/.credentials.json`
//!
//! The token is then used to call the Anthropic usage API.
//!
//! # Token Expiration
//!
//! **Important:** Anthropic's OAuth does NOT support automatic token refresh.
//! When the token expires (typically after a few hours), users can:
//!
//! 1. Try `claude setup-token` for a long-lived token (1 year, experimental)
//! 2. Or re-authenticate via `claude /login` (standard method)
//!
//! See: https://github.com/anthropics/claude-code/issues/12447
//!
//! # Quota Windows
//!
//! Claude provides several quota windows:
//! - **5-hour**: Rolling rate limit window
//! - **7-day**: Weekly usage tracking (all models)
//! - **7-day-opus**: Weekly usage for Opus models specifically
//! - **7-day-sonnet**: Weekly usage for Sonnet models specifically
//!
//! # Extra Credits
//!
//! Users on certain plans may have extra credits (pay-as-you-go overflow).
//! This is tracked separately from the quota windows.
//!
//! # Example
//!
//! ```ignore
//! use recap_core::services::quota::{ClaudeQuotaProvider, QuotaProvider};
//!
//! let provider = ClaudeQuotaProvider::new();
//! if provider.is_available().await {
//!     let snapshots = provider.fetch_quota().await?;
//!     for snapshot in snapshots {
//!         println!("{}: {:.1}%", snapshot.window_type, snapshot.used_percent);
//!     }
//! }
//! ```

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;

use super::provider::{QuotaError, QuotaProvider};
use super::types::{AccountInfo, QuotaProviderType, QuotaSnapshot, QuotaWindowType};

// ============================================================================
// Constants
// ============================================================================

/// Anthropic OAuth usage API endpoint
const USAGE_API_URL: &str = "https://api.anthropic.com/api/oauth/usage";

/// Anthropic OAuth profile API endpoint
const PROFILE_API_URL: &str = "https://api.anthropic.com/api/oauth/profile";

/// OAuth beta header value for API access
const OAUTH_BETA_HEADER: &str = "oauth-2025-04-20";

/// HTTP request timeout in seconds
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Default user ID when none is provided
const DEFAULT_USER_ID: &str = "default";

/// macOS Keychain service name for Claude Code credentials
#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "Claude Code-credentials";

/// Windows Credential Manager service name for Claude Code credentials
#[cfg(target_os = "windows")]
const CREDENTIAL_SERVICE: &str = "Claude Code-credentials";

// ============================================================================
// Credentials Types
// ============================================================================

/// Claude credentials file structure (~/.claude/.credentials.json)
///
/// The file contains a nested structure with `claudeAiOauth` as the key.
#[derive(Debug, Deserialize)]
struct CredentialsFile {
    /// Nested OAuth credentials
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: Option<ClaudeOAuthCredentials>,
}

/// OAuth credentials stored within the claudeAiOauth object
#[derive(Debug, Deserialize)]
struct ClaudeOAuthCredentials {
    /// OAuth access token
    #[serde(rename = "accessToken")]
    access_token: Option<String>,

    /// OAuth refresh token (present in file but NOT functional - Anthropic doesn't support token refresh)
    /// See: https://github.com/anthropics/claude-code/issues/12447
    #[serde(rename = "refreshToken")]
    #[allow(dead_code)]
    refresh_token: Option<String>,

    /// Expiration time in milliseconds since epoch
    #[serde(rename = "expiresAt")]
    expires_at: Option<i64>,

    /// OAuth scopes (optional)
    scopes: Option<Vec<String>>,

    /// Subscription type (optional)
    #[serde(rename = "subscriptionType")]
    subscription_type: Option<String>,

    /// Rate limit tier (optional)
    #[serde(rename = "rateLimitTier")]
    rate_limit_tier: Option<String>,
}


// ============================================================================
// API Response Types
// ============================================================================

/// Response from Anthropic's OAuth usage API
#[derive(Debug, Deserialize)]
struct OAuthUsageResponse {
    /// 5-hour rolling window usage
    five_hour: Option<UsageWindow>,

    /// 7-day rolling window usage (all models)
    seven_day: Option<UsageWindow>,

    /// 7-day rolling window for Opus models
    seven_day_opus: Option<UsageWindow>,

    /// 7-day rolling window for Sonnet models
    seven_day_sonnet: Option<UsageWindow>,

    /// Extra credits/pay-as-you-go usage
    extra_usage: Option<ExtraUsage>,
}

/// A single usage window from the API
#[derive(Debug, Deserialize)]
struct UsageWindow {
    /// Utilization as a ratio (0.0 to 1.0)
    utilization: Option<f64>,

    /// When this window resets (ISO8601 format)
    resets_at: Option<String>,
}

/// Extra credits usage information
#[derive(Debug, Deserialize)]
struct ExtraUsage {
    /// Whether extra credits are enabled for this account
    is_enabled: Option<bool>,

    /// Credits used in current period
    used_credits: Option<f64>,

    /// Monthly credit limit
    monthly_limit: Option<f64>,

    /// Currency (e.g., "USD")
    #[allow(dead_code)]
    currency: Option<String>,
}

/// Response from Anthropic's OAuth profile API
#[derive(Debug, Deserialize, serde::Serialize)]
struct OAuthProfileResponse {
    /// Account information
    account: Option<ProfileAccount>,
    /// Organization information
    organization: Option<ProfileOrganization>,
}

/// Account information from profile API
#[derive(Debug, Deserialize, serde::Serialize)]
struct ProfileAccount {
    /// Account UUID
    #[allow(dead_code)]
    uuid: Option<String>,
    /// Full name
    full_name: Option<String>,
    /// Display name
    display_name: Option<String>,
    /// Email address
    email: Option<String>,
    /// Has Claude Max subscription
    has_claude_max: Option<bool>,
    /// Has Claude Pro subscription
    has_claude_pro: Option<bool>,
    /// Account creation date
    #[allow(dead_code)]
    created_at: Option<String>,
}

/// Organization information from profile API
#[derive(Debug, Deserialize, serde::Serialize)]
struct ProfileOrganization {
    /// Organization UUID
    #[allow(dead_code)]
    uuid: Option<String>,
    /// Organization name
    #[allow(dead_code)]
    name: Option<String>,
    /// Organization type (e.g., "claude_max")
    organization_type: Option<String>,
    /// Rate limit tier
    #[allow(dead_code)]
    rate_limit_tier: Option<String>,
    /// Whether extra usage is enabled
    has_extra_usage_enabled: Option<bool>,
    /// Subscription status (e.g., "active")
    subscription_status: Option<String>,
}

// ============================================================================
// ClaudeQuotaProvider
// ============================================================================

/// Quota provider for Claude Code
///
/// Fetches quota usage from Anthropic's OAuth API using the access token.
/// Token sources (in order of priority):
/// 1. Manual token (configured via settings)
/// 2. macOS Keychain (service: "Claude Code-credentials")
/// 3. File: `~/.claude/.credentials.json`
pub struct ClaudeQuotaProvider {
    /// Path to credentials file (fallback)
    credentials_path: PathBuf,

    /// HTTP client for API requests
    client: Client,

    /// User ID to associate with snapshots
    user_id: String,

    /// Manual OAuth token (highest priority, configured via settings)
    manual_token: Option<String>,
}

impl ClaudeQuotaProvider {
    /// Create a new ClaudeQuotaProvider with default settings
    ///
    /// Uses automatic credential discovery (Keychain → file).
    pub fn new() -> Self {
        let credentials_path = Self::default_credentials_path();
        Self::with_credentials_path(credentials_path)
    }

    /// Create a ClaudeQuotaProvider with a custom credentials path
    ///
    /// Useful for testing or non-standard configurations.
    pub fn with_credentials_path(credentials_path: PathBuf) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .unwrap_or_default();

        Self {
            credentials_path,
            client,
            user_id: DEFAULT_USER_ID.to_string(),
            manual_token: None,
        }
    }

    /// Set the user ID for snapshots
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = user_id.into();
        self
    }

    /// Set a manual OAuth token (highest priority)
    ///
    /// When set, this token is used instead of Keychain or file-based credentials.
    /// This allows users to configure their token via the Settings UI if
    /// automatic credential discovery fails.
    pub fn with_manual_token(mut self, token: Option<String>) -> Self {
        self.manual_token = token.filter(|t| !t.is_empty());
        self
    }

    /// Get the default credentials path
    fn default_credentials_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".claude")
            .join(".credentials.json")
    }

    /// Load the OAuth access token
    ///
    /// Priority order:
    /// 1. Manual token (configured via settings)
    /// 2. macOS Keychain (service: "Claude Code-credentials")
    /// 3. File: `~/.claude/.credentials.json`
    fn load_oauth_token(&self) -> Result<String, QuotaError> {
        // Priority 1: Manual token (configured via settings)
        if let Some(ref token) = self.manual_token {
            log::info!("[quota:claude] Using manually configured OAuth token");
            return Ok(token.clone());
        }

        // Priority 2: On macOS, try Keychain
        #[cfg(target_os = "macos")]
        {
            match Self::load_from_keychain() {
                Ok(token) => {
                    log::debug!("[quota:claude] Successfully loaded OAuth token from Keychain");
                    return Ok(token);
                }
                Err(e) => {
                    log::debug!("[quota:claude] Keychain lookup failed: {}, trying file fallback", e);
                }
            }
        }

        // Priority 2: On Windows, try Credential Manager
        #[cfg(target_os = "windows")]
        {
            match Self::load_from_windows_credential_manager() {
                Ok(token) => {
                    log::debug!("[quota:claude] Successfully loaded OAuth token from Windows Credential Manager");
                    return Ok(token);
                }
                Err(e) => {
                    log::debug!("[quota:claude] Windows Credential Manager lookup failed: {}, trying file fallback", e);
                }
            }
        }

        // Priority 3: Fall back to file-based credentials
        self.load_from_file()
    }

    /// Load OAuth token from macOS Keychain
    #[cfg(target_os = "macos")]
    fn load_from_keychain() -> Result<String, QuotaError> {
        use security_framework::passwords::get_generic_password;

        log::debug!("[quota:claude] Attempting to load OAuth token from macOS Keychain");

        // Get current username for the account
        let username = std::env::var("USER").unwrap_or_else(|_| "default".to_string());

        // Read from Keychain
        let password_bytes = get_generic_password(KEYCHAIN_SERVICE, &username).map_err(|e| {
            log::debug!("[quota:claude] Keychain access failed: {}", e);
            QuotaError::NotInstalled(format!(
                "Claude Code credentials not found in Keychain. Please run 'claude /login'."
            ))
        })?;

        // Convert to string
        let content = String::from_utf8(password_bytes.to_vec()).map_err(|e| {
            log::error!("[quota:claude] Keychain data is not valid UTF-8: {}", e);
            QuotaError::ParseError("Invalid credentials data in Keychain".to_string())
        })?;

        // Parse the JSON credentials
        Self::parse_credentials_json(&content)
    }

    /// Load OAuth token from Windows Credential Manager
    #[cfg(target_os = "windows")]
    fn load_from_windows_credential_manager() -> Result<String, QuotaError> {
        use keyring::Entry;

        log::debug!("[quota:claude] Attempting to load OAuth token from Windows Credential Manager");

        // Get current username for the account
        let username = std::env::var("USERNAME").unwrap_or_else(|_| "default".to_string());

        // Create keyring entry and get password
        let entry = Entry::new(CREDENTIAL_SERVICE, &username).map_err(|e| {
            log::debug!("[quota:claude] Failed to create keyring entry: {}", e);
            QuotaError::NotInstalled(format!(
                "Claude Code credentials not found in Windows Credential Manager. Please run 'claude /login'."
            ))
        })?;

        let content = entry.get_password().map_err(|e| {
            log::debug!("[quota:claude] Windows Credential Manager access failed: {}", e);
            QuotaError::NotInstalled(format!(
                "Claude Code credentials not found in Windows Credential Manager. Please run 'claude /login'."
            ))
        })?;

        // Parse the JSON credentials
        Self::parse_credentials_json(&content)
    }

    /// Load OAuth token from credentials file
    fn load_from_file(&self) -> Result<String, QuotaError> {
        log::debug!(
            "[quota:claude] Loading OAuth token from file: {:?}",
            self.credentials_path
        );

        // Check if file exists
        if !self.credentials_path.exists() {
            log::warn!(
                "[quota:claude] Credentials file not found: {:?}",
                self.credentials_path
            );
            return Err(QuotaError::NotInstalled(
                "Claude credentials file not found. Please log in to Claude Code.".to_string(),
            ));
        }

        // Read file content
        let content = std::fs::read_to_string(&self.credentials_path)?;

        // Parse the JSON credentials
        Self::parse_credentials_json(&content)
    }

    /// Parse credentials JSON and extract the access token
    fn parse_credentials_json(content: &str) -> Result<String, QuotaError> {
        let (token, _, _) = Self::parse_credentials_full(content)?;
        Ok(token)
    }

    /// Parse credentials JSON and extract full info (token, subscription_type, rate_limit_tier)
    fn parse_credentials_full(content: &str) -> Result<(String, Option<String>, Option<String>), QuotaError> {
        let credentials_file: CredentialsFile = serde_json::from_str(content).map_err(|e| {
            log::error!("[quota:claude] Failed to parse credentials: {}", e);
            QuotaError::ParseError(format!("Invalid credentials file format: {}", e))
        })?;

        // Extract the nested OAuth credentials
        let oauth_credentials = credentials_file.claude_ai_oauth.ok_or_else(|| {
            log::warn!("[quota:claude] No claudeAiOauth object in credentials");
            QuotaError::Unauthorized(
                "No OAuth credentials found. Please log in to Claude Code.".to_string(),
            )
        })?;

        // Extract access token
        let token = oauth_credentials.access_token.ok_or_else(|| {
            log::warn!("[quota:claude] No access token in credentials");
            QuotaError::Unauthorized(
                "No access token found. Please log in to Claude Code.".to_string(),
            )
        })?;

        if token.is_empty() {
            return Err(QuotaError::Unauthorized(
                "Access token is empty. Try 'claude setup-token' or re-login with 'claude /login'.".to_string(),
            ));
        }

        log::debug!("[quota:claude] Successfully parsed OAuth credentials");
        Ok((
            token,
            oauth_credentials.subscription_type,
            oauth_credentials.rate_limit_tier,
        ))
    }

    /// Load account info from credentials (subscription type, rate limit tier)
    fn load_account_info(&self) -> Result<AccountInfo, QuotaError> {
        // Priority 1: Manual token (no account info available)
        if self.manual_token.is_some() {
            return Ok(AccountInfo {
                email: None,
                plan: None,
                display_name: None,
                is_active: true,
                raw_data: None,
            });
        }

        // Priority 2: Try Keychain on macOS
        #[cfg(target_os = "macos")]
        {
            if let Ok(content) = Self::load_keychain_content() {
                if let Ok((_, subscription_type, _)) = Self::parse_credentials_full(&content) {
                    return Ok(AccountInfo {
                        email: None,
                        plan: subscription_type,
                        display_name: None,
                        is_active: true,
                        raw_data: None,
                    });
                }
            }
        }

        // Priority 3: File fallback
        if self.credentials_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&self.credentials_path) {
                if let Ok((_, subscription_type, _)) = Self::parse_credentials_full(&content) {
                    return Ok(AccountInfo {
                        email: None,
                        plan: subscription_type,
                        display_name: None,
                        is_active: true,
                        raw_data: None,
                    });
                }
            }
        }

        Ok(AccountInfo {
            email: None,
            plan: None,
            display_name: None,
            is_active: false,
            raw_data: None,
        })
    }

    /// Load raw content from macOS Keychain
    #[cfg(target_os = "macos")]
    fn load_keychain_content() -> Result<String, QuotaError> {
        use security_framework::passwords::get_generic_password;

        let username = std::env::var("USER").unwrap_or_else(|_| "default".to_string());
        let password_bytes = get_generic_password(KEYCHAIN_SERVICE, &username).map_err(|e| {
            QuotaError::NotInstalled(format!("Keychain access failed: {}", e))
        })?;

        String::from_utf8(password_bytes.to_vec()).map_err(|e| {
            QuotaError::ParseError(format!("Invalid UTF-8 in Keychain: {}", e))
        })
    }

    /// Call the Anthropic profile API to get account info
    async fn call_profile_api(&self, token: &str) -> Result<AccountInfo, QuotaError> {
        log::info!("[quota:claude] Fetching profile from Anthropic API");

        let response = self
            .client
            .get(PROFILE_API_URL)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json")
            .header("anthropic-beta", OAUTH_BETA_HEADER)
            .header("User-Agent", "Recap")
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            log::warn!("[quota:claude] Profile API failed: HTTP {}", status);
            return Err(QuotaError::ApiError(format!("Profile API returned HTTP {}", status)));
        }

        let profile: OAuthProfileResponse = response.json().await.map_err(|e| {
            log::error!("[quota:claude] Failed to parse profile response: {}", e);
            QuotaError::ParseError(format!("Invalid profile response: {}", e))
        })?;

        // Extract account info
        let account = profile.account.as_ref();
        let org = profile.organization.as_ref();

        // Determine plan from organization type or account flags
        let plan = org
            .and_then(|o| o.organization_type.clone())
            .or_else(|| {
                account.and_then(|a| {
                    if a.has_claude_max == Some(true) {
                        Some("max".to_string())
                    } else if a.has_claude_pro == Some(true) {
                        Some("pro".to_string())
                    } else {
                        None
                    }
                })
            });

        // Subscription status
        let is_active = org
            .and_then(|o| o.subscription_status.as_ref())
            .map(|s| s == "active")
            .unwrap_or(true);

        // Store raw response for debugging
        let raw_data = serde_json::to_string(&profile).ok();

        Ok(AccountInfo {
            email: account.and_then(|a| a.email.clone()),
            display_name: account.and_then(|a| a.display_name.clone()),
            plan,
            is_active,
            raw_data,
        })
    }

    /// Call the Anthropic usage API
    async fn call_usage_api(&self, token: &str) -> Result<OAuthUsageResponse, QuotaError> {
        log::info!("[quota:claude] Fetching quota from Anthropic API");

        let response = self
            .client
            .get(USAGE_API_URL)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("anthropic-beta", OAUTH_BETA_HEADER)
            .header("User-Agent", "Recap")
            .send()
            .await?;

        let status = response.status();
        log::debug!("[quota:claude] API response status: {}", status);

        if status == 401 || status == 403 {
            log::warn!("[quota:claude] Authentication failed: HTTP {}", status);
            return Err(QuotaError::Unauthorized(format!(
                "API authentication failed (HTTP {}). Token expired. Try 'claude setup-token' for a long-lived token, or re-login with 'claude /login'.",
                status
            )));
        }

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            log::error!(
                "[quota:claude] API error: HTTP {} - {}",
                status,
                error_body
            );
            return Err(QuotaError::ApiError(format!(
                "API returned HTTP {}: {}",
                status, error_body
            )));
        }

        let response_text = response.text().await?;
        log::debug!(
            "[quota:claude] API response body: {}",
            &response_text[..std::cmp::min(200, response_text.len())]
        );

        let usage: OAuthUsageResponse = serde_json::from_str(&response_text).map_err(|e| {
            log::error!("[quota:claude] Failed to parse API response: {}", e);
            QuotaError::ParseError(format!("Invalid API response: {}", e))
        })?;

        log::info!("[quota:claude] Successfully fetched quota data");
        Ok(usage)
    }

    /// Convert API response to quota snapshots
    fn response_to_snapshots(&self, response: OAuthUsageResponse) -> Vec<QuotaSnapshot> {
        let mut snapshots = Vec::new();

        // Process 5-hour window
        if let Some(window) = response.five_hour {
            if let Some(snapshot) = self.window_to_snapshot(window, QuotaWindowType::FiveHour, None)
            {
                snapshots.push(snapshot);
            }
        }

        // Process 7-day window (all models)
        if let Some(window) = response.seven_day {
            if let Some(snapshot) = self.window_to_snapshot(window, QuotaWindowType::SevenDay, None)
            {
                snapshots.push(snapshot);
            }
        }

        // Process 7-day Opus window
        if let Some(window) = response.seven_day_opus {
            if let Some(snapshot) =
                self.window_to_snapshot(window, QuotaWindowType::SevenDayOpus, Some("opus"))
            {
                snapshots.push(snapshot);
            }
        }

        // Process 7-day Sonnet window
        if let Some(window) = response.seven_day_sonnet {
            if let Some(snapshot) =
                self.window_to_snapshot(window, QuotaWindowType::SevenDaySonnet, Some("sonnet"))
            {
                snapshots.push(snapshot);
            }
        }

        // Add extra credits info to the most relevant snapshot (or create one)
        if let Some(extra) = response.extra_usage {
            if extra.is_enabled == Some(true) {
                if let (Some(used), Some(limit)) = (extra.used_credits, extra.monthly_limit) {
                    // If we have a 7-day snapshot, attach extra credits to it
                    if let Some(snapshot) = snapshots
                        .iter_mut()
                        .find(|s| s.window_type == QuotaWindowType::SevenDay)
                    {
                        snapshot.extra_credits = Some(super::types::ExtraCredits { used, limit });
                    } else if !snapshots.is_empty() {
                        // Attach to first snapshot
                        snapshots[0].extra_credits =
                            Some(super::types::ExtraCredits { used, limit });
                    }
                }
            }
        }

        log::debug!(
            "[quota:claude] Converted response to {} snapshots",
            snapshots.len()
        );
        snapshots
    }

    /// Convert a single usage window to a quota snapshot
    fn window_to_snapshot(
        &self,
        window: UsageWindow,
        window_type: QuotaWindowType,
        model: Option<&str>,
    ) -> Option<QuotaSnapshot> {
        // Skip if no utilization data
        let utilization = window.utilization?;

        log::info!(
            "[quota:claude] Window {:?} raw utilization value: {}",
            window_type,
            utilization
        );

        // The API appears to return utilization as a percentage (28.0 for 28%)
        // NOT as a ratio (0.28 for 28%), so we should NOT multiply by 100
        let used_percent = utilization;

        log::info!(
            "[quota:claude] Window {:?} used_percent: {}%",
            window_type,
            used_percent
        );

        let mut snapshot =
            QuotaSnapshot::new(&self.user_id, QuotaProviderType::Claude, window_type, used_percent);

        // Add model if specified
        if let Some(model_name) = model {
            snapshot = snapshot.with_model(model_name);
        }

        // Parse and add reset time
        if let Some(resets_at_str) = window.resets_at {
            if let Ok(resets_at) = DateTime::parse_from_rfc3339(&resets_at_str) {
                snapshot = snapshot.with_resets_at(resets_at.with_timezone(&Utc));
            } else {
                log::warn!(
                    "[quota:claude] Failed to parse resets_at: {}",
                    resets_at_str
                );
            }
        }

        Some(snapshot)
    }
}

impl Default for ClaudeQuotaProvider {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// QuotaProvider Implementation
// ============================================================================

#[async_trait]
impl QuotaProvider for ClaudeQuotaProvider {
    fn provider_id(&self) -> &'static str {
        "claude"
    }

    fn display_name(&self) -> &'static str {
        "Claude Code"
    }

    async fn fetch_quota(&self) -> Result<Vec<QuotaSnapshot>, QuotaError> {
        // Load OAuth token
        let token = self.load_oauth_token()?;

        // Call API
        let response = self.call_usage_api(&token).await?;

        // Convert to snapshots
        let snapshots = self.response_to_snapshots(response);

        if snapshots.is_empty() {
            log::warn!("[quota:claude] No quota data returned from API");
        }

        Ok(snapshots)
    }

    async fn is_available(&self) -> bool {
        // Try to load OAuth token (from Keychain on macOS, or file fallback)
        match self.load_oauth_token() {
            Ok(_) => {
                log::debug!("[quota:claude] Provider is available");
                true
            }
            Err(e) => {
                log::debug!("[quota:claude] Provider not available: {}", e);
                false
            }
        }
    }

    async fn get_account_info(&self) -> Result<Option<AccountInfo>, QuotaError> {
        // Try to load OAuth token first
        let token = match self.load_oauth_token() {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };

        // Call the profile API to get account info
        match self.call_profile_api(&token).await {
            Ok(info) => Ok(Some(info)),
            Err(e) => {
                log::warn!("[quota:claude] Failed to get account info from profile API: {}", e);
                // Fall back to local credentials
                match self.load_account_info() {
                    Ok(info) if info.plan.is_some() => Ok(Some(info)),
                    _ => Ok(None),
                }
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_usage_response() {
        let json = r#"{
            "five_hour": {
                "utilization": 0.25,
                "resets_at": "2024-01-15T12:30:00Z"
            },
            "seven_day": {
                "utilization": 0.75,
                "resets_at": "2024-01-20T00:00:00Z"
            }
        }"#;

        let response: OAuthUsageResponse = serde_json::from_str(json).unwrap();

        assert!(response.five_hour.is_some());
        let five_hour = response.five_hour.unwrap();
        assert_eq!(five_hour.utilization, Some(0.25));
        assert_eq!(
            five_hour.resets_at,
            Some("2024-01-15T12:30:00Z".to_string())
        );

        assert!(response.seven_day.is_some());
        let seven_day = response.seven_day.unwrap();
        assert_eq!(seven_day.utilization, Some(0.75));

        assert!(response.seven_day_opus.is_none());
        assert!(response.seven_day_sonnet.is_none());
        assert!(response.extra_usage.is_none());
    }

    #[test]
    fn test_parse_with_extra_usage() {
        let json = r#"{
            "five_hour": {
                "utilization": 0.5
            },
            "seven_day": {
                "utilization": 0.8
            },
            "extra_usage": {
                "is_enabled": true,
                "used_credits": 15.50,
                "monthly_limit": 100.0,
                "currency": "USD"
            }
        }"#;

        let response: OAuthUsageResponse = serde_json::from_str(json).unwrap();

        assert!(response.extra_usage.is_some());
        let extra = response.extra_usage.unwrap();
        assert_eq!(extra.is_enabled, Some(true));
        assert_eq!(extra.used_credits, Some(15.50));
        assert_eq!(extra.monthly_limit, Some(100.0));
        assert_eq!(extra.currency, Some("USD".to_string()));
    }

    #[test]
    fn test_parse_with_model_specific_windows() {
        let json = r#"{
            "five_hour": {
                "utilization": 0.3
            },
            "seven_day": {
                "utilization": 0.5
            },
            "seven_day_opus": {
                "utilization": 0.9,
                "resets_at": "2024-01-22T00:00:00Z"
            },
            "seven_day_sonnet": {
                "utilization": 0.2
            }
        }"#;

        let response: OAuthUsageResponse = serde_json::from_str(json).unwrap();

        assert!(response.seven_day_opus.is_some());
        let opus = response.seven_day_opus.unwrap();
        assert_eq!(opus.utilization, Some(0.9));

        assert!(response.seven_day_sonnet.is_some());
        let sonnet = response.seven_day_sonnet.unwrap();
        assert_eq!(sonnet.utilization, Some(0.2));
    }

    #[test]
    fn test_response_to_snapshots() {
        let provider = ClaudeQuotaProvider::with_credentials_path(PathBuf::from("/tmp/test"))
            .with_user_id("test_user");

        // The API returns utilization as percentage values (25.0, 75.0), not ratios (0.25, 0.75)
        let response = OAuthUsageResponse {
            five_hour: Some(UsageWindow {
                utilization: Some(25.0),
                resets_at: Some("2024-01-15T12:30:00Z".to_string()),
            }),
            seven_day: Some(UsageWindow {
                utilization: Some(75.0),
                resets_at: None,
            }),
            seven_day_opus: None,
            seven_day_sonnet: None,
            extra_usage: None,
        };

        let snapshots = provider.response_to_snapshots(response);

        assert_eq!(snapshots.len(), 2);

        // Check 5-hour snapshot
        let five_hour = &snapshots[0];
        assert_eq!(five_hour.window_type, QuotaWindowType::FiveHour);
        assert!((five_hour.used_percent - 25.0).abs() < 0.001);
        assert!(five_hour.resets_at.is_some());
        assert_eq!(five_hour.user_id, "test_user");

        // Check 7-day snapshot
        let seven_day = &snapshots[1];
        assert_eq!(seven_day.window_type, QuotaWindowType::SevenDay);
        assert!((seven_day.used_percent - 75.0).abs() < 0.001);
        assert!(seven_day.resets_at.is_none());
    }

    #[test]
    fn test_response_with_extra_credits() {
        let provider = ClaudeQuotaProvider::with_credentials_path(PathBuf::from("/tmp/test"));

        let response = OAuthUsageResponse {
            five_hour: None,
            seven_day: Some(UsageWindow {
                utilization: Some(50.0),  // 50%
                resets_at: None,
            }),
            seven_day_opus: None,
            seven_day_sonnet: None,
            extra_usage: Some(ExtraUsage {
                is_enabled: Some(true),
                used_credits: Some(25.0),
                monthly_limit: Some(100.0),
                currency: Some("USD".to_string()),
            }),
        };

        let snapshots = provider.response_to_snapshots(response);

        assert_eq!(snapshots.len(), 1);
        assert!((snapshots[0].used_percent - 50.0).abs() < 0.001);
        assert!(snapshots[0].extra_credits.is_some());

        let extra = snapshots[0].extra_credits.as_ref().unwrap();
        assert!((extra.used - 25.0).abs() < 0.001);
        assert!((extra.limit - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_credentials() {
        let json = r#"{
            "claudeAiOauth": {
                "accessToken": "test_token_123",
                "refreshToken": "refresh_token_456",
                "expiresAt": 1705320000000,
                "scopes": ["read", "write"],
                "rateLimitTier": "max"
            }
        }"#;

        let creds: CredentialsFile = serde_json::from_str(json).unwrap();
        assert!(creds.claude_ai_oauth.is_some());
        let oauth = creds.claude_ai_oauth.unwrap();
        assert_eq!(oauth.access_token, Some("test_token_123".to_string()));
        assert_eq!(oauth.refresh_token, Some("refresh_token_456".to_string()));
        assert_eq!(oauth.expires_at, Some(1705320000000));
        assert_eq!(oauth.scopes, Some(vec!["read".to_string(), "write".to_string()]));
        assert_eq!(oauth.rate_limit_tier, Some("max".to_string()));
    }

    #[test]
    fn test_parse_credentials_missing_token() {
        let json = r#"{
            "claudeAiOauth": {
                "expiresAt": 1705320000000
            }
        }"#;

        let creds: CredentialsFile = serde_json::from_str(json).unwrap();
        assert!(creds.claude_ai_oauth.is_some());
        let oauth = creds.claude_ai_oauth.unwrap();
        assert!(oauth.access_token.is_none());
    }

    #[test]
    fn test_parse_credentials_missing_oauth_object() {
        let json = r#"{}"#;

        let creds: CredentialsFile = serde_json::from_str(json).unwrap();
        assert!(creds.claude_ai_oauth.is_none());
    }

    #[test]
    fn test_provider_id() {
        let provider = ClaudeQuotaProvider::new();
        assert_eq!(provider.provider_id(), "claude");
        assert_eq!(provider.display_name(), "Claude Code");
    }

    #[test]
    fn test_default_credentials_path() {
        let path = ClaudeQuotaProvider::default_credentials_path();
        assert!(path.ends_with(".claude/.credentials.json"));
    }

    #[test]
    fn test_empty_response() {
        let provider = ClaudeQuotaProvider::with_credentials_path(PathBuf::from("/tmp/test"));

        let response = OAuthUsageResponse {
            five_hour: None,
            seven_day: None,
            seven_day_opus: None,
            seven_day_sonnet: None,
            extra_usage: None,
        };

        let snapshots = provider.response_to_snapshots(response);
        assert!(snapshots.is_empty());
    }

    #[test]
    fn test_window_without_utilization() {
        let provider = ClaudeQuotaProvider::with_credentials_path(PathBuf::from("/tmp/test"));

        let response = OAuthUsageResponse {
            five_hour: Some(UsageWindow {
                utilization: None, // Missing utilization
                resets_at: Some("2024-01-15T12:00:00Z".to_string()),
            }),
            seven_day: None,
            seven_day_opus: None,
            seven_day_sonnet: None,
            extra_usage: None,
        };

        let snapshots = provider.response_to_snapshots(response);
        // Should skip windows without utilization
        assert!(snapshots.is_empty());
    }
}
