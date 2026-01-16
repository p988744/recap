//! Authentication module - JWT token management

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use std::sync::OnceLock;

use crate::models::{Claims, User};

/// JWT secret key - reads from environment variable or generates a secure random key
/// In production, set RECAP_JWT_SECRET environment variable
fn get_jwt_secret() -> &'static [u8] {
    static JWT_SECRET: OnceLock<Vec<u8>> = OnceLock::new();

    JWT_SECRET.get_or_init(|| {
        match std::env::var("RECAP_JWT_SECRET") {
            Ok(secret) if secret.len() >= 32 => {
                // Use environment variable if it's set and long enough
                secret.into_bytes()
            }
            Ok(secret) if !secret.is_empty() => {
                // Warn if secret is too short but still use it
                eprintln!("WARNING: RECAP_JWT_SECRET is shorter than 32 characters. Consider using a longer secret.");
                secret.into_bytes()
            }
            _ => {
                // Generate a secure random secret for this session
                // Note: This means tokens won't persist across app restarts
                eprintln!("WARNING: RECAP_JWT_SECRET not set. Generating random secret. Tokens won't persist across restarts.");
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let secret: Vec<u8> = (0..64).map(|_| rng.gen::<u8>()).collect();
                secret
            }
        }
    })
}

const TOKEN_EXPIRY_DAYS: i64 = 7;

/// Create a JWT token for a user
pub fn create_token(user: &User) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(TOKEN_EXPIRY_DAYS))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: user.id.clone(),
        email: user.email.clone(),
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(get_jwt_secret()),
    )
}

/// Verify and decode a JWT token
pub fn verify_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(get_jwt_secret()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

/// Hash a password
pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    bcrypt::verify(password, hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // ========================================================================
    // Helper Functions
    // ========================================================================

    fn create_test_user() -> User {
        User {
            id: "user-123".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            name: "Test User".to_string(),
            username: Some("testuser".to_string()),
            employee_id: None,
            department_id: None,
            title: None,
            gitlab_url: None,
            gitlab_pat: None,
            jira_url: None,
            jira_email: None,
            jira_pat: None,
            tempo_token: None,
            is_active: true,
            is_admin: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // ========================================================================
    // Password Hashing Tests
    // ========================================================================

    #[test]
    fn test_hash_password() {
        let password = "test_password";
        let hash = hash_password(password).unwrap();
        assert!(!hash.is_empty());
        assert_ne!(hash, password);
    }

    #[test]
    fn test_verify_password() {
        let password = "test_password";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_hash_password_different_hashes() {
        // Same password should produce different hashes (due to salt)
        let password = "test_password";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(verify_password(password, &hash1).unwrap());
        assert!(verify_password(password, &hash2).unwrap());
    }

    #[test]
    fn test_hash_password_empty_password() {
        let password = "";
        let hash = hash_password(password).unwrap();
        assert!(!hash.is_empty());
        assert!(verify_password(password, &hash).unwrap());
    }

    #[test]
    fn test_hash_password_unicode() {
        let password = "密碼測試🔐";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("密碼測試", &hash).unwrap());
    }

    #[test]
    fn test_verify_password_invalid_hash() {
        let result = verify_password("password", "invalid_hash");
        assert!(result.is_err());
    }

    // ========================================================================
    // JWT Token Tests
    // ========================================================================

    #[test]
    fn test_create_token_success() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();

        // Token should be a non-empty string
        assert!(!token.is_empty());

        // Token should have three parts separated by dots (header.payload.signature)
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_verify_token_success() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();

        let claims = verify_token(&token).unwrap();

        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.email, "test@example.com");
        assert!(claims.exp > Utc::now().timestamp());
    }

    #[test]
    fn test_verify_token_roundtrip() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let claims = verify_token(&token).unwrap();

        // Claims should match user data
        assert_eq!(claims.sub, user.id);
        assert_eq!(claims.email, user.email);
    }

    #[test]
    fn test_verify_token_invalid_token() {
        let result = verify_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_token_malformed_token() {
        let result = verify_token("not-a-jwt");
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_token_empty_token() {
        let result = verify_token("");
        assert!(result.is_err());
    }

    #[test]
    fn test_token_expiration_in_future() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let claims = verify_token(&token).unwrap();

        // Token should expire 7 days from now
        let expected_exp = Utc::now()
            .checked_add_signed(Duration::days(TOKEN_EXPIRY_DAYS))
            .unwrap()
            .timestamp();

        // Allow 10 seconds tolerance
        assert!((claims.exp - expected_exp).abs() < 10);
    }

    #[test]
    fn test_create_token_different_users() {
        let user1 = create_test_user();
        let mut user2 = create_test_user();
        user2.id = "user-456".to_string();
        user2.email = "other@example.com".to_string();

        let token1 = create_token(&user1).unwrap();
        let token2 = create_token(&user2).unwrap();

        // Different users should produce different tokens
        assert_ne!(token1, token2);

        // Each token should verify to correct user
        let claims1 = verify_token(&token1).unwrap();
        let claims2 = verify_token(&token2).unwrap();

        assert_eq!(claims1.sub, "user-123");
        assert_eq!(claims2.sub, "user-456");
    }

    #[test]
    fn test_create_token_unicode_email() {
        let mut user = create_test_user();
        user.email = "用戶@example.com".to_string();

        let token = create_token(&user).unwrap();
        let claims = verify_token(&token).unwrap();

        assert_eq!(claims.email, "用戶@example.com");
    }
}
