//! Unified error handling for recap-core

use thiserror::Error;

/// Core error type for recap-core
#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("Password hashing error: {0}")]
    Bcrypt(#[from] bcrypt::BcryptError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias for recap-core
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Create an authentication error
    pub fn auth(msg: impl Into<String>) -> Self {
        Error::Auth(msg.into())
    }

    /// Create a configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Error::Config(msg.into())
    }

    /// Create a validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Error::Validation(msg.into())
    }

    /// Create a not found error
    pub fn not_found(msg: impl Into<String>) -> Self {
        Error::NotFound(msg.into())
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Error::Internal(msg.into())
    }
}

// Convert to String for Tauri command returns
impl From<Error> for String {
    fn from(err: Error) -> Self {
        err.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::auth("Invalid token");
        assert_eq!(err.to_string(), "Authentication error: Invalid token");
    }

    #[test]
    fn test_error_conversion_to_string() {
        let err = Error::validation("Invalid input");
        let s: String = err.into();
        assert!(s.contains("Validation error"));
    }
}
