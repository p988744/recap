//! Quota tracking types
//!
//! Types for tracking API quota usage across different providers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// Provider Types
// ============================================================================

/// Type of quota provider
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaProviderType {
    /// Claude Code (Anthropic)
    Claude,
    /// Antigravity (Google/Gemini Code)
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

impl std::str::FromStr for QuotaProviderType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" | "claude_code" => Ok(QuotaProviderType::Claude),
            "antigravity" | "gemini" => Ok(QuotaProviderType::Antigravity),
            _ => Err(format!("Unknown provider type: {}", s)),
        }
    }
}

// ============================================================================
// Window Types
// ============================================================================

/// Type of quota window/period
///
/// Different providers may have different quota windows:
/// - Claude: 5-hour rolling window for rate limits, 7-day for usage
/// - Antigravity: Monthly quotas
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaWindowType {
    /// 5-hour rolling window (Claude rate limit)
    FiveHour,
    /// 7-day rolling window (Claude usage - all models)
    SevenDay,
    /// 7-day rolling window for Opus models specifically
    SevenDayOpus,
    /// 7-day rolling window for Sonnet models specifically
    SevenDaySonnet,
    /// Monthly quota period
    Monthly,
}

impl std::fmt::Display for QuotaWindowType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuotaWindowType::FiveHour => write!(f, "5_hour"),
            QuotaWindowType::SevenDay => write!(f, "7_day"),
            QuotaWindowType::SevenDayOpus => write!(f, "7_day_opus"),
            QuotaWindowType::SevenDaySonnet => write!(f, "7_day_sonnet"),
            QuotaWindowType::Monthly => write!(f, "monthly"),
        }
    }
}

impl std::str::FromStr for QuotaWindowType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "5_hour" | "five_hour" | "5hour" => Ok(QuotaWindowType::FiveHour),
            "7_day" | "seven_day" | "7day" => Ok(QuotaWindowType::SevenDay),
            "7_day_opus" | "seven_day_opus" => Ok(QuotaWindowType::SevenDayOpus),
            "7_day_sonnet" | "seven_day_sonnet" => Ok(QuotaWindowType::SevenDaySonnet),
            "monthly" | "month" => Ok(QuotaWindowType::Monthly),
            _ => Err(format!("Unknown window type: {}", s)),
        }
    }
}

// ============================================================================
// Snapshot Types
// ============================================================================

/// A point-in-time snapshot of quota usage
///
/// This represents a single measurement of quota usage at a specific time.
/// Multiple snapshots over time allow tracking usage trends and predicting
/// when quota limits might be reached.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaSnapshot {
    /// Unique identifier for this snapshot
    pub id: String,
    /// User who owns this quota
    pub user_id: String,
    /// Provider this quota is for
    pub provider: QuotaProviderType,
    /// Model this quota applies to (if model-specific)
    pub model: Option<String>,
    /// Type of quota window
    pub window_type: QuotaWindowType,
    /// Percentage of quota used (0.0 - 100.0)
    pub used_percent: f64,
    /// When the quota resets
    pub resets_at: Option<DateTime<Utc>>,
    /// Extra credits used (if applicable)
    pub extra_credits: Option<ExtraCredits>,
    /// Raw API response for debugging
    pub raw_response: Option<String>,
    /// When this snapshot was taken
    pub created_at: DateTime<Utc>,
}

impl QuotaSnapshot {
    /// Create a new quota snapshot
    pub fn new(
        user_id: impl Into<String>,
        provider: QuotaProviderType,
        window_type: QuotaWindowType,
        used_percent: f64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.into(),
            provider,
            model: None,
            window_type,
            used_percent,
            resets_at: None,
            extra_credits: None,
            raw_response: None,
            created_at: Utc::now(),
        }
    }

    /// Set the model for this snapshot
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the reset time for this snapshot
    pub fn with_resets_at(mut self, resets_at: DateTime<Utc>) -> Self {
        self.resets_at = Some(resets_at);
        self
    }

    /// Set extra credits info
    pub fn with_extra_credits(mut self, used: f64, limit: f64) -> Self {
        self.extra_credits = Some(ExtraCredits { used, limit });
        self
    }

    /// Set raw response for debugging
    pub fn with_raw_response(mut self, raw: impl Into<String>) -> Self {
        self.raw_response = Some(raw.into());
        self
    }
}

/// Extra credits information (for plans that include bonus credits)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraCredits {
    /// Credits used
    pub used: f64,
    /// Credit limit
    pub limit: f64,
}

// ============================================================================
// Account Info
// ============================================================================

/// Account information from a provider
///
/// Contains user profile and subscription details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    /// User email
    pub email: Option<String>,
    /// User display name
    pub display_name: Option<String>,
    /// Plan/tier name (e.g., "pro", "free", "max")
    pub plan: Option<String>,
    /// Whether the account is active/valid
    pub is_active: bool,
    /// Raw account data for debugging
    pub raw_data: Option<String>,
}

impl AccountInfo {
    /// Create account info with just active status
    pub fn new(is_active: bool) -> Self {
        Self {
            email: None,
            display_name: None,
            plan: None,
            is_active,
            raw_data: None,
        }
    }

    /// Create account info with email and plan
    pub fn with_details(email: impl Into<String>, plan: impl Into<String>) -> Self {
        Self {
            email: Some(email.into()),
            display_name: None,
            plan: Some(plan.into()),
            is_active: true,
            raw_data: None,
        }
    }
}

// ============================================================================
// Alert Level
// ============================================================================

/// Alert level for quota usage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertLevel {
    /// Normal usage, no alert
    Normal,
    /// Usage is approaching limit (e.g., >80%)
    Warning,
    /// Usage is at or near limit (e.g., >95%)
    Critical,
}

impl AlertLevel {
    /// Determine alert level based on usage percentage and thresholds
    pub fn from_usage(
        used_percent: f64,
        warning_threshold: f64,
        critical_threshold: f64,
    ) -> Self {
        if used_percent >= critical_threshold {
            AlertLevel::Critical
        } else if used_percent >= warning_threshold {
            AlertLevel::Warning
        } else {
            AlertLevel::Normal
        }
    }
}

// ============================================================================
// Settings
// ============================================================================

/// Settings for quota tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaSettings {
    /// How often to poll for quota updates (in minutes)
    pub interval_minutes: u32,
    /// Percentage at which to show warning (0-100)
    pub warning_threshold: f64,
    /// Percentage at which to show critical alert (0-100)
    pub critical_threshold: f64,
    /// Whether to show quota in menu bar / tray
    pub show_in_tray: bool,
    /// Whether quota tracking is enabled
    pub enabled: bool,
}

impl Default for QuotaSettings {
    fn default() -> Self {
        Self {
            interval_minutes: 15,
            warning_threshold: 80.0,
            critical_threshold: 95.0,
            show_in_tray: true,
            enabled: true,
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
    fn test_provider_type_display() {
        assert_eq!(QuotaProviderType::Claude.to_string(), "claude");
        assert_eq!(QuotaProviderType::Antigravity.to_string(), "antigravity");
    }

    #[test]
    fn test_provider_type_from_str() {
        assert_eq!(
            "claude".parse::<QuotaProviderType>().unwrap(),
            QuotaProviderType::Claude
        );
        assert_eq!(
            "antigravity".parse::<QuotaProviderType>().unwrap(),
            QuotaProviderType::Antigravity
        );
        assert_eq!(
            "claude_code".parse::<QuotaProviderType>().unwrap(),
            QuotaProviderType::Claude
        );
    }

    #[test]
    fn test_window_type_display() {
        assert_eq!(QuotaWindowType::FiveHour.to_string(), "5_hour");
        assert_eq!(QuotaWindowType::SevenDay.to_string(), "7_day");
        assert_eq!(QuotaWindowType::SevenDayOpus.to_string(), "7_day_opus");
        assert_eq!(QuotaWindowType::Monthly.to_string(), "monthly");
    }

    #[test]
    fn test_window_type_from_str() {
        assert_eq!(
            "5_hour".parse::<QuotaWindowType>().unwrap(),
            QuotaWindowType::FiveHour
        );
        assert_eq!(
            "7_day".parse::<QuotaWindowType>().unwrap(),
            QuotaWindowType::SevenDay
        );
        assert_eq!(
            "monthly".parse::<QuotaWindowType>().unwrap(),
            QuotaWindowType::Monthly
        );
    }

    #[test]
    fn test_quota_snapshot_builder() {
        let snapshot = QuotaSnapshot::new("user1", QuotaProviderType::Claude, QuotaWindowType::FiveHour, 75.5)
            .with_model("claude-sonnet-4")
            .with_extra_credits(10.0, 100.0);

        assert_eq!(snapshot.user_id, "user1");
        assert_eq!(snapshot.provider, QuotaProviderType::Claude);
        assert_eq!(snapshot.window_type, QuotaWindowType::FiveHour);
        assert_eq!(snapshot.used_percent, 75.5);
        assert_eq!(snapshot.model, Some("claude-sonnet-4".to_string()));
        assert!(snapshot.extra_credits.is_some());
    }

    #[test]
    fn test_alert_level_from_usage() {
        let settings = QuotaSettings::default();

        assert_eq!(
            AlertLevel::from_usage(50.0, settings.warning_threshold, settings.critical_threshold),
            AlertLevel::Normal
        );
        assert_eq!(
            AlertLevel::from_usage(85.0, settings.warning_threshold, settings.critical_threshold),
            AlertLevel::Warning
        );
        assert_eq!(
            AlertLevel::from_usage(98.0, settings.warning_threshold, settings.critical_threshold),
            AlertLevel::Critical
        );
    }

    #[test]
    fn test_quota_settings_default() {
        let settings = QuotaSettings::default();
        assert_eq!(settings.interval_minutes, 15);
        assert_eq!(settings.warning_threshold, 80.0);
        assert_eq!(settings.critical_threshold, 95.0);
        assert!(settings.show_in_tray);
        assert!(settings.enabled);
    }

    #[test]
    fn test_account_info_constructors() {
        let info1 = AccountInfo::new(true);
        assert!(info1.is_active);
        assert!(info1.email.is_none());

        let info2 = AccountInfo::with_details("test@example.com", "pro");
        assert!(info2.is_active);
        assert_eq!(info2.email, Some("test@example.com".to_string()));
        assert_eq!(info2.plan, Some("pro".to_string()));
    }
}
