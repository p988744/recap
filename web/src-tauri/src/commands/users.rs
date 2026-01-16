//! Users commands
//!
//! Tauri commands for user profile operations.
//! Uses trait-based dependency injection for testability.

use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use tauri::State;

use recap_core::auth::verify_token;
use recap_core::models::UserResponse;

use super::AppState;

// ============================================================================
// Request/Response types
// ============================================================================

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub title: Option<String>,
    pub employee_id: Option<String>,
    pub department_id: Option<String>,
}

// ============================================================================
// Repository Trait
// ============================================================================

/// Profile repository trait - abstracts database operations for testability
#[async_trait]
pub trait ProfileRepository: Send + Sync {
    /// Find user by ID
    async fn find_by_id(&self, id: &str) -> Result<Option<crate::models::User>, String>;

    /// Update user profile fields
    async fn update_profile(
        &self,
        user_id: &str,
        request: &UpdateProfileRequest,
    ) -> Result<(), String>;
}

// ============================================================================
// SQLite Repository Implementation (Production)
// ============================================================================

/// SQLite implementation of ProfileRepository
pub struct SqliteProfileRepository<'a> {
    pool: &'a sqlx::SqlitePool,
}

impl<'a> SqliteProfileRepository<'a> {
    pub fn new(pool: &'a sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl<'a> ProfileRepository for SqliteProfileRepository<'a> {
    async fn find_by_id(&self, id: &str) -> Result<Option<crate::models::User>, String> {
        sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(self.pool)
            .await
            .map_err(|e| e.to_string())
    }

    async fn update_profile(
        &self,
        user_id: &str,
        request: &UpdateProfileRequest,
    ) -> Result<(), String> {
        let now = Utc::now();

        if let Some(name) = &request.name {
            sqlx::query("UPDATE users SET name = ?, updated_at = ? WHERE id = ?")
                .bind(name)
                .bind(now)
                .bind(user_id)
                .execute(self.pool)
                .await
                .map_err(|e| e.to_string())?;
        }

        if let Some(email) = &request.email {
            sqlx::query("UPDATE users SET email = ?, updated_at = ? WHERE id = ?")
                .bind(email)
                .bind(now)
                .bind(user_id)
                .execute(self.pool)
                .await
                .map_err(|e| e.to_string())?;
        }

        if let Some(title) = &request.title {
            sqlx::query("UPDATE users SET title = ?, updated_at = ? WHERE id = ?")
                .bind(title)
                .bind(now)
                .bind(user_id)
                .execute(self.pool)
                .await
                .map_err(|e| e.to_string())?;
        }

        if let Some(employee_id) = &request.employee_id {
            sqlx::query("UPDATE users SET employee_id = ?, updated_at = ? WHERE id = ?")
                .bind(employee_id)
                .bind(now)
                .bind(user_id)
                .execute(self.pool)
                .await
                .map_err(|e| e.to_string())?;
        }

        if let Some(department_id) = &request.department_id {
            sqlx::query("UPDATE users SET department_id = ?, updated_at = ? WHERE id = ?")
                .bind(department_id)
                .bind(now)
                .bind(user_id)
                .execute(self.pool)
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}

// ============================================================================
// Core Business Logic (Testable, uses trait)
// ============================================================================

/// Get user profile - testable business logic
pub async fn get_profile_impl<R: ProfileRepository>(
    repo: &R,
    token: &str,
) -> Result<UserResponse, String> {
    let claims = verify_token(token).map_err(|e| e.to_string())?;

    let user = repo
        .find_by_id(&claims.sub)
        .await?
        .ok_or_else(|| "User not found".to_string())?;

    Ok(UserResponse::from(user))
}

/// Update user profile - testable business logic
pub async fn update_profile_impl<R: ProfileRepository>(
    repo: &R,
    token: &str,
    request: UpdateProfileRequest,
) -> Result<UserResponse, String> {
    let claims = verify_token(token).map_err(|e| e.to_string())?;

    // Verify user exists
    let _user = repo
        .find_by_id(&claims.sub)
        .await?
        .ok_or_else(|| "User not found".to_string())?;

    // Update profile
    repo.update_profile(&claims.sub, &request).await?;

    // Return updated user
    let updated_user = repo
        .find_by_id(&claims.sub)
        .await?
        .ok_or_else(|| "User not found after update".to_string())?;

    Ok(UserResponse::from(updated_user))
}

// ============================================================================
// Tauri Commands (Thin wrappers)
// ============================================================================

/// Get current user profile
#[tauri::command]
pub async fn get_profile(
    state: State<'_, AppState>,
    token: String,
) -> Result<UserResponse, String> {
    let db = state.db.lock().await;
    let repo = SqliteProfileRepository::new(&db.pool);
    get_profile_impl(&repo, &token).await
}

/// Update user profile
#[tauri::command]
pub async fn update_profile(
    state: State<'_, AppState>,
    token: String,
    request: UpdateProfileRequest,
) -> Result<UserResponse, String> {
    let db = state.db.lock().await;
    let repo = SqliteProfileRepository::new(&db.pool);
    update_profile_impl(&repo, &token, request).await
}

// ============================================================================
// Tests with Mock Repository
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use recap_core::auth::create_token;
    use std::collections::HashMap;
    use std::sync::Mutex;

    // ========================================================================
    // Mock Repository
    // ========================================================================

    pub struct MockProfileRepository {
        users: Mutex<HashMap<String, crate::models::User>>,
    }

    impl MockProfileRepository {
        pub fn new() -> Self {
            Self {
                users: Mutex::new(HashMap::new()),
            }
        }

        pub fn with_user(self, user: crate::models::User) -> Self {
            self.users.lock().unwrap().insert(user.id.clone(), user);
            self
        }

        pub fn create_test_user(id: &str, name: &str) -> crate::models::User {
            crate::models::User {
                id: id.to_string(),
                email: format!("{}@test.com", name),
                password_hash: "hash".to_string(),
                name: name.to_string(),
                username: Some(name.to_string()),
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
    }

    #[async_trait]
    impl ProfileRepository for MockProfileRepository {
        async fn find_by_id(&self, id: &str) -> Result<Option<crate::models::User>, String> {
            Ok(self.users.lock().unwrap().get(id).cloned())
        }

        async fn update_profile(
            &self,
            user_id: &str,
            request: &UpdateProfileRequest,
        ) -> Result<(), String> {
            let mut users = self.users.lock().unwrap();
            if let Some(user) = users.get_mut(user_id) {
                if let Some(name) = &request.name {
                    user.name = name.clone();
                }
                if let Some(email) = &request.email {
                    user.email = email.clone();
                }
                if let Some(title) = &request.title {
                    user.title = Some(title.clone());
                }
                if let Some(employee_id) = &request.employee_id {
                    user.employee_id = Some(employee_id.clone());
                }
                if let Some(department_id) = &request.department_id {
                    user.department_id = Some(department_id.clone());
                }
                user.updated_at = Utc::now();
                Ok(())
            } else {
                Err("User not found".to_string())
            }
        }
    }

    // ========================================================================
    // get_profile Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_profile_success() {
        let user = MockProfileRepository::create_test_user("user-1", "testuser");
        let repo = MockProfileRepository::new().with_user(user.clone());
        let token = create_token(&user).unwrap();

        let result = get_profile_impl(&repo, &token).await.unwrap();

        assert_eq!(result.id, "user-1");
        assert_eq!(result.name, "testuser");
    }

    #[tokio::test]
    async fn test_get_profile_invalid_token() {
        let repo = MockProfileRepository::new();

        let result = get_profile_impl(&repo, "invalid-token").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_profile_user_not_found() {
        let user = MockProfileRepository::create_test_user("user-1", "testuser");
        let token = create_token(&user).unwrap();
        let repo = MockProfileRepository::new(); // Empty repo

        let result = get_profile_impl(&repo, &token).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "User not found");
    }

    // ========================================================================
    // update_profile Tests
    // ========================================================================

    #[tokio::test]
    async fn test_update_profile_name() {
        let user = MockProfileRepository::create_test_user("user-1", "oldname");
        let repo = MockProfileRepository::new().with_user(user.clone());
        let token = create_token(&user).unwrap();

        let request = UpdateProfileRequest {
            name: Some("newname".to_string()),
            ..Default::default()
        };

        let result = update_profile_impl(&repo, &token, request).await.unwrap();

        assert_eq!(result.name, "newname");
    }

    #[tokio::test]
    async fn test_update_profile_email() {
        let user = MockProfileRepository::create_test_user("user-1", "testuser");
        let repo = MockProfileRepository::new().with_user(user.clone());
        let token = create_token(&user).unwrap();

        let request = UpdateProfileRequest {
            email: Some("newemail@example.com".to_string()),
            ..Default::default()
        };

        let result = update_profile_impl(&repo, &token, request).await.unwrap();

        assert_eq!(result.email, "newemail@example.com");
    }

    #[tokio::test]
    async fn test_update_profile_title() {
        let user = MockProfileRepository::create_test_user("user-1", "testuser");
        let repo = MockProfileRepository::new().with_user(user.clone());
        let token = create_token(&user).unwrap();

        let request = UpdateProfileRequest {
            title: Some("Senior Developer".to_string()),
            ..Default::default()
        };

        let result = update_profile_impl(&repo, &token, request).await.unwrap();

        assert_eq!(result.title, Some("Senior Developer".to_string()));
    }

    #[tokio::test]
    async fn test_update_profile_multiple_fields() {
        let user = MockProfileRepository::create_test_user("user-1", "oldname");
        let repo = MockProfileRepository::new().with_user(user.clone());
        let token = create_token(&user).unwrap();

        let request = UpdateProfileRequest {
            name: Some("newname".to_string()),
            email: Some("new@example.com".to_string()),
            title: Some("Manager".to_string()),
            employee_id: Some("EMP001".to_string()),
            department_id: Some("DEPT001".to_string()),
        };

        let result = update_profile_impl(&repo, &token, request).await.unwrap();

        assert_eq!(result.name, "newname");
        assert_eq!(result.email, "new@example.com");
        assert_eq!(result.title, Some("Manager".to_string()));
        assert_eq!(result.employee_id, Some("EMP001".to_string()));
        assert_eq!(result.department_id, Some("DEPT001".to_string()));
    }

    #[tokio::test]
    async fn test_update_profile_no_changes() {
        let user = MockProfileRepository::create_test_user("user-1", "testuser");
        let repo = MockProfileRepository::new().with_user(user.clone());
        let token = create_token(&user).unwrap();

        let request = UpdateProfileRequest::default(); // No fields set

        let result = update_profile_impl(&repo, &token, request).await.unwrap();

        assert_eq!(result.name, "testuser"); // Unchanged
    }

    #[tokio::test]
    async fn test_update_profile_invalid_token() {
        let repo = MockProfileRepository::new();

        let request = UpdateProfileRequest {
            name: Some("newname".to_string()),
            ..Default::default()
        };

        let result = update_profile_impl(&repo, "invalid-token", request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_profile_user_not_found() {
        let user = MockProfileRepository::create_test_user("user-1", "testuser");
        let token = create_token(&user).unwrap();
        let repo = MockProfileRepository::new(); // Empty repo

        let request = UpdateProfileRequest {
            name: Some("newname".to_string()),
            ..Default::default()
        };

        let result = update_profile_impl(&repo, &token, request).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "User not found");
    }
}
