//! Antigravity (Gemini Code) Session Parser
//!
//! Parses local Gemini session files from ~/.gemini/tmp/*/chats/session-*.json
//! into hourly buckets for snapshot capture.

use chrono::{DateTime, Local, Timelike};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::services::snapshot::{HourlyBucket, ToolCallRecord};

// ==================== Session File Types ====================

#[derive(Debug, Deserialize)]
pub struct GeminiSession {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "projectHash")]
    pub project_hash: String,
    #[serde(rename = "startTime")]
    pub start_time: Option<String>,
    #[serde(rename = "lastUpdated")]
    pub last_updated: Option<String>,
    pub messages: Vec<GeminiMessage>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiMessage {
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub message_type: String, // "user" or "gemini"
    pub content: String,
    pub thoughts: Option<Vec<GeminiThought>>,
    pub tokens: Option<GeminiTokens>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiThought {
    pub subject: String,
    pub description: String,
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct GeminiTokens {
    pub input: i64,
    pub output: i64,
    pub cached: i64,
    pub thoughts: i64,
    pub tool: i64,
    pub total: i64,
}

// ==================== Discovery ====================

/// Discover all Gemini session directories
pub fn discover_gemini_sessions() -> Vec<GeminiSessionPath> {
    let mut sessions = Vec::new();

    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return sessions,
    };

    let gemini_tmp = home.join(".gemini").join("tmp");
    if !gemini_tmp.exists() {
        return sessions;
    }

    // Iterate through project hash directories
    if let Ok(entries) = fs::read_dir(&gemini_tmp) {
        for entry in entries.flatten() {
            let project_dir = entry.path();
            if !project_dir.is_dir() {
                continue;
            }

            let chats_dir = project_dir.join("chats");
            if !chats_dir.exists() {
                continue;
            }

            // Find session files
            if let Ok(chat_entries) = fs::read_dir(&chats_dir) {
                for chat_entry in chat_entries.flatten() {
                    let path = chat_entry.path();
                    if path.extension().map(|e| e == "json").unwrap_or(false) {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if name.starts_with("session-") {
                                let project_hash = project_dir
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("")
                                    .to_string();

                                sessions.push(GeminiSessionPath {
                                    path,
                                    project_hash,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    sessions
}

#[derive(Debug)]
pub struct GeminiSessionPath {
    pub path: PathBuf,
    pub project_hash: String,
}

// ==================== Parsing ====================

/// Parse a Gemini session file
pub fn parse_gemini_session(path: &PathBuf) -> Result<GeminiSession, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read session file: {}", e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse session JSON: {}", e))
}

/// Parse a Gemini session into hourly buckets
pub fn parse_session_into_hourly_buckets(session: &GeminiSession) -> Vec<HourlyBucket> {
    let mut buckets_map: HashMap<String, HourlyBucket> = HashMap::new();

    for message in &session.messages {
        // Parse timestamp and truncate to hour
        let hour_bucket = match truncate_to_hour(&message.timestamp) {
            Some(h) => h,
            None => continue,
        };

        let bucket = buckets_map.entry(hour_bucket.clone()).or_insert_with(|| HourlyBucket {
            hour_bucket,
            user_messages: Vec::new(),
            assistant_summaries: Vec::new(),
            tool_calls: Vec::new(),
            files_modified: Vec::new(),
            git_commits: Vec::new(),
            message_count: 0,
        });

        match message.message_type.as_str() {
            "user" => {
                // Add user message (truncate to 500 chars like Claude Code)
                let truncated: String = message.content.chars().take(500).collect();
                if is_meaningful_message(&truncated) {
                    bucket.user_messages.push(truncated);
                    bucket.message_count += 1;
                }
            }
            "gemini" => {
                // Add assistant summary (truncate to 200 chars)
                let truncated: String = message.content.chars().take(200).collect();
                bucket.assistant_summaries.push(truncated);

                // Extract tool calls from thoughts if available
                if let Some(thoughts) = &message.thoughts {
                    for thought in thoughts {
                        // Thoughts often describe tool usage
                        if thought.subject.contains("Tool") ||
                           thought.subject.contains("Search") ||
                           thought.subject.contains("File") ||
                           thought.description.contains("run_shell") ||
                           thought.description.contains("read_file") ||
                           thought.description.contains("write_file") {
                            bucket.tool_calls.push(ToolCallRecord {
                                tool: thought.subject.clone(),
                                input_summary: thought.description.chars().take(200).collect(),
                                timestamp: thought.timestamp.clone(),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Convert to sorted vec
    let mut buckets: Vec<HourlyBucket> = buckets_map.into_values().collect();
    buckets.sort_by(|a, b| a.hour_bucket.cmp(&b.hour_bucket));

    buckets
}

/// Truncate an ISO 8601 timestamp to its hour boundary in local timezone
fn truncate_to_hour(timestamp: &str) -> Option<String> {
    let dt = DateTime::parse_from_rfc3339(timestamp).ok()?;
    let local_dt = dt.with_timezone(&Local);
    let truncated = local_dt
        .with_minute(0)
        .and_then(|d| d.with_second(0))
        .and_then(|d| d.with_nanosecond(0))?;
    Some(truncated.format("%Y-%m-%dT%H:%M:%S").to_string())
}

/// Check if a message is meaningful (not just empty or whitespace)
fn is_meaningful_message(msg: &str) -> bool {
    let trimmed = msg.trim();
    !trimmed.is_empty() && trimmed.len() > 5
}

// ==================== Project Path Resolution ====================

/// Try to resolve project path from session data
///
/// The session file doesn't directly store the project path, but we can try to:
/// 1. Look for file paths mentioned in the content
/// 2. Match with known project paths from API
pub fn extract_project_path_from_session(session: &GeminiSession) -> Option<String> {
    // Look for common path patterns in messages
    for message in &session.messages {
        // Look for absolute paths in user or assistant messages
        for line in message.content.lines() {
            // Common patterns: ~/project, /Users/*/project, /home/*/project
            if let Some(path) = extract_path_from_line(line) {
                return Some(path);
            }
        }

        // Check thoughts for file operations
        if let Some(thoughts) = &message.thoughts {
            for thought in thoughts {
                if let Some(path) = extract_path_from_line(&thought.description) {
                    return Some(path);
                }
            }
        }
    }

    None
}

fn extract_path_from_line(line: &str) -> Option<String> {
    // Look for file:// URIs
    if let Some(pos) = line.find("file://") {
        let path = line[pos + 7..].split_whitespace().next()?;
        let cleaned = path.trim_end_matches(|c| c == '"' || c == '\'' || c == ')' || c == ']');
        if cleaned.starts_with('/') {
            return Some(cleaned.to_string());
        }
    }

    // Look for absolute paths
    for word in line.split_whitespace() {
        let cleaned = word.trim_matches(|c| c == '"' || c == '\'' || c == '`' || c == '(' || c == ')');
        if cleaned.starts_with('/') && cleaned.contains('/') && !cleaned.starts_with("//") {
            // Check if it looks like a project path (not a system path)
            if cleaned.contains("/Users/") || cleaned.contains("/home/") {
                // Extract up to a reasonable depth
                let parts: Vec<&str> = cleaned.split('/').collect();
                if parts.len() >= 4 {
                    // Take up to the project directory (e.g., /Users/name/Projects/myproject)
                    let path = parts[..std::cmp::min(parts.len(), 6)].join("/");
                    if std::path::Path::new(&path).exists() {
                        return Some(path);
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_to_hour() {
        let ts = "2025-09-22T05:55:26.502Z";
        let result = truncate_to_hour(ts);
        assert!(result.is_some());
        // Note: exact result depends on local timezone
        let hour = result.unwrap();
        assert!(hour.contains("T") && hour.ends_with(":00:00"));
    }

    #[test]
    fn test_is_meaningful_message() {
        assert!(is_meaningful_message("Hello, can you help me?"));
        assert!(!is_meaningful_message(""));
        assert!(!is_meaningful_message("   "));
        assert!(!is_meaningful_message("hi")); // Too short
    }

    #[test]
    fn test_extract_path_from_line() {
        let line = "I'll work in file:///Users/test/project/src/main.rs";
        let path = extract_path_from_line(line);
        assert_eq!(path, Some("/Users/test/project/src/main.rs".to_string()));
    }
}
