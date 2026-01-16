//! Auth module tests
//!
//! Unit tests using mock repository for testability.

use async_trait::async_trait;
use chrono::Utc;
use recap_core::auth::{create_token, hash_password};
use std::collections::HashMap;
use std::sync::Mutex;

use crate::models::User;
use super::repository::UserRepository;
use super::service::{
    auto_login_impl, get_app_status_impl, get_current_user_impl, login_impl, register_user_impl,
};
use super::types::{LoginRequest, NewUser, RegisterRequest};

// ============================================================================
// Mock Repository
// ============================================================================

/// Mock implementation of UserRepository for testing
pub struct MockUserRepository {
    users: Mutex<HashMap<String, User>>,
}

impl MockUserRepository {
    pub fn new() -> Self {
        Self {
            users: Mutex::new(HashMap::new()),
        }
    }

    /// Add a test user to the mock repository
    pub fn with_user(self, user: User) -> Self {
        self.users.lock().unwrap().insert(user.id.clone(), user);
        self
    }

    /// Create a test user with minimal required fields
    pub fn create_test_user(id: &str, username: &str, password_hash: &str) -> User {
        User {
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

    async fn get_first_user(&self) -> Result<Option<User>, String> {
        let users = self.users.lock().unwrap();
        let first = users.values().min_by_key(|u| u.created_at);
        Ok(first.cloned())
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, String> {
        let users = self.users.lock().unwrap();
        let user = users.values().find(|u| u.username.as_deref() == Some(username));
        Ok(user.cloned())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<User>, String> {
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

    async fn create_user(&self, new_user: NewUser) -> Result<User, String> {
        let now = Utc::now();
        let user = User {
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

// ============================================================================
// get_app_status Tests
// ============================================================================

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

// ============================================================================
// register_user Tests
// ============================================================================

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

// ============================================================================
// login Tests
// ============================================================================

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

// ============================================================================
// auto_login Tests
// ============================================================================

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

// ============================================================================
// get_current_user Tests
// ============================================================================

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
