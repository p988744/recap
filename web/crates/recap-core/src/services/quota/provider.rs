//! Quota provider trait and error types
//!
//! Defines the interface that quota providers must implement.

use async_trait::async_trait;
use thiserror::Error;

use super::types::{AccountInfo, QuotaSnapshot};

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur when fetching quota information
#[derive(Error, Debug)]
pub enum QuotaError {
    /// Provider is not installed or configured
    #[error("Provider not installed: {0}")]
    NotInstalled(String),

    /// Authentication failed or token is invalid
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// API returned an error
    #[error("API error: {0}")]
    ApiError(String),

    /// Failed to parse API response
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Network request failed
    #[error("Network error: {0}")]
    NetworkError(String),

    /// OAuth token has expired
    #[error("Token expired")]
    TokenExpired,

    /// I/O error (e.g., reading config files)
    #[error("IO error: {0}")]
    IoError(String),

    /// General/unknown error
    #[error("{0}")]
    Other(String),
}

impl From<std::io::Error> for QuotaError {
    fn from(err: std::io::Error) -> Self {
        QuotaError::IoError(err.to_string())
    }
}

impl From<reqwest::Error> for QuotaError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            QuotaError::NetworkError("Request timed out".to_string())
        } else if err.is_connect() {
            QuotaError::NetworkError("Connection failed".to_string())
        } else if err.is_status() {
            match err.status() {
                Some(status) if status.as_u16() == 401 => {
                    QuotaError::Unauthorized("Invalid or expired credentials".to_string())
                }
                Some(status) if status.as_u16() == 403 => {
                    QuotaError::Unauthorized("Access forbidden".to_string())
                }
                Some(status) => QuotaError::ApiError(format!("HTTP {}", status)),
                None => QuotaError::NetworkError(err.to_string()),
            }
        } else {
            QuotaError::NetworkError(err.to_string())
        }
    }
}

impl From<serde_json::Error> for QuotaError {
    fn from(err: serde_json::Error) -> Self {
        QuotaError::ParseError(err.to_string())
    }
}

// ============================================================================
// Provider Trait
// ============================================================================

/// Trait for quota data providers
///
/// Implement this trait to add support for a new quota provider.
/// Each provider is responsible for:
/// 1. Fetching current quota usage from the provider's API
/// 2. Parsing the response into `QuotaSnapshot` structs
/// 3. Checking if the provider is available/authenticated
///
/// # Example Implementation
///
/// ```ignore
/// use async_trait::async_trait;
/// use recap_core::services::quota::{QuotaProvider, QuotaError, QuotaSnapshot, AccountInfo};
///
/// struct MyProvider;
///
/// #[async_trait]
/// impl QuotaProvider for MyProvider {
///     fn provider_id(&self) -> &'static str {
///         "my_provider"
///     }
///
///     async fn fetch_quota(&self) -> Result<Vec<QuotaSnapshot>, QuotaError> {
///         // Fetch quota from API
///         todo!()
///     }
///
///     async fn is_available(&self) -> bool {
///         // Check if provider is configured
///         true
///     }
///
///     async fn get_account_info(&self) -> Result<Option<AccountInfo>, QuotaError> {
///         // Get account details
///         Ok(None)
///     }
/// }
/// ```
#[async_trait]
pub trait QuotaProvider: Send + Sync {
    /// Unique identifier for this provider
    ///
    /// This is used to identify the provider in the database and UI.
    /// Should be a lowercase string like "claude" or "antigravity".
    fn provider_id(&self) -> &'static str;

    /// Human-readable display name for this provider
    ///
    /// Used in UI elements. Defaults to the provider_id if not overridden.
    fn display_name(&self) -> &'static str {
        self.provider_id()
    }

    /// Fetch current quota usage from the provider
    ///
    /// Returns a list of quota snapshots, one for each quota type/window
    /// the provider tracks. For example, Claude may return snapshots for
    /// both 5-hour rate limits and 7-day usage limits.
    ///
    /// # Errors
    ///
    /// Returns `QuotaError` if:
    /// - Provider is not installed (`NotInstalled`)
    /// - Authentication fails (`Unauthorized`, `TokenExpired`)
    /// - API request fails (`NetworkError`, `ApiError`)
    /// - Response cannot be parsed (`ParseError`)
    async fn fetch_quota(&self) -> Result<Vec<QuotaSnapshot>, QuotaError>;

    /// Check if this provider is currently available
    ///
    /// Returns `true` if the provider can be used (i.e., is installed,
    /// configured, and has valid credentials).
    ///
    /// This is a quick check and should not make network requests.
    /// Use this to determine which providers to show in the UI.
    async fn is_available(&self) -> bool;

    /// Get account information from the provider
    ///
    /// Returns account details like email, plan name, etc. if available.
    /// Returns `Ok(None)` if account info is not supported or not available.
    ///
    /// # Errors
    ///
    /// Returns `QuotaError` if fetching account info fails.
    async fn get_account_info(&self) -> Result<Option<AccountInfo>, QuotaError>;

    /// Refresh authentication tokens if needed
    ///
    /// Some providers (like Claude) use OAuth tokens that expire.
    /// This method should refresh the token if it's expired or about to expire.
    ///
    /// Default implementation does nothing (for providers that don't need refresh).
    async fn refresh_auth(&self) -> Result<(), QuotaError> {
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quota_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let quota_err: QuotaError = io_err.into();
        assert!(matches!(quota_err, QuotaError::IoError(_)));
        assert!(quota_err.to_string().contains("file not found"));
    }

    #[test]
    fn test_quota_error_from_serde() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let quota_err: QuotaError = json_err.into();
        assert!(matches!(quota_err, QuotaError::ParseError(_)));
    }

    #[test]
    fn test_quota_error_display() {
        assert_eq!(
            QuotaError::NotInstalled("Claude".to_string()).to_string(),
            "Provider not installed: Claude"
        );
        assert_eq!(
            QuotaError::Unauthorized("bad token".to_string()).to_string(),
            "Unauthorized: bad token"
        );
        assert_eq!(
            QuotaError::TokenExpired.to_string(),
            "Token expired"
        );
    }
}
