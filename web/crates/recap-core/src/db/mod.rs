//! Database module - SQLx with SQLite

use crate::error::{Error, Result};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::PathBuf;

/// Database state
#[derive(Clone)]
pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    /// Create a new database connection with default path
    pub async fn new() -> Result<Self> {
        let db_path = get_db_path()?;
        Self::open(db_path).await
    }

    /// Create a new database connection with a specific path
    pub async fn open(db_path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
        log::info!("Connecting to database: {}", db_path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;

        Ok(db)
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<()> {
        log::info!("Running database migrations...");

        // Create users table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                name TEXT NOT NULL,
                employee_id TEXT,
                department_id TEXT,
                title TEXT,
                gitlab_url TEXT,
                gitlab_pat TEXT,
                jira_url TEXT,
                jira_email TEXT,
                jira_pat TEXT,
                tempo_token TEXT,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                is_admin BOOLEAN NOT NULL DEFAULT 0,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create work_items table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS work_items (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT 'manual',
                source_id TEXT,
                source_url TEXT,
                title TEXT NOT NULL,
                description TEXT,
                hours REAL NOT NULL DEFAULT 0,
                date DATE NOT NULL,
                jira_issue_key TEXT,
                jira_issue_suggested TEXT,
                jira_issue_title TEXT,
                category TEXT,
                tags TEXT,
                yearly_goal_id TEXT,
                synced_to_tempo BOOLEAN NOT NULL DEFAULT 0,
                tempo_worklog_id TEXT,
                synced_at DATETIME,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create gitlab_projects table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS gitlab_projects (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                gitlab_project_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                path_with_namespace TEXT NOT NULL,
                gitlab_url TEXT NOT NULL,
                default_branch TEXT NOT NULL DEFAULT 'main',
                enabled BOOLEAN NOT NULL DEFAULT 1,
                last_synced DATETIME,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id),
                UNIQUE(user_id, gitlab_project_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create git_repos table for local Git repositories
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS git_repos (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                path TEXT NOT NULL,
                name TEXT NOT NULL,
                enabled BOOLEAN NOT NULL DEFAULT 1,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id),
                UNIQUE(user_id, path)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create index for git_repos
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_git_repos_user_id ON git_repos(user_id)")
            .execute(&self.pool)
            .await?;

        // Add source_mode column to users table
        sqlx::query("ALTER TABLE users ADD COLUMN source_mode TEXT DEFAULT 'claude'")
            .execute(&self.pool)
            .await
            .ok(); // Ignore error if column already exists

        // Migration: Add parent_id column for grouping
        sqlx::query(
            "ALTER TABLE work_items ADD COLUMN parent_id TEXT REFERENCES work_items(id)"
        )
        .execute(&self.pool)
        .await
        .ok(); // Ignore error if column already exists

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_work_items_user_id ON work_items(user_id)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_work_items_date ON work_items(date)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_work_items_parent_id ON work_items(parent_id)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_gitlab_projects_user_id ON gitlab_projects(user_id)")
            .execute(&self.pool)
            .await?;

        // Add content_hash column for deduplication
        sqlx::query("ALTER TABLE work_items ADD COLUMN content_hash TEXT")
            .execute(&self.pool)
            .await
            .ok(); // Ignore error if column already exists

        // Create unique index on content_hash + user_id for deduplication
        sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_work_items_content_hash ON work_items(user_id, content_hash) WHERE content_hash IS NOT NULL")
            .execute(&self.pool)
            .await?;

        // Create index for GitLab deduplication (source + source_id)
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_work_items_source_source_id ON work_items(source, source_id)")
            .execute(&self.pool)
            .await?;

        // Create composite index for date-based filtering by source
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_work_items_source_date ON work_items(source, date)")
            .execute(&self.pool)
            .await?;

        // Create composite index for user + date queries (most common)
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_work_items_user_date ON work_items(user_id, date)")
            .execute(&self.pool)
            .await?;

        // Add LLM configuration fields to users table
        sqlx::query("ALTER TABLE users ADD COLUMN llm_provider TEXT DEFAULT 'openai'")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("ALTER TABLE users ADD COLUMN llm_model TEXT DEFAULT 'gpt-4o-mini'")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("ALTER TABLE users ADD COLUMN llm_api_key TEXT")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("ALTER TABLE users ADD COLUMN llm_base_url TEXT")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("ALTER TABLE users ADD COLUMN daily_work_hours REAL DEFAULT 8.0")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("ALTER TABLE users ADD COLUMN normalize_hours BOOLEAN DEFAULT 1")
            .execute(&self.pool)
            .await
            .ok();

        // Add username column for login (separate from display name)
        sqlx::query("ALTER TABLE users ADD COLUMN username TEXT")
            .execute(&self.pool)
            .await
            .ok();

        // For existing users, set username = name if username is null
        sqlx::query("UPDATE users SET username = name WHERE username IS NULL")
            .execute(&self.pool)
            .await
            .ok();

        // Create unique index on username
        sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_users_username ON users(username)")
            .execute(&self.pool)
            .await
            .ok();

        // Create sync_status table for tracking auto-sync state
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sync_status (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                source TEXT NOT NULL,
                source_path TEXT,
                last_sync_at DATETIME,
                last_item_count INTEGER DEFAULT 0,
                status TEXT DEFAULT 'idle',
                error_message TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create index on sync_status for faster lookups
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_sync_status_user_source ON sync_status(user_id, source)")
            .execute(&self.pool)
            .await?;

        // Add hours tracking columns for commit-centric worklog
        sqlx::query("ALTER TABLE work_items ADD COLUMN hours_source TEXT DEFAULT 'manual'")
            .execute(&self.pool)
            .await
            .ok(); // 'user_modified' | 'session' | 'commit_interval' | 'heuristic' | 'manual'

        sqlx::query("ALTER TABLE work_items ADD COLUMN hours_estimated REAL")
            .execute(&self.pool)
            .await
            .ok(); // System-calculated hours (preserved even if user overrides)

        sqlx::query("ALTER TABLE work_items ADD COLUMN commit_hash TEXT")
            .execute(&self.pool)
            .await
            .ok(); // Git commit hash for commit-based items

        sqlx::query("ALTER TABLE work_items ADD COLUMN session_id TEXT")
            .execute(&self.pool)
            .await
            .ok(); // Claude session ID for session-based items

        // Create index for commit hash lookups
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_work_items_commit_hash ON work_items(commit_hash) WHERE commit_hash IS NOT NULL")
            .execute(&self.pool)
            .await
            .ok();

        // Add start_time and end_time columns for session timing (Timeline support)
        sqlx::query("ALTER TABLE work_items ADD COLUMN start_time TEXT")
            .execute(&self.pool)
            .await
            .ok(); // ISO 8601 timestamp for session start

        sqlx::query("ALTER TABLE work_items ADD COLUMN end_time TEXT")
            .execute(&self.pool)
            .await
            .ok(); // ISO 8601 timestamp for session end

        // Add project_path column for better filtering
        sqlx::query("ALTER TABLE work_items ADD COLUMN project_path TEXT")
            .execute(&self.pool)
            .await
            .ok();

        // Create index for timeline queries (source + date + start_time)
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_work_items_timeline ON work_items(user_id, date, start_time) WHERE start_time IS NOT NULL")
            .execute(&self.pool)
            .await
            .ok();

        log::info!("Database migrations completed");
        Ok(())
    }
}

/// Get database file path
/// Priority: RECAP_DB_PATH env var > default app data directory
pub fn get_db_path() -> Result<PathBuf> {
    // Check for environment variable override
    if let Ok(path) = std::env::var("RECAP_DB_PATH") {
        return Ok(PathBuf::from(path));
    }

    // Default: use app data directory
    let dirs = directories::ProjectDirs::from("com", "recap", "Recap")
        .ok_or_else(|| Error::config("Could not determine project directories"))?;

    Ok(dirs.data_dir().join("recap.db"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to ensure env var tests don't run in parallel
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_get_db_path_default() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // Without env var, should return default path
        std::env::remove_var("RECAP_DB_PATH");
        let path = get_db_path().unwrap();
        assert!(path.to_string_lossy().contains("recap.db"));
    }

    #[test]
    fn test_get_db_path_env_override() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let test_path = "/tmp/test_recap.db";
        std::env::set_var("RECAP_DB_PATH", test_path);
        let path = get_db_path().unwrap();
        assert_eq!(path.to_string_lossy(), test_path);
        std::env::remove_var("RECAP_DB_PATH");
    }
}
