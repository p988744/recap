//! Background Sync Service
//!
//! Provides scheduled automatic synchronization of work items from various sources.
//! Runs in the background while the app is in the system tray.

use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, Duration};

// =============================================================================
// Configuration
// =============================================================================

/// Background sync configuration
#[derive(Debug, Clone)]
pub struct BackgroundSyncConfig {
    /// Whether background sync is enabled
    pub enabled: bool,
    /// Sync interval in minutes (5, 15, 30, 60)
    pub interval_minutes: u32,
    /// Sync local Git repositories
    pub sync_git: bool,
    /// Sync Claude Code sessions
    pub sync_claude: bool,
    /// Sync GitLab (requires configuration)
    pub sync_gitlab: bool,
    /// Sync Jira/Tempo (requires configuration)
    pub sync_jira: bool,
}

impl Default for BackgroundSyncConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_minutes: 15,
            sync_git: true,
            sync_claude: true,
            sync_gitlab: false,
            sync_jira: false,
        }
    }
}

// =============================================================================
// Sync Status
// =============================================================================

/// Current status of the background sync service
#[derive(Debug, Clone, Default)]
pub struct SyncServiceStatus {
    /// Whether the service is currently running
    pub is_running: bool,
    /// Whether a sync is in progress
    pub is_syncing: bool,
    /// Last sync timestamp (ISO 8601)
    pub last_sync_at: Option<String>,
    /// Next scheduled sync timestamp (ISO 8601)
    pub next_sync_at: Option<String>,
    /// Last sync result message
    pub last_result: Option<String>,
    /// Last error message (if any)
    pub last_error: Option<String>,
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
    pub error: Option<String>,
}

// =============================================================================
// Background Sync Service
// =============================================================================

/// Background sync service that manages scheduled synchronization
pub struct BackgroundSyncService {
    /// Current configuration
    config: Arc<RwLock<BackgroundSyncConfig>>,
    /// Current status
    status: Arc<RwLock<SyncServiceStatus>>,
    /// Shutdown signal sender
    shutdown_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    /// Database connection for sync operations
    db: Arc<Mutex<recap_core::Database>>,
    /// User ID for sync operations
    user_id: Arc<RwLock<Option<String>>>,
}

impl BackgroundSyncService {
    /// Create a new background sync service
    pub fn new(db: Arc<Mutex<recap_core::Database>>) -> Self {
        Self {
            config: Arc::new(RwLock::new(BackgroundSyncConfig::default())),
            status: Arc::new(RwLock::new(SyncServiceStatus::default())),
            shutdown_tx: Arc::new(Mutex::new(None)),
            db,
            user_id: Arc::new(RwLock::new(None)),
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

    /// Get the current status
    pub async fn get_status(&self) -> SyncServiceStatus {
        self.status.read().await.clone()
    }

    /// Start the background sync service
    pub async fn start(&self) {
        let config = self.config.read().await;
        if !config.enabled {
            log::info!("Background sync is disabled, not starting");
            return;
        }

        // Check if already running
        {
            let status = self.status.read().await;
            if status.is_running {
                log::info!("Background sync service is already running");
                return;
            }
        }

        let interval_minutes = config.interval_minutes;
        drop(config);

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        {
            let mut tx = self.shutdown_tx.lock().await;
            *tx = Some(shutdown_tx);
        }

        // Update status
        {
            let mut status = self.status.write().await;
            status.is_running = true;
            status.next_sync_at = Some(Self::calculate_next_sync(interval_minutes));
        }

        log::info!("Starting background sync service with {}min interval", interval_minutes);

        // Clone references for the spawned task
        let config = Arc::clone(&self.config);
        let status = Arc::clone(&self.status);
        let db = Arc::clone(&self.db);
        let user_id = Arc::clone(&self.user_id);

        // Spawn the sync loop
        tokio::spawn(async move {
            let mut timer = interval(Duration::from_secs(interval_minutes as u64 * 60));

            // Skip the first tick (immediate)
            timer.tick().await;

            loop {
                tokio::select! {
                    _ = timer.tick() => {
                        // Check if still enabled
                        let cfg = config.read().await;
                        if !cfg.enabled {
                            log::info!("Background sync disabled, stopping loop");
                            break;
                        }
                        let sync_config = cfg.clone();
                        drop(cfg);

                        // Get user ID
                        let uid = user_id.read().await.clone();
                        if uid.is_none() {
                            log::warn!("No user ID set, skipping sync");
                            continue;
                        }
                        let uid = uid.unwrap();

                        // Perform sync
                        Self::perform_sync(&db, &status, &sync_config, &uid).await;

                        // Update next sync time
                        let mut st = status.write().await;
                        st.next_sync_at = Some(Self::calculate_next_sync(sync_config.interval_minutes));
                    }
                    _ = &mut shutdown_rx => {
                        log::info!("Background sync service received shutdown signal");
                        break;
                    }
                }
            }

            // Update status on exit
            let mut st = status.write().await;
            st.is_running = false;
            st.next_sync_at = None;
            log::info!("Background sync service stopped");
        });
    }

    /// Stop the background sync service
    pub async fn stop(&self) {
        let tx = {
            let mut guard = self.shutdown_tx.lock().await;
            guard.take()
        };

        if let Some(tx) = tx {
            let _ = tx.send(());
            log::info!("Sent shutdown signal to background sync service");
        }

        // Update status
        let mut status = self.status.write().await;
        status.is_running = false;
        status.next_sync_at = None;
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
            }];
        }

        Self::perform_sync(&self.db, &self.status, &config, &uid.unwrap()).await
    }

    /// Perform the actual sync operation
    async fn perform_sync(
        db: &Arc<Mutex<recap_core::Database>>,
        status: &Arc<RwLock<SyncServiceStatus>>,
        config: &BackgroundSyncConfig,
        user_id: &str,
    ) -> Vec<SyncOperationResult> {
        log::info!("Starting background sync for user: {}", user_id);

        // Mark as syncing
        {
            let mut st = status.write().await;
            st.is_syncing = true;
        }

        let mut results = Vec::new();

        // Phase 1: Sync sources + capture snapshots (hold db lock briefly)
        let pool = {
            let db_guard = db.lock().await;

            // Sync Claude sessions (primary source)
            if config.sync_claude {
                let result = Self::sync_claude_sessions(&db_guard, user_id).await;
                results.push(result);
            }

            // Sync Git repos (placeholder)
            if config.sync_git {
                results.push(SyncOperationResult {
                    source: "git".to_string(),
                    success: true,
                    items_synced: 0,
                    error: None,
                });
            }

            // Sync GitLab (placeholder)
            if config.sync_gitlab {
                results.push(SyncOperationResult {
                    source: "gitlab".to_string(),
                    success: true,
                    items_synced: 0,
                    error: None,
                });
            }

            // Sync Jira (placeholder)
            if config.sync_jira {
                results.push(SyncOperationResult {
                    source: "jira".to_string(),
                    success: true,
                    items_synced: 0,
                    error: None,
                });
            }

            // Phase 2: Capture hourly snapshots
            if config.sync_claude {
                let projects = recap_core::services::SyncService::discover_project_paths();
                let mut snapshot_count = 0;
                for project in &projects {
                    match recap_core::services::snapshot::capture_snapshots_for_project(
                        &db_guard.pool,
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

            // Clone pool before releasing the db lock so compaction can use it without blocking
            db_guard.pool.clone()
        }; // db_guard dropped here — other commands can now access the database

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
                }
                Err(e) => {
                    log::warn!("Compaction cycle error: {}", e);
                }
            }
        }

        // Update status
        {
            let mut st = status.write().await;
            st.is_syncing = false;
            st.last_sync_at = Some(chrono::Utc::now().to_rfc3339());

            let total_items: i32 = results.iter().map(|r| r.items_synced).sum();
            let errors: Vec<String> = results
                .iter()
                .filter_map(|r| r.error.clone())
                .collect();

            if errors.is_empty() {
                st.last_result = Some(format!("成功同步 {} 筆項目", total_items));
                st.last_error = None;
            } else {
                st.last_result = Some(format!("同步完成，{} 個錯誤", errors.len()));
                st.last_error = Some(errors.join("; "));
            }
        }

        log::info!("Background sync completed: {} sources processed", results.len());
        results
    }

    /// Sync Claude Code sessions using project discovery with git root resolution
    async fn sync_claude_sessions(db: &recap_core::Database, user_id: &str) -> SyncOperationResult {
        log::info!("Syncing Claude sessions for user: {}", user_id);

        // Discover all projects using multi-strategy discovery + git root grouping
        let projects = recap_core::services::SyncService::discover_project_paths();

        if projects.is_empty() {
            return SyncOperationResult {
                source: "claude".to_string(),
                success: true,
                items_synced: 0,
                error: None,
            };
        }

        log::info!("Discovered {} Claude projects", projects.len());
        for project in &projects {
            log::debug!(
                "  Project: {} ({}) - {} dirs",
                project.name,
                project.canonical_path,
                project.claude_dirs.len()
            );
        }

        match recap_core::services::sync_discovered_projects(&db.pool, user_id, &projects).await {
            Ok(result) => {
                let items_synced = (result.work_items_created + result.work_items_updated) as i32;
                SyncOperationResult {
                    source: "claude".to_string(),
                    success: true,
                    items_synced,
                    error: None,
                }
            }
            Err(e) => {
                log::error!("Claude sync error: {}", e);
                SyncOperationResult {
                    source: "claude".to_string(),
                    success: false,
                    items_synced: 0,
                    error: Some(e),
                }
            }
        }
    }

    /// Calculate the next sync timestamp
    fn calculate_next_sync(interval_minutes: u32) -> String {
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
        assert!(status.last_sync_at.is_none());
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
            error: None,
        };
        assert_eq!(result.source, "git");
        assert!(result.success);
        assert_eq!(result.items_synced, 5);
    }
}
