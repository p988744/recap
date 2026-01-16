//! User repository
//!
//! Abstracts database operations for testability using trait-based dependency injection.

use async_trait::async_trait;
use crate::models::User;
use super::types::NewUser;

/// User repository trait - abstracts database operations for testability
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Get total user count
    async fn get_user_count(&self) -> Result<i64, String>;

    /// Get the first user (ordered by creation time)
    async fn get_first_user(&self) -> Result<Option<User>, String>;

    /// Find user by username
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, String>;

    /// Find user by ID
    async fn find_by_id(&self, id: &str) -> Result<Option<User>, String>;

    /// Check if username exists
    async fn username_exists(&self, username: &str) -> Result<bool, String>;

    /// Check if email exists
    async fn email_exists(&self, email: &str) -> Result<bool, String>;

    /// Create a new user
    async fn create_user(&self, user: NewUser) -> Result<User, String>;
}

/// SQLite implementation of UserRepository
pub struct SqliteUserRepository<'a> {
    pool: &'a sqlx::SqlitePool,
}

impl<'a> SqliteUserRepository<'a> {
    pub fn new(pool: &'a sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl<'a> UserRepository for SqliteUserRepository<'a> {
    async fn get_user_count(&self) -> Result<i64, String> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(count.0)
    }

    async fn get_first_user(&self) -> Result<Option<User>, String> {
        sqlx::query_as("SELECT * FROM users ORDER BY created_at LIMIT 1")
            .fetch_optional(self.pool)
            .await
            .map_err(|e| e.to_string())
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, String> {
        sqlx::query_as("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(self.pool)
            .await
            .map_err(|e| e.to_string())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<User>, String> {
        sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(self.pool)
            .await
            .map_err(|e| e.to_string())
    }

    async fn username_exists(&self, username: &str) -> Result<bool, String> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE username = ?")
            .bind(username)
            .fetch_one(self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(count.0 > 0)
    }

    async fn email_exists(&self, email: &str) -> Result<bool, String> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE email = ?")
            .bind(email)
            .fetch_one(self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(count.0 > 0)
    }

    async fn create_user(&self, user: NewUser) -> Result<User, String> {
        let now = chrono::Utc::now();

        sqlx::query(
            r#"
            INSERT INTO users (id, username, email, password_hash, name, title, is_admin, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&user.id)
        .bind(&user.username)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.name)
        .bind(&user.title)
        .bind(user.is_admin)
        .bind(now)
        .bind(now)
        .execute(self.pool)
        .await
        .map_err(|e| e.to_string())?;

        self.find_by_id(&user.id)
            .await?
            .ok_or_else(|| "Failed to fetch created user".to_string())
    }
}
