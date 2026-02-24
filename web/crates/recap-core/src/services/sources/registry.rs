//! Source Registry
//!
//! This module provides a registry of available sync sources and functions
//! to get enabled sources based on configuration.

use std::collections::HashSet;

use super::{SyncSource, ClaudeSource};

/// Configuration for which sources to sync
#[derive(Debug, Clone, Default)]
pub struct SyncConfig {
    /// Whether background sync is enabled
    pub enabled: bool,
    /// Sync interval in minutes
    pub interval_minutes: u32,
    /// Enabled source names (e.g., "claude_code")
    pub enabled_sources: HashSet<String>,
}

impl SyncConfig {
    /// Create a new sync config with default settings
    pub fn new() -> Self {
        let mut enabled_sources = HashSet::new();
        enabled_sources.insert("claude_code".to_string());

        Self {
            enabled: true,
            interval_minutes: 15,
            enabled_sources,
        }
    }

    /// Check if a source is enabled
    pub fn is_source_enabled(&self, source_name: &str) -> bool {
        self.enabled_sources.contains(source_name)
    }

    /// Enable a source
    pub fn enable_source(&mut self, source_name: impl Into<String>) {
        self.enabled_sources.insert(source_name.into());
    }

    /// Disable a source
    pub fn disable_source(&mut self, source_name: &str) {
        self.enabled_sources.remove(source_name);
    }

    /// Create config from legacy BackgroundSyncConfig fields
    pub fn from_legacy(
        enabled: bool,
        interval_minutes: u32,
        sync_claude: bool,
        sync_git: bool,
        sync_gitlab: bool,
        sync_jira: bool,
    ) -> Self {
        let mut enabled_sources = HashSet::new();

        if sync_claude {
            enabled_sources.insert("claude_code".to_string());
        }
        if sync_git {
            enabled_sources.insert("git".to_string());
        }
        if sync_gitlab {
            enabled_sources.insert("gitlab".to_string());
        }
        if sync_jira {
            enabled_sources.insert("jira".to_string());
        }

        Self {
            enabled,
            interval_minutes,
            enabled_sources,
        }
    }
}

/// Get all registered sync sources
///
/// Returns all available sync sources regardless of whether they are enabled
/// or currently available.
pub fn get_all_sources() -> Vec<Box<dyn SyncSource>> {
    vec![
        Box::new(ClaudeSource::new()),
    ]
}

/// Get enabled sources based on configuration
///
/// Returns only sources that are:
/// 1. Enabled in the configuration
/// 2. Currently available
///
/// This is the main entry point for background sync to get sources to sync.
pub async fn get_enabled_sources(config: &SyncConfig) -> Vec<Box<dyn SyncSource>> {
    let mut sources: Vec<Box<dyn SyncSource>> = Vec::new();

    if config.is_source_enabled("claude_code") {
        let source = ClaudeSource::new();
        if source.is_available().await {
            sources.push(Box::new(source));
        }
    }

    // Future sources can be added here:
    // if config.is_source_enabled("git") {
    //     sources.push(Box::new(GitSource::new()));
    // }

    sources
}

/// Get source by name
pub fn get_source_by_name(name: &str) -> Option<Box<dyn SyncSource>> {
    match name {
        "claude_code" => Some(Box::new(ClaudeSource::new())),
        _ => None,
    }
}

/// Get all registered source names
pub fn get_source_names() -> Vec<&'static str> {
    vec!["claude_code"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_config_default() {
        let config = SyncConfig::new();
        assert!(config.enabled);
        assert_eq!(config.interval_minutes, 15);
        assert!(config.is_source_enabled("claude_code"));
        assert!(!config.is_source_enabled("git"));
    }

    #[test]
    fn test_sync_config_enable_disable() {
        let mut config = SyncConfig::new();

        config.disable_source("claude_code");
        assert!(!config.is_source_enabled("claude_code"));

        config.enable_source("git");
        assert!(config.is_source_enabled("git"));
    }

    #[test]
    fn test_sync_config_from_legacy() {
        let config = SyncConfig::from_legacy(
            true,
            30,
            true,
            true,
            false,
            false,
        );

        assert!(config.enabled);
        assert_eq!(config.interval_minutes, 30);
        assert!(config.is_source_enabled("claude_code"));
        assert!(config.is_source_enabled("git"));
        assert!(!config.is_source_enabled("gitlab"));
    }

    #[test]
    fn test_get_all_sources() {
        let sources = get_all_sources();
        assert_eq!(sources.len(), 1);

        let names: Vec<_> = sources.iter().map(|s| s.source_name()).collect();
        assert!(names.contains(&"claude_code"));
    }

    #[test]
    fn test_get_source_by_name() {
        let claude = get_source_by_name("claude_code");
        assert!(claude.is_some());
        assert_eq!(claude.unwrap().source_name(), "claude_code");

        let unknown = get_source_by_name("unknown");
        assert!(unknown.is_none());
    }

    #[test]
    fn test_get_source_names() {
        let names = get_source_names();
        assert!(names.contains(&"claude_code"));
    }
}
