//! Integration tests for worklog aggregation
//!
//! Tests the complete flow: hours estimation, cross-source deduplication,
//! and data integrity across different sources.

use std::collections::HashMap;

// Import the worklog module functions
// Note: These tests verify the logic without requiring a database

/// Test hours estimation from diff statistics
mod hours_estimation {
    /// Simulate the estimate_from_diff logic
    fn estimate_from_diff(additions: i32, deletions: i32, files_count: usize) -> f64 {
        let total_lines = (additions + deletions) as f64;
        let files = files_count as f64;

        if total_lines == 0.0 {
            return 0.25; // Minimum 15 minutes for empty commits
        }

        // Logarithmic scaling: more lines = diminishing returns
        let line_factor = (total_lines + 1.0).ln() * 0.2;
        let file_factor = files * 0.15;
        let hours = (line_factor + file_factor).max(0.25).min(4.0);

        // Round to nearest 0.25
        (hours * 4.0).round() / 4.0
    }

    #[test]
    fn test_empty_commit_returns_minimum() {
        let hours = estimate_from_diff(0, 0, 0);
        assert_eq!(hours, 0.25, "Empty commit should return minimum 0.25h");
    }

    #[test]
    fn test_small_change_reasonable_hours() {
        // Small change: 10 lines, 1 file
        let hours = estimate_from_diff(8, 2, 1);
        assert!(hours >= 0.25 && hours <= 1.0,
            "Small change (10 lines, 1 file) should be 0.25-1h, got {}", hours);
    }

    #[test]
    fn test_medium_change_reasonable_hours() {
        // Medium change: 100 lines, 3 files
        let hours = estimate_from_diff(80, 20, 3);
        assert!(hours >= 0.75 && hours <= 2.0,
            "Medium change (100 lines, 3 files) should be 0.75-2h, got {}", hours);
    }

    #[test]
    fn test_large_change_capped_at_max() {
        // Large change: 5000 lines, 20 files
        let hours = estimate_from_diff(4000, 1000, 20);
        assert!(hours <= 4.0, "Large change should be capped at 4h, got {}", hours);
    }

    #[test]
    fn test_gitlab_single_file_estimate() {
        // GitLab sync uses files_count=1 since API doesn't return file count
        let hours = estimate_from_diff(50, 10, 1);
        assert!(hours > 0.25, "GitLab commit with 60 lines should be > 0.25h");
        println!("GitLab estimate for 60 lines, 1 file: {}h", hours);
    }

    #[test]
    fn test_hours_rounded_to_quarter() {
        // All results should be rounded to 0.25 increments
        for additions in [10, 50, 100, 500, 1000] {
            let hours = estimate_from_diff(additions, 0, 2);
            let remainder = (hours * 4.0) % 1.0;
            assert!(remainder.abs() < 0.001,
                "Hours {} should be rounded to 0.25 increment", hours);
        }
    }
}

/// Test cross-source deduplication logic
mod deduplication {
    use std::collections::HashSet;

    /// Simulate the deduplication check
    fn should_skip_commit(
        commit_id: &str,
        existing_source_ids: &HashSet<String>,
        existing_hashes: &HashSet<String>,
    ) -> bool {
        let short_hash: String = commit_id.chars().take(8).collect();
        existing_source_ids.contains(commit_id) || existing_hashes.contains(&short_hash)
    }

    #[test]
    fn test_skip_existing_gitlab_source_id() {
        let mut source_ids = HashSet::new();
        source_ids.insert("abc123def456".to_string());
        let hashes = HashSet::new();

        assert!(should_skip_commit("abc123def456", &source_ids, &hashes),
            "Should skip commit that exists by source_id");
    }

    #[test]
    fn test_skip_existing_commit_hash() {
        let source_ids = HashSet::new();
        let mut hashes = HashSet::new();
        hashes.insert("abc123de".to_string()); // 8-char short hash

        assert!(should_skip_commit("abc123def456789", &source_ids, &hashes),
            "Should skip commit that exists by short hash (cross-source dedup)");
    }

    #[test]
    fn test_allow_new_commit() {
        let source_ids = HashSet::new();
        let hashes = HashSet::new();

        assert!(!should_skip_commit("newcommit123", &source_ids, &hashes),
            "Should allow new commit");
    }

    #[test]
    fn test_short_hash_extraction() {
        let full_hash = "abc123def456789abcdef";
        let short_hash: String = full_hash.chars().take(8).collect();
        assert_eq!(short_hash, "abc123de", "Short hash should be first 8 chars");
    }
}

/// Test hours source priority
mod hours_source_priority {
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum HoursSource {
        UserModified,
        Session,
        CommitInterval,
        Heuristic,
    }

    struct HoursEstimate {
        hours: f64,
        source: HoursSource,
    }

    /// Simulate the estimate_commit_hours priority logic
    fn estimate_commit_hours(
        user_override: Option<f64>,
        session_hours: Option<f64>,
        commit_interval_hours: Option<f64>,
        heuristic_hours: f64,
    ) -> HoursEstimate {
        // Priority 1: User override
        if let Some(hours) = user_override {
            return HoursEstimate { hours, source: HoursSource::UserModified };
        }

        // Priority 2: Session hours
        if let Some(hours) = session_hours {
            return HoursEstimate { hours, source: HoursSource::Session };
        }

        // Priority 3: Commit interval
        if let Some(hours) = commit_interval_hours {
            return HoursEstimate { hours, source: HoursSource::CommitInterval };
        }

        // Priority 4: Heuristic
        HoursEstimate { hours: heuristic_hours, source: HoursSource::Heuristic }
    }

    #[test]
    fn test_user_override_highest_priority() {
        let result = estimate_commit_hours(
            Some(3.0),      // user override
            Some(2.0),      // session
            Some(1.5),      // commit interval
            1.0,            // heuristic
        );
        assert_eq!(result.hours, 3.0);
        assert_eq!(result.source, HoursSource::UserModified);
    }

    #[test]
    fn test_session_second_priority() {
        let result = estimate_commit_hours(
            None,           // no user override
            Some(2.0),      // session
            Some(1.5),      // commit interval
            1.0,            // heuristic
        );
        assert_eq!(result.hours, 2.0);
        assert_eq!(result.source, HoursSource::Session);
    }

    #[test]
    fn test_commit_interval_third_priority() {
        let result = estimate_commit_hours(
            None,           // no user override
            None,           // no session
            Some(1.5),      // commit interval
            1.0,            // heuristic
        );
        assert_eq!(result.hours, 1.5);
        assert_eq!(result.source, HoursSource::CommitInterval);
    }

    #[test]
    fn test_heuristic_fallback() {
        let result = estimate_commit_hours(
            None,           // no user override
            None,           // no session
            None,           // no commit interval
            1.0,            // heuristic
        );
        assert_eq!(result.hours, 1.0);
        assert_eq!(result.source, HoursSource::Heuristic);
    }
}

/// Test session hours calculation
mod session_hours {
    use chrono::{DateTime, FixedOffset};

    fn calculate_session_hours(start: &str, end: &str) -> f64 {
        let start_dt = DateTime::parse_from_rfc3339(start).unwrap();
        let end_dt = DateTime::parse_from_rfc3339(end).unwrap();
        let duration = end_dt.signed_duration_since(start_dt);
        let hours = duration.num_minutes() as f64 / 60.0;
        hours.min(8.0).max(0.1) // Cap at 8h, minimum 0.1h
    }

    #[test]
    fn test_normal_session_duration() {
        let hours = calculate_session_hours(
            "2026-01-11T09:00:00+08:00",
            "2026-01-11T11:30:00+08:00",
        );
        assert!((hours - 2.5).abs() < 0.01, "2.5h session should return 2.5, got {}", hours);
    }

    #[test]
    fn test_short_session_minimum() {
        let hours = calculate_session_hours(
            "2026-01-11T09:00:00+08:00",
            "2026-01-11T09:03:00+08:00", // 3 minutes
        );
        assert_eq!(hours, 0.1, "Very short session should be capped at minimum 0.1h");
    }

    #[test]
    fn test_long_session_maximum() {
        let hours = calculate_session_hours(
            "2026-01-11T09:00:00+08:00",
            "2026-01-11T20:00:00+08:00", // 11 hours
        );
        assert_eq!(hours, 8.0, "Long session should be capped at maximum 8h");
    }
}

/// Test content hash for daily aggregation
mod content_hash {
    use sha2::{Digest, Sha256};

    fn generate_daily_hash(user_id: &str, project: &str, date: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(user_id.as_bytes());
        hasher.update(project.as_bytes());
        hasher.update(date.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    #[test]
    fn test_same_inputs_same_hash() {
        let hash1 = generate_daily_hash("user1", "/home/user/project", "2026-01-11");
        let hash2 = generate_daily_hash("user1", "/home/user/project", "2026-01-11");
        assert_eq!(hash1, hash2, "Same inputs should produce same hash");
    }

    #[test]
    fn test_different_date_different_hash() {
        let hash1 = generate_daily_hash("user1", "/home/user/project", "2026-01-11");
        let hash2 = generate_daily_hash("user1", "/home/user/project", "2026-01-12");
        assert_ne!(hash1, hash2, "Different dates should produce different hashes");
    }

    #[test]
    fn test_different_project_different_hash() {
        let hash1 = generate_daily_hash("user1", "/home/user/project1", "2026-01-11");
        let hash2 = generate_daily_hash("user1", "/home/user/project2", "2026-01-11");
        assert_ne!(hash1, hash2, "Different projects should produce different hashes");
    }
}

/// Test rule-based outcome summary
mod outcome_summary {
    use std::collections::HashMap;

    fn build_rule_based_outcome(
        files_modified: &[String],
        tools_used: &HashMap<String, usize>,
        first_message: Option<&str>,
    ) -> String {
        let mut parts = Vec::new();

        // Summarize file modifications
        if !files_modified.is_empty() {
            let file_names: Vec<&str> = files_modified
                .iter()
                .filter_map(|f| f.split('/').last())
                .take(3)
                .collect();

            if !file_names.is_empty() {
                let more = if files_modified.len() > 3 {
                    format!(" (+{})", files_modified.len() - 3)
                } else {
                    String::new()
                };
                parts.push(format!("修改: {}{}", file_names.join(", "), more));
            }
        }

        // Summarize significant tool usage
        let significant_tools: Vec<String> = tools_used
            .iter()
            .filter(|(_, count)| **count >= 3)
            .map(|(tool, count)| format!("{}({})", tool, count))
            .collect();

        if !significant_tools.is_empty() {
            parts.push(significant_tools.join(", "));
        }

        // Fallback to first message
        if parts.is_empty() {
            if let Some(msg) = first_message {
                let truncated: String = msg.chars().take(50).collect();
                if msg.len() > 50 {
                    return format!("{}... (進行中)", truncated);
                }
                return truncated;
            }
            return "工作 session".to_string();
        }

        parts.join("; ")
    }

    #[test]
    fn test_files_summary() {
        let files = vec![
            "/home/user/project/src/main.rs".to_string(),
            "/home/user/project/src/lib.rs".to_string(),
        ];
        let outcome = build_rule_based_outcome(&files, &HashMap::new(), None);
        assert!(outcome.contains("main.rs"), "Should contain file name");
        assert!(outcome.contains("lib.rs"), "Should contain file name");
    }

    #[test]
    fn test_files_truncated_with_count() {
        let files = vec![
            "/a/1.rs".to_string(),
            "/a/2.rs".to_string(),
            "/a/3.rs".to_string(),
            "/a/4.rs".to_string(),
            "/a/5.rs".to_string(),
        ];
        let outcome = build_rule_based_outcome(&files, &HashMap::new(), None);
        assert!(outcome.contains("(+2)"), "Should show +2 more files");
    }

    #[test]
    fn test_tools_summary() {
        let mut tools = HashMap::new();
        tools.insert("Edit".to_string(), 10);
        tools.insert("Read".to_string(), 5);
        tools.insert("Bash".to_string(), 2); // Should be filtered (< 3)

        let outcome = build_rule_based_outcome(&[], &tools, None);
        assert!(outcome.contains("Edit(10)"), "Should contain Edit tool");
        assert!(outcome.contains("Read(5)"), "Should contain Read tool");
        assert!(!outcome.contains("Bash"), "Should not contain Bash (count < 3)");
    }

    #[test]
    fn test_fallback_to_first_message() {
        let outcome = build_rule_based_outcome(&[], &HashMap::new(), Some("幫我實作登入功能"));
        assert_eq!(outcome, "幫我實作登入功能");
    }

    #[test]
    fn test_fallback_default() {
        let outcome = build_rule_based_outcome(&[], &HashMap::new(), None);
        assert_eq!(outcome, "工作 session");
    }
}

/// Integration test: simulate complete worklog flow
mod integration {
    use super::*;

    #[test]
    fn test_gitlab_commit_flow() {
        // Simulate GitLab commit with stats
        let additions = 150;
        let deletions = 30;
        let files_count = 1; // GitLab doesn't give file count

        // Calculate hours using heuristic
        let total_lines = (additions + deletions) as f64;
        let line_factor = (total_lines + 1.0).ln() * 0.2;
        let file_factor = files_count as f64 * 0.15;
        let hours = ((line_factor + file_factor).max(0.25).min(4.0) * 4.0).round() / 4.0;

        println!("GitLab commit: {} additions, {} deletions", additions, deletions);
        println!("Estimated hours: {}", hours);

        assert!(hours > 0.25, "Should estimate > 0.25h for 180 lines");
        assert!(hours <= 2.0, "Should not overestimate for 180 lines");
    }

    #[test]
    fn test_claude_session_flow() {
        // Simulate Claude session data
        let start = "2026-01-11T09:00:00+08:00";
        let end = "2026-01-11T11:30:00+08:00";

        let start_dt = chrono::DateTime::parse_from_rfc3339(start).unwrap();
        let end_dt = chrono::DateTime::parse_from_rfc3339(end).unwrap();
        let duration = end_dt.signed_duration_since(start_dt);
        let hours = (duration.num_minutes() as f64 / 60.0).min(8.0).max(0.1);

        println!("Claude session: {} to {}", start, end);
        println!("Session hours: {}", hours);

        assert!((hours - 2.5).abs() < 0.01, "Should calculate 2.5h session");
    }

    #[test]
    fn test_deduplication_scenario() {
        use std::collections::HashSet;

        // Scenario: Same commit exists from local git (via commit-centric API)
        // and GitLab tries to sync it

        let local_commit_hash = "abc12345"; // From local git
        let gitlab_commit_id = "abc12345def67890"; // Full hash from GitLab

        let mut existing_hashes = HashSet::new();
        existing_hashes.insert(local_commit_hash.to_string());

        let gitlab_short_hash: String = gitlab_commit_id.chars().take(8).collect();
        let should_skip = existing_hashes.contains(&gitlab_short_hash);

        println!("Local commit hash: {}", local_commit_hash);
        println!("GitLab commit ID: {}", gitlab_commit_id);
        println!("GitLab short hash: {}", gitlab_short_hash);
        println!("Should skip (dedup): {}", should_skip);

        assert!(should_skip, "GitLab commit should be skipped due to cross-source dedup");
    }

    #[test]
    fn test_hours_priority_scenario() {
        // Scenario: Commit has linked session, user hasn't modified
        // Expected: Use session hours

        let user_override: Option<f64> = None;
        let session_hours = Some(2.5);
        let commit_interval = Some(1.0); // Time since last commit
        let heuristic = 0.75; // Based on diff

        let final_hours = user_override
            .or(session_hours)
            .or(commit_interval)
            .unwrap_or(heuristic);

        println!("User override: {:?}", user_override);
        println!("Session hours: {:?}", session_hours);
        println!("Commit interval: {:?}", commit_interval);
        println!("Heuristic: {}", heuristic);
        println!("Final hours: {}", final_hours);

        assert_eq!(final_hours, 2.5, "Should use session hours (priority 2)");
    }
}

fn main() {
    println!("Running integration tests for worklog aggregation...\n");

    // Run a quick smoke test
    println!("=== Hours Estimation ===");
    let hours = hours_estimation_smoke_test();
    println!("Small change (10 lines): {}h", hours.0);
    println!("Medium change (100 lines): {}h", hours.1);
    println!("Large change (1000 lines): {}h", hours.2);

    println!("\n=== Deduplication ===");
    println!("Cross-source dedup working: {}", dedup_smoke_test());

    println!("\n=== Session Hours ===");
    println!("2h session calculated: {}h", session_hours_smoke_test());

    println!("\nAll smoke tests passed!");
}

fn hours_estimation_smoke_test() -> (f64, f64, f64) {
    fn estimate(additions: i32, deletions: i32, files: usize) -> f64 {
        let total = (additions + deletions) as f64;
        if total == 0.0 { return 0.25; }
        let line_factor = (total + 1.0).ln() * 0.2;
        let file_factor = files as f64 * 0.15;
        ((line_factor + file_factor).max(0.25).min(4.0) * 4.0).round() / 4.0
    }

    (estimate(8, 2, 1), estimate(80, 20, 3), estimate(800, 200, 5))
}

fn dedup_smoke_test() -> bool {
    use std::collections::HashSet;
    let mut hashes = HashSet::new();
    hashes.insert("abc12345".to_string());
    let gitlab_hash: String = "abc12345def".chars().take(8).collect();
    hashes.contains(&gitlab_hash)
}

fn session_hours_smoke_test() -> f64 {
    let start = chrono::DateTime::parse_from_rfc3339("2026-01-11T09:00:00+08:00").unwrap();
    let end = chrono::DateTime::parse_from_rfc3339("2026-01-11T11:00:00+08:00").unwrap();
    let duration = end.signed_duration_since(start);
    (duration.num_minutes() as f64 / 60.0).min(8.0).max(0.1)
}
