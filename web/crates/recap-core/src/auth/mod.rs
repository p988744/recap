//! Authentication module - JWT token management

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use std::sync::OnceLock;

use crate::models::{Claims, User};

/// Get the path to the persisted JWT secret file in the app data directory
fn get_secret_file_path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("com", "recap", "Recap")
        .map(|dirs| dirs.data_dir().join(".jwt_secret"))
}

/// JWT secret key - reads from environment variable, persisted file, or auto-generates
fn get_jwt_secret() -> &'static [u8] {
    static JWT_SECRET: OnceLock<Vec<u8>> = OnceLock::new();

    JWT_SECRET.get_or_init(|| {
        // 1. Check environment variable first
        match std::env::var("RECAP_JWT_SECRET") {
            Ok(secret) if secret.len() >= 32 => {
                return secret.into_bytes();
            }
            Ok(secret) if !secret.is_empty() => {
                eprintln!("WARNING: RECAP_JWT_SECRET is shorter than 32 characters. Consider using a longer secret.");
                return secret.into_bytes();
            }
            _ => {}
        }

        // 2. Try to read from persisted file
        if let Some(path) = get_secret_file_path() {
            if let Ok(secret) = std::fs::read_to_string(&path) {
                let secret = secret.trim().to_string();
                if secret.len() >= 32 {
                    log::info!("Loaded JWT secret from {}", path.display());
                    return secret.into_bytes();
                }
            }
        }

        // 3. Generate and persist a new secret
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let secret: Vec<u8> = (0..64).map(|_| rng.gen::<u8>()).collect();
        let hex_secret: String = secret.iter().map(|b| format!("{:02x}", b)).collect();

        if let Some(path) = get_secret_file_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match std::fs::write(&path, &hex_secret) {
                Ok(_) => {
                    // Set restrictive permissions on Unix
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
                    }
                    log::info!("Generated and saved JWT secret to {}", path.display());
                }
                Err(e) => {
                    eprintln!("WARNING: Failed to save JWT secret to {}: {}. Tokens won't persist across restarts.", path.display(), e);
                }
            }
        } else {
            eprintln!("WARNING: Could not determine app data directory. Tokens won't persist across restarts.");
        }

        hex_secret.into_bytes()
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
