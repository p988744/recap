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

        // Enable WAL mode for better concurrent read/write performance
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&pool)
            .await?;

        // Set busy timeout to 5 seconds — retry on SQLITE_BUSY instead of failing immediately
        sqlx::query("PRAGMA busy_timeout = 5000")
            .execute(&pool)
            .await?;

        // Synchronous NORMAL is safe with WAL and faster than FULL
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&pool)
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

        // Create project_preferences table for project visibility management
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS project_preferences (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                project_name TEXT NOT NULL,
                project_path TEXT,
                hidden BOOLEAN DEFAULT 0,
                display_name TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, project_name)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_project_prefs_user ON project_preferences(user_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_project_prefs_hidden ON project_preferences(user_id, hidden)")
            .execute(&self.pool)
            .await?;

        // Add claude_session_path column to users table (default: ~/.claude)
        sqlx::query("ALTER TABLE users ADD COLUMN claude_session_path TEXT")
            .execute(&self.pool)
            .await
            .ok();

        // Add git_repo_path column to project_preferences (for manual projects)
        sqlx::query("ALTER TABLE project_preferences ADD COLUMN git_repo_path TEXT")
            .execute(&self.pool)
            .await
            .ok();

        // Add manual_added flag to project_preferences
        sqlx::query("ALTER TABLE project_preferences ADD COLUMN manual_added BOOLEAN DEFAULT 0")
            .execute(&self.pool)
            .await
            .ok();

        // Add antigravity_session_path column to users table (default: ~/.gemini/antigravity)
        sqlx::query("ALTER TABLE users ADD COLUMN antigravity_session_path TEXT")
            .execute(&self.pool)
            .await
            .ok();

        // Add timezone and week_start_day columns to users table
        sqlx::query("ALTER TABLE users ADD COLUMN timezone TEXT")
            .execute(&self.pool)
            .await
            .ok(); // NULL = system default
        sqlx::query("ALTER TABLE users ADD COLUMN week_start_day INTEGER DEFAULT 1")
            .execute(&self.pool)
            .await
            .ok(); // 0=Sun, 1=Mon, ..., 6=Sat

        // Add onboarding_completed column to track if user has completed initial setup
        sqlx::query("ALTER TABLE users ADD COLUMN onboarding_completed BOOLEAN DEFAULT 0")
            .execute(&self.pool)
            .await
            .ok();

        // Add background sync config columns
        sqlx::query("ALTER TABLE users ADD COLUMN sync_enabled BOOLEAN DEFAULT 1")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("ALTER TABLE users ADD COLUMN sync_interval_minutes INTEGER DEFAULT 15")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("ALTER TABLE users ADD COLUMN compaction_interval_minutes INTEGER DEFAULT 60")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("ALTER TABLE users ADD COLUMN auto_generate_summaries BOOLEAN DEFAULT 1")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("ALTER TABLE users ADD COLUMN sync_git BOOLEAN DEFAULT 1")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("ALTER TABLE users ADD COLUMN sync_claude BOOLEAN DEFAULT 1")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("ALTER TABLE users ADD COLUMN sync_antigravity BOOLEAN DEFAULT 1")
            .execute(&self.pool)
            .await
            .ok();

        // Add claude_oauth_token column for manual OAuth token fallback
        // Used when automatic credential discovery (Keychain/file) fails
        sqlx::query("ALTER TABLE users ADD COLUMN claude_oauth_token TEXT")
            .execute(&self.pool)
            .await
            .ok();

        // Create snapshot_raw_data table for hourly session snapshots
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS snapshot_raw_data (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                session_id TEXT NOT NULL,
                project_path TEXT NOT NULL,
                hour_bucket TEXT NOT NULL,
                user_messages TEXT,
                assistant_messages TEXT,
                tool_calls TEXT,
                files_modified TEXT,
                git_commits TEXT,
                message_count INTEGER DEFAULT 0,
                raw_size_bytes INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(session_id, hour_bucket)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_snapshots_user_hour ON snapshot_raw_data(user_id, hour_bucket)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_snapshots_session ON snapshot_raw_data(session_id)")
            .execute(&self.pool)
            .await?;

        // Create work_summaries table for compacted summaries at multiple time scales
        // NOTE: This table stores AUTOMATIC compaction results from snapshot_raw_data
        // - Used by: Worklog page, background compaction service
        // - Scale: hourly, daily, weekly, monthly
        // - Key: (user_id, project_path, scale, period_start)
        // - Source: snapshot_raw_data → compaction service
        // See also: project_summaries (for LLM-generated reports from work_items)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS work_summaries (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                project_path TEXT,
                scale TEXT NOT NULL,
                period_start TEXT NOT NULL,
                period_end TEXT NOT NULL,
                summary TEXT NOT NULL,
                key_activities TEXT,
                git_commits_summary TEXT,
                previous_context TEXT,
                source_snapshot_ids TEXT,
                llm_model TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, project_path, scale, period_start)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_summaries_user_scale ON work_summaries(user_id, scale, period_start)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_summaries_project ON work_summaries(project_path, scale)")
            .execute(&self.pool)
            .await?;

        // Create llm_usage_logs table for tracking LLM API token usage and costs
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS llm_usage_logs (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                prompt_tokens INTEGER,
                completion_tokens INTEGER,
                total_tokens INTEGER,
                estimated_cost REAL,
                purpose TEXT NOT NULL,
                duration_ms INTEGER,
                status TEXT NOT NULL DEFAULT 'success',
                error_message TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_llm_usage_user_date ON llm_usage_logs(user_id, created_at)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_llm_usage_provider ON llm_usage_logs(user_id, provider, created_at)")
            .execute(&self.pool)
            .await?;

        // Create project_issue_mappings table for Tempo sync
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS project_issue_mappings (
                project_path TEXT NOT NULL,
                user_id TEXT NOT NULL,
                jira_issue_key TEXT NOT NULL,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (project_path, user_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Project descriptions for AI context
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS project_descriptions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                project_name TEXT NOT NULL,
                goal TEXT,
                tech_stack TEXT,
                key_features TEXT,
                notes TEXT,
                orphaned BOOLEAN DEFAULT 0,
                orphaned_at DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, project_name)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Project summaries cache (LLM-generated reports from work_items)
        // NOTE: This table stores LLM-GENERATED summaries triggered manually
        // - Used by: Projects page report generation
        // - summary_type: "report" (專案報告) | "timeline" (時間軸摘要)
        // - time_unit: "day" | "week" | "month" | "quarter" | "year"
        // - Key: (user_id, project_name, summary_type, time_unit, period_start)
        // - Source: work_items → LLM service
        // See also: work_summaries (for automatic compaction from snapshot_raw_data)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS project_summaries (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                project_name TEXT NOT NULL,
                summary_type TEXT NOT NULL DEFAULT 'report',
                time_unit TEXT NOT NULL DEFAULT 'week',
                period_start DATE NOT NULL,
                period_end DATE NOT NULL,
                period_label TEXT,
                summary TEXT NOT NULL,
                data_hash TEXT,
                orphaned BOOLEAN DEFAULT 0,
                orphaned_at DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, project_name, summary_type, time_unit, period_start)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Migration: Add new columns if table exists with old schema
        sqlx::query("ALTER TABLE project_summaries ADD COLUMN summary_type TEXT NOT NULL DEFAULT 'report'")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("ALTER TABLE project_summaries ADD COLUMN time_unit TEXT NOT NULL DEFAULT 'week'")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("ALTER TABLE project_summaries ADD COLUMN period_label TEXT")
            .execute(&self.pool)
            .await
            .ok();

        // Migration: Fix unique constraint if table was created with old schema
        // SQLite doesn't support ALTER TABLE ADD CONSTRAINT, so we need to recreate the table
        self.migrate_project_summaries_unique_constraint().await?;

        // Create worklog_sync_records table for tracking Tempo sync status
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS worklog_sync_records (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                project_path TEXT NOT NULL,
                date TEXT NOT NULL,
                jira_issue_key TEXT NOT NULL,
                hours REAL NOT NULL,
                description TEXT,
                tempo_worklog_id TEXT,
                synced_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, project_path, date)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_sync_records_user_date ON worklog_sync_records(user_id, date)")
            .execute(&self.pool)
            .await?;

        // Create llm_batch_jobs table for tracking OpenAI Batch API jobs
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS llm_batch_jobs (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                openai_batch_id TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                purpose TEXT NOT NULL,
                total_requests INTEGER NOT NULL DEFAULT 0,
                completed_requests INTEGER NOT NULL DEFAULT 0,
                failed_requests INTEGER NOT NULL DEFAULT 0,
                input_file_id TEXT,
                output_file_id TEXT,
                error_file_id TEXT,
                error_message TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                submitted_at DATETIME,
                completed_at DATETIME,
                FOREIGN KEY (user_id) REFERENCES users(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_batch_jobs_user ON llm_batch_jobs(user_id, status)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_batch_jobs_openai ON llm_batch_jobs(openai_batch_id)")
            .execute(&self.pool)
            .await?;

        // Create llm_batch_requests table for mapping batch requests to compaction targets
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS llm_batch_requests (
                id TEXT PRIMARY KEY,
                batch_job_id TEXT NOT NULL,
                custom_id TEXT NOT NULL,
                project_path TEXT NOT NULL,
                hour_bucket TEXT NOT NULL,
                prompt TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                response TEXT,
                error_message TEXT,
                prompt_tokens INTEGER,
                completion_tokens INTEGER,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                completed_at DATETIME,
                FOREIGN KEY (batch_job_id) REFERENCES llm_batch_jobs(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_batch_requests_job ON llm_batch_requests(batch_job_id)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_batch_requests_custom ON llm_batch_requests(batch_job_id, custom_id)")
            .execute(&self.pool)
            .await?;

        // Create quota_snapshots table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS quota_snapshots (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                model TEXT,
                window_type TEXT NOT NULL,
                used_percent REAL NOT NULL,
                resets_at TEXT,
                extra_credits_used REAL,
                extra_credits_limit REAL,
                raw_response TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create index for quota queries
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_quota_provider_time
            ON quota_snapshots(user_id, provider, created_at)
            "#,
        )
        .execute(&self.pool)
        .await?;

        log::info!("[quota:db] quota_snapshots table created");

        log::info!("Database migrations completed");
        Ok(())
    }

    /// Migrate project_summaries table to add proper UNIQUE constraint
    /// SQLite doesn't support ALTER TABLE ADD CONSTRAINT, so we recreate the table
    async fn migrate_project_summaries_unique_constraint(&self) -> Result<()> {
        // Check if we need to migrate by trying to create a unique index
        // If it fails with "already exists", we're good
        // If it fails with constraint issues, we need to migrate
        let check_result = sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_project_summaries_unique
             ON project_summaries(user_id, project_name, summary_type, time_unit, period_start)"
        )
        .execute(&self.pool)
        .await;

        match check_result {
            Ok(_) => {
                // Index created or already exists with same definition - we're good
                log::info!("project_summaries unique constraint verified");
                return Ok(());
            }
            Err(e) => {
                let err_str = e.to_string();
                // If there's a constraint issue, we need to migrate
                if !err_str.contains("UNIQUE constraint") && !err_str.contains("already exists") {
                    log::info!("Migrating project_summaries table to fix unique constraint: {}", err_str);
                } else {
                    // Some other error, log and continue
                    log::warn!("project_summaries index check: {}", err_str);
                    return Ok(());
                }
            }
        }

        // Begin transaction for safe migration
        let mut tx = self.pool.begin().await?;

        // Step 1: Create new table with correct schema
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS project_summaries_new (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                project_name TEXT NOT NULL,
                summary_type TEXT NOT NULL DEFAULT 'report',
                time_unit TEXT NOT NULL DEFAULT 'week',
                period_start DATE NOT NULL,
                period_end DATE NOT NULL,
                period_label TEXT,
                summary TEXT NOT NULL,
                data_hash TEXT,
                orphaned BOOLEAN DEFAULT 0,
                orphaned_at DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, project_name, summary_type, time_unit, period_start)
            )
            "#,
        )
        .execute(&mut *tx)
        .await?;

        // Step 2: Copy data (handle duplicates by taking the latest one)
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO project_summaries_new
            SELECT * FROM project_summaries
            WHERE id IN (
                SELECT id FROM project_summaries p1
                WHERE created_at = (
                    SELECT MAX(created_at) FROM project_summaries p2
                    WHERE p1.user_id = p2.user_id
                    AND p1.project_name = p2.project_name
                    AND COALESCE(p1.summary_type, 'report') = COALESCE(p2.summary_type, 'report')
                    AND COALESCE(p1.time_unit, 'week') = COALESCE(p2.time_unit, 'week')
                    AND p1.period_start = p2.period_start
                )
            )
            "#,
        )
        .execute(&mut *tx)
        .await?;

        // Step 3: Drop old table
        sqlx::query("DROP TABLE project_summaries")
            .execute(&mut *tx)
            .await?;

        // Step 4: Rename new table
        sqlx::query("ALTER TABLE project_summaries_new RENAME TO project_summaries")
            .execute(&mut *tx)
            .await?;

        // Commit transaction
        tx.commit().await?;

        log::info!("Successfully migrated project_summaries table with proper unique constraint");
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
