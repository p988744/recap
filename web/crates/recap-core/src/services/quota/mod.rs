//! Quota tracking module
//!
//! This module provides quota tracking for various AI coding assistants,
//! allowing users to monitor their API usage and avoid hitting rate limits.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │ QuotaStore (storage layer)                              │
//! │   - save_snapshot()                                     │
//! │   - get_latest_snapshot()                               │
//! │   - get_history()                                       │
//! └─────────────────────────────────────────────────────────┘
//!          ▲
//!          │
//! ┌─────────────────────────────────────────────────────────┐
//! │ trait QuotaProvider                                     │
//! │   - fetch_quota() -> Vec<QuotaSnapshot>                 │
//! │   - is_available() -> bool                              │
//! │   - get_account_info() -> Option<AccountInfo>           │
//! └─────────────────────────────────────────────────────────┘
//!          │
//!     ┌────┴────┐
//!     ▼         ▼
//! ┌──────┐  ┌──────┐
//! │Claude│  │Antig.│
//! │OAuth │  │(TBD) │
//! └──────┘  └──────┘
//! ```
//!
//! # Supported Providers
//!
//! - **Claude** (implemented): Uses OAuth to access Anthropic's usage API
//! - **Antigravity** (planned): Google's Gemini Code assistant
//!
//! # Usage
//!
//! ```ignore
//! use recap_core::services::quota::{ClaudeQuotaProvider, QuotaProvider, QuotaStore};
//!
//! // Create provider
//! let provider = ClaudeQuotaProvider::new()?;
//!
//! // Check if available
//! if provider.is_available().await {
//!     // Fetch current quota
//!     let snapshots = provider.fetch_quota().await?;
//!
//!     // Save to database
//!     let store = QuotaStore::new(pool);
//!     for snapshot in snapshots {
//!         store.save_snapshot(&snapshot).await?;
//!     }
//! }
//! ```

pub mod types;
pub mod provider;
pub mod claude;
pub mod store;
pub mod timer;
pub mod cost;

// Re-export main types
pub use types::{
    QuotaProviderType,
    QuotaWindowType,
    QuotaSnapshot,
    ExtraCredits,
    AccountInfo,
    AlertLevel,
    QuotaSettings,
};

// Re-export provider trait and error
pub use provider::{QuotaProvider, QuotaError};

// Re-export providers
pub use claude::ClaudeQuotaProvider;

// Re-export store
pub use store::{QuotaStore, StoredQuotaSnapshot};

// Re-export timer types
pub use timer::{
    QuotaPollingConfig,
    QuotaPollingStatus,
    QuotaPollingState,
    SharedPollingState,
    AlertState,
    create_shared_state,
    MIN_INTERVAL_MINUTES,
    DEFAULT_INTERVAL_MINUTES,
};

// Re-export cost calculator
pub use cost::{
    CostCalculator,
    CostSummary,
    DailyUsage,
    ModelUsage,
    TokenUsage,
};
