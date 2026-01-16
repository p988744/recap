//! Tauri Commands module
//!
//! This module contains all Tauri commands that replace the HTTP API.
//! Commands are called directly from the frontend via `invoke()`.

pub mod auth;
pub mod background_sync;
pub mod claude;
pub mod config;
pub mod gitlab;
pub mod reports;
pub mod sources;
pub mod sync;
pub mod tempo;
pub mod tray;
pub mod users;
pub mod work_items;

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
