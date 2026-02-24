//! Snapshot Capture Service
//!
//! Captures raw session data into hourly buckets and persists them
//! to the `snapshot_raw_data` table. Each bucket contains user messages,
//! assistant responses, tool calls, files modified, and git commits
//! for a specific session within a one-hour window.

use chrono::{DateTime, Local, Timelike};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use crate::utils::create_command;
use uuid::Uuid;

use super::session_parser::{
    extract_tool_detail, is_meaningful_message, SessionMessage, ToolUseContent,
};
use super::sync::DiscoveredProject;
use super::worklog::{get_commits_in_time_range, get_git_user_email};

// ============ Types ============

/// A tool call record within an hourly bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool: String,
    pub input_summary: String,
    pub timestamp: String,
}

/// A git commit snapshot within an hourly bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitSnapshot {
    pub hash: String,
    pub message: String,
    pub timestamp: String,
    pub additions: i32,
    pub deletions: i32,
}

/// An hourly bucket of session data
#[derive(Debug, Clone)]
pub struct HourlyBucket {
    pub hour_bucket: String,
    pub user_messages: Vec<String>,
    pub assistant_summaries: Vec<String>,
    pub tool_calls: Vec<ToolCallRecord>,
    pub files_modified: Vec<String>,
    pub git_commits: Vec<CommitSnapshot>,
    pub message_count: usize,
}

/// Result of a snapshot capture cycle
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotCaptureResult {
    pub snapshots_created: usize,
    pub snapshots_updated: usize,
    pub errors: Vec<String>,
}

// ============ Parsing ============

/// Truncate an ISO 8601 timestamp to its hour boundary in local timezone.
/// e.g., "2026-01-26T02:35:00Z" → "2026-01-26T10:00:00" (if local is UTC+8)
///
/// Converts UTC to local timezone so that hour bucketing and date grouping
/// align with the user's actual working hours.
fn truncate_to_hour(timestamp: &str) -> Option<String> {
    let dt = DateTime::parse_from_rfc3339(timestamp).ok()?;
    let local_dt = dt.with_timezone(&Local);
    let truncated = local_dt
        .with_minute(0)
        .and_then(|d| d.with_second(0))
        .and_then(|d| d.with_nanosecond(0))?;
    Some(truncated.format("%Y-%m-%dT%H:%M:%S").to_string())
}

/// Parse a JSONL session file into hourly buckets.
///
/// Each bucket contains messages, tool calls, and file modifications
/// that occurred within that hour. Timestamps are truncated to hour boundaries.
pub fn parse_session_into_hourly_buckets(path: &PathBuf) -> Vec<HourlyBucket> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let reader = BufReader::new(file);

    let mut buckets: HashMap<String, HourlyBucket> = HashMap::new();

    for line in reader.lines().flatten() {
        let msg: SessionMessage = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let timestamp = match &msg.timestamp {
            Some(ts) => ts.clone(),
            None => continue,
        };

        let hour_key = match truncate_to_hour(&timestamp) {
            Some(h) => h,
            None => continue,
        };

        let bucket = buckets.entry(hour_key.clone()).or_insert_with(|| HourlyBucket {
            hour_bucket: hour_key,
            user_messages: Vec::new(),
            assistant_summaries: Vec::new(),
            tool_calls: Vec::new(),
            files_modified: Vec::new(),
            git_commits: Vec::new(),
            message_count: 0,
        });

        if let Some(ref message) = msg.message {
            match message.role.as_deref() {
                Some("user") => {
                    if let Some(content) = &message.content {
                        if let serde_json::Value::String(s) = content {
                            if is_meaningful_message(s) {
                                let capped: String = s.chars().take(500).collect();
                                bucket.user_messages.push(capped);
                                bucket.message_count += 1;
                            }
                        }
                    }
                }
                Some("assistant") => {
                    if let Some(content) = &message.content {
                        match content {
                            serde_json::Value::String(s) => {
                                let truncated: String = s.chars().take(200).collect();
                                bucket.assistant_summaries.push(truncated);
                            }
                            serde_json::Value::Array(arr) => {
                                for item in arr {
                                    // Extract text blocks (skip tool_use)
                                    if let Some(content_type) =
                                        item.get("type").and_then(|v| v.as_str())
                                    {
                                        if content_type == "text" {
                                            if let Some(text) =
                                                item.get("text").and_then(|v| v.as_str())
                                            {
                                                let truncated: String =
                                                    text.chars().take(200).collect();
                                                bucket.assistant_summaries.push(truncated);
                                            }
                                        }
                                    }

                                    // Extract tool calls
                                    if let Ok(tool_use) =
                                        serde_json::from_value::<ToolUseContent>(item.clone())
                                    {
                                        if tool_use.content_type.as_deref() == Some("tool_use") {
                                            if let Some(tool_name) = &tool_use.name {
                                                let input_summary = tool_use
                                                    .input
                                                    .as_ref()
                                                    .and_then(|input| {
                                                        extract_tool_detail(tool_name, input)
                                                    })
                                                    .unwrap_or_default();

                                                bucket.tool_calls.push(ToolCallRecord {
                                                    tool: tool_name.clone(),
                                                    input_summary,
                                                    timestamp: timestamp.clone(),
                                                });

                                                // Track file modifications
                                                if matches!(tool_name.as_str(), "Edit" | "Write")
                                                {
                                                    if let Some(input) = &tool_use.input {
                                                        if let Some(detail) =
                                                            extract_tool_detail(tool_name, input)
                                                        {
                                                            if !bucket
                                                                .files_modified
                                                                .contains(&detail)
                                                                && bucket.files_modified.len() < 50
                                                            {
                                                                bucket
                                                                    .files_modified
                                                                    .push(detail);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let mut result: Vec<HourlyBucket> = buckets.into_values().collect();
    result.sort_by(|a, b| a.hour_bucket.cmp(&b.hour_bucket));
    result
}

/// Enrich hourly buckets with git commit data from the project repository.
pub fn enrich_buckets_with_git_commits(
    buckets: &mut [HourlyBucket],
    project_path: &str,
) {
    use chrono::{Local, NaiveDateTime, TimeZone};
    use super::sync::resolve_git_root;

    // Resolve the actual git root from the project path
    // (project_path may be a subdirectory of the git repo)
    let git_root = resolve_git_root(project_path);
    let author = get_git_user_email(&git_root);

    for bucket in buckets.iter_mut() {
        // Parse hour_bucket to get time range
        // hour_bucket can be either:
        // - RFC3339 with timezone: "2026-01-30T10:00:00+08:00"
        // - Local time without timezone: "2026-01-30T10:00:00"
        let start = &bucket.hour_bucket;
        let (start_str, end_str) = match DateTime::parse_from_rfc3339(start) {
            Ok(dt) => {
                let end_dt = dt + chrono::Duration::hours(1);
                (dt.to_rfc3339(), end_dt.to_rfc3339())
            }
            Err(_) => {
                // Try parsing as NaiveDateTime (local time without timezone)
                match NaiveDateTime::parse_from_str(start, "%Y-%m-%dT%H:%M:%S") {
                    Ok(ndt) => {
                        let local_start = Local.from_local_datetime(&ndt).single();
                        let local_end = Local.from_local_datetime(&(ndt + chrono::Duration::hours(1))).single();
                        match (local_start, local_end) {
                            (Some(s), Some(e)) => (s.to_rfc3339(), e.to_rfc3339()),
                            _ => continue,
                        }
                    }
                    Err(_) => continue,
                }
            }
        };

        let commits = get_commits_in_time_range(&git_root, &start_str, &end_str, author.as_deref());
        for commit in commits {
            // Get file changes for additions/deletions
            let (additions, deletions) = get_commit_stats(&git_root, &commit.hash);

            bucket.git_commits.push(CommitSnapshot {
                hash: commit.hash,
                message: commit.message,
                timestamp: commit.time,
                additions,
                deletions,
            });
        }
    }
}

/// Get additions/deletions for a specific commit hash
fn get_commit_stats(repo_path: &str, hash: &str) -> (i32, i32) {
    let repo_dir = PathBuf::from(repo_path);
    if !repo_dir.exists() {
        return (0, 0);
    }

    let output = create_command("git")
        .arg("show")
        .arg("--numstat")
        .arg("--format=")
        .arg(hash)
        .current_dir(&repo_dir)
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return (0, 0),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut total_add = 0;
    let mut total_del = 0;

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && parts[0] != "-" && parts[1] != "-" {
            total_add += parts[0].parse::<i32>().unwrap_or(0);
            total_del += parts[1].parse::<i32>().unwrap_or(0);
        }
    }

    (total_add, total_del)
}

// ============ Persistence ============

/// Save hourly buckets to the snapshot_raw_data table.
/// Uses UPSERT (ON CONFLICT) to update existing records.
pub async fn save_hourly_snapshots(
    pool: &SqlitePool,
    user_id: &str,
    session_id: &str,
    project_path: &str,
    buckets: &[HourlyBucket],
) -> Result<usize, String> {
    let mut saved = 0;

    for bucket in buckets {
        let id = Uuid::new_v4().to_string();

        let user_messages_json =
            serde_json::to_string(&bucket.user_messages).unwrap_or_else(|_| "[]".to_string());
        let assistant_messages_json = serde_json::to_string(&bucket.assistant_summaries)
            .unwrap_or_else(|_| "[]".to_string());
        let tool_calls_json =
            serde_json::to_string(&bucket.tool_calls).unwrap_or_else(|_| "[]".to_string());
        let files_modified_json =
            serde_json::to_string(&bucket.files_modified).unwrap_or_else(|_| "[]".to_string());
        let git_commits_json =
            serde_json::to_string(&bucket.git_commits).unwrap_or_else(|_| "[]".to_string());

        let raw_size = user_messages_json.len()
            + assistant_messages_json.len()
            + tool_calls_json.len()
            + files_modified_json.len()
            + git_commits_json.len();

        let result = sqlx::query(
            r#"
            INSERT INTO snapshot_raw_data (id, user_id, session_id, project_path, hour_bucket,
                user_messages, assistant_messages, tool_calls, files_modified, git_commits,
                message_count, raw_size_bytes)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(session_id, hour_bucket) DO UPDATE SET
                user_messages = excluded.user_messages,
                assistant_messages = excluded.assistant_messages,
                tool_calls = excluded.tool_calls,
                files_modified = excluded.files_modified,
                git_commits = excluded.git_commits,
                message_count = excluded.message_count,
                raw_size_bytes = excluded.raw_size_bytes
            "#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(session_id)
        .bind(project_path)
        .bind(&bucket.hour_bucket)
        .bind(&user_messages_json)
        .bind(&assistant_messages_json)
        .bind(&tool_calls_json)
        .bind(&files_modified_json)
        .bind(&git_commits_json)
        .bind(bucket.message_count as i32)
        .bind(raw_size as i32)
        .execute(pool)
        .await;

        match result {
            Ok(_) => saved += 1,
            Err(e) => {
                log::warn!("Failed to save snapshot for {}/{}: {}", session_id, bucket.hour_bucket, e);
            }
        }
    }

    Ok(saved)
}

/// Extract session ID from a JSONL file path.
/// Typically the filename without extension (e.g., "abc123.jsonl" → "abc123").
fn extract_session_id(path: &PathBuf) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Capture snapshots for all sessions in a discovered project.
pub async fn capture_snapshots_for_project(
    pool: &SqlitePool,
    user_id: &str,
    project: &DiscoveredProject,
) -> Result<usize, String> {
    // Skip root filesystem path — MCP/no-context sessions have no meaningful project
    if project.canonical_path == "/" || project.canonical_path.is_empty() {
        return Ok(0);
    }

    let mut total_saved = 0;

    for claude_dir in &project.claude_dirs {
        // Look for JSONL files in the project's Claude directories
        let jsonl_files = find_jsonl_files(claude_dir);

        for jsonl_path in &jsonl_files {
            let session_id = extract_session_id(jsonl_path);

            // Parse session into hourly buckets
            let mut buckets = parse_session_into_hourly_buckets(jsonl_path);

            if buckets.is_empty() {
                continue;
            }

            // Enrich with git commit data
            enrich_buckets_with_git_commits(&mut buckets, &project.canonical_path);

            // Save to database
            match save_hourly_snapshots(pool, user_id, &session_id, &project.canonical_path, &buckets).await {
                Ok(n) => total_saved += n,
                Err(e) => {
                    log::warn!("Failed to save snapshots for session {}: {}", session_id, e);
                }
            }
        }
    }

    Ok(total_saved)
}

/// Find all .jsonl files in a directory (non-recursive)
fn find_jsonl_files(dir: &PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "jsonl" {
                        files.push(path);
                    }
                }
            }
        }
    }

    files
}

// ============ Tests ============

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_jsonl_line(role: &str, content: &str, timestamp: &str) -> String {
        if role == "user" {
            format!(
                r#"{{"timestamp":"{}","message":{{"role":"user","content":"{}"}}}}"#,
                timestamp, content
            )
        } else {
            format!(
                r#"{{"timestamp":"{}","message":{{"role":"assistant","content":"{}"}}}}"#,
                timestamp, content
            )
        }
    }

    fn make_tool_use_line(tool_name: &str, file_path: &str, timestamp: &str) -> String {
        format!(
            r#"{{"timestamp":"{}","message":{{"role":"assistant","content":[{{"type":"tool_use","name":"{}","input":{{"file_path":"{}"}}}}]}}}}"#,
            timestamp, tool_name, file_path
        )
    }

    #[test]
    fn test_truncate_to_hour() {
        // Converts to local timezone and truncates to hour boundary.
        // Output is naive local time (no offset), format: YYYY-MM-DDTHH:00:00
        let result = truncate_to_hour("2026-01-26T14:35:22+00:00");
        assert!(result.is_some());
        let r = result.unwrap();
        // Should end with :00:00 (truncated to hour)
        assert!(r.ends_with(":00:00"), "Expected truncated hour, got: {}", r);
        // Should NOT contain timezone offset
        assert!(!r.contains('+'), "Should not contain offset, got: {}", r);
        assert!(!r.ends_with('Z'), "Should not end with Z, got: {}", r);
        // Should be 19 chars: YYYY-MM-DDTHH:MM:SS
        assert_eq!(r.len(), 19, "Expected 19 chars, got: {} ({})", r.len(), r);

        // Another input
        let result2 = truncate_to_hour("2026-01-26T00:59:59+00:00");
        assert!(result2.is_some());
        let r2 = result2.unwrap();
        assert!(r2.ends_with(":00:00"));
        assert_eq!(r2.len(), 19);

        assert!(truncate_to_hour("invalid").is_none());
    }

    #[test]
    fn test_parse_session_into_hourly_buckets_multi_hour() {
        let mut file = NamedTempFile::new().unwrap();

        // Hour 14: 2 messages
        writeln!(file, "{}", make_jsonl_line("user", "Help me implement login", "2026-01-26T14:05:00+00:00")).unwrap();
        writeln!(file, "{}", make_jsonl_line("assistant", "Sure, I will help with login implementation", "2026-01-26T14:06:00+00:00")).unwrap();

        // Hour 15: 1 message + tool call
        writeln!(file, "{}", make_jsonl_line("user", "Now fix the tests please", "2026-01-26T15:10:00+00:00")).unwrap();
        writeln!(file, "{}", make_tool_use_line("Edit", "/src/test.rs", "2026-01-26T15:11:00+00:00")).unwrap();

        let path = file.path().to_path_buf();
        let buckets = parse_session_into_hourly_buckets(&path);

        assert_eq!(buckets.len(), 2, "Should have 2 hourly buckets");
        assert_eq!(buckets[0].user_messages.len(), 1);
        assert_eq!(buckets[0].assistant_summaries.len(), 1);
        assert_eq!(buckets[1].user_messages.len(), 1);
        assert_eq!(buckets[1].tool_calls.len(), 1);
        assert_eq!(buckets[1].files_modified.len(), 1);
    }

    #[test]
    fn test_parse_session_empty_file() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();
        let buckets = parse_session_into_hourly_buckets(&path);
        assert!(buckets.is_empty());
    }

    #[test]
    fn test_parse_session_nonexistent_file() {
        let path = PathBuf::from("/nonexistent/file.jsonl");
        let buckets = parse_session_into_hourly_buckets(&path);
        assert!(buckets.is_empty());
    }

    #[test]
    fn test_assistant_text_truncation() {
        let mut file = NamedTempFile::new().unwrap();

        let long_text = "a".repeat(500);
        writeln!(
            file,
            r#"{{"timestamp":"2026-01-26T14:05:00+00:00","message":{{"role":"assistant","content":"{}"}}}}"#,
            long_text
        )
        .unwrap();

        let path = file.path().to_path_buf();
        let buckets = parse_session_into_hourly_buckets(&path);

        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].assistant_summaries[0].len(), 200);
    }

    #[test]
    fn test_user_message_capped() {
        let mut file = NamedTempFile::new().unwrap();

        let long_msg = "x".repeat(1000);
        writeln!(
            file,
            r#"{{"timestamp":"2026-01-26T14:05:00+00:00","message":{{"role":"user","content":"{}"}}}}"#,
            long_msg
        )
        .unwrap();

        let path = file.path().to_path_buf();
        let buckets = parse_session_into_hourly_buckets(&path);

        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].user_messages[0].len(), 500);
    }

    #[test]
    fn test_extract_session_id() {
        let path = PathBuf::from("/some/dir/abc123.jsonl");
        assert_eq!(extract_session_id(&path), "abc123");

        let path2 = PathBuf::from("/some/dir/my-session.jsonl");
        assert_eq!(extract_session_id(&path2), "my-session");
    }

    #[test]
    fn test_git_commits_included_in_bucket() {
        // This test verifies the CommitSnapshot structure can be serialized
        let commit = CommitSnapshot {
            hash: "abc123".to_string(),
            message: "feat: add login".to_string(),
            timestamp: "2026-01-26T14:30:00+00:00".to_string(),
            additions: 50,
            deletions: 10,
        };
        let json = serde_json::to_string(&commit).unwrap();
        assert!(json.contains("abc123"));
        assert!(json.contains("feat: add login"));
    }

    #[test]
    fn test_find_jsonl_files() {
        let dir = tempfile::tempdir().unwrap();
        let jsonl_path = dir.path().join("session1.jsonl");
        let txt_path = dir.path().join("notes.txt");
        fs::write(&jsonl_path, "{}").unwrap();
        fs::write(&txt_path, "hello").unwrap();

        let files = find_jsonl_files(&dir.path().to_path_buf());
        assert_eq!(files.len(), 1);
        assert!(files[0].to_string_lossy().contains("session1.jsonl"));
    }

    #[test]
    fn test_enrich_buckets_with_git_commits() {
        // Test that enrich_buckets_with_git_commits can find commits
        // using resolve_git_root to find the actual git repository

        // Get the path to this crate (crates/recap-core)
        let crate_path = env!("CARGO_MANIFEST_DIR");
        // This is a subdirectory of the git repo, so resolve_git_root should find the parent

        // Create a bucket with a known time range that has commits
        let mut buckets = vec![HourlyBucket {
            hour_bucket: "2026-01-30T09:00:00".to_string(), // Local time without timezone
            user_messages: vec![],
            assistant_summaries: vec![],
            tool_calls: vec![],
            files_modified: vec![],
            git_commits: vec![],
            message_count: 0,
        }];

        // Enrich with commits - should find the commit at 09:28:59
        enrich_buckets_with_git_commits(&mut buckets, crate_path);

        println!("Bucket hour: {}", buckets[0].hour_bucket);
        println!("Found {} commits", buckets[0].git_commits.len());
        for c in &buckets[0].git_commits {
            println!("  - {} | {} | {}", c.hash, c.timestamp, c.message);
        }

        // Should have found at least 1 commit (the one at 09:28:59)
        assert!(
            !buckets[0].git_commits.is_empty(),
            "Should find commits in 09:00-10:00 range using resolve_git_root"
        );
    }

    #[test]
    fn test_enrich_buckets_resolves_git_root() {
        use crate::services::sync::resolve_git_root;

        // Test that resolve_git_root works correctly
        let crate_path = env!("CARGO_MANIFEST_DIR"); // crates/recap-core
        let git_root = resolve_git_root(crate_path);

        println!("crate_path: {}", crate_path);
        println!("git_root: {}", git_root);

        // The git root should be 2 levels up from crates/recap-core
        assert!(
            std::path::Path::new(&git_root).join(".git").exists(),
            "resolve_git_root should find the actual git root"
        );
    }
}
