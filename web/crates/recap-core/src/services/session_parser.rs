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
    use std::io::Write;
    use tempfile::NamedTempFile;

    // ========================================================================
    // generate_daily_hash Tests
    // ========================================================================

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
    fn test_generate_daily_hash_different_users() {
        let hash1 = generate_daily_hash("user1", "/home/project", "2026-01-11");
        let hash2 = generate_daily_hash("user2", "/home/project", "2026-01-11");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_generate_daily_hash_different_projects() {
        let hash1 = generate_daily_hash("user1", "/home/project1", "2026-01-11");
        let hash2 = generate_daily_hash("user1", "/home/project2", "2026-01-11");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_generate_daily_hash_format() {
        let hash = generate_daily_hash("user", "project", "2026-01-15");
        // SHA256 hash should be 64 hex characters
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // ========================================================================
    // is_meaningful_message Tests
    // ========================================================================

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
    fn test_is_meaningful_message_exact_boundary() {
        // Exactly 10 characters should be meaningful
        assert!(is_meaningful_message("1234567890"));
        // 9 characters should not be meaningful
        assert!(!is_meaningful_message("123456789"));
    }

    #[test]
    fn test_is_meaningful_message_whitespace() {
        // Whitespace should be trimmed
        assert!(!is_meaningful_message("   short   "));
        assert!(is_meaningful_message("   1234567890   "));
    }

    #[test]
    fn test_is_meaningful_message_case_insensitive() {
        assert!(!is_meaningful_message("WARMUP"));
        assert!(!is_meaningful_message("WaRmUp"));
    }

    // ========================================================================
    // extract_tool_detail Tests
    // ========================================================================

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
    fn test_extract_tool_detail_write() {
        let input = serde_json::json!({
            "file_path": "/home/user/project/README.md"
        });
        let detail = extract_tool_detail("Write", &input);
        assert!(detail.is_some());
        assert!(detail.unwrap().contains("README.md"));
    }

    #[test]
    fn test_extract_tool_detail_read() {
        let input = serde_json::json!({
            "file_path": "/home/user/project/config.json"
        });
        let detail = extract_tool_detail("Read", &input);
        assert!(detail.is_some());
        assert!(detail.unwrap().contains("config.json"));
    }

    #[test]
    fn test_extract_tool_detail_short_path() {
        let input = serde_json::json!({
            "file_path": "src/main.rs"
        });
        let detail = extract_tool_detail("Edit", &input);
        assert_eq!(detail, Some("src/main.rs".to_string()));
    }

    #[test]
    fn test_extract_tool_detail_long_path() {
        let input = serde_json::json!({
            "file_path": "/home/user/projects/myapp/src/components/Button.tsx"
        });
        let detail = extract_tool_detail("Edit", &input);
        assert!(detail.is_some());
        let detail = detail.unwrap();
        // Should truncate to last 3 parts with .../ prefix
        assert!(detail.starts_with(".../"));
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

    #[test]
    fn test_extract_tool_detail_glob() {
        let input = serde_json::json!({
            "pattern": "**/*.rs"
        });
        let detail = extract_tool_detail("Glob", &input);
        assert_eq!(detail, Some("**/*.rs".to_string()));
    }

    #[test]
    fn test_extract_tool_detail_grep() {
        let input = serde_json::json!({
            "pattern": "fn main"
        });
        let detail = extract_tool_detail("Grep", &input);
        assert_eq!(detail, Some("fn main".to_string()));
    }

    #[test]
    fn test_extract_tool_detail_task() {
        let input = serde_json::json!({
            "description": "Explore codebase"
        });
        let detail = extract_tool_detail("Task", &input);
        assert_eq!(detail, Some("Explore codebase".to_string()));
    }

    #[test]
    fn test_extract_tool_detail_task_long_description() {
        let long_desc = "a".repeat(100);
        let input = serde_json::json!({
            "description": long_desc
        });
        let detail = extract_tool_detail("Task", &input);
        assert!(detail.is_some());
        assert_eq!(detail.unwrap().len(), 50);
    }

    #[test]
    fn test_extract_tool_detail_unknown_tool() {
        let input = serde_json::json!({
            "some_field": "value"
        });
        let detail = extract_tool_detail("UnknownTool", &input);
        assert!(detail.is_none());
    }

    #[test]
    fn test_extract_tool_detail_missing_field() {
        let input = serde_json::json!({
            "other_field": "value"
        });
        let detail = extract_tool_detail("Edit", &input);
        assert!(detail.is_none());
    }

    // ========================================================================
    // ToolUsage Tests
    // ========================================================================

    #[test]
    fn test_tool_usage_creation() {
        let usage = ToolUsage {
            tool_name: "Edit".to_string(),
            count: 5,
        };

        assert_eq!(usage.tool_name, "Edit");
        assert_eq!(usage.count, 5);
    }

    #[test]
    fn test_tool_usage_clone() {
        let usage = ToolUsage {
            tool_name: "Bash".to_string(),
            count: 10,
        };

        let cloned = usage.clone();

        assert_eq!(usage.tool_name, cloned.tool_name);
        assert_eq!(usage.count, cloned.count);
    }

    // ========================================================================
    // SessionMetadata Tests
    // ========================================================================

    #[test]
    fn test_session_metadata_creation() {
        let metadata = SessionMetadata {
            cwd: Some("/home/user/project".to_string()),
            first_ts: "2026-01-15T10:00:00Z".to_string(),
            last_ts: "2026-01-15T12:00:00Z".to_string(),
            first_msg: Some("Help me fix this bug".to_string()),
            message_count: 5,
        };

        assert_eq!(metadata.cwd, Some("/home/user/project".to_string()));
        assert_eq!(metadata.first_ts, "2026-01-15T10:00:00Z");
        assert_eq!(metadata.message_count, 5);
    }

    #[test]
    fn test_session_metadata_clone() {
        let metadata = SessionMetadata {
            cwd: Some("/path".to_string()),
            first_ts: "2026-01-15T10:00:00Z".to_string(),
            last_ts: "2026-01-15T11:00:00Z".to_string(),
            first_msg: None,
            message_count: 3,
        };

        let cloned = metadata.clone();

        assert_eq!(metadata.cwd, cloned.cwd);
        assert_eq!(metadata.first_ts, cloned.first_ts);
    }

    // ========================================================================
    // ParsedSession Tests
    // ========================================================================

    #[test]
    fn test_parsed_session_creation() {
        let session = ParsedSession {
            cwd: "/home/user/project".to_string(),
            first_timestamp: Some("2026-01-15T10:00:00Z".to_string()),
            last_timestamp: Some("2026-01-15T12:00:00Z".to_string()),
            message_count: 10,
            tool_usage: vec![
                ToolUsage { tool_name: "Edit".to_string(), count: 5 },
            ],
            files_modified: vec!["src/main.rs".to_string()],
            first_message: Some("Help me implement X".to_string()),
        };

        assert_eq!(session.cwd, "/home/user/project");
        assert_eq!(session.message_count, 10);
        assert_eq!(session.tool_usage.len(), 1);
        assert_eq!(session.files_modified.len(), 1);
    }

    #[test]
    fn test_parsed_session_clone() {
        let session = ParsedSession {
            cwd: "/path".to_string(),
            first_timestamp: None,
            last_timestamp: None,
            message_count: 0,
            tool_usage: vec![],
            files_modified: vec![],
            first_message: None,
        };

        let cloned = session.clone();

        assert_eq!(session.cwd, cloned.cwd);
        assert_eq!(session.message_count, cloned.message_count);
    }

    // ========================================================================
    // parse_session_fast Tests
    // ========================================================================

    #[test]
    fn test_parse_session_fast_valid_file() {
        let mut temp_file = NamedTempFile::new().unwrap();

        // Write valid JSONL content
        writeln!(temp_file, r#"{{"cwd":"/home/user/project","timestamp":"2026-01-15T10:00:00Z","type":"user","message":{{"role":"user","content":"This is a meaningful test message"}}}}"#).unwrap();
        writeln!(temp_file, r#"{{"timestamp":"2026-01-15T10:30:00Z","type":"assistant","message":{{"role":"assistant","content":"Response here"}}}}"#).unwrap();
        writeln!(temp_file, r#"{{"timestamp":"2026-01-15T11:00:00Z","type":"user","message":{{"role":"user","content":"Another meaningful message here"}}}}"#).unwrap();

        let path = PathBuf::from(temp_file.path());
        let result = parse_session_fast(&path);

        assert!(result.is_some());
        let metadata = result.unwrap();
        assert_eq!(metadata.cwd, Some("/home/user/project".to_string()));
        assert_eq!(metadata.first_ts, "2026-01-15T10:00:00Z");
        assert_eq!(metadata.last_ts, "2026-01-15T11:00:00Z");
        assert!(metadata.first_msg.is_some());
        assert!(metadata.message_count >= 1);
    }

    #[test]
    fn test_parse_session_fast_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = PathBuf::from(temp_file.path());

        let result = parse_session_fast(&path);

        // Empty file should return None (no timestamps)
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_session_fast_nonexistent_file() {
        let path = PathBuf::from("/nonexistent/path/to/file.jsonl");
        let result = parse_session_fast(&path);

        assert!(result.is_none());
    }

    #[test]
    fn test_parse_session_fast_invalid_json() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "not valid json").unwrap();
        writeln!(temp_file, "also not valid").unwrap();

        let path = PathBuf::from(temp_file.path());
        let result = parse_session_fast(&path);

        // Invalid JSON should return None (no timestamps parsed)
        assert!(result.is_none());
    }

    // ========================================================================
    // parse_session_full Tests
    // ========================================================================

    #[test]
    fn test_parse_session_full_valid_file() {
        let mut temp_file = NamedTempFile::new().unwrap();

        writeln!(temp_file, r#"{{"cwd":"/home/user/project","timestamp":"2026-01-15T10:00:00Z","message":{{"role":"user","content":"This is a meaningful user message"}}}}"#).unwrap();
        writeln!(temp_file, r#"{{"timestamp":"2026-01-15T10:30:00Z","message":{{"role":"assistant","content":[{{"type":"tool_use","name":"Edit","input":{{"file_path":"src/main.rs"}}}}]}}}}"#).unwrap();
        writeln!(temp_file, r#"{{"timestamp":"2026-01-15T11:00:00Z","message":{{"role":"user","content":"Another meaningful message"}}}}"#).unwrap();

        let path = PathBuf::from(temp_file.path());
        let result = parse_session_full(&path);

        assert!(result.is_some());
        let session = result.unwrap();
        assert_eq!(session.cwd, "/home/user/project");
        assert!(session.first_timestamp.is_some());
        assert!(session.last_timestamp.is_some());
        assert!(session.message_count >= 1);
    }

    #[test]
    fn test_parse_session_full_with_tool_usage() {
        let mut temp_file = NamedTempFile::new().unwrap();

        writeln!(temp_file, r#"{{"cwd":"/project","timestamp":"2026-01-15T10:00:00Z","message":{{"role":"user","content":"Please help me with this task"}}}}"#).unwrap();
        writeln!(temp_file, r#"{{"timestamp":"2026-01-15T10:05:00Z","message":{{"role":"assistant","content":[{{"type":"tool_use","name":"Read","input":{{"file_path":"README.md"}}}},{{"type":"tool_use","name":"Edit","input":{{"file_path":"src/lib.rs"}}}}]}}}}"#).unwrap();
        writeln!(temp_file, r#"{{"timestamp":"2026-01-15T10:10:00Z","message":{{"role":"assistant","content":[{{"type":"tool_use","name":"Edit","input":{{"file_path":"src/main.rs"}}}}]}}}}"#).unwrap();

        let path = PathBuf::from(temp_file.path());
        let result = parse_session_full(&path);

        assert!(result.is_some());
        let session = result.unwrap();
        // Should have tool usage recorded
        assert!(!session.tool_usage.is_empty());
        // Edit should have been used twice
        let edit_usage = session.tool_usage.iter().find(|t| t.tool_name == "Edit");
        assert!(edit_usage.is_some());
        assert_eq!(edit_usage.unwrap().count, 2);
    }

    #[test]
    fn test_parse_session_full_files_modified() {
        let mut temp_file = NamedTempFile::new().unwrap();

        writeln!(temp_file, r#"{{"cwd":"/project","timestamp":"2026-01-15T10:00:00Z","message":{{"role":"user","content":"Please modify these files for me"}}}}"#).unwrap();
        writeln!(temp_file, r#"{{"timestamp":"2026-01-15T10:05:00Z","message":{{"role":"assistant","content":[{{"type":"tool_use","name":"Write","input":{{"file_path":"new_file.rs"}}}},{{"type":"tool_use","name":"Edit","input":{{"file_path":"existing.rs"}}}}]}}}}"#).unwrap();

        let path = PathBuf::from(temp_file.path());
        let result = parse_session_full(&path);

        assert!(result.is_some());
        let session = result.unwrap();
        // Should track files modified by Edit and Write
        assert!(!session.files_modified.is_empty());
    }

    #[test]
    fn test_parse_session_full_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = PathBuf::from(temp_file.path());

        let result = parse_session_full(&path);

        // Should return Some with defaults for empty file
        assert!(result.is_some());
        let session = result.unwrap();
        assert_eq!(session.cwd, "");
        assert!(session.first_timestamp.is_none());
    }

    #[test]
    fn test_parse_session_full_nonexistent_file() {
        let path = PathBuf::from("/nonexistent/path/to/file.jsonl");
        let result = parse_session_full(&path);

        assert!(result.is_none());
    }

    // ========================================================================
    // SessionMessage Deserialization Tests
    // ========================================================================

    #[test]
    fn test_session_message_deserialization() {
        let json = r#"{
            "cwd": "/home/user/project",
            "timestamp": "2026-01-15T10:00:00Z",
            "type": "user",
            "message": {
                "role": "user",
                "content": "Hello"
            }
        }"#;

        let msg: SessionMessage = serde_json::from_str(json).unwrap();

        assert_eq!(msg.cwd, Some("/home/user/project".to_string()));
        assert_eq!(msg.timestamp, Some("2026-01-15T10:00:00Z".to_string()));
        assert_eq!(msg.msg_type, Some("user".to_string()));
        assert!(msg.message.is_some());
    }

    #[test]
    fn test_session_message_partial_deserialization() {
        let json = r#"{"timestamp": "2026-01-15T10:00:00Z"}"#;

        let msg: SessionMessage = serde_json::from_str(json).unwrap();

        assert!(msg.cwd.is_none());
        assert_eq!(msg.timestamp, Some("2026-01-15T10:00:00Z".to_string()));
        assert!(msg.msg_type.is_none());
        assert!(msg.message.is_none());
    }

    // ========================================================================
    // ToolUseContent Deserialization Tests
    // ========================================================================

    #[test]
    fn test_tool_use_content_deserialization() {
        let json = r#"{
            "type": "tool_use",
            "name": "Edit",
            "input": {"file_path": "src/main.rs"}
        }"#;

        let tool_use: ToolUseContent = serde_json::from_str(json).unwrap();

        assert_eq!(tool_use.content_type, Some("tool_use".to_string()));
        assert_eq!(tool_use.name, Some("Edit".to_string()));
        assert!(tool_use.input.is_some());
    }
}
