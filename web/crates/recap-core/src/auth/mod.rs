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
}
