//! Session Parser - Shared utilities for parsing Claude Code sessions
//!
//! This module consolidates all session parsing logic used across:
//! - Claude sync (commands/claude.rs)
//! - Sync service (services/sync.rs)
//! - Work items (commands/work_items.rs)

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

// ============ Hash Generation ============

/// Generate content hash for deduplication (user + project + date = unique work item)
pub fn generate_daily_hash(user_id: &str, project: &str, date: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(user_id.as_bytes());
    hasher.update(project.as_bytes());
    hasher.update(date.as_bytes());
    format!("{:x}", hasher.finalize())
}

// ============ Message Validation ============

/// Check if a message is meaningful (not warmup, not system commands, has content)
pub fn is_meaningful_message(content: &str) -> bool {
    let trimmed = content.trim().to_lowercase();
    if trimmed == "warmup" || trimmed.starts_with("warmup") {
        return false;
    }
    if trimmed.starts_with("<command-") || trimmed.starts_with("<system-") {
        return false;
    }
    trimmed.len() >= 10
}

// ============ Tool Detail Extraction ============

/// Extract relevant detail from tool input for display
pub fn extract_tool_detail(tool_name: &str, input: &serde_json::Value) -> Option<String> {
    match tool_name {
        "Edit" | "Write" | "Read" => {
            input.get("file_path")
                .and_then(|v| v.as_str())
                .map(|p| {
                    let parts: Vec<&str> = p.split('/').collect();
                    if parts.len() > 3 {
                        format!(".../{}", parts[parts.len() - 3..].join("/"))
                    } else {
                        p.to_string()
                    }
                })
        }
        "Bash" => {
            input.get("command")
                .and_then(|v| v.as_str())
                .map(|c| {
                    let truncated: String = c.chars().take(60).collect();
                    if c.len() > 60 {
                        format!("{}...", truncated)
                    } else {
                        truncated
                    }
                })
        }
        "Glob" | "Grep" => {
            input.get("pattern")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        }
        "Task" => {
            input.get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.chars().take(50).collect())
        }
        _ => None,
    }
}

// ============ Session Parsing Types ============

/// Session message for parsing JSONL
#[derive(Debug, Deserialize)]
pub struct SessionMessage {
    pub cwd: Option<String>,
    pub timestamp: Option<String>,
    #[serde(rename = "type")]
    pub msg_type: Option<String>,
    pub message: Option<MessageContent>,
}

#[derive(Debug, Deserialize)]
pub struct MessageContent {
    pub role: Option<String>,
    pub content: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ToolUseContent {
    #[serde(rename = "type")]
    pub content_type: Option<String>,
    pub name: Option<String>,
    pub input: Option<serde_json::Value>,
}

/// Tool usage tracking
#[derive(Debug, Clone)]
pub struct ToolUsage {
    pub tool_name: String,
    pub count: usize,
}

/// Parsed session metadata (lightweight)
#[derive(Debug, Clone)]
pub struct SessionMetadata {
    pub cwd: Option<String>,
    pub first_ts: String,
    pub last_ts: String,
    pub first_msg: Option<String>,
    pub message_count: usize,
}

/// Full parsed session data
#[derive(Debug, Clone)]
pub struct ParsedSession {
    pub cwd: String,
    pub first_timestamp: Option<String>,
    pub last_timestamp: Option<String>,
    pub message_count: usize,
    pub tool_usage: Vec<ToolUsage>,
    pub files_modified: Vec<String>,
    pub first_message: Option<String>,
}

// ============ Session Parsing Functions ============

/// Fast session parsing - only extracts timestamps and first message
/// Used by timeline display where full parsing is not needed
pub fn parse_session_fast(path: &PathBuf) -> Option<SessionMetadata> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut cwd: Option<String> = None;
    let mut first_ts: Option<String> = None;
    let mut last_ts: Option<String> = None;
    let mut first_msg: Option<String> = None;
    let mut message_count: usize = 0;

    for line in reader.lines().flatten() {
        if let Ok(msg) = serde_json::from_str::<SessionMessage>(&line) {
            // Extract cwd from first message that has it
            if cwd.is_none() {
                cwd = msg.cwd;
            }

            // Track timestamps
            if let Some(ts) = &msg.timestamp {
                if first_ts.is_none() {
                    first_ts = Some(ts.clone());
                }
                last_ts = Some(ts.clone());
            }

            // Extract first meaningful user message
            if first_msg.is_none() {
                if let Some(ref message) = msg.message {
                    if message.role.as_deref() == Some("user") {
                        if let Some(content) = &message.content {
                            if let serde_json::Value::String(s) = content {
                                if is_meaningful_message(s) {
                                    first_msg = Some(s.chars().take(200).collect());
                                    message_count += 1;
                                }
                            }
                        }
                    }
                }
            } else if let Some(ref message) = msg.message {
                if message.role.as_deref() == Some("user") {
                    if let Some(content) = &message.content {
                        if let serde_json::Value::String(s) = content {
                            if is_meaningful_message(s) {
                                message_count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    let first_ts = first_ts?;
    let last_ts = last_ts?;

    Some(SessionMetadata {
        cwd,
        first_ts,
        last_ts,
        first_msg,
        message_count,
    })
}

/// Full session parsing - extracts all details including tool usage
/// Used by sync operations where full data is needed
pub fn parse_session_full(path: &PathBuf) -> Option<ParsedSession> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut cwd: Option<String> = None;
    let mut first_message: Option<String> = None;
    let mut first_timestamp: Option<String> = None;
    let mut last_timestamp: Option<String> = None;
    let mut meaningful_message_count: usize = 0;

    let mut tool_counts: HashMap<String, usize> = HashMap::new();
    let mut files_modified: Vec<String> = Vec::new();

    for line in reader.lines().flatten() {
        if let Ok(msg) = serde_json::from_str::<SessionMessage>(&line) {
            if cwd.is_none() {
                cwd = msg.cwd;
            }

            if let Some(ts) = &msg.timestamp {
                if first_timestamp.is_none() {
                    first_timestamp = Some(ts.clone());
                }
                last_timestamp = Some(ts.clone());
            }

            if let Some(ref message) = msg.message {
                // User messages
                if message.role.as_deref() == Some("user") {
                    if let Some(content) = &message.content {
                        if let serde_json::Value::String(s) = content {
                            if is_meaningful_message(s) {
                                meaningful_message_count += 1;
                                if first_message.is_none() {
                                    first_message = Some(s.chars().take(200).collect());
                                }
                            }
                        }
                    }
                }

                // Assistant messages - extract tool usage
                if message.role.as_deref() == Some("assistant") {
                    if let Some(content) = &message.content {
                        if let serde_json::Value::Array(arr) = content {
                            for item in arr {
                                if let Ok(tool_use) =
                                    serde_json::from_value::<ToolUseContent>(item.clone())
                                {
                                    if tool_use.content_type.as_deref() == Some("tool_use") {
                                        if let Some(tool_name) = &tool_use.name {
                                            *tool_counts.entry(tool_name.clone()).or_insert(0) += 1;

                                            // Track file modifications
                                            if let Some(input) = &tool_use.input {
                                                if let Some(detail) =
                                                    extract_tool_detail(tool_name, input)
                                                {
                                                    if matches!(
                                                        tool_name.as_str(),
                                                        "Edit" | "Write"
                                                    ) && !files_modified.contains(&detail)
                                                        && files_modified.len() < 50
                                                    {
                                                        files_modified.push(detail);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let tool_usage: Vec<ToolUsage> = tool_counts
        .into_iter()
        .map(|(name, count)| ToolUsage {
            tool_name: name,
            count,
        })
        .collect();

    Some(ParsedSession {
        cwd: cwd.unwrap_or_default(),
        first_timestamp,
        last_timestamp,
        message_count: meaningful_message_count,
        tool_usage,
        files_modified,
        first_message,
    })
}

// ============ Tests ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_daily_hash_consistent() {
        let hash1 = generate_daily_hash("user1", "/home/project", "2026-01-11");
        let hash2 = generate_daily_hash("user1", "/home/project", "2026-01-11");
        assert_eq!(hash1, hash2, "Same inputs should produce same hash");
    }

    #[test]
    fn test_generate_daily_hash_different_inputs() {
        let hash1 = generate_daily_hash("user1", "/home/project", "2026-01-11");
        let hash2 = generate_daily_hash("user1", "/home/project", "2026-01-12");
        assert_ne!(hash1, hash2, "Different dates should produce different hashes");
    }

    #[test]
    fn test_is_meaningful_message_valid() {
        assert!(is_meaningful_message("Please help me implement this feature"));
        assert!(is_meaningful_message("這是一個有意義的訊息"));
    }

    #[test]
    fn test_is_meaningful_message_warmup() {
        assert!(!is_meaningful_message("warmup"));
        assert!(!is_meaningful_message("Warmup test"));
        assert!(!is_meaningful_message("warmup message"));
    }

    #[test]
    fn test_is_meaningful_message_system() {
        assert!(!is_meaningful_message("<command-name>test</command-name>"));
        assert!(!is_meaningful_message("<system-reminder>test</system-reminder>"));
    }

    #[test]
    fn test_is_meaningful_message_too_short() {
        assert!(!is_meaningful_message("hi"));
        assert!(!is_meaningful_message("short"));
    }

    #[test]
    fn test_extract_tool_detail_edit() {
        let input = serde_json::json!({
            "file_path": "/home/user/project/src/main.rs"
        });
        let detail = extract_tool_detail("Edit", &input);
        assert!(detail.is_some());
        assert!(detail.unwrap().contains("main.rs"));
    }

    #[test]
    fn test_extract_tool_detail_bash() {
        let input = serde_json::json!({
            "command": "cargo build --release"
        });
        let detail = extract_tool_detail("Bash", &input);
        assert_eq!(detail, Some("cargo build --release".to_string()));
    }

    #[test]
    fn test_extract_tool_detail_long_command() {
        let long_cmd = "a".repeat(100);
        let input = serde_json::json!({
            "command": long_cmd
        });
        let detail = extract_tool_detail("Bash", &input);
        assert!(detail.is_some());
        let detail = detail.unwrap();
        assert!(detail.len() <= 63); // 60 + "..."
        assert!(detail.ends_with("..."));
    }
}
