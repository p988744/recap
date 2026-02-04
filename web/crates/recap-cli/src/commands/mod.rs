//! CLI commands module
//!
//! Contains all CLI command implementations.

pub mod claude;
pub mod config;
pub mod dashboard;
pub mod report;
pub mod source;
pub mod sync;
pub mod tempo_report;
pub mod work;

use crate::output::OutputFormat;
use recap_core::Database;

/// Shared context for all commands
pub struct Context {
    pub db: Database,
    pub format: OutputFormat,
    pub quiet: bool,
    /// Enable debug logging
    pub debug: bool,
}
