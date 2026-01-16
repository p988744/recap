//! Work Items helpers
//!
//! Helper functions for session parsing (used for tests and internal operations).

use recap_core::services::is_meaningful_message;

/// Session metadata extracted from JSONL files
#[allow(dead_code)]
pub struct SessionMetadata {
    pub first_ts: String,
    pub last_ts: String,
    pub first_msg: Option<String>,
    pub cwd: Option<String>,
}

/// Optimized version: reads only the beginning and end of the file
/// instead of parsing the entire JSONL file line by line
#[allow(dead_code)]
pub fn parse_session_timestamps_fast(path: &std::path::PathBuf) -> Option<SessionMetadata> {
    use std::io::{BufRead, BufReader, Seek, SeekFrom};

    let file = std::fs::File::open(path).ok()?;
    let file_size = file.metadata().ok()?.len();

    // For small files (< 50KB), use the original approach
    if file_size < 50_000 {
        return parse_session_timestamps_full(path);
    }

    let mut reader = BufReader::new(file);

    let mut first_ts: Option<String> = None;
    let mut last_ts: Option<String> = None;
    let mut first_msg: Option<String> = None;
    let mut cwd: Option<String> = None;
    let mut meaningful_count = 0;

    // Read first 20 lines to get first_ts, cwd, and first_msg
    let mut lines_read = 0;
    let max_initial_lines = 20;

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) {
                    if first_ts.is_none() {
                        if let Some(ts) = msg.get("timestamp").and_then(|v| v.as_str()) {
                            first_ts = Some(ts.to_string());
                        }
                    }

                    if cwd.is_none() {
                        if let Some(c) = msg.get("cwd").and_then(|v| v.as_str()) {
                            cwd = Some(c.to_string());
                        }
                    }

                    if first_msg.is_none() {
                        if let Some(message) = msg.get("message") {
                            if message.get("role").and_then(|r| r.as_str()) == Some("user") {
                                if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                                    if is_meaningful_message(content) {
                                        meaningful_count += 1;
                                        first_msg = Some(content.trim().chars().take(150).collect());
                                    }
                                }
                            }
                        }
                    }
                }

                lines_read += 1;
                if lines_read >= max_initial_lines && first_ts.is_some() && cwd.is_some() {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    // If we couldn't get first_ts, fall back to full parse
    if first_ts.is_none() {
        return parse_session_timestamps_full(path);
    }

    // Read the last ~32KB of the file to find the last timestamp
    let tail_size: u64 = 32_000.min(file_size);
    let seek_pos = file_size.saturating_sub(tail_size);

    if reader.seek(SeekFrom::Start(seek_pos)).is_ok() {
        // Skip partial line if we're not at the start
        if seek_pos > 0 {
            let mut skip_line = String::new();
            let _ = reader.read_line(&mut skip_line);
        }

        // Read remaining lines to find the last timestamp
        for line in reader.lines().flatten() {
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(ts) = msg.get("timestamp").and_then(|v| v.as_str()) {
                    last_ts = Some(ts.to_string());
                }

                // Also look for meaningful messages in case we missed one
                if meaningful_count == 0 {
                    if let Some(message) = msg.get("message") {
                        if message.get("role").and_then(|r| r.as_str()) == Some("user") {
                            if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                                if is_meaningful_message(content) {
                                    meaningful_count += 1;
                                    if first_msg.is_none() {
                                        first_msg = Some(content.trim().chars().take(150).collect());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if meaningful_count == 0 {
        return None;
    }

    let last_ts = last_ts.or_else(|| first_ts.clone());
    match (first_ts, last_ts) {
        (Some(f), Some(l)) => Some(SessionMetadata {
            first_ts: f,
            last_ts: l,
            first_msg,
            cwd,
        }),
        _ => None,
    }
}

/// Full file parse (used for small files or as fallback)
#[allow(dead_code)]
pub fn parse_session_timestamps_full(path: &std::path::PathBuf) -> Option<SessionMetadata> {
    use std::io::{BufRead, BufReader};

    let file = std::fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut first_ts: Option<String> = None;
    let mut last_ts: Option<String> = None;
    let mut first_msg: Option<String> = None;
    let mut cwd: Option<String> = None;
    let mut meaningful_count = 0;

    for line in reader.lines().flatten() {
        if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) {
            if let Some(ts) = msg.get("timestamp").and_then(|v| v.as_str()) {
                if first_ts.is_none() {
                    first_ts = Some(ts.to_string());
                }
                last_ts = Some(ts.to_string());
            }

            if cwd.is_none() {
                if let Some(c) = msg.get("cwd").and_then(|v| v.as_str()) {
                    cwd = Some(c.to_string());
                }
            }

            if first_msg.is_none() {
                if let Some(message) = msg.get("message") {
                    if message.get("role").and_then(|r| r.as_str()) == Some("user") {
                        if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                            if is_meaningful_message(content) {
                                meaningful_count += 1;
                                first_msg = Some(content.trim().chars().take(150).collect());
                            }
                        }
                    }
                }
            }
        }
    }

    if meaningful_count == 0 {
        return None;
    }

    match (first_ts, last_ts) {
        (Some(f), Some(l)) => Some(SessionMetadata {
            first_ts: f,
            last_ts: l,
            first_msg,
            cwd,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use recap_core::services::{calculate_session_hours, get_commits_in_time_range, TimelineCommit};

    // Alias for backward compatibility with existing tests
    fn calculate_hours(start: &str, end: &str) -> f64 {
        calculate_session_hours(start, end)
    }

    fn get_commits_in_range(project_path: &str, start: &str, end: &str) -> Vec<TimelineCommit> {
        get_commits_in_time_range(project_path, start, end)
    }

    fn create_test_jsonl(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_calculate_hours_valid() {
        let start = "2025-01-10T09:00:00+08:00";
        let end = "2025-01-10T11:30:00+08:00";
        let hours = calculate_hours(start, end);
        assert!((hours - 2.5).abs() < 0.01);
    }

    #[test]
    fn test_calculate_hours_max_cap() {
        // Should cap at 8 hours
        let start = "2025-01-10T00:00:00+08:00";
        let end = "2025-01-10T12:00:00+08:00";
        let hours = calculate_hours(start, end);
        assert!((hours - 8.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_hours_invalid_format() {
        let hours = calculate_hours("invalid", "also-invalid");
        assert!((hours - 0.5).abs() < 0.01); // Default fallback
    }

    #[test]
    fn test_parse_session_timestamps_full_basic() {
        let content = r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"This is a meaningful test message"}}
{"timestamp":"2025-01-10T09:30:00+08:00","message":{"role":"assistant","content":"Response"}}
{"timestamp":"2025-01-10T10:00:00+08:00","message":{"role":"user","content":"Another message"}}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_full(&path);
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert_eq!(metadata.first_ts, "2025-01-10T09:00:00+08:00");
        assert_eq!(metadata.last_ts, "2025-01-10T10:00:00+08:00");
        assert_eq!(metadata.cwd, Some("/home/user/project".to_string()));
        assert!(metadata.first_msg.is_some());
    }

    #[test]
    fn test_parse_session_timestamps_full_no_meaningful_message() {
        let content = r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project"}
{"timestamp":"2025-01-10T09:30:00+08:00","message":{"role":"user","content":"short"}}
{"timestamp":"2025-01-10T10:00:00+08:00","message":{"role":"user","content":"warmup test"}}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_full(&path);
        assert!(result.is_none()); // No meaningful message found
    }

    #[test]
    fn test_parse_session_timestamps_full_skip_command_messages() {
        let content = r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"<command-name>test</command-name>"}}
{"timestamp":"2025-01-10T09:30:00+08:00","message":{"role":"user","content":"This is a real meaningful message here"}}
{"timestamp":"2025-01-10T10:00:00+08:00"}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_full(&path);
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert!(metadata.first_msg.unwrap().contains("real meaningful"));
    }

    #[test]
    fn test_parse_session_timestamps_fast_small_file() {
        // Small files should use full parse
        let content = r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"This is a meaningful test message"}}
{"timestamp":"2025-01-10T10:00:00+08:00"}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_fast(&path);
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert_eq!(metadata.first_ts, "2025-01-10T09:00:00+08:00");
        assert_eq!(metadata.last_ts, "2025-01-10T10:00:00+08:00");
    }

    #[test]
    fn test_parse_session_timestamps_fast_large_file() {
        // Create a large file (> 50KB) to test the fast path
        let mut content = String::new();

        // First line with timestamp and meaningful message
        content.push_str(r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"This is a meaningful test message for the session"}}"#);
        content.push('\n');

        // Add padding lines to make file > 50KB
        for i in 0..500 {
            content.push_str(&format!(
                r#"{{"timestamp":"2025-01-10T09:{:02}:00+08:00","message":{{"role":"assistant","content":"Response line {} with some padding text to make this longer and reach the size threshold we need for testing the fast path optimization"}}}}"#,
                i % 60,
                i
            ));
            content.push('\n');
        }

        // Last line with final timestamp
        content.push_str(r#"{"timestamp":"2025-01-10T17:00:00+08:00","message":{"role":"assistant","content":"Final response"}}"#);

        let file = create_test_jsonl(&content);
        let path = file.path().to_path_buf();

        // Verify file is large enough
        let file_size = std::fs::metadata(&path).unwrap().len();
        assert!(file_size > 50_000, "Test file should be > 50KB, got {} bytes", file_size);

        let result = parse_session_timestamps_fast(&path);
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert_eq!(metadata.first_ts, "2025-01-10T09:00:00+08:00");
        assert_eq!(metadata.last_ts, "2025-01-10T17:00:00+08:00");
        assert_eq!(metadata.cwd, Some("/home/user/project".to_string()));
    }

    #[test]
    fn test_get_commits_in_range_empty_path() {
        let commits = get_commits_in_range("", "2025-01-10T00:00:00+08:00", "2025-01-10T23:59:59+08:00");
        assert!(commits.is_empty());
    }

    #[test]
    fn test_get_commits_in_range_nonexistent_path() {
        let commits = get_commits_in_range("/nonexistent/path", "2025-01-10T00:00:00+08:00", "2025-01-10T23:59:59+08:00");
        assert!(commits.is_empty());
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_parse_empty_file() {
        let file = create_test_jsonl("");
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_full(&path);
        assert!(result.is_none());

        let result_fast = parse_session_timestamps_fast(&path);
        assert!(result_fast.is_none());
    }

    #[test]
    fn test_parse_single_line_file() {
        let content = r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"This is a single meaningful message"}}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_full(&path);
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert_eq!(metadata.first_ts, "2025-01-10T09:00:00+08:00");
        assert_eq!(metadata.last_ts, "2025-01-10T09:00:00+08:00");
    }

    #[test]
    fn test_parse_corrupted_json_with_valid_lines() {
        let content = r#"{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"Valid meaningful message here"}}
{this is not valid json at all}
{"timestamp":"2025-01-10T10:00:00+08:00"}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_full(&path);
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert_eq!(metadata.first_ts, "2025-01-10T09:00:00+08:00");
        assert_eq!(metadata.last_ts, "2025-01-10T10:00:00+08:00");
    }

    #[test]
    fn test_parse_message_truncation() {
        // Message longer than 150 chars should be truncated
        let long_message = "A".repeat(200);
        let content = format!(
            r#"{{"timestamp":"2025-01-10T09:00:00+08:00","cwd":"/home/user/project","message":{{"role":"user","content":"{}"}}}}"#,
            long_message
        );

        let file = create_test_jsonl(&content);
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_full(&path);
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert!(metadata.first_msg.is_some());
        assert_eq!(metadata.first_msg.unwrap().len(), 150);
    }

    #[test]
    fn test_parse_midnight_crossing_session() {
        // Session that starts before midnight and ends after
        let content = r#"{"timestamp":"2025-01-10T23:30:00+08:00","cwd":"/home/user/project","message":{"role":"user","content":"Late night meaningful work session"}}
{"timestamp":"2025-01-11T00:30:00+08:00","message":{"role":"assistant","content":"Response"}}"#;

        let file = create_test_jsonl(content);
        let path = file.path().to_path_buf();

        let result = parse_session_timestamps_full(&path);
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert_eq!(metadata.first_ts, "2025-01-10T23:30:00+08:00");
        assert_eq!(metadata.last_ts, "2025-01-11T00:30:00+08:00");

        // Hours should be 1 hour
        let hours = calculate_hours(&metadata.first_ts, &metadata.last_ts);
        assert!((hours - 1.0).abs() < 0.01);
    }
}
