//! Services module
//!
//! Contains background services for the Tauri application.

pub mod background_sync;

pub use background_sync::BackgroundSyncService;
