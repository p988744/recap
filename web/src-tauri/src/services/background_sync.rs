//! Background Sync Service
//!
//! Provides scheduled automatic synchronization of work items from various sources.
//! Runs in the background while the app is in the system tray.
//!
//! This service uses the `SyncSource` trait abstraction from `recap_core::services::sources`
//! to dynamically discover and sync work items from multiple data sources.

use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;
use tokio::sync::{Mutex, RwLock};
use tokio::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};

use recap_core::services::sources::{SyncConfig, SourceSyncResult};

// =============================================================================
// Configuration
// =============================================================================

/// Background sync configuration
///
/// This struct maintains backward compatibility with the legacy field-based
/// configuration while internally converting to the new `SyncConfig` system.
///
/// Tasks are separated into two categories:
/// 1. **Data Sync** (frequent): Discovery and extraction from sources
/// 2. **Data Compaction** (periodic): Hierarchical summary generation
#[derive(Debug, Clone)]
pub struct BackgroundSyncConfig {
    /// Whether background sync is enabled
    pub enabled: bool,
    /// Data sync interval in minutes (5, 15, 30, 60)
    pub interval_minutes: u32,
    /// Data compaction interval in minutes (30, 60, 180, 360, 720, 1440)
    pub compaction_interval_minutes: u32,
    /// Sync local Git repositories
    pub sync_git: bool,
    /// Sync Claude Code sessions
    pub sync_claude: bool,
    /// Sync Antigravity (Gemini Code) sessions
    pub sync_antigravity: bool,
    /// Sync GitLab (requires configuration)
    pub sync_gitlab: bool,
    /// Sync Jira/Tempo (requires configuration)
    pub sync_jira: bool,
    /// Auto-generate timeline summaries for completed periods
    pub auto_generate_summaries: bool,
}

impl Default for BackgroundSyncConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_minutes: 15,
            compaction_interval_minutes: 30,
            sync_git: true,
            sync_claude: true,
            sync_antigravity: true,
            sync_gitlab: false,
            sync_jira: false,
            auto_generate_summaries: true,
        }
    }
}

impl BackgroundSyncConfig {
    /// Convert to the new SyncConfig format
    pub fn to_sync_config(&self) -> SyncConfig {
        SyncConfig::from_legacy(
            self.enabled,
            self.interval_minutes,
            self.sync_claude,
            self.sync_antigravity,
            self.sync_git,
            self.sync_gitlab,
            self.sync_jira,
        )
    }
}

// =============================================================================
// Service Lifecycle
// =============================================================================

/// Service lifecycle states
///
/// ```text
///                    ┌──────────┐
///                    │  Created │
///                    └────┬─────┘
///                         │ start()
///                         ▼
///   stop()          ┌──────────┐
/// ┌─────────────────│   Idle   │◄────────────────┐
/// │                 └────┬─────┘                 │
/// │                      │ begin_sync()          │
/// │                      ▼                       │
/// │                 ┌──────────┐                 │
/// │                 │ Syncing  │─────────────────┘
/// │                 └────┬─────┘  complete_sync()
/// │                      │
/// │                      │ (unrecoverable error)
/// │                      ▼
/// │                 ┌──────────┐
/// └────────────────►│ Stopped  │
///                   └──────────┘
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceLifecycle {
    /// Service created but not started
    Created,
    /// Running and waiting for next sync
    Idle {
        last_sync_at: Option<String>,
        next_sync_at: Option<String>,
    },
    /// Currently performing sync
    Syncing {
        started_at: String,
    },
    /// Service stopped
    Stopped,
}

impl Default for ServiceLifecycle {
    fn default() -> Self {
        Self::Created
    }
}

impl ServiceLifecycle {
    /// Check if service is running (Idle or Syncing)
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Idle { .. } | Self::Syncing { .. })
    }

    /// Check if currently syncing
    pub fn is_syncing(&self) -> bool {
        matches!(self, Self::Syncing { .. })
    }

    /// Get last sync timestamp
    pub fn last_sync_at(&self) -> Option<&str> {
        match self {
            Self::Idle { last_sync_at, .. } => last_sync_at.as_deref(),
            _ => None,
        }
    }

    /// Get next sync timestamp
    pub fn next_sync_at(&self) -> Option<&str> {
        match self {
            Self::Idle { next_sync_at, .. } => next_sync_at.as_deref(),
            _ => None,
        }
    }

    /// Transition: Created/Stopped -> Idle
    pub fn start(self, next_sync_at: Option<String>) -> Result<Self, ServiceLifecycleError> {
        match self {
            Self::Created | Self::Stopped => Ok(Self::Idle {
                last_sync_at: None,
                next_sync_at,
            }),
            Self::Idle { .. } => Err(ServiceLifecycleError::AlreadyRunning),
            Self::Syncing { .. } => Err(ServiceLifecycleError::SyncInProgress),
        }
    }

    /// Transition: Idle -> Syncing
    pub fn begin_sync(self) -> Result<Self, ServiceLifecycleError> {
        match self {
            Self::Idle { .. } => Ok(Self::Syncing {
                started_at: chrono::Utc::now().to_rfc3339(),
            }),
            Self::Created => Err(ServiceLifecycleError::NotStarted),
            Self::Stopped => Err(ServiceLifecycleError::ServiceStopped),
            Self::Syncing { .. } => Err(ServiceLifecycleError::SyncInProgress),
        }
    }

    /// Transition: Syncing -> Idle
    pub fn complete_sync(self, next_sync_at: Option<String>) -> Result<Self, ServiceLifecycleError> {
        match self {
            Self::Syncing { started_at } => {
                let _ = started_at; // Use started_at for logging if needed
                Ok(Self::Idle {
                    last_sync_at: Some(chrono::Utc::now().to_rfc3339()),
                    next_sync_at,
                })
            }
            _ => Err(ServiceLifecycleError::NotSyncing),
        }
    }

    /// Update next_sync_at time (only effective in Idle state)
    pub fn update_next_sync_at(self, next_sync_at: Option<String>) -> Self {
        match self {
            Self::Idle { last_sync_at, .. } => Self::Idle {
                last_sync_at,
                next_sync_at,
            },
            other => other,
        }
    }

    /// Transition: Any -> Stopped
    pub fn stop(self) -> Self {
        Self::Stopped
    }
}

/// Lifecycle transition errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceLifecycleError {
    /// Service is already running
    AlreadyRunning,
    /// Service has not been started
    NotStarted,
    /// Service is stopped
    ServiceStopped,
    /// A sync operation is already in progress
    SyncInProgress,
    /// No sync operation is in progress
    NotSyncing,
}

impl std::fmt::Display for ServiceLifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyRunning => write!(f, "服務已經在運行中"),
            Self::NotStarted => write!(f, "服務尚未啟動"),
            Self::ServiceStopped => write!(f, "服務已停止"),
            Self::SyncInProgress => write!(f, "同步正在進行中"),
            Self::NotSyncing => write!(f, "目前沒有同步在進行"),
        }
    }
}

impl std::error::Error for ServiceLifecycleError {}

// =============================================================================
// Sync Status (for API response)
// =============================================================================

/// Current status of the background sync service (API response)
#[derive(Debug, Clone, Default)]
pub struct SyncServiceStatus {
    /// Whether the service is currently running
    pub is_running: bool,
    /// Whether a data sync is in progress
    pub is_syncing: bool,
    /// Whether compaction is in progress
    pub is_compacting: bool,
    /// Last data sync timestamp (ISO 8601)
    pub last_sync_at: Option<String>,
    /// Last compaction timestamp (ISO 8601)
    pub last_compaction_at: Option<String>,
    /// Next scheduled sync timestamp (ISO 8601)
    pub next_sync_at: Option<String>,
    /// Next scheduled compaction timestamp (ISO 8601)
    pub next_compaction_at: Option<String>,
    /// Last sync result message
    pub last_result: Option<String>,
    /// Last error message (if any)
    pub last_error: Option<String>,
}

impl SyncServiceStatus {
    /// Create from lifecycle state
    pub fn from_lifecycle(
        lifecycle: &ServiceLifecycle,
        is_compacting: bool,
        last_compaction_at: Option<String>,
        next_compaction_at: Option<String>,
        last_result: Option<String>,
        last_error: Option<String>,
    ) -> Self {
        Self {
            is_running: lifecycle.is_running(),
            is_syncing: lifecycle.is_syncing(),
            is_compacting,
            last_sync_at: lifecycle.last_sync_at().map(|s| s.to_string()),
            last_compaction_at,
            next_sync_at: lifecycle.next_sync_at().map(|s| s.to_string()),
            next_compaction_at,
            last_result,
            last_error,
        }
    }
}

// =============================================================================
// Sync Result
// =============================================================================

/// Result of a single sync operation
#[derive(Debug, Clone, Default)]
pub struct SyncOperationResult {
    pub source: String,
    pub success: bool,
    pub items_synced: i32,
    pub projects_scanned: i32,
    pub items_created: i32,
    pub error: Option<String>,
}

impl From<SourceSyncResult> for SyncOperationResult {
    fn from(result: SourceSyncResult) -> Self {
        let items_synced = (result.work_items_created + result.work_items_updated) as i32;
        Self {
            source: result.source,
            success: result.error.is_none(),
            items_synced,
            projects_scanned: result.projects_scanned as i32,
            items_created: result.work_items_created as i32,
            error: result.error,
        }
    }
}

// =============================================================================
// Background Sync Service
// =============================================================================

/// Background sync service that manages scheduled synchronization
///
/// Uses a formal lifecycle state machine to ensure correct state transitions.
/// All sync operations must go through `execute_sync` to guarantee proper
/// state management.
///
/// Tasks are separated:
/// - **Data Sync**: Frequent (every N minutes) - discovery and extraction
/// - **Data Compaction**: Periodic (every N hours) - hierarchical summary generation
pub struct BackgroundSyncService {
    /// Current configuration
    config: Arc<RwLock<BackgroundSyncConfig>>,
    /// Service lifecycle state (single source of truth)
    lifecycle: Arc<RwLock<ServiceLifecycle>>,
    /// Last data sync timestamp (persisted separately for display during Syncing state)
    last_sync_at: Arc<RwLock<Option<String>>>,
    /// Last compaction timestamp
    last_compaction_at: Arc<RwLock<Option<String>>>,
    /// Next scheduled compaction timestamp
    next_compaction_at: Arc<RwLock<Option<String>>>,
    /// Last sync result message
    last_result: Arc<RwLock<Option<String>>>,
    /// Last error message
    last_error: Arc<RwLock<Option<String>>>,
    /// Job scheduler instance
    scheduler: Arc<Mutex<Option<JobScheduler>>>,
    /// Data sync job ID (for querying next fire time)
    sync_job_id: Arc<RwLock<Option<uuid::Uuid>>>,
    /// Compaction job ID (for querying next fire time)
    compaction_job_id: Arc<RwLock<Option<uuid::Uuid>>>,
    /// Database connection for sync operations
    db: Arc<Mutex<recap_core::Database>>,
    /// User ID for sync operations
    user_id: Arc<RwLock<Option<String>>>,
    /// Whether compaction is currently in progress
    is_compacting: Arc<RwLock<bool>>,
}

impl BackgroundSyncService {
    /// Create a new background sync service
    pub fn new(db: Arc<Mutex<recap_core::Database>>) -> Self {
        Self {
            config: Arc::new(RwLock::new(BackgroundSyncConfig::default())),
            lifecycle: Arc::new(RwLock::new(ServiceLifecycle::Created)),
            last_sync_at: Arc::new(RwLock::new(None)),
            last_compaction_at: Arc::new(RwLock::new(None)),
            next_compaction_at: Arc::new(RwLock::new(None)),
            last_result: Arc::new(RwLock::new(None)),
            last_error: Arc::new(RwLock::new(None)),
            scheduler: Arc::new(Mutex::new(None)),
            sync_job_id: Arc::new(RwLock::new(None)),
            compaction_job_id: Arc::new(RwLock::new(None)),
            db,
            user_id: Arc::new(RwLock::new(None)),
            is_compacting: Arc::new(RwLock::new(false)),
        }
    }

    /// Get last compaction timestamp
    pub async fn get_last_compaction_at(&self) -> Option<String> {
        self.last_compaction_at.read().await.clone()
    }

    /// Record compaction completion
    ///
    /// Updates `last_compaction_at`. The `next_compaction_at` is managed by
    /// the scheduler — `refresh_next_times()` will query the real next fire time.
    pub async fn record_compaction_completed(&self) {
        let now = chrono::Utc::now().to_rfc3339();

        // Update last_compaction_at
        {
            let mut last = self.last_compaction_at.write().await;
            *last = Some(now);
        }

        // Refresh next_compaction_at from scheduler (if running)
        // Clone scheduler out of Mutex to avoid holding it across awaits
        let sched = {
            let guard = self.scheduler.lock().await;
            guard.clone()
        };
        if let Some(mut sched) = sched {
            if let Some(job_id) = *self.compaction_job_id.read().await {
                Self::update_next_compaction_from_scheduler(
                    &mut sched, job_id, &self.next_compaction_at
                ).await;
            }
        }
    }

    /// Initialize timestamps from database on startup
    ///
    /// This should be called after the service is created to restore
    /// the last known sync and compaction timestamps from persistent storage.
    ///
    /// - `last_sync_at`: Read from sync_status table (MAX of all sources)
    /// - `last_compaction_at`: Read from work_summaries table (MAX created_at)
    /// - `next_compaction_at`: Estimated from last_compaction_at + interval (scheduler overwrites once started)
    pub async fn initialize_timestamps_from_db(&self, user_id: &str) {
        let pool = {
            let db = self.db.lock().await;
            db.pool.clone()
        };

        let compaction_interval = self.config.read().await.compaction_interval_minutes;

        // Load last_sync_at from sync_status table
        let sync_result = sqlx::query_scalar::<_, Option<String>>(
            "SELECT MAX(last_sync_at) FROM sync_status WHERE user_id = ? AND last_sync_at IS NOT NULL"
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await;

        if let Ok(Some(last_sync)) = sync_result {
            log::info!("Restored last_sync_at from database: {}", last_sync);
            let mut sync_time = self.last_sync_at.write().await;
            *sync_time = Some(last_sync);
        }

        // Load last_compaction_at from work_summaries table (MAX created_at)
        let compaction_result = sqlx::query_scalar::<_, Option<String>>(
            "SELECT MAX(created_at) FROM work_summaries WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await;

        if let Ok(Some(last_compaction)) = compaction_result {
            log::info!("Restored last_compaction_at from database: {}", last_compaction);
            {
                let mut compaction_time = self.last_compaction_at.write().await;
                *compaction_time = Some(last_compaction.clone());
            }

            // Calculate next_compaction_at based on last + interval
            if let Ok(last_dt) = chrono::DateTime::parse_from_rfc3339(&last_compaction) {
                let next_dt = last_dt + chrono::Duration::minutes(compaction_interval as i64);
                let now = chrono::Utc::now();
                // If next time is in the past, schedule for now + interval
                let next_compaction = if next_dt < now {
                    Self::calculate_next_compaction(compaction_interval)
                } else {
                    next_dt.to_rfc3339()
                };
                log::info!("Calculated next_compaction_at: {}", next_compaction);
                let mut next_time = self.next_compaction_at.write().await;
                *next_time = Some(next_compaction);
            }
        } else {
            // No previous compaction, set next compaction to now + interval
            let next_compaction = Self::calculate_next_compaction(compaction_interval);
            log::info!("No previous compaction, setting next_compaction_at: {}", next_compaction);
            let mut next_time = self.next_compaction_at.write().await;
            *next_time = Some(next_compaction);
        }
    }

    /// Set the user ID for sync operations
    pub async fn set_user_id(&self, user_id: String) {
        let mut uid = self.user_id.write().await;
        *uid = Some(user_id);
    }

    /// Update the sync configuration
    pub async fn update_config(&self, new_config: BackgroundSyncConfig) {
        let mut config = self.config.write().await;
        let was_enabled = config.enabled;
        let old_interval = config.interval_minutes;
        *config = new_config.clone();
        drop(config);

        // Restart if interval changed or enabled state changed
        if new_config.enabled && (!was_enabled || new_config.interval_minutes != old_interval) {
            self.restart().await;
        } else if !new_config.enabled && was_enabled {
            self.stop().await;
        }
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> BackgroundSyncConfig {
        self.config.read().await.clone()
    }

    /// Get the current status (API response format)
    pub async fn get_status(&self) -> SyncServiceStatus {
        // Refresh next fire times from the scheduler before reading
        self.refresh_next_times().await;

        let lifecycle = self.lifecycle.read().await;
        let last_sync_at = self.last_sync_at.read().await.clone();
        let last_compaction_at = self.last_compaction_at.read().await.clone();
        let next_compaction_at = self.next_compaction_at.read().await.clone();
        let last_result = self.last_result.read().await.clone();
        let last_error = self.last_error.read().await.clone();
        let is_compacting = *self.is_compacting.read().await;

        SyncServiceStatus {
            is_running: lifecycle.is_running(),
            is_syncing: lifecycle.is_syncing(),
            is_compacting,
            last_sync_at,
            last_compaction_at,
            next_sync_at: lifecycle.next_sync_at().map(|s| s.to_string()),
            next_compaction_at,
            last_result,
            last_error,
        }
    }

    /// Update the current status (DEPRECATED: use execute_sync instead)
    /// This method exists for backward compatibility during migration.
    #[deprecated(note = "Use execute_sync for proper lifecycle management")]
    pub async fn update_status(&self, new_status: SyncServiceStatus) {
        // Update last_result and last_error
        {
            let mut result = self.last_result.write().await;
            *result = new_status.last_result;
        }
        {
            let mut error = self.last_error.write().await;
            *error = new_status.last_error;
        }

        // Try to reconcile lifecycle state with the status
        let mut lifecycle = self.lifecycle.write().await;
        if new_status.is_syncing && !lifecycle.is_syncing() {
            if let Ok(new_state) = lifecycle.clone().begin_sync() {
                *lifecycle = new_state;
            }
        } else if !new_status.is_syncing && lifecycle.is_syncing() {
            let interval = self.config.read().await.interval_minutes;
            if let Ok(new_state) = lifecycle.clone().complete_sync(Some(Self::calculate_next_sync(interval))) {
                *lifecycle = new_state;
            }
        }
    }

    /// Get the current lifecycle state
    pub async fn get_lifecycle(&self) -> ServiceLifecycle {
        self.lifecycle.read().await.clone()
    }

    /// Begin a sync operation (transition to Syncing state)
    ///
    /// Call this at the start of any sync operation. Must be paired with
    /// `complete_sync_operation` when the sync finishes.
    ///
    /// If the service is in Created or Stopped state, it will automatically
    /// transition to Idle first, then to Syncing.
    pub async fn begin_sync_operation(&self) -> Result<(), ServiceLifecycleError> {
        let interval = self.config.read().await.interval_minutes;
        let mut lifecycle = self.lifecycle.write().await;

        // Auto-start if in Created or Stopped state
        if matches!(*lifecycle, ServiceLifecycle::Created | ServiceLifecycle::Stopped) {
            log::info!("Auto-starting service for sync operation");
            *lifecycle = ServiceLifecycle::Idle {
                last_sync_at: None,
                next_sync_at: Some(Self::calculate_next_sync(interval)),
            };
        }

        // Now transition to Syncing
        match lifecycle.clone().begin_sync() {
            Ok(new_state) => {
                *lifecycle = new_state;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Complete a sync operation (transition back to Idle state)
    ///
    /// Call this when a sync operation finishes, regardless of success or failure.
    /// Records the results and errors for status reporting.
    pub async fn complete_sync_operation(&self, results: &[SyncOperationResult]) {
        let interval = self.config.read().await.interval_minutes;
        let now = chrono::Utc::now().to_rfc3339();

        // Update separate last_sync_at field (persists during all states)
        {
            let mut last_sync = self.last_sync_at.write().await;
            *last_sync = Some(now);
        }

        // Transition lifecycle
        {
            let mut lifecycle = self.lifecycle.write().await;
            if let Ok(new_state) = lifecycle.clone().complete_sync(Some(Self::calculate_next_sync(interval))) {
                *lifecycle = new_state;
            }
        }

        // Record results
        let total_projects: i32 = results.iter().map(|r| r.projects_scanned).sum();
        let total_created: i32 = results.iter().map(|r| r.items_created).sum();
        let errors: Vec<String> = results.iter().filter_map(|r| r.error.clone()).collect();

        {
            let mut last_result = self.last_result.write().await;
            *last_result = Some(format!("已掃描 {} 個專案，發現 {} 筆新資料", total_projects, total_created));
        }
        {
            let mut last_error = self.last_error.write().await;
            *last_error = if errors.is_empty() { None } else { Some(errors.join("; ")) };
        }
    }

    /// Start the background sync service
    ///
    /// Uses `tokio-cron-scheduler` to schedule two independent jobs:
    /// 1. **Data Sync Job** - Runs every N minutes for discovery and extraction
    /// 2. **Compaction Job** - Runs every N hours for hierarchical summary generation
    ///
    /// The scheduler manages timing independently of job execution duration,
    /// ensuring accurate "next sync" times even when sync operations take long.
    pub async fn start(&self) {
        let config = self.config.read().await;
        if !config.enabled {
            log::info!("Background sync is disabled, not starting");
            return;
        }

        let interval_minutes = config.interval_minutes;
        let compaction_interval_minutes = config.compaction_interval_minutes;
        let auto_generate_summaries = config.auto_generate_summaries;
        drop(config);

        // Transition lifecycle: Created/Stopped -> Idle
        {
            let mut lifecycle = self.lifecycle.write().await;
            match lifecycle.clone().start(Some(Self::calculate_next_sync(interval_minutes))) {
                Ok(new_state) => *lifecycle = new_state,
                Err(ServiceLifecycleError::AlreadyRunning) => {
                    log::info!("Background sync service is already running");
                    return;
                }
                Err(e) => {
                    log::warn!("Cannot start service: {}", e);
                    return;
                }
            }
        }

        // Create the job scheduler
        let sched = match JobScheduler::new().await {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to create job scheduler: {:?}", e);
                let mut lifecycle = self.lifecycle.write().await;
                *lifecycle = lifecycle.clone().stop();
                return;
            }
        };

        log::info!(
            "Starting background sync service: data sync every {}min, compaction every {}min",
            interval_minutes,
            compaction_interval_minutes
        );

        // ===== Job 1: Data Sync (frequent) =====
        let sync_job = {
            let config = Arc::clone(&self.config);
            let lifecycle = Arc::clone(&self.lifecycle);
            let last_sync_at = Arc::clone(&self.last_sync_at);
            let last_result = Arc::clone(&self.last_result);
            let last_error = Arc::clone(&self.last_error);
            let db = Arc::clone(&self.db);
            let user_id = Arc::clone(&self.user_id);
            let scheduler_ref = Arc::clone(&self.scheduler);
            let sync_job_id_ref = Arc::clone(&self.sync_job_id);

            Job::new_repeated_async(
                Duration::from_secs(interval_minutes as u64 * 60),
                move |_uuid, _lock| {
                    let config = Arc::clone(&config);
                    let lifecycle = Arc::clone(&lifecycle);
                    let last_sync_at = Arc::clone(&last_sync_at);
                    let last_result = Arc::clone(&last_result);
                    let last_error = Arc::clone(&last_error);
                    let db = Arc::clone(&db);
                    let user_id = Arc::clone(&user_id);
                    let scheduler_ref = Arc::clone(&scheduler_ref);
                    let sync_job_id_ref = Arc::clone(&sync_job_id_ref);

                    Box::pin(async move {
                        // Check config.enabled
                        let cfg = config.read().await;
                        if !cfg.enabled {
                            log::info!("Background sync disabled, skipping data sync tick");
                            return;
                        }
                        let sync_config = cfg.clone();
                        drop(cfg);

                        // Check user_id
                        let uid = user_id.read().await.clone();
                        let uid = match uid {
                            Some(id) => id,
                            None => {
                                log::warn!("No user ID set, skipping data sync");
                                return;
                            }
                        };

                        // Overlap prevention: skip if already syncing
                        {
                            let lc = lifecycle.read().await;
                            if lc.is_syncing() {
                                log::warn!("Previous sync still running, skipping this tick");
                                return;
                            }
                        }

                        // Perform sync
                        Self::perform_data_sync(
                            &db,
                            &lifecycle,
                            &last_sync_at,
                            &last_result,
                            &last_error,
                            &sync_config,
                            &uid,
                        ).await;

                        // Update next_sync_at from scheduler's real next fire time
                        // Clone scheduler out of Mutex, then query (avoids holding Mutex across await)
                        let sched = {
                            let guard = scheduler_ref.lock().await;
                            guard.clone()
                        };
                        if let (Some(mut sched), Some(job_id)) = (sched, *sync_job_id_ref.read().await) {
                            Self::update_next_sync_from_scheduler(&mut sched, job_id, &lifecycle).await;
                        }
                    }) as Pin<Box<dyn Future<Output = ()> + Send>>
                },
            )
        };

        let sync_job = match sync_job {
            Ok(job) => job,
            Err(e) => {
                log::error!("Failed to create data sync job: {:?}", e);
                let mut lifecycle = self.lifecycle.write().await;
                *lifecycle = lifecycle.clone().stop();
                return;
            }
        };

        let sync_id = match sched.add(sync_job).await {
            Ok(id) => {
                log::info!("Data sync job added with ID: {}", id);
                id
            }
            Err(e) => {
                log::error!("Failed to add data sync job: {:?}", e);
                let mut lifecycle = self.lifecycle.write().await;
                *lifecycle = lifecycle.clone().stop();
                return;
            }
        };

        // Store sync job ID
        {
            let mut id = self.sync_job_id.write().await;
            *id = Some(sync_id);
        }

        // ===== Job 2: Data Compaction (periodic) =====
        if auto_generate_summaries {
            let config = Arc::clone(&self.config);
            let db = Arc::clone(&self.db);
            let user_id = Arc::clone(&self.user_id);
            let last_compaction_at = Arc::clone(&self.last_compaction_at);
            let next_compaction_at = Arc::clone(&self.next_compaction_at);
            let is_compacting = Arc::clone(&self.is_compacting);
            let scheduler_ref = Arc::clone(&self.scheduler);
            let compaction_job_id_ref = Arc::clone(&self.compaction_job_id);

            // Set initial next_compaction_at
            {
                let next = Self::calculate_next_compaction(compaction_interval_minutes);
                let mut nca = self.next_compaction_at.write().await;
                *nca = Some(next);
            }

            let compaction_job = Job::new_repeated_async(
                Duration::from_secs(compaction_interval_minutes as u64 * 60),
                move |_uuid, _lock| {
                    let config = Arc::clone(&config);
                    let db = Arc::clone(&db);
                    let user_id = Arc::clone(&user_id);
                    let last_compaction_at = Arc::clone(&last_compaction_at);
                    let next_compaction_at = Arc::clone(&next_compaction_at);
                    let is_compacting = Arc::clone(&is_compacting);
                    let scheduler_ref = Arc::clone(&scheduler_ref);
                    let compaction_job_id_ref = Arc::clone(&compaction_job_id_ref);

                    Box::pin(async move {
                        // Check config
                        let cfg = config.read().await;
                        if !cfg.enabled || !cfg.auto_generate_summaries {
                            log::info!("Compaction disabled, skipping");
                            return;
                        }
                        drop(cfg);

                        // Check user_id
                        let uid = user_id.read().await.clone();
                        let uid = match uid {
                            Some(id) => id,
                            None => {
                                log::warn!("No user ID set, skipping compaction");
                                return;
                            }
                        };

                        // Overlap prevention: skip if already compacting
                        {
                            let compacting = is_compacting.read().await;
                            if *compacting {
                                log::warn!("Previous compaction still running, skipping this tick");
                                return;
                            }
                        }

                        // Perform compaction
                        Self::perform_compaction(
                            &db,
                            &last_compaction_at,
                            &is_compacting,
                            &uid,
                        ).await;

                        // Update next_compaction_at from scheduler's real next fire time
                        let sched = {
                            let guard = scheduler_ref.lock().await;
                            guard.clone()
                        };
                        if let (Some(mut sched), Some(job_id)) = (sched, *compaction_job_id_ref.read().await) {
                            Self::update_next_compaction_from_scheduler(&mut sched, job_id, &next_compaction_at).await;
                        }
                    }) as Pin<Box<dyn Future<Output = ()> + Send>>
                },
            );

            match compaction_job {
                Ok(job) => {
                    match sched.add(job).await {
                        Ok(id) => {
                            log::info!("Compaction job added with ID: {}", id);
                            let mut cid = self.compaction_job_id.write().await;
                            *cid = Some(id);
                        }
                        Err(e) => {
                            log::error!("Failed to add compaction job: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to create compaction job: {:?}", e);
                }
            }
        }

        // Start the scheduler
        if let Err(e) = sched.start().await {
            log::error!("Failed to start job scheduler: {:?}", e);
            let mut lifecycle = self.lifecycle.write().await;
            *lifecycle = lifecycle.clone().stop();
            return;
        }

        // Store scheduler instance
        {
            let mut guard = self.scheduler.lock().await;
            *guard = Some(sched);
        }

        log::info!("Background sync scheduler started successfully");
    }

    /// Stop the background sync service
    pub async fn stop(&self) {
        // Shutdown the scheduler (stops all jobs)
        let sched = {
            let mut guard = self.scheduler.lock().await;
            guard.take()
        };

        if let Some(mut sched) = sched {
            if let Err(e) = sched.shutdown().await {
                log::warn!("Error shutting down scheduler: {:?}", e);
            }
            log::info!("Job scheduler shut down");
        }

        // Clear job IDs
        {
            let mut id = self.sync_job_id.write().await;
            *id = None;
        }
        {
            let mut id = self.compaction_job_id.write().await;
            *id = None;
        }

        // Transition lifecycle to Stopped
        let mut lifecycle = self.lifecycle.write().await;
        *lifecycle = lifecycle.clone().stop();
    }

    /// Restart the background sync service
    pub async fn restart(&self) {
        self.stop().await;
        // Small delay to ensure clean shutdown
        tokio::time::sleep(Duration::from_millis(100)).await;
        self.start().await;
    }

    /// Trigger an immediate sync
    pub async fn trigger_sync(&self) -> Vec<SyncOperationResult> {
        let config = self.config.read().await.clone();
        let uid = self.user_id.read().await.clone();

        if uid.is_none() {
            log::warn!("No user ID set, cannot trigger sync");
            return vec![SyncOperationResult {
                source: "system".to_string(),
                success: false,
                items_synced: 0,
                error: Some("No user logged in".to_string()),
                ..Default::default()
            }];
        }

        Self::perform_sync_with_lifecycle(
            &self.db,
            &self.lifecycle,
            &self.last_sync_at,
            &self.last_result,
            &self.last_error,
            &config,
            &uid.unwrap(),
        ).await
    }

    /// Execute a sync operation with proper lifecycle management
    ///
    /// This is the ONLY way to perform sync operations. It guarantees:
    /// 1. Lifecycle state transition to Syncing at start
    /// 2. Lifecycle state transition to Idle at end
    /// 3. Proper error handling and result recording
    ///
    /// # Example
    /// ```ignore
    /// let results = service.execute_sync(|pool, user_id| async move {
    ///     // Your sync logic here
    ///     Ok(vec![SyncOperationResult::default()])
    /// }).await;
    /// ```
    pub async fn execute_sync<F, Fut>(&self, sync_fn: F) -> Result<Vec<SyncOperationResult>, String>
    where
        F: FnOnce(sqlx::SqlitePool, String) -> Fut,
        Fut: std::future::Future<Output = Result<Vec<SyncOperationResult>, String>>,
    {
        let uid = self.user_id.read().await.clone();
        if uid.is_none() {
            return Err("No user logged in".to_string());
        }
        let user_id = uid.unwrap();

        // Get pool
        let pool = {
            let db = self.db.lock().await;
            db.pool.clone()
        };

        // Transition to Syncing
        {
            let mut lifecycle = self.lifecycle.write().await;
            match lifecycle.clone().begin_sync() {
                Ok(new_state) => *lifecycle = new_state,
                Err(e) => return Err(e.to_string()),
            }
        }

        log::info!("Starting sync via execute_sync for user: {}", user_id);

        // Execute the sync function
        let result = sync_fn(pool, user_id).await;

        // Update last_sync_at (persists during all states)
        {
            let mut last_sync = self.last_sync_at.write().await;
            *last_sync = Some(chrono::Utc::now().to_rfc3339());
        }

        // Transition back to Idle and record results
        let interval = self.config.read().await.interval_minutes;
        {
            let mut lifecycle = self.lifecycle.write().await;
            if let Ok(new_state) = lifecycle.clone().complete_sync(Some(Self::calculate_next_sync(interval))) {
                *lifecycle = new_state;
            }
        }

        // Record result/error
        match &result {
            Ok(results) => {
                let total_projects: i32 = results.iter().map(|r| r.projects_scanned).sum();
                let total_created: i32 = results.iter().map(|r| r.items_created).sum();
                let errors: Vec<String> = results.iter().filter_map(|r| r.error.clone()).collect();

                {
                    let mut last_result = self.last_result.write().await;
                    *last_result = Some(format!("已掃描 {} 個專案，發現 {} 筆新資料", total_projects, total_created));
                }
                {
                    let mut last_error = self.last_error.write().await;
                    *last_error = if errors.is_empty() { None } else { Some(errors.join("; ")) };
                }
            }
            Err(e) => {
                let mut last_error = self.last_error.write().await;
                *last_error = Some(e.clone());
            }
        }

        log::info!("Sync completed via execute_sync");
        result
    }

    /// Perform data sync only (Phase 1: Sources, Phase 2: Snapshots)
    ///
    /// This is the frequent task that runs every N minutes.
    /// Does NOT include compaction or timeline summary generation.
    async fn perform_data_sync(
        db: &Arc<Mutex<recap_core::Database>>,
        lifecycle: &Arc<RwLock<ServiceLifecycle>>,
        last_sync_at: &Arc<RwLock<Option<String>>>,
        last_result: &Arc<RwLock<Option<String>>>,
        last_error: &Arc<RwLock<Option<String>>>,
        config: &BackgroundSyncConfig,
        user_id: &str,
    ) -> Vec<SyncOperationResult> {
        log::info!("========== 開始資料同步 ==========");
        log::info!("使用者: {}", user_id);
        log::info!("同步設定: Git={}, Claude={}, Antigravity={}, GitLab={}, Jira={}",
            config.sync_git, config.sync_claude, config.sync_antigravity,
            config.sync_gitlab, config.sync_jira);

        // Transition to Syncing
        {
            let mut lc = lifecycle.write().await;
            match lc.clone().begin_sync() {
                Ok(new_state) => *lc = new_state,
                Err(e) => {
                    log::warn!("Cannot begin sync: {}", e);
                    return vec![SyncOperationResult {
                        source: "system".to_string(),
                        success: false,
                        error: Some(e.to_string()),
                        ..Default::default()
                    }];
                }
            }
        }

        let mut results = Vec::new();

        // Clone pool immediately
        let pool = {
            let db_guard = db.lock().await;
            db_guard.pool.clone()
        };

        // Phase 1: Sync all enabled sources
        log::info!("---------- Phase 1: 同步資料來源 ----------");
        let sync_config = config.to_sync_config();
        let sources = recap_core::services::sources::get_enabled_sources(&sync_config).await;
        log::info!("已啟用的資料來源: {} 個", sources.len());

        for (idx, source) in sources.iter().enumerate() {
            log::info!("[{}/{}] 開始同步: {}", idx + 1, sources.len(), source.display_name());

            match source.sync_sessions(&pool, user_id).await {
                Ok(source_result) => {
                    let result = SyncOperationResult::from(source_result);
                    log::info!(
                        "[{}/{}] {} 同步完成: 掃描 {} 個專案, 發現 {} 筆資料, 新增 {} 筆",
                        idx + 1, sources.len(),
                        source.display_name(),
                        result.projects_scanned,
                        result.items_synced,
                        result.items_created
                    );
                    results.push(result);
                }
                Err(e) => {
                    log::error!("[{}/{}] {} 同步失敗: {}", idx + 1, sources.len(), source.display_name(), e);
                    results.push(SyncOperationResult {
                        source: source.source_name().to_string(),
                        success: false,
                        error: Some(e),
                        ..Default::default()
                    });
                }
            }
        }

        // Phase 2: Capture hourly snapshots
        log::info!("---------- Phase 2: 擷取快照 ----------");
        if config.sync_claude {
            let projects = recap_core::services::SyncService::discover_project_paths();
            log::info!("發現 {} 個專案需要擷取快照", projects.len());
            let mut snapshot_count = 0;
            let mut snapshot_errors = 0;
            for (idx, project) in projects.iter().enumerate() {
                match recap_core::services::snapshot::capture_snapshots_for_project(
                    &pool,
                    user_id,
                    project,
                )
                .await
                {
                    Ok(n) => {
                        if n > 0 {
                            log::info!("[{}/{}] {} 擷取 {} 個快照", idx + 1, projects.len(), project.name, n);
                        }
                        snapshot_count += n;
                    }
                    Err(e) => {
                        log::warn!("[{}/{}] {} 快照擷取失敗: {}", idx + 1, projects.len(), project.name, e);
                        snapshot_errors += 1;
                    }
                }
            }
            log::info!("快照擷取完成: {} 個成功, {} 個失敗", snapshot_count, snapshot_errors);
        } else {
            log::info!("Claude 同步未啟用，跳過快照擷取");
        }

        // Update last_sync_at (memory)
        let now = chrono::Utc::now();
        let now_str = now.to_rfc3339();
        {
            let mut sync_time = last_sync_at.write().await;
            *sync_time = Some(now_str.clone());
        }

        // Persist to database: update sync_status table
        let total_items: i32 = results.iter().map(|r| r.items_synced).sum();
        if let Err(e) = sqlx::query(
            r#"
            UPDATE sync_status
            SET status = 'success',
                last_sync_at = ?,
                last_item_count = ?,
                error_message = NULL,
                updated_at = ?
            WHERE user_id = ?
            "#
        )
        .bind(&now)
        .bind(total_items)
        .bind(&now)
        .bind(user_id)
        .execute(&pool)
        .await
        {
            log::warn!("Failed to persist sync status to database: {}", e);
        }

        // Transition back to Idle
        // Pass None for next_sync_at — the scheduler job closure will update it
        // with the real next fire time from the scheduler after this returns.
        {
            let mut lc = lifecycle.write().await;
            let next_sync: Option<String> = None;
            if let Ok(new_state) = lc.clone().complete_sync(next_sync) {
                *lc = new_state;
            }
        }

        // Record results
        let total_projects: i32 = results.iter().map(|r| r.projects_scanned).sum();
        let total_items: i32 = results.iter().map(|r| r.items_synced).sum();
        let total_created: i32 = results.iter().map(|r| r.items_created).sum();
        let errors: Vec<String> = results.iter().filter_map(|r| r.error.clone()).collect();

        {
            let mut result = last_result.write().await;
            *result = Some(format!("已掃描 {} 個專案，發現 {} 筆新資料", total_projects, total_created));
        }
        {
            let mut error = last_error.write().await;
            *error = if errors.is_empty() { None } else { Some(errors.join("; ")) };
        }

        // 單行摘要 log - 方便事後追蹤每次同步紀錄
        let now_local = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let next_sync = Self::calculate_next_sync(config.interval_minutes);
        let next_local = chrono::DateTime::parse_from_rfc3339(&next_sync)
            .map(|dt| dt.with_timezone(&chrono::Local).format("%H:%M:%S").to_string())
            .unwrap_or_else(|_| "N/A".to_string());

        if errors.is_empty() {
            log::info!(
                "[SYNC] {} | 來源:{} 專案:{} 資料:{} 新增:{} | 下次:{}",
                now_local, results.len(), total_projects, total_items, total_created, next_local
            );
        } else {
            log::warn!(
                "[SYNC] {} | 來源:{} 專案:{} 資料:{} 新增:{} 錯誤:{} | 下次:{}",
                now_local, results.len(), total_projects, total_items, total_created, errors.len(), next_local
            );
        }

        log::debug!("========== 資料同步結束 ==========");
        results
    }

    /// Perform data compaction (Phase 3: Hourly/Daily, Phase 4: Timeline Summaries)
    ///
    /// This is the periodic task that runs every N hours.
    /// Performs hierarchical compaction from small to large time units.
    async fn perform_compaction(
        db: &Arc<Mutex<recap_core::Database>>,
        last_compaction_at: &Arc<RwLock<Option<String>>>,
        is_compacting: &Arc<RwLock<bool>>,
        user_id: &str,
    ) {
        // Set compacting state
        {
            let mut compacting = is_compacting.write().await;
            *compacting = true;
        }

        log::info!("Starting data compaction for user: {}", user_id);

        let pool = {
            let db_guard = db.lock().await;
            db_guard.pool.clone()
        };

        // Phase 3: Hourly → Daily compaction
        let llm = recap_core::services::llm::create_llm_service(&pool, user_id)
            .await
            .ok();

        match recap_core::services::compaction::run_compaction_cycle(
            &pool,
            llm.as_ref(),
            user_id,
        )
        .await
        {
            Ok(cr) => {
                if cr.hourly_compacted > 0 || cr.daily_compacted > 0 {
                    log::info!(
                        "Compaction: {} hourly, {} daily summaries created",
                        cr.hourly_compacted,
                        cr.daily_compacted
                    );
                }
                // Log LLM warnings
                if !cr.llm_warnings.is_empty() {
                    log::warn!("LLM warnings: {}", cr.llm_warnings.join("; "));
                }
            }
            Err(e) => {
                log::warn!("Compaction cycle error: {}", e);
            }
        }

        // Phase 4: Timeline summaries (hierarchical: week → month → quarter → year)
        // Process in order from small to large time units
        let time_units = ["week", "month", "quarter", "year"];
        match crate::commands::projects::summaries::generate_all_completed_summaries(
            &pool,
            user_id,
            &time_units,
        )
        .await
        {
            Ok(count) => {
                if count > 0 {
                    log::info!("Generated {} timeline summaries", count);
                }
            }
            Err(e) => {
                log::warn!("Timeline summary generation error: {}", e);
            }
        }

        // Update last_compaction_at
        {
            let mut compaction_time = last_compaction_at.write().await;
            *compaction_time = Some(chrono::Utc::now().to_rfc3339());
        }

        // Clear compacting state
        {
            let mut compacting = is_compacting.write().await;
            *compacting = false;
        }

        log::info!("Data compaction completed");
    }

    /// Perform the actual sync operation with lifecycle management (FULL SYNC)
    ///
    /// This is the internal implementation used by both the timer loop and trigger_sync.
    /// Uses the lifecycle state machine to ensure correct state transitions.
    async fn perform_sync_with_lifecycle(
        db: &Arc<Mutex<recap_core::Database>>,
        lifecycle: &Arc<RwLock<ServiceLifecycle>>,
        last_sync_at: &Arc<RwLock<Option<String>>>,
        last_result: &Arc<RwLock<Option<String>>>,
        last_error: &Arc<RwLock<Option<String>>>,
        config: &BackgroundSyncConfig,
        user_id: &str,
    ) -> Vec<SyncOperationResult> {
        log::info!("Starting background sync for user: {}", user_id);

        // Transition to Syncing
        {
            let mut lc = lifecycle.write().await;
            match lc.clone().begin_sync() {
                Ok(new_state) => *lc = new_state,
                Err(e) => {
                    log::warn!("Cannot begin sync: {}", e);
                    return vec![SyncOperationResult {
                        source: "system".to_string(),
                        success: false,
                        error: Some(e.to_string()),
                        ..Default::default()
                    }];
                }
            }
        }

        let mut results = Vec::new();

        // Clone pool immediately — never hold Mutex during I/O to avoid db lock contention
        let pool = {
            let db_guard = db.lock().await;
            db_guard.pool.clone()
        }; // Mutex released immediately

        // Convert to new SyncConfig format and get enabled sources
        let sync_config = config.to_sync_config();
        let sources = recap_core::services::sources::get_enabled_sources(&sync_config).await;

        // Phase 1: Sync all enabled sources using the trait abstraction
        for source in &sources {
            log::info!("Syncing {} for user: {}", source.display_name(), user_id);

            match source.sync_sessions(&pool, user_id).await {
                Ok(source_result) => {
                    let result = SyncOperationResult::from(source_result);
                    log::info!(
                        "{} sync complete: {} sessions processed, {} created, {} updated",
                        source.display_name(),
                        result.items_synced,
                        result.items_created,
                        result.projects_scanned
                    );
                    results.push(result);
                }
                Err(e) => {
                    log::error!("{} sync error: {}", source.display_name(), e);
                    results.push(SyncOperationResult {
                        source: source.source_name().to_string(),
                        success: false,
                        error: Some(e),
                        ..Default::default()
                    });
                }
            }
        }

        // Stub results for not-yet-implemented sources
        if config.sync_git && !sync_config.is_source_enabled("git") {
            results.push(SyncOperationResult {
                source: "git".to_string(),
                success: true,
                ..Default::default()
            });
        }

        if config.sync_gitlab && !sync_config.is_source_enabled("gitlab") {
            results.push(SyncOperationResult {
                source: "gitlab".to_string(),
                success: true,
                ..Default::default()
            });
        }

        if config.sync_jira && !sync_config.is_source_enabled("jira") {
            results.push(SyncOperationResult {
                source: "jira".to_string(),
                success: true,
                ..Default::default()
            });
        }

        // Phase 2: Capture hourly snapshots (uses pool directly, no Mutex)
        if config.sync_claude {
            let projects = recap_core::services::SyncService::discover_project_paths();
            let mut snapshot_count = 0;
            for project in &projects {
                match recap_core::services::snapshot::capture_snapshots_for_project(
                    &pool,
                    user_id,
                    project,
                )
                .await
                {
                    Ok(n) => snapshot_count += n,
                    Err(e) => {
                        log::warn!("Snapshot capture error for {}: {}", project.name, e);
                    }
                }
            }
            if snapshot_count > 0 {
                log::info!("Captured {} hourly snapshots", snapshot_count);
            }
        }

        // Phase 3: Run compaction cycle (uses pool directly, does NOT hold db lock)
        if config.sync_claude {
            let llm = recap_core::services::llm::create_llm_service(&pool, user_id)
                .await
                .ok();
            match recap_core::services::compaction::run_compaction_cycle(
                &pool,
                llm.as_ref(),
                user_id,
            )
            .await
            {
                Ok(cr) => {
                    if cr.hourly_compacted > 0 || cr.daily_compacted > 0 {
                        log::info!(
                            "Compaction: {} hourly, {} daily summaries created",
                            cr.hourly_compacted,
                            cr.daily_compacted
                        );
                    }
                    // Surface LLM warnings to last_error so users can see them
                    if !cr.llm_warnings.is_empty() {
                        let mut error = last_error.write().await;
                        *error = Some(cr.llm_warnings.join("; "));
                    }
                }
                Err(e) => {
                    log::warn!("Compaction cycle error: {}", e);
                }
            }
        }

        // Phase 4: Generate timeline summaries for completed periods
        if config.auto_generate_summaries {
            // Generate summaries for week, month, quarter, year (skip day for efficiency)
            let time_units = ["week", "month", "quarter", "year"];
            match crate::commands::projects::summaries::generate_all_completed_summaries(
                &pool,
                user_id,
                &time_units,
            )
            .await
            {
                Ok(count) => {
                    if count > 0 {
                        log::info!("Generated {} timeline summaries", count);
                    }
                }
                Err(e) => {
                    log::warn!("Timeline summary generation error: {}", e);
                }
            }
        }

        // Update last_sync_at (persists during all states)
        {
            let mut sync_time = last_sync_at.write().await;
            *sync_time = Some(chrono::Utc::now().to_rfc3339());
        }

        // Transition back to Idle and record results
        {
            let mut lc = lifecycle.write().await;
            // Calculate next sync time
            let next_sync = Some(Self::calculate_next_sync(config.interval_minutes));
            if let Ok(new_state) = lc.clone().complete_sync(next_sync) {
                *lc = new_state;
            }
        }

        // Record results
        {
            let total_projects: i32 = results.iter().map(|r| r.projects_scanned).sum();
            let total_created: i32 = results.iter().map(|r| r.items_created).sum();
            let errors: Vec<String> = results.iter().filter_map(|r| r.error.clone()).collect();

            {
                let mut result = last_result.write().await;
                *result = Some(format!("已掃描 {} 個專案，發現 {} 筆新資料", total_projects, total_created));
            }
            {
                let mut error = last_error.write().await;
                *error = if errors.is_empty() { None } else { Some(errors.join("; ")) };
            }
        }

        log::info!("Background sync completed: {} sources processed", results.len());
        results
    }

    /// Query the scheduler for real next fire times and update state.
    ///
    /// Called from `get_status()` to ensure the frontend sees accurate times.
    async fn refresh_next_times(&self) {
        // Clone scheduler out of the Mutex to avoid holding it across awaits on other locks
        let sched = {
            let guard = self.scheduler.lock().await;
            guard.clone()
        };
        let Some(mut sched) = sched else { return };

        // Refresh sync job next time
        if let Some(job_id) = *self.sync_job_id.read().await {
            if let Ok(Some(next_time)) = sched.next_tick_for_job(job_id).await {
                let mut lc = self.lifecycle.write().await;
                *lc = lc.clone().update_next_sync_at(Some(next_time.to_rfc3339()));
            }
        }

        // Refresh compaction job next time
        if let Some(job_id) = *self.compaction_job_id.read().await {
            if let Ok(Some(next_time)) = sched.next_tick_for_job(job_id).await {
                let mut nca = self.next_compaction_at.write().await;
                *nca = Some(next_time.to_rfc3339());
            }
        }
    }

    /// Static helper: update next_sync_at from scheduler inside a job closure.
    ///
    /// Called after data sync completes to set the real next fire time.
    async fn update_next_sync_from_scheduler(
        scheduler: &mut JobScheduler,
        sync_job_id: uuid::Uuid,
        lifecycle: &Arc<RwLock<ServiceLifecycle>>,
    ) {
        match scheduler.next_tick_for_job(sync_job_id).await {
            Ok(Some(next_time)) => {
                let mut lc = lifecycle.write().await;
                *lc = lc.clone().update_next_sync_at(Some(next_time.to_rfc3339()));
                log::debug!("Updated next_sync_at from scheduler: {}", next_time.to_rfc3339());
            }
            Ok(None) => {
                log::debug!("No next tick for sync job (scheduler may be shutting down)");
            }
            Err(e) => {
                log::warn!("Failed to query next tick for sync job: {:?}", e);
            }
        }
    }

    /// Static helper: update next_compaction_at from scheduler inside a job closure.
    ///
    /// Called after compaction completes to set the real next fire time.
    async fn update_next_compaction_from_scheduler(
        scheduler: &mut JobScheduler,
        compaction_job_id: uuid::Uuid,
        next_compaction_at: &Arc<RwLock<Option<String>>>,
    ) {
        match scheduler.next_tick_for_job(compaction_job_id).await {
            Ok(Some(next_time)) => {
                let mut nca = next_compaction_at.write().await;
                *nca = Some(next_time.to_rfc3339());
                log::debug!("Updated next_compaction_at from scheduler: {}", next_time.to_rfc3339());
            }
            Ok(None) => {
                log::debug!("No next tick for compaction job (scheduler may be shutting down)");
            }
            Err(e) => {
                log::warn!("Failed to query next tick for compaction job: {:?}", e);
            }
        }
    }

    /// Calculate the next sync timestamp
    fn calculate_next_sync(interval_minutes: u32) -> String {
        let next = chrono::Utc::now() + chrono::Duration::minutes(interval_minutes as i64);
        next.to_rfc3339()
    }

    /// Calculate the next compaction timestamp
    fn calculate_next_compaction(interval_minutes: u32) -> String {
        let next = chrono::Utc::now() + chrono::Duration::minutes(interval_minutes as i64);
        next.to_rfc3339()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BackgroundSyncConfig::default();
        assert!(config.enabled);
        assert_eq!(config.interval_minutes, 15);
        assert!(config.sync_git);
        assert!(config.sync_claude);
        assert!(!config.sync_gitlab);
        assert!(!config.sync_jira);
    }

    #[test]
    fn test_sync_service_status_default() {
        let status = SyncServiceStatus::default();
        assert!(!status.is_running);
        assert!(!status.is_syncing);
        assert!(!status.is_compacting);
        assert!(status.last_sync_at.is_none());
        assert!(status.last_compaction_at.is_none());
        assert!(status.next_sync_at.is_none());
    }

    #[test]
    fn test_calculate_next_sync() {
        let next = BackgroundSyncService::calculate_next_sync(15);
        assert!(!next.is_empty());
        // Verify it's a valid RFC3339 timestamp
        assert!(chrono::DateTime::parse_from_rfc3339(&next).is_ok());
    }

    #[test]
    fn test_sync_operation_result() {
        let result = SyncOperationResult {
            source: "git".to_string(),
            success: true,
            items_synced: 5,
            projects_scanned: 3,
            items_created: 2,
            error: None,
        };
        assert_eq!(result.source, "git");
        assert!(result.success);
        assert_eq!(result.items_synced, 5);
        assert_eq!(result.projects_scanned, 3);
        assert_eq!(result.items_created, 2);
    }

    // =========================================================================
    // Lifecycle State Machine Tests
    // =========================================================================

    #[test]
    fn test_lifecycle_default_is_created() {
        let lifecycle = ServiceLifecycle::default();
        assert_eq!(lifecycle, ServiceLifecycle::Created);
        assert!(!lifecycle.is_running());
        assert!(!lifecycle.is_syncing());
    }

    #[test]
    fn test_lifecycle_start_from_created() {
        let lifecycle = ServiceLifecycle::Created;
        let result = lifecycle.start(Some("2026-01-30T12:00:00Z".to_string()));
        assert!(result.is_ok());

        let new_state = result.unwrap();
        assert!(new_state.is_running());
        assert!(!new_state.is_syncing());
        assert_eq!(new_state.next_sync_at(), Some("2026-01-30T12:00:00Z"));
    }

    #[test]
    fn test_lifecycle_start_from_stopped() {
        let lifecycle = ServiceLifecycle::Stopped;
        let result = lifecycle.start(None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_running());
    }

    #[test]
    fn test_lifecycle_cannot_start_when_already_running() {
        let lifecycle = ServiceLifecycle::Idle {
            last_sync_at: None,
            next_sync_at: None,
        };
        let result = lifecycle.start(None);
        assert_eq!(result.unwrap_err(), ServiceLifecycleError::AlreadyRunning);
    }

    #[test]
    fn test_lifecycle_begin_sync_from_idle() {
        let lifecycle = ServiceLifecycle::Idle {
            last_sync_at: None,
            next_sync_at: None,
        };
        let result = lifecycle.begin_sync();
        assert!(result.is_ok());

        let new_state = result.unwrap();
        assert!(new_state.is_syncing());
        assert!(new_state.is_running());
    }

    #[test]
    fn test_lifecycle_cannot_begin_sync_when_not_started() {
        let lifecycle = ServiceLifecycle::Created;
        let result = lifecycle.begin_sync();
        assert_eq!(result.unwrap_err(), ServiceLifecycleError::NotStarted);
    }

    #[test]
    fn test_lifecycle_cannot_begin_sync_when_already_syncing() {
        let lifecycle = ServiceLifecycle::Syncing {
            started_at: "2026-01-30T12:00:00Z".to_string(),
        };
        let result = lifecycle.begin_sync();
        assert_eq!(result.unwrap_err(), ServiceLifecycleError::SyncInProgress);
    }

    #[test]
    fn test_lifecycle_complete_sync() {
        let lifecycle = ServiceLifecycle::Syncing {
            started_at: "2026-01-30T12:00:00Z".to_string(),
        };
        let result = lifecycle.complete_sync(Some("2026-01-30T12:15:00Z".to_string()));
        assert!(result.is_ok());

        let new_state = result.unwrap();
        assert!(!new_state.is_syncing());
        assert!(new_state.is_running());
        assert!(new_state.last_sync_at().is_some());
        assert_eq!(new_state.next_sync_at(), Some("2026-01-30T12:15:00Z"));
    }

    #[test]
    fn test_lifecycle_cannot_complete_sync_when_not_syncing() {
        let lifecycle = ServiceLifecycle::Idle {
            last_sync_at: None,
            next_sync_at: None,
        };
        let result = lifecycle.complete_sync(None);
        assert_eq!(result.unwrap_err(), ServiceLifecycleError::NotSyncing);
    }

    #[test]
    fn test_lifecycle_stop_from_any_state() {
        // From Created
        let lifecycle = ServiceLifecycle::Created;
        assert_eq!(lifecycle.stop(), ServiceLifecycle::Stopped);

        // From Idle
        let lifecycle = ServiceLifecycle::Idle {
            last_sync_at: None,
            next_sync_at: None,
        };
        assert_eq!(lifecycle.stop(), ServiceLifecycle::Stopped);

        // From Syncing
        let lifecycle = ServiceLifecycle::Syncing {
            started_at: "2026-01-30T12:00:00Z".to_string(),
        };
        assert_eq!(lifecycle.stop(), ServiceLifecycle::Stopped);
    }

    #[test]
    fn test_lifecycle_full_flow() {
        // Created -> Idle -> Syncing -> Idle -> Stopped
        let lifecycle = ServiceLifecycle::Created;

        // Start
        let lifecycle = lifecycle.start(Some("next".to_string())).unwrap();
        assert!(matches!(lifecycle, ServiceLifecycle::Idle { .. }));

        // Begin sync
        let lifecycle = lifecycle.begin_sync().unwrap();
        assert!(matches!(lifecycle, ServiceLifecycle::Syncing { .. }));

        // Complete sync
        let lifecycle = lifecycle.complete_sync(Some("next2".to_string())).unwrap();
        assert!(matches!(lifecycle, ServiceLifecycle::Idle { .. }));
        assert!(lifecycle.last_sync_at().is_some());

        // Stop
        let lifecycle = lifecycle.stop();
        assert_eq!(lifecycle, ServiceLifecycle::Stopped);
    }

    #[test]
    fn test_status_from_lifecycle() {
        let lifecycle = ServiceLifecycle::Idle {
            last_sync_at: Some("2026-01-30T12:00:00Z".to_string()),
            next_sync_at: Some("2026-01-30T12:15:00Z".to_string()),
        };

        let status = SyncServiceStatus::from_lifecycle(
            &lifecycle,
            false, // is_compacting
            Some("2026-01-30T11:00:00Z".to_string()), // last_compaction_at
            Some("2026-01-30T17:00:00Z".to_string()), // next_compaction_at
            Some("結果".to_string()),
            None,
        );

        assert!(status.is_running);
        assert!(!status.is_syncing);
        assert!(!status.is_compacting);
        assert_eq!(status.next_compaction_at, Some("2026-01-30T17:00:00Z".to_string()));
        assert_eq!(status.last_sync_at, Some("2026-01-30T12:00:00Z".to_string()));
        assert_eq!(status.last_compaction_at, Some("2026-01-30T11:00:00Z".to_string()));
        assert_eq!(status.next_sync_at, Some("2026-01-30T12:15:00Z".to_string()));
        assert_eq!(status.last_result, Some("結果".to_string()));
        assert!(status.last_error.is_none());
    }
}
