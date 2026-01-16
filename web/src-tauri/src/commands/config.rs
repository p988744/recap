//! Config commands
//!
//! Tauri commands for configuration operations.
//! Uses trait-based dependency injection for testability.

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::State;

use recap_core::auth::verify_token;

use super::AppState;

// ============================================================================
// Request/Response types
// ============================================================================

#[derive(Debug, Clone, Serialize, Default)]
pub struct ConfigResponse {
    // Jira settings
    pub jira_url: Option<String>,
    pub auth_type: String,
    pub jira_configured: bool,
    pub tempo_configured: bool,

    // LLM settings
    pub llm_provider: String,
    pub llm_model: String,
    pub llm_base_url: Option<String>,
    pub llm_configured: bool,

    // Work settings
    pub daily_work_hours: f64,
    pub normalize_hours: bool,

    // GitLab settings
    pub gitlab_url: Option<String>,
    pub gitlab_configured: bool,

    // Git repos
    pub use_git_mode: bool,
    pub git_repos: Vec<String>,
    pub outlook_enabled: bool,
}

#[derive(Debug, Clone, Default)]
pub struct UserConfigRow {
    pub jira_url: Option<String>,
    pub jira_pat: Option<String>,
    pub jira_email: Option<String>,
    pub tempo_token: Option<String>,
    pub gitlab_url: Option<String>,
    pub gitlab_pat: Option<String>,
    pub llm_provider: Option<String>,
    pub llm_model: Option<String>,
    pub llm_api_key: Option<String>,
    pub llm_base_url: Option<String>,
    pub daily_work_hours: Option<f64>,
    pub normalize_hours: Option<bool>,
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for UserConfigRow {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Self {
            jira_url: row.try_get("jira_url")?,
            jira_pat: row.try_get("jira_pat")?,
            jira_email: row.try_get("jira_email")?,
            tempo_token: row.try_get("tempo_token")?,
            gitlab_url: row.try_get("gitlab_url")?,
            gitlab_pat: row.try_get("gitlab_pat")?,
            llm_provider: row.try_get("llm_provider")?,
            llm_model: row.try_get("llm_model")?,
            llm_api_key: row.try_get("llm_api_key")?,
            llm_base_url: row.try_get("llm_base_url")?,
            daily_work_hours: row.try_get("daily_work_hours")?,
            normalize_hours: row.try_get("normalize_hours")?,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateConfigRequest {
    pub daily_work_hours: Option<f64>,
    pub normalize_hours: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateLlmConfigRequest {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateJiraConfigRequest {
    pub jira_url: Option<String>,
    pub jira_pat: Option<String>,
    pub jira_email: Option<String>,
    pub jira_api_token: Option<String>,
    pub auth_type: Option<String>,
    pub tempo_api_token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

// ============================================================================
// Repository Trait
// ============================================================================

/// Config repository trait - abstracts database operations for testability
#[async_trait]
pub trait ConfigRepository: Send + Sync {
    /// Get user configuration
    async fn get_user_config(&self, user_id: &str) -> Result<UserConfigRow, String>;

    /// Update daily work hours
    async fn update_daily_work_hours(&self, user_id: &str, hours: f64) -> Result<(), String>;

    /// Update normalize hours setting
    async fn update_normalize_hours(&self, user_id: &str, normalize: bool) -> Result<(), String>;

    /// Update LLM configuration
    async fn update_llm_config(
        &self,
        user_id: &str,
        provider: &str,
        model: &str,
        api_key: Option<&str>,
        base_url: Option<&str>,
    ) -> Result<(), String>;

    /// Update Jira URL
    async fn update_jira_url(&self, user_id: &str, url: &str) -> Result<(), String>;

    /// Update Jira PAT auth (clears email)
    async fn update_jira_pat_auth(&self, user_id: &str, pat: &str) -> Result<(), String>;

    /// Update Jira basic auth API token
    async fn update_jira_api_token(&self, user_id: &str, api_token: &str) -> Result<(), String>;

    /// Update Jira email
    async fn update_jira_email(&self, user_id: &str, email: &str) -> Result<(), String>;

    /// Update Tempo token
    async fn update_tempo_token(&self, user_id: &str, token: &str) -> Result<(), String>;
}

// ============================================================================
// SQLite Repository Implementation (Production)
// ============================================================================

/// SQLite implementation of ConfigRepository
pub struct SqliteConfigRepository<'a> {
    pool: &'a sqlx::SqlitePool,
}

impl<'a> SqliteConfigRepository<'a> {
    pub fn new(pool: &'a sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl<'a> ConfigRepository for SqliteConfigRepository<'a> {
    async fn get_user_config(&self, user_id: &str) -> Result<UserConfigRow, String> {
        sqlx::query_as(
            r#"SELECT
                jira_url, jira_pat, jira_email, tempo_token,
                gitlab_url, gitlab_pat,
                llm_provider, llm_model, llm_api_key, llm_base_url,
                daily_work_hours, normalize_hours
            FROM users WHERE id = ?"#,
        )
        .bind(user_id)
        .fetch_one(self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    async fn update_daily_work_hours(&self, user_id: &str, hours: f64) -> Result<(), String> {
        let now = Utc::now();
        sqlx::query("UPDATE users SET daily_work_hours = ?, updated_at = ? WHERE id = ?")
            .bind(hours)
            .bind(now)
            .bind(user_id)
            .execute(self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn update_normalize_hours(&self, user_id: &str, normalize: bool) -> Result<(), String> {
        let now = Utc::now();
        sqlx::query("UPDATE users SET normalize_hours = ?, updated_at = ? WHERE id = ?")
            .bind(normalize)
            .bind(now)
            .bind(user_id)
            .execute(self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn update_llm_config(
        &self,
        user_id: &str,
        provider: &str,
        model: &str,
        api_key: Option<&str>,
        base_url: Option<&str>,
    ) -> Result<(), String> {
        let now = Utc::now();
        sqlx::query(
            r#"UPDATE users SET
                llm_provider = ?,
                llm_model = ?,
                llm_api_key = ?,
                llm_base_url = ?,
                updated_at = ?
            WHERE id = ?"#,
        )
        .bind(provider)
        .bind(model)
        .bind(api_key)
        .bind(base_url)
        .bind(now)
        .bind(user_id)
        .execute(self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn update_jira_url(&self, user_id: &str, url: &str) -> Result<(), String> {
        let now = Utc::now();
        sqlx::query("UPDATE users SET jira_url = ?, updated_at = ? WHERE id = ?")
            .bind(url)
            .bind(now)
            .bind(user_id)
            .execute(self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn update_jira_pat_auth(&self, user_id: &str, pat: &str) -> Result<(), String> {
        let now = Utc::now();
        sqlx::query("UPDATE users SET jira_pat = ?, jira_email = NULL, updated_at = ? WHERE id = ?")
            .bind(pat)
            .bind(now)
            .bind(user_id)
            .execute(self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn update_jira_api_token(&self, user_id: &str, api_token: &str) -> Result<(), String> {
        let now = Utc::now();
        sqlx::query("UPDATE users SET jira_pat = ?, updated_at = ? WHERE id = ?")
            .bind(api_token)
            .bind(now)
            .bind(user_id)
            .execute(self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn update_jira_email(&self, user_id: &str, email: &str) -> Result<(), String> {
        let now = Utc::now();
        sqlx::query("UPDATE users SET jira_email = ?, updated_at = ? WHERE id = ?")
            .bind(email)
            .bind(now)
            .bind(user_id)
            .execute(self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn update_tempo_token(&self, user_id: &str, token: &str) -> Result<(), String> {
        let now = Utc::now();
        sqlx::query("UPDATE users SET tempo_token = ?, updated_at = ? WHERE id = ?")
            .bind(token)
            .bind(now)
            .bind(user_id)
            .execute(self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

// ============================================================================
// Pure Business Logic (Testable without repository)
// ============================================================================

/// Determine auth type based on configured credentials
pub(crate) fn determine_auth_type(jira_email: &Option<String>, jira_pat: &Option<String>) -> String {
    if jira_email.is_some() && jira_pat.is_some() {
        "basic".to_string()
    } else if jira_pat.is_some() {
        "pat".to_string()
    } else {
        "none".to_string()
    }
}

/// Validate LLM provider
pub(crate) fn validate_llm_provider(provider: &str) -> Result<(), String> {
    let valid_providers = ["openai", "anthropic", "ollama", "openai-compatible"];
    if valid_providers.contains(&provider) {
        Ok(())
    } else {
        Err("Invalid LLM provider".to_string())
    }
}

/// Get default base URL for Ollama if not provided
pub(crate) fn get_ollama_base_url(provider: &str, base_url: Option<String>) -> Option<String> {
    if provider == "ollama" && base_url.is_none() {
        Some("http://localhost:11434".to_string())
    } else {
        base_url
    }
}

/// Build ConfigResponse from UserConfigRow
pub(crate) fn build_config_response(user: &UserConfigRow) -> ConfigResponse {
    ConfigResponse {
        jira_url: user.jira_url.clone(),
        auth_type: determine_auth_type(&user.jira_email, &user.jira_pat),
        jira_configured: user.jira_pat.is_some(),
        tempo_configured: user.tempo_token.is_some(),

        llm_provider: user
            .llm_provider
            .clone()
            .unwrap_or_else(|| "openai".to_string()),
        llm_model: user
            .llm_model
            .clone()
            .unwrap_or_else(|| "gpt-4o-mini".to_string()),
        llm_base_url: user.llm_base_url.clone(),
        llm_configured: user.llm_api_key.is_some(),

        daily_work_hours: user.daily_work_hours.unwrap_or(8.0),
        normalize_hours: user.normalize_hours.unwrap_or(true),

        gitlab_url: user.gitlab_url.clone(),
        gitlab_configured: user.gitlab_pat.is_some(),

        use_git_mode: false,
        git_repos: vec![],
        outlook_enabled: false,
    }
}

// ============================================================================
// Core Business Logic (Testable, uses trait)
// ============================================================================

/// Get config - testable business logic
pub async fn get_config_impl<R: ConfigRepository>(
    repo: &R,
    token: &str,
) -> Result<ConfigResponse, String> {
    let claims = verify_token(token).map_err(|e| e.to_string())?;
    let user = repo.get_user_config(&claims.sub).await?;
    Ok(build_config_response(&user))
}

/// Update config - testable business logic
pub async fn update_config_impl<R: ConfigRepository>(
    repo: &R,
    token: &str,
    request: UpdateConfigRequest,
) -> Result<MessageResponse, String> {
    let claims = verify_token(token).map_err(|e| e.to_string())?;

    if let Some(hours) = request.daily_work_hours {
        repo.update_daily_work_hours(&claims.sub, hours).await?;
    }

    if let Some(normalize) = request.normalize_hours {
        repo.update_normalize_hours(&claims.sub, normalize).await?;
    }

    Ok(MessageResponse {
        message: "Config updated".to_string(),
    })
}

/// Update LLM config - testable business logic
pub async fn update_llm_config_impl<R: ConfigRepository>(
    repo: &R,
    token: &str,
    request: UpdateLlmConfigRequest,
) -> Result<MessageResponse, String> {
    let claims = verify_token(token).map_err(|e| e.to_string())?;

    // Validate provider
    validate_llm_provider(&request.provider)?;

    // For Ollama, default base_url if not provided
    let base_url = get_ollama_base_url(&request.provider, request.base_url);

    repo.update_llm_config(
        &claims.sub,
        &request.provider,
        &request.model,
        request.api_key.as_deref(),
        base_url.as_deref(),
    )
    .await?;

    Ok(MessageResponse {
        message: "LLM configuration updated".to_string(),
    })
}

/// Update Jira config - testable business logic
pub async fn update_jira_config_impl<R: ConfigRepository>(
    repo: &R,
    token: &str,
    request: UpdateJiraConfigRequest,
) -> Result<MessageResponse, String> {
    let claims = verify_token(token).map_err(|e| e.to_string())?;

    // Update Jira URL if provided
    if let Some(url) = &request.jira_url {
        repo.update_jira_url(&claims.sub, url).await?;
    }

    // Update auth credentials based on auth type
    if request.auth_type.as_deref() == Some("pat") {
        if let Some(pat) = &request.jira_pat {
            repo.update_jira_pat_auth(&claims.sub, pat).await?;
        }
    } else if request.auth_type.as_deref() == Some("basic") {
        if let Some(api_token) = &request.jira_api_token {
            repo.update_jira_api_token(&claims.sub, api_token).await?;
        }
        if let Some(email) = &request.jira_email {
            repo.update_jira_email(&claims.sub, email).await?;
        }
    }

    // Update Tempo token if provided
    if let Some(tempo_token) = &request.tempo_api_token {
        repo.update_tempo_token(&claims.sub, tempo_token).await?;
    }

    Ok(MessageResponse {
        message: "Jira configuration updated".to_string(),
    })
}

// ============================================================================
// Tauri Commands (Thin wrappers)
// ============================================================================

/// Get current user configuration
#[tauri::command]
pub async fn get_config(
    state: State<'_, AppState>,
    token: String,
) -> Result<ConfigResponse, String> {
    let db = state.db.lock().await;
    let repo = SqliteConfigRepository::new(&db.pool);
    get_config_impl(&repo, &token).await
}

/// Update general config settings
#[tauri::command]
pub async fn update_config(
    state: State<'_, AppState>,
    token: String,
    request: UpdateConfigRequest,
) -> Result<MessageResponse, String> {
    let db = state.db.lock().await;
    let repo = SqliteConfigRepository::new(&db.pool);
    update_config_impl(&repo, &token, request).await
}

/// Update LLM configuration
#[tauri::command]
pub async fn update_llm_config(
    state: State<'_, AppState>,
    token: String,
    request: UpdateLlmConfigRequest,
) -> Result<MessageResponse, String> {
    let db = state.db.lock().await;
    let repo = SqliteConfigRepository::new(&db.pool);
    update_llm_config_impl(&repo, &token, request).await
}

/// Update Jira configuration
#[tauri::command]
pub async fn update_jira_config(
    state: State<'_, AppState>,
    token: String,
    request: UpdateJiraConfigRequest,
) -> Result<MessageResponse, String> {
    let db = state.db.lock().await;
    let repo = SqliteConfigRepository::new(&db.pool);
    update_jira_config_impl(&repo, &token, request).await
}

// ============================================================================
// Tests with Mock Repository
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use recap_core::auth::create_token;
    use std::sync::Mutex;

    // ========================================================================
    // Mock Repository
    // ========================================================================

    pub struct MockConfigRepository {
        config: Mutex<Option<UserConfigRow>>,
        should_fail: bool,
    }

    impl MockConfigRepository {
        pub fn new() -> Self {
            Self {
                config: Mutex::new(None),
                should_fail: false,
            }
        }

        pub fn with_config(self, config: UserConfigRow) -> Self {
            *self.config.lock().unwrap() = Some(config);
            self
        }

        pub fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }

        fn check_failure(&self) -> Result<(), String> {
            if self.should_fail {
                Err("Database error".to_string())
            } else {
                Ok(())
            }
        }
    }

    #[async_trait]
    impl ConfigRepository for MockConfigRepository {
        async fn get_user_config(&self, _user_id: &str) -> Result<UserConfigRow, String> {
            self.check_failure()?;
            self.config
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| "User not found".to_string())
        }

        async fn update_daily_work_hours(&self, _user_id: &str, hours: f64) -> Result<(), String> {
            self.check_failure()?;
            if let Some(config) = self.config.lock().unwrap().as_mut() {
                config.daily_work_hours = Some(hours);
            }
            Ok(())
        }

        async fn update_normalize_hours(
            &self,
            _user_id: &str,
            normalize: bool,
        ) -> Result<(), String> {
            self.check_failure()?;
            if let Some(config) = self.config.lock().unwrap().as_mut() {
                config.normalize_hours = Some(normalize);
            }
            Ok(())
        }

        async fn update_llm_config(
            &self,
            _user_id: &str,
            provider: &str,
            model: &str,
            api_key: Option<&str>,
            base_url: Option<&str>,
        ) -> Result<(), String> {
            self.check_failure()?;
            if let Some(config) = self.config.lock().unwrap().as_mut() {
                config.llm_provider = Some(provider.to_string());
                config.llm_model = Some(model.to_string());
                config.llm_api_key = api_key.map(|s| s.to_string());
                config.llm_base_url = base_url.map(|s| s.to_string());
            }
            Ok(())
        }

        async fn update_jira_url(&self, _user_id: &str, url: &str) -> Result<(), String> {
            self.check_failure()?;
            if let Some(config) = self.config.lock().unwrap().as_mut() {
                config.jira_url = Some(url.to_string());
            }
            Ok(())
        }

        async fn update_jira_pat_auth(&self, _user_id: &str, pat: &str) -> Result<(), String> {
            self.check_failure()?;
            if let Some(config) = self.config.lock().unwrap().as_mut() {
                config.jira_pat = Some(pat.to_string());
                config.jira_email = None;
            }
            Ok(())
        }

        async fn update_jira_api_token(
            &self,
            _user_id: &str,
            api_token: &str,
        ) -> Result<(), String> {
            self.check_failure()?;
            if let Some(config) = self.config.lock().unwrap().as_mut() {
                config.jira_pat = Some(api_token.to_string());
            }
            Ok(())
        }

        async fn update_jira_email(&self, _user_id: &str, email: &str) -> Result<(), String> {
            self.check_failure()?;
            if let Some(config) = self.config.lock().unwrap().as_mut() {
                config.jira_email = Some(email.to_string());
            }
            Ok(())
        }

        async fn update_tempo_token(&self, _user_id: &str, token: &str) -> Result<(), String> {
            self.check_failure()?;
            if let Some(config) = self.config.lock().unwrap().as_mut() {
                config.tempo_token = Some(token.to_string());
            }
            Ok(())
        }
    }

    // Test user helper
    fn create_test_user() -> crate::models::User {
        crate::models::User {
            id: "user-1".to_string(),
            email: "test@test.com".to_string(),
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
    // Pure Function Tests
    // ========================================================================

    #[test]
    fn test_determine_auth_type_basic() {
        let email = Some("test@test.com".to_string());
        let pat = Some("pat123".to_string());
        assert_eq!(determine_auth_type(&email, &pat), "basic");
    }

    #[test]
    fn test_determine_auth_type_pat() {
        let email = None;
        let pat = Some("pat123".to_string());
        assert_eq!(determine_auth_type(&email, &pat), "pat");
    }

    #[test]
    fn test_determine_auth_type_none() {
        let email: Option<String> = None;
        let pat: Option<String> = None;
        assert_eq!(determine_auth_type(&email, &pat), "none");
    }

    #[test]
    fn test_validate_llm_provider_valid() {
        assert!(validate_llm_provider("openai").is_ok());
        assert!(validate_llm_provider("anthropic").is_ok());
        assert!(validate_llm_provider("ollama").is_ok());
        assert!(validate_llm_provider("openai-compatible").is_ok());
    }

    #[test]
    fn test_validate_llm_provider_invalid() {
        assert!(validate_llm_provider("invalid").is_err());
        assert!(validate_llm_provider("").is_err());
    }

    #[test]
    fn test_get_ollama_base_url_default() {
        let result = get_ollama_base_url("ollama", None);
        assert_eq!(result, Some("http://localhost:11434".to_string()));
    }

    #[test]
    fn test_get_ollama_base_url_custom() {
        let result = get_ollama_base_url("ollama", Some("http://custom:1234".to_string()));
        assert_eq!(result, Some("http://custom:1234".to_string()));
    }

    #[test]
    fn test_get_ollama_base_url_non_ollama() {
        let result = get_ollama_base_url("openai", None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_build_config_response_defaults() {
        let config = UserConfigRow::default();
        let response = build_config_response(&config);

        assert_eq!(response.auth_type, "none");
        assert!(!response.jira_configured);
        assert!(!response.tempo_configured);
        assert_eq!(response.llm_provider, "openai");
        assert_eq!(response.llm_model, "gpt-4o-mini");
        assert!(!response.llm_configured);
        assert_eq!(response.daily_work_hours, 8.0);
        assert!(response.normalize_hours);
        assert!(!response.gitlab_configured);
    }

    #[test]
    fn test_build_config_response_configured() {
        let config = UserConfigRow {
            jira_url: Some("https://jira.example.com".to_string()),
            jira_pat: Some("pat123".to_string()),
            jira_email: Some("test@example.com".to_string()),
            tempo_token: Some("tempo123".to_string()),
            gitlab_url: Some("https://gitlab.example.com".to_string()),
            gitlab_pat: Some("gitlab_pat".to_string()),
            llm_provider: Some("anthropic".to_string()),
            llm_model: Some("claude-3".to_string()),
            llm_api_key: Some("sk-123".to_string()),
            llm_base_url: None,
            daily_work_hours: Some(7.5),
            normalize_hours: Some(false),
        };
        let response = build_config_response(&config);

        assert_eq!(response.auth_type, "basic");
        assert!(response.jira_configured);
        assert!(response.tempo_configured);
        assert_eq!(response.llm_provider, "anthropic");
        assert_eq!(response.llm_model, "claude-3");
        assert!(response.llm_configured);
        assert_eq!(response.daily_work_hours, 7.5);
        assert!(!response.normalize_hours);
        assert!(response.gitlab_configured);
    }

    // ========================================================================
    // get_config Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_config_success() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let config = UserConfigRow {
            llm_provider: Some("openai".to_string()),
            llm_model: Some("gpt-4".to_string()),
            daily_work_hours: Some(8.0),
            ..Default::default()
        };
        let repo = MockConfigRepository::new().with_config(config);

        let result = get_config_impl(&repo, &token).await.unwrap();

        assert_eq!(result.llm_provider, "openai");
        assert_eq!(result.llm_model, "gpt-4");
        assert_eq!(result.daily_work_hours, 8.0);
    }

    #[tokio::test]
    async fn test_get_config_invalid_token() {
        let repo = MockConfigRepository::new();

        let result = get_config_impl(&repo, "invalid-token").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_config_user_not_found() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let repo = MockConfigRepository::new(); // No config

        let result = get_config_impl(&repo, &token).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "User not found");
    }

    // ========================================================================
    // update_config Tests
    // ========================================================================

    #[tokio::test]
    async fn test_update_config_daily_hours() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let config = UserConfigRow::default();
        let repo = MockConfigRepository::new().with_config(config);

        let request = UpdateConfigRequest {
            daily_work_hours: Some(7.5),
            normalize_hours: None,
        };

        let result = update_config_impl(&repo, &token, request).await.unwrap();

        assert_eq!(result.message, "Config updated");
    }

    #[tokio::test]
    async fn test_update_config_normalize_hours() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let config = UserConfigRow::default();
        let repo = MockConfigRepository::new().with_config(config);

        let request = UpdateConfigRequest {
            daily_work_hours: None,
            normalize_hours: Some(false),
        };

        let result = update_config_impl(&repo, &token, request).await.unwrap();

        assert_eq!(result.message, "Config updated");
    }

    #[tokio::test]
    async fn test_update_config_invalid_token() {
        let repo = MockConfigRepository::new();
        let request = UpdateConfigRequest::default();

        let result = update_config_impl(&repo, "invalid", request).await;

        assert!(result.is_err());
    }

    // ========================================================================
    // update_llm_config Tests
    // ========================================================================

    #[tokio::test]
    async fn test_update_llm_config_openai() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let config = UserConfigRow::default();
        let repo = MockConfigRepository::new().with_config(config);

        let request = UpdateLlmConfigRequest {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            api_key: Some("sk-123".to_string()),
            base_url: None,
        };

        let result = update_llm_config_impl(&repo, &token, request).await.unwrap();

        assert_eq!(result.message, "LLM configuration updated");
    }

    #[tokio::test]
    async fn test_update_llm_config_ollama_default_url() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let config = UserConfigRow::default();
        let repo = MockConfigRepository::new().with_config(config);

        let request = UpdateLlmConfigRequest {
            provider: "ollama".to_string(),
            model: "llama2".to_string(),
            api_key: None,
            base_url: None, // Should default to localhost
        };

        let result = update_llm_config_impl(&repo, &token, request).await.unwrap();

        assert_eq!(result.message, "LLM configuration updated");
    }

    #[tokio::test]
    async fn test_update_llm_config_invalid_provider() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let config = UserConfigRow::default();
        let repo = MockConfigRepository::new().with_config(config);

        let request = UpdateLlmConfigRequest {
            provider: "invalid".to_string(),
            model: "model".to_string(),
            api_key: None,
            base_url: None,
        };

        let result = update_llm_config_impl(&repo, &token, request).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid LLM provider");
    }

    #[tokio::test]
    async fn test_update_llm_config_invalid_token() {
        let repo = MockConfigRepository::new();
        let request = UpdateLlmConfigRequest {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            api_key: None,
            base_url: None,
        };

        let result = update_llm_config_impl(&repo, "invalid", request).await;

        assert!(result.is_err());
    }

    // ========================================================================
    // update_jira_config Tests
    // ========================================================================

    #[tokio::test]
    async fn test_update_jira_config_url() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let config = UserConfigRow::default();
        let repo = MockConfigRepository::new().with_config(config);

        let request = UpdateJiraConfigRequest {
            jira_url: Some("https://jira.example.com".to_string()),
            ..Default::default()
        };

        let result = update_jira_config_impl(&repo, &token, request)
            .await
            .unwrap();

        assert_eq!(result.message, "Jira configuration updated");
    }

    #[tokio::test]
    async fn test_update_jira_config_pat_auth() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let config = UserConfigRow::default();
        let repo = MockConfigRepository::new().with_config(config);

        let request = UpdateJiraConfigRequest {
            auth_type: Some("pat".to_string()),
            jira_pat: Some("my-pat-token".to_string()),
            ..Default::default()
        };

        let result = update_jira_config_impl(&repo, &token, request)
            .await
            .unwrap();

        assert_eq!(result.message, "Jira configuration updated");
    }

    #[tokio::test]
    async fn test_update_jira_config_basic_auth() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let config = UserConfigRow::default();
        let repo = MockConfigRepository::new().with_config(config);

        let request = UpdateJiraConfigRequest {
            auth_type: Some("basic".to_string()),
            jira_api_token: Some("api-token".to_string()),
            jira_email: Some("test@example.com".to_string()),
            ..Default::default()
        };

        let result = update_jira_config_impl(&repo, &token, request)
            .await
            .unwrap();

        assert_eq!(result.message, "Jira configuration updated");
    }

    #[tokio::test]
    async fn test_update_jira_config_tempo_token() {
        let user = create_test_user();
        let token = create_token(&user).unwrap();
        let config = UserConfigRow::default();
        let repo = MockConfigRepository::new().with_config(config);

        let request = UpdateJiraConfigRequest {
            tempo_api_token: Some("tempo-token".to_string()),
            ..Default::default()
        };

        let result = update_jira_config_impl(&repo, &token, request)
            .await
            .unwrap();

        assert_eq!(result.message, "Jira configuration updated");
    }

    #[tokio::test]
    async fn test_update_jira_config_invalid_token() {
        let repo = MockConfigRepository::new();
        let request = UpdateJiraConfigRequest::default();

        let result = update_jira_config_impl(&repo, "invalid", request).await;

        assert!(result.is_err());
    }
}
