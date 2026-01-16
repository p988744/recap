//! Auth commands
//!
//! Tauri commands for authentication operations.
//! Uses trait-based dependency injection for testability.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use recap_core::{
    auth::{create_token, hash_password, verify_password},
    models::UserResponse,
};

use super::AppState;

// ============================================================================
// Request/Response types
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub name: String,
    pub email: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppStatus {
    pub has_users: bool,
    pub user_count: i64,
    pub first_user: Option<UserResponse>,
    pub local_mode: bool,
}

// ============================================================================
// Repository Trait (Abstraction for database operations)
// ============================================================================

/// User repository trait - abstracts database operations for testability
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Get total user count
    async fn get_user_count(&self) -> Result<i64, String>;

    /// Get the first user (ordered by creation time)
    async fn get_first_user(&self) -> Result<Option<crate::models::User>, String>;

    /// Find user by username
    async fn find_by_username(&self, username: &str) -> Result<Option<crate::models::User>, String>;

    /// Find user by ID
    async fn find_by_id(&self, id: &str) -> Result<Option<crate::models::User>, String>;

    /// Check if username exists
    async fn username_exists(&self, username: &str) -> Result<bool, String>;

    /// Check if email exists
    async fn email_exists(&self, email: &str) -> Result<bool, String>;

    /// Create a new user
    async fn create_user(&self, user: NewUser) -> Result<crate::models::User, String>;
}

/// Data for creating a new user
#[derive(Debug, Clone)]
pub struct NewUser {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub name: String,
    pub title: Option<String>,
    pub is_admin: bool,
}

// ============================================================================
// SQLite Repository Implementation (Production)
// ============================================================================

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

    async fn get_first_user(&self) -> Result<Option<crate::models::User>, String> {
        sqlx::query_as("SELECT * FROM users ORDER BY created_at LIMIT 1")
            .fetch_optional(self.pool)
            .await
            .map_err(|e| e.to_string())
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<crate::models::User>, String> {
        sqlx::query_as("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(self.pool)
            .await
            .map_err(|e| e.to_string())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<crate::models::User>, String> {
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

    async fn create_user(&self, user: NewUser) -> Result<crate::models::User, String> {
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

// ============================================================================
// Core Business Logic (Testable, uses trait)
// ============================================================================

/// Get app status - testable business logic
pub async fn get_app_status_impl<R: UserRepository>(repo: &R) -> Result<AppStatus, String> {
    let count = repo.get_user_count().await?;
    let first_user = if count > 0 {
        repo.get_first_user().await?
    } else {
        None
    };

    Ok(AppStatus {
        has_users: count > 0,
        user_count: count,
        first_user: first_user.map(UserResponse::from),
        local_mode: true,
    })
}

/// Register user - testable business logic
pub async fn register_user_impl<R: UserRepository>(
    repo: &R,
    request: RegisterRequest,
) -> Result<UserResponse, String> {
    // Check if username already exists
    if repo.username_exists(&request.username).await? {
        return Err("Username already exists".to_string());
    }

    // Generate email if not provided
    let email = request
        .email
        .clone()
        .unwrap_or_else(|| format!("{}@local", &request.username));

    // Check if email already exists
    if repo.email_exists(&email).await? {
        return Err("Email already registered".to_string());
    }

    // Check if this is the first user (will be admin)
    let is_first_user = repo.get_user_count().await? == 0;

    // Hash password
    let password_hash = hash_password(&request.password).map_err(|e| e.to_string())?;

    // Create user
    let new_user = NewUser {
        id: Uuid::new_v4().to_string(),
        username: request.username,
        email,
        password_hash,
        name: request.name,
        title: request.title,
        is_admin: is_first_user,
    };

    let user = repo.create_user(new_user).await?;
    Ok(UserResponse::from(user))
}

/// Login - testable business logic
pub async fn login_impl<R: UserRepository>(
    repo: &R,
    request: LoginRequest,
) -> Result<TokenResponse, String> {
    // Find user by username
    let user = repo
        .find_by_username(&request.username)
        .await?
        .ok_or_else(|| "Invalid credentials".to_string())?;

    // Verify password
    let valid = verify_password(&request.password, &user.password_hash).map_err(|e| e.to_string())?;

    if !valid {
        return Err("Invalid credentials".to_string());
    }

    if !user.is_active {
        return Err("Account is disabled".to_string());
    }

    // Create token
    let token = create_token(&user).map_err(|e| e.to_string())?;

    Ok(TokenResponse {
        access_token: token,
        token_type: "bearer".to_string(),
        expires_in: 7 * 24 * 60 * 60, // 7 days in seconds
    })
}

/// Auto-login - testable business logic
pub async fn auto_login_impl<R: UserRepository>(repo: &R) -> Result<TokenResponse, String> {
    // Get first user
    let user = repo
        .get_first_user()
        .await?
        .ok_or_else(|| "No user found".to_string())?;

    if !user.is_active {
        return Err("Account is disabled".to_string());
    }

    // Create token
    let token = create_token(&user).map_err(|e| e.to_string())?;

    Ok(TokenResponse {
        access_token: token,
        token_type: "bearer".to_string(),
        expires_in: 7 * 24 * 60 * 60,
    })
}

/// Get current user - testable business logic
pub async fn get_current_user_impl<R: UserRepository>(
    repo: &R,
    token: &str,
) -> Result<UserResponse, String> {
    // Verify token and get claims
    let claims = recap_core::auth::verify_token(token).map_err(|e| e.to_string())?;

    let user = repo
        .find_by_id(&claims.sub)
        .await?
        .ok_or_else(|| "User not found".to_string())?;

    Ok(UserResponse::from(user))
}

// ============================================================================
// Tauri Commands (Thin wrappers)
// ============================================================================

/// Get app status (has_users, local_mode, etc.)
#[tauri::command]
pub async fn get_app_status(state: State<'_, AppState>) -> Result<AppStatus, String> {
    let db = state.db.lock().await;
    let repo = SqliteUserRepository::new(&db.pool);
    get_app_status_impl(&repo).await
}

/// Register a new user
#[tauri::command]
pub async fn register_user(
    state: State<'_, AppState>,
    request: RegisterRequest,
) -> Result<UserResponse, String> {
    let db = state.db.lock().await;
    let repo = SqliteUserRepository::new(&db.pool);
    register_user_impl(&repo, request).await
}

/// Login and get token
#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    request: LoginRequest,
) -> Result<TokenResponse, String> {
    let db = state.db.lock().await;
    let repo = SqliteUserRepository::new(&db.pool);
    login_impl(&repo, request).await
}

/// Auto-login for local mode (uses first user)
#[tauri::command]
pub async fn auto_login(state: State<'_, AppState>) -> Result<TokenResponse, String> {
    let db = state.db.lock().await;
    let repo = SqliteUserRepository::new(&db.pool);
    auto_login_impl(&repo).await
}

/// Get current user by token
#[tauri::command]
pub async fn get_current_user(
    state: State<'_, AppState>,
    token: String,
) -> Result<UserResponse, String> {
    let db = state.db.lock().await;
    let repo = SqliteUserRepository::new(&db.pool);
    get_current_user_impl(&repo, &token).await
}

// ============================================================================
// Tests with Mock Repository
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::sync::Mutex;

    // ========================================================================
    // Mock Repository (only compiled in test mode)
    // ========================================================================

    /// Mock implementation of UserRepository for testing
    pub struct MockUserRepository {
        users: Mutex<HashMap<String, crate::models::User>>,
    }

    impl MockUserRepository {
        pub fn new() -> Self {
            Self {
                users: Mutex::new(HashMap::new()),
            }
        }

        /// Add a test user to the mock repository
        pub fn with_user(self, user: crate::models::User) -> Self {
            self.users.lock().unwrap().insert(user.id.clone(), user);
            self
        }

        /// Create a test user with minimal required fields
        pub fn create_test_user(id: &str, username: &str, password_hash: &str) -> crate::models::User {
            crate::models::User {
                id: id.to_string(),
                email: format!("{}@test.com", username),
                password_hash: password_hash.to_string(),
                name: format!("Test {}", username),
                username: Some(username.to_string()),
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
    impl UserRepository for MockUserRepository {
        async fn get_user_count(&self) -> Result<i64, String> {
            Ok(self.users.lock().unwrap().len() as i64)
        }

        async fn get_first_user(&self) -> Result<Option<crate::models::User>, String> {
            let users = self.users.lock().unwrap();
            let first = users.values().min_by_key(|u| u.created_at);
            Ok(first.cloned())
        }

        async fn find_by_username(&self, username: &str) -> Result<Option<crate::models::User>, String> {
            let users = self.users.lock().unwrap();
            let user = users.values().find(|u| u.username.as_deref() == Some(username));
            Ok(user.cloned())
        }

        async fn find_by_id(&self, id: &str) -> Result<Option<crate::models::User>, String> {
            Ok(self.users.lock().unwrap().get(id).cloned())
        }

        async fn username_exists(&self, username: &str) -> Result<bool, String> {
            let users = self.users.lock().unwrap();
            Ok(users.values().any(|u| u.username.as_deref() == Some(username)))
        }

        async fn email_exists(&self, email: &str) -> Result<bool, String> {
            let users = self.users.lock().unwrap();
            Ok(users.values().any(|u| u.email == email))
        }

        async fn create_user(&self, new_user: NewUser) -> Result<crate::models::User, String> {
            let now = Utc::now();
            let user = crate::models::User {
                id: new_user.id.clone(),
                email: new_user.email,
                password_hash: new_user.password_hash,
                name: new_user.name,
                username: Some(new_user.username),
                employee_id: None,
                department_id: None,
                title: new_user.title,
                gitlab_url: None,
                gitlab_pat: None,
                jira_url: None,
                jira_email: None,
                jira_pat: None,
                tempo_token: None,
                is_active: true,
                is_admin: new_user.is_admin,
                created_at: now,
                updated_at: now,
            };
            self.users.lock().unwrap().insert(user.id.clone(), user.clone());
            Ok(user)
        }
    }

    // ========================================================================
    // get_app_status Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_app_status_no_users() {
        let repo = MockUserRepository::new();

        let status = get_app_status_impl(&repo).await.unwrap();

        assert!(!status.has_users);
        assert_eq!(status.user_count, 0);
        assert!(status.first_user.is_none());
        assert!(status.local_mode);
    }

    #[tokio::test]
    async fn test_get_app_status_with_users() {
        let user = MockUserRepository::create_test_user("user-1", "testuser", "hash");
        let repo = MockUserRepository::new().with_user(user);

        let status = get_app_status_impl(&repo).await.unwrap();

        assert!(status.has_users);
        assert_eq!(status.user_count, 1);
        assert!(status.first_user.is_some());
        assert_eq!(status.first_user.unwrap().username, Some("testuser".to_string()));
    }

    // ========================================================================
    // register_user Tests
    // ========================================================================

    #[tokio::test]
    async fn test_register_user_success() {
        let repo = MockUserRepository::new();

        let request = RegisterRequest {
            username: "newuser".to_string(),
            password: "password123".to_string(),
            name: "New User".to_string(),
            email: Some("new@example.com".to_string()),
            title: Some("Developer".to_string()),
        };

        let result = register_user_impl(&repo, request).await.unwrap();

        assert_eq!(result.username, Some("newuser".to_string()));
        assert_eq!(result.name, "New User");
        assert!(result.is_admin); // First user should be admin
    }

    #[tokio::test]
    async fn test_register_user_generates_email() {
        let repo = MockUserRepository::new();

        let request = RegisterRequest {
            username: "localuser".to_string(),
            password: "password123".to_string(),
            name: "Local User".to_string(),
            email: None, // No email provided
            title: None,
        };

        let result = register_user_impl(&repo, request).await.unwrap();

        assert_eq!(result.email, "localuser@local");
    }

    #[tokio::test]
    async fn test_register_user_duplicate_username() {
        let user = MockUserRepository::create_test_user("user-1", "existing", "hash");
        let repo = MockUserRepository::new().with_user(user);

        let request = RegisterRequest {
            username: "existing".to_string(),
            password: "password123".to_string(),
            name: "Duplicate User".to_string(),
            email: Some("new@example.com".to_string()),
            title: None,
        };

        let result = register_user_impl(&repo, request).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Username already exists");
    }

    #[tokio::test]
    async fn test_register_user_duplicate_email() {
        let mut user = MockUserRepository::create_test_user("user-1", "existing", "hash");
        user.email = "taken@example.com".to_string();
        let repo = MockUserRepository::new().with_user(user);

        let request = RegisterRequest {
            username: "newuser".to_string(),
            password: "password123".to_string(),
            name: "New User".to_string(),
            email: Some("taken@example.com".to_string()),
            title: None,
        };

        let result = register_user_impl(&repo, request).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Email already registered");
    }

    #[tokio::test]
    async fn test_register_second_user_not_admin() {
        let first_user = MockUserRepository::create_test_user("user-1", "first", "hash");
        let repo = MockUserRepository::new().with_user(first_user);

        let request = RegisterRequest {
            username: "second".to_string(),
            password: "password123".to_string(),
            name: "Second User".to_string(),
            email: Some("second@example.com".to_string()),
            title: None,
        };

        let result = register_user_impl(&repo, request).await.unwrap();

        assert!(!result.is_admin); // Second user should NOT be admin
    }

    // ========================================================================
    // login Tests
    // ========================================================================

    #[tokio::test]
    async fn test_login_success() {
        // Create user with known password hash
        let password = "correctpassword";
        let password_hash = hash_password(password).unwrap();
        let user = MockUserRepository::create_test_user("user-1", "testuser", &password_hash);
        let repo = MockUserRepository::new().with_user(user);

        let request = LoginRequest {
            username: "testuser".to_string(),
            password: password.to_string(),
        };

        let result = login_impl(&repo, request).await.unwrap();

        assert!(!result.access_token.is_empty());
        assert_eq!(result.token_type, "bearer");
        assert_eq!(result.expires_in, 7 * 24 * 60 * 60);
    }

    #[tokio::test]
    async fn test_login_invalid_username() {
        let repo = MockUserRepository::new();

        let request = LoginRequest {
            username: "nonexistent".to_string(),
            password: "password".to_string(),
        };

        let result = login_impl(&repo, request).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid credentials");
    }

    #[tokio::test]
    async fn test_login_invalid_password() {
        let password_hash = hash_password("correctpassword").unwrap();
        let user = MockUserRepository::create_test_user("user-1", "testuser", &password_hash);
        let repo = MockUserRepository::new().with_user(user);

        let request = LoginRequest {
            username: "testuser".to_string(),
            password: "wrongpassword".to_string(),
        };

        let result = login_impl(&repo, request).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid credentials");
    }

    #[tokio::test]
    async fn test_login_disabled_account() {
        let password_hash = hash_password("password").unwrap();
        let mut user = MockUserRepository::create_test_user("user-1", "testuser", &password_hash);
        user.is_active = false; // Disable account
        let repo = MockUserRepository::new().with_user(user);

        let request = LoginRequest {
            username: "testuser".to_string(),
            password: "password".to_string(),
        };

        let result = login_impl(&repo, request).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Account is disabled");
    }

    // ========================================================================
    // auto_login Tests
    // ========================================================================

    #[tokio::test]
    async fn test_auto_login_success() {
        let user = MockUserRepository::create_test_user("user-1", "testuser", "hash");
        let repo = MockUserRepository::new().with_user(user);

        let result = auto_login_impl(&repo).await.unwrap();

        assert!(!result.access_token.is_empty());
        assert_eq!(result.token_type, "bearer");
    }

    #[tokio::test]
    async fn test_auto_login_no_users() {
        let repo = MockUserRepository::new();

        let result = auto_login_impl(&repo).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No user found");
    }

    #[tokio::test]
    async fn test_auto_login_disabled_user() {
        let mut user = MockUserRepository::create_test_user("user-1", "testuser", "hash");
        user.is_active = false;
        let repo = MockUserRepository::new().with_user(user);

        let result = auto_login_impl(&repo).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Account is disabled");
    }

    // ========================================================================
    // get_current_user Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_current_user_success() {
        let user = MockUserRepository::create_test_user("user-1", "testuser", "hash");
        let repo = MockUserRepository::new().with_user(user.clone());

        // Create a valid token for this user
        let token = create_token(&user).unwrap();

        let result = get_current_user_impl(&repo, &token).await.unwrap();

        assert_eq!(result.id, "user-1");
        assert_eq!(result.username, Some("testuser".to_string()));
    }

    #[tokio::test]
    async fn test_get_current_user_invalid_token() {
        let repo = MockUserRepository::new();

        let result = get_current_user_impl(&repo, "invalid-token").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_current_user_user_not_found() {
        let user = MockUserRepository::create_test_user("user-1", "testuser", "hash");
        // Create token but don't add user to repo
        let token = create_token(&user).unwrap();
        let repo = MockUserRepository::new(); // Empty repo

        let result = get_current_user_impl(&repo, &token).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "User not found");
    }
}
