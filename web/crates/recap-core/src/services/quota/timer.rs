//! Quota polling timer
//!
//! Background timer that periodically fetches quota data from all providers
//! and stores snapshots for history tracking.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    QuotaPollingService                       │
//! │                                                             │
//! │  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
//! │  │ Config       │    │ Timer Loop   │    │ Alert State  │  │
//! │  │ - interval   │    │ - poll()     │    │ - last_alert │  │
//! │  │ - thresholds │    │ - sleep()    │    │ - provider   │  │
//! │  └──────────────┘    └──────────────┘    └──────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//!           │                    │                    │
//!           ▼                    ▼                    ▼
//!    ┌──────────┐         ┌──────────┐         ┌──────────┐
//!    │ Providers│         │QuotaStore│         │Callbacks │
//!    │ (Claude) │         │ (SQLite) │         │(Tray/Ntf)│
//!    └──────────┘         └──────────┘         └──────────┘
//! ```
//!
//! # Features
//!
//! - Configurable polling interval (minimum 5 minutes, default 15 minutes)
//! - Threshold-based alerts (warning at 80%, critical at 95%)
//! - Deduplication of alerts (only notify once per threshold crossing)
//! - Graceful shutdown via cancellation token
//! - Tray title updates with latest quota percentage

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::types::{AlertLevel, QuotaProviderType};

// ============================================================================
// Constants
// ============================================================================

/// Minimum polling interval in minutes
pub const MIN_INTERVAL_MINUTES: u32 = 5;

/// Default polling interval in minutes
pub const DEFAULT_INTERVAL_MINUTES: u32 = 15;

/// Default warning threshold (percentage)
pub const DEFAULT_WARNING_THRESHOLD: f64 = 80.0;

/// Default critical threshold (percentage)
pub const DEFAULT_CRITICAL_THRESHOLD: f64 = 95.0;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for quota polling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaPollingConfig {
    /// Whether polling is enabled
    pub enabled: bool,
    /// Polling interval in minutes (minimum 5)
    pub interval_minutes: u32,
    /// Warning threshold percentage (0-100)
    pub warning_threshold: f64,
    /// Critical threshold percentage (0-100)
    pub critical_threshold: f64,
    /// Whether to show notifications on threshold crossing
    pub notify_on_threshold: bool,
    /// Whether to update tray title with quota percentage
    pub update_tray: bool,
}

impl Default for QuotaPollingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_minutes: DEFAULT_INTERVAL_MINUTES,
            warning_threshold: DEFAULT_WARNING_THRESHOLD,
            critical_threshold: DEFAULT_CRITICAL_THRESHOLD,
            notify_on_threshold: true,
            update_tray: true,
        }
    }
}

impl QuotaPollingConfig {
    /// Create a new configuration with the specified interval
    pub fn with_interval(interval_minutes: u32) -> Self {
        Self {
            interval_minutes: interval_minutes.max(MIN_INTERVAL_MINUTES),
            ..Default::default()
        }
    }

    /// Validate and normalize the configuration
    pub fn validate(&self) -> Self {
        Self {
            enabled: self.enabled,
            interval_minutes: self.interval_minutes.max(MIN_INTERVAL_MINUTES),
            warning_threshold: self.warning_threshold.clamp(0.0, 100.0),
            critical_threshold: self.critical_threshold.clamp(0.0, 100.0),
            notify_on_threshold: self.notify_on_threshold,
            update_tray: self.update_tray,
        }
    }
}

// ============================================================================
// Alert State
// ============================================================================

/// State for tracking alert levels to prevent spam
#[derive(Debug, Clone, Default)]
pub struct AlertState {
    /// Last alert level per provider and window type
    last_alerts: HashMap<(QuotaProviderType, String), AlertLevel>,
}

impl AlertState {
    /// Create a new alert state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if we should send an alert for this usage level
    ///
    /// Returns `Some(AlertLevel)` if we should send an alert, `None` otherwise.
    /// Only sends alerts when crossing a threshold (e.g., Normal -> Warning).
    pub fn should_alert(
        &mut self,
        provider: QuotaProviderType,
        window_type: &str,
        current_percent: f64,
        warning_threshold: f64,
        critical_threshold: f64,
    ) -> Option<AlertLevel> {
        let key = (provider, window_type.to_string());
        let current_level =
            AlertLevel::from_usage(current_percent, warning_threshold, critical_threshold);
        let last_level = self.last_alerts.get(&key).copied().unwrap_or(AlertLevel::Normal);

        // Update stored level
        self.last_alerts.insert(key, current_level);

        // Only alert if level increased (got worse)
        match (last_level, current_level) {
            (AlertLevel::Normal, AlertLevel::Warning) => Some(AlertLevel::Warning),
            (AlertLevel::Normal, AlertLevel::Critical) => Some(AlertLevel::Critical),
            (AlertLevel::Warning, AlertLevel::Critical) => Some(AlertLevel::Critical),
            _ => None,
        }
    }

    /// Reset alert state for a provider
    pub fn reset(&mut self, provider: QuotaProviderType) {
        self.last_alerts
            .retain(|key, _| key.0 != provider);
    }

    /// Clear all alert state
    pub fn clear(&mut self) {
        self.last_alerts.clear();
    }
}

// ============================================================================
// Polling Service Status
// ============================================================================

/// Status of the quota polling service
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuotaPollingStatus {
    /// Whether the service is currently running
    pub is_running: bool,
    /// Whether a poll is currently in progress
    pub is_polling: bool,
    /// Last poll timestamp (ISO 8601)
    pub last_poll_at: Option<String>,
    /// Next scheduled poll timestamp (ISO 8601)
    pub next_poll_at: Option<String>,
    /// Last error message (if any)
    pub last_error: Option<String>,
    /// Current quota percentages by provider
    pub current_quotas: HashMap<String, f64>,
}

// ============================================================================
// Polling Service State (Internal)
// ============================================================================

/// Internal state for the polling service
#[derive(Debug, Default)]
pub struct QuotaPollingState {
    /// Current configuration
    pub config: QuotaPollingConfig,
    /// Alert state for deduplication
    pub alert_state: AlertState,
    /// Current status
    pub status: QuotaPollingStatus,
    /// Whether the service is running
    pub is_running: bool,
}

impl QuotaPollingState {
    /// Create a new polling state with the given configuration
    pub fn new(config: QuotaPollingConfig) -> Self {
        Self {
            config: config.validate(),
            alert_state: AlertState::new(),
            status: QuotaPollingStatus::default(),
            is_running: false,
        }
    }

    /// Update the configuration
    pub fn update_config(&mut self, config: QuotaPollingConfig) {
        self.config = config.validate();
    }

    /// Get the polling interval in seconds
    pub fn interval_secs(&self) -> u64 {
        self.config.interval_minutes as u64 * 60
    }

    /// Mark as started
    pub fn start(&mut self) {
        self.is_running = true;
        self.status.is_running = true;
        self.update_next_poll_time();
    }

    /// Mark as stopped
    pub fn stop(&mut self) {
        self.is_running = false;
        self.status.is_running = false;
        self.status.next_poll_at = None;
    }

    /// Mark poll as starting
    pub fn begin_poll(&mut self) {
        self.status.is_polling = true;
    }

    /// Mark poll as complete
    pub fn complete_poll(&mut self, error: Option<String>) {
        self.status.is_polling = false;
        self.status.last_poll_at = Some(chrono::Utc::now().to_rfc3339());
        self.status.last_error = error;
        self.update_next_poll_time();
    }

    /// Update a provider's quota percentage
    pub fn update_quota(&mut self, provider: &str, percent: f64) {
        self.status
            .current_quotas
            .insert(provider.to_string(), percent);
    }

    /// Calculate and set the next poll time
    fn update_next_poll_time(&mut self) {
        if self.is_running {
            let next = chrono::Utc::now()
                + chrono::Duration::seconds(self.interval_secs() as i64);
            self.status.next_poll_at = Some(next.to_rfc3339());
        }
    }
}

/// Shared state wrapper for thread-safe access
pub type SharedPollingState = Arc<RwLock<QuotaPollingState>>;

/// Create a new shared polling state
pub fn create_shared_state(config: QuotaPollingConfig) -> SharedPollingState {
    Arc::new(RwLock::new(QuotaPollingState::new(config)))
}

// ============================================================================
// Callback Types
// ============================================================================

/// Callback for updating the tray title
pub type TrayUpdateCallback = Box<dyn Fn(Option<f64>, Option<f64>) + Send + Sync>;

/// Callback for sending notifications
pub type NotificationCallback = Box<dyn Fn(AlertLevel, &str, &str, f64) + Send + Sync>;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Configuration Tests
    // =========================================================================

    #[test]
    fn test_default_config() {
        let config = QuotaPollingConfig::default();
        assert!(config.enabled);
        assert_eq!(config.interval_minutes, DEFAULT_INTERVAL_MINUTES);
        assert_eq!(config.warning_threshold, DEFAULT_WARNING_THRESHOLD);
        assert_eq!(config.critical_threshold, DEFAULT_CRITICAL_THRESHOLD);
        assert!(config.notify_on_threshold);
        assert!(config.update_tray);
    }

    #[test]
    fn test_config_with_interval() {
        let config = QuotaPollingConfig::with_interval(30);
        assert_eq!(config.interval_minutes, 30);
    }

    #[test]
    fn test_config_enforces_minimum_interval() {
        let config = QuotaPollingConfig::with_interval(1);
        assert_eq!(config.interval_minutes, MIN_INTERVAL_MINUTES);
    }

    #[test]
    fn test_config_validate() {
        let config = QuotaPollingConfig {
            enabled: true,
            interval_minutes: 2, // Below minimum
            warning_threshold: 150.0, // Above 100
            critical_threshold: -10.0, // Below 0
            notify_on_threshold: true,
            update_tray: true,
        };

        let validated = config.validate();
        assert_eq!(validated.interval_minutes, MIN_INTERVAL_MINUTES);
        assert_eq!(validated.warning_threshold, 100.0);
        assert_eq!(validated.critical_threshold, 0.0);
    }

    // =========================================================================
    // Alert State Tests
    // =========================================================================

    #[test]
    fn test_alert_state_new() {
        let state = AlertState::new();
        assert!(state.last_alerts.is_empty());
    }

    #[test]
    fn test_alert_state_normal_to_warning() {
        let mut state = AlertState::new();
        let result = state.should_alert(
            QuotaProviderType::Claude,
            "5_hour",
            85.0, // Current usage
            80.0, // Warning threshold
            95.0, // Critical threshold
        );
        assert_eq!(result, Some(AlertLevel::Warning));
    }

    #[test]
    fn test_alert_state_normal_to_critical() {
        let mut state = AlertState::new();
        let result = state.should_alert(
            QuotaProviderType::Claude,
            "5_hour",
            98.0, // Current usage
            80.0, // Warning threshold
            95.0, // Critical threshold
        );
        assert_eq!(result, Some(AlertLevel::Critical));
    }

    #[test]
    fn test_alert_state_warning_to_critical() {
        let mut state = AlertState::new();

        // First: Normal -> Warning
        state.should_alert(QuotaProviderType::Claude, "5_hour", 85.0, 80.0, 95.0);

        // Second: Warning -> Critical
        let result = state.should_alert(
            QuotaProviderType::Claude,
            "5_hour",
            98.0,
            80.0,
            95.0,
        );
        assert_eq!(result, Some(AlertLevel::Critical));
    }

    #[test]
    fn test_alert_state_no_spam_same_level() {
        let mut state = AlertState::new();

        // First call: Normal -> Warning (alert)
        let result1 = state.should_alert(QuotaProviderType::Claude, "5_hour", 85.0, 80.0, 95.0);
        assert_eq!(result1, Some(AlertLevel::Warning));

        // Second call: Still Warning (no alert)
        let result2 = state.should_alert(QuotaProviderType::Claude, "5_hour", 87.0, 80.0, 95.0);
        assert_eq!(result2, None);
    }

    #[test]
    fn test_alert_state_no_alert_on_decrease() {
        let mut state = AlertState::new();

        // Start at Warning
        state.should_alert(QuotaProviderType::Claude, "5_hour", 85.0, 80.0, 95.0);

        // Drop to Normal (no alert)
        let result = state.should_alert(QuotaProviderType::Claude, "5_hour", 50.0, 80.0, 95.0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_alert_state_different_providers() {
        let mut state = AlertState::new();

        // Claude at warning
        let result1 = state.should_alert(QuotaProviderType::Claude, "5_hour", 85.0, 80.0, 95.0);
        assert_eq!(result1, Some(AlertLevel::Warning));

        // Antigravity at warning (separate tracking)
        let result2 = state.should_alert(QuotaProviderType::Antigravity, "monthly", 85.0, 80.0, 95.0);
        assert_eq!(result2, Some(AlertLevel::Warning));
    }

    #[test]
    fn test_alert_state_different_windows() {
        let mut state = AlertState::new();

        // 5-hour window at warning
        let result1 = state.should_alert(QuotaProviderType::Claude, "5_hour", 85.0, 80.0, 95.0);
        assert_eq!(result1, Some(AlertLevel::Warning));

        // 7-day window at warning (separate tracking)
        let result2 = state.should_alert(QuotaProviderType::Claude, "7_day", 85.0, 80.0, 95.0);
        assert_eq!(result2, Some(AlertLevel::Warning));
    }

    #[test]
    fn test_alert_state_reset() {
        let mut state = AlertState::new();

        // Set up some state
        state.should_alert(QuotaProviderType::Claude, "5_hour", 85.0, 80.0, 95.0);
        state.should_alert(QuotaProviderType::Claude, "7_day", 90.0, 80.0, 95.0);
        state.should_alert(QuotaProviderType::Antigravity, "monthly", 85.0, 80.0, 95.0);

        // Reset Claude
        state.reset(QuotaProviderType::Claude);

        // Claude alerts should fire again
        let result1 = state.should_alert(QuotaProviderType::Claude, "5_hour", 85.0, 80.0, 95.0);
        assert_eq!(result1, Some(AlertLevel::Warning));

        // Antigravity should not (still tracked)
        let result2 = state.should_alert(QuotaProviderType::Antigravity, "monthly", 85.0, 80.0, 95.0);
        assert_eq!(result2, None);
    }

    #[test]
    fn test_alert_state_clear() {
        let mut state = AlertState::new();

        // Set up some state
        state.should_alert(QuotaProviderType::Claude, "5_hour", 85.0, 80.0, 95.0);

        // Clear all
        state.clear();

        // Should alert again
        let result = state.should_alert(QuotaProviderType::Claude, "5_hour", 85.0, 80.0, 95.0);
        assert_eq!(result, Some(AlertLevel::Warning));
    }

    // =========================================================================
    // Polling State Tests
    // =========================================================================

    #[test]
    fn test_polling_state_new() {
        let config = QuotaPollingConfig::default();
        let state = QuotaPollingState::new(config);

        assert!(!state.is_running);
        assert_eq!(state.config.interval_minutes, DEFAULT_INTERVAL_MINUTES);
    }

    #[test]
    fn test_polling_state_interval_secs() {
        let config = QuotaPollingConfig::with_interval(15);
        let state = QuotaPollingState::new(config);
        assert_eq!(state.interval_secs(), 15 * 60);
    }

    #[test]
    fn test_polling_state_start_stop() {
        let config = QuotaPollingConfig::default();
        let mut state = QuotaPollingState::new(config);

        // Start
        state.start();
        assert!(state.is_running);
        assert!(state.status.is_running);
        assert!(state.status.next_poll_at.is_some());

        // Stop
        state.stop();
        assert!(!state.is_running);
        assert!(!state.status.is_running);
        assert!(state.status.next_poll_at.is_none());
    }

    #[test]
    fn test_polling_state_begin_complete_poll() {
        let config = QuotaPollingConfig::default();
        let mut state = QuotaPollingState::new(config);
        state.start();

        // Begin poll
        state.begin_poll();
        assert!(state.status.is_polling);

        // Complete poll
        state.complete_poll(None);
        assert!(!state.status.is_polling);
        assert!(state.status.last_poll_at.is_some());
        assert!(state.status.last_error.is_none());
    }

    #[test]
    fn test_polling_state_complete_poll_with_error() {
        let config = QuotaPollingConfig::default();
        let mut state = QuotaPollingState::new(config);
        state.start();
        state.begin_poll();

        state.complete_poll(Some("Network error".to_string()));
        assert_eq!(state.status.last_error, Some("Network error".to_string()));
    }

    #[test]
    fn test_polling_state_update_quota() {
        let config = QuotaPollingConfig::default();
        let mut state = QuotaPollingState::new(config);

        state.update_quota("claude", 75.5);
        state.update_quota("antigravity", 30.0);

        assert_eq!(state.status.current_quotas.get("claude"), Some(&75.5));
        assert_eq!(state.status.current_quotas.get("antigravity"), Some(&30.0));
    }

    #[test]
    fn test_polling_state_update_config() {
        let config = QuotaPollingConfig::default();
        let mut state = QuotaPollingState::new(config);

        let new_config = QuotaPollingConfig::with_interval(30);
        state.update_config(new_config);

        assert_eq!(state.config.interval_minutes, 30);
    }

    // =========================================================================
    // Shared State Tests
    // =========================================================================

    #[tokio::test]
    async fn test_create_shared_state() {
        let config = QuotaPollingConfig::default();
        let shared = create_shared_state(config);

        let state = shared.read().await;
        assert_eq!(state.config.interval_minutes, DEFAULT_INTERVAL_MINUTES);
    }

    #[tokio::test]
    async fn test_shared_state_concurrent_access() {
        let config = QuotaPollingConfig::default();
        let shared = create_shared_state(config);

        // Start in one task
        {
            let mut state = shared.write().await;
            state.start();
        }

        // Read in another
        {
            let state = shared.read().await;
            assert!(state.is_running);
        }
    }
}
