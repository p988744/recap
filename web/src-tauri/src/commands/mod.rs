//! Tauri Commands module
//!
//! This module contains all Tauri commands that replace the HTTP API.
//! Commands are called directly from the frontend via `invoke()`.

pub mod antigravity;
pub mod auth;
pub mod background_sync;
pub mod batch_compaction;
pub mod claude;
pub mod config;
pub mod danger_zone;
pub mod gitlab;
pub mod llm_usage;
pub mod notification;
pub mod projects;
pub mod quota;
pub mod quota_timer;
pub mod reports;
pub mod snapshots;
pub mod sources;
pub mod sync;
pub mod tempo;
pub mod tray;
pub mod users;
pub mod work_items;
pub mod worklog_sync;

use crate::services::BackgroundSyncService;
use recap_core::Database;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Application state shared across all commands
pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub background_sync: BackgroundSyncService,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        let db = Arc::new(Mutex::new(db));
        Self {
            background_sync: BackgroundSyncService::new(Arc::clone(&db)),
            db,
        }
    }
}
