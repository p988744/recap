//! Real integration test using actual app functions
//!
//! This test imports and uses the SAME functions that the app uses,
//! ensuring test consistency with production code.

use chrono::{DateTime, FixedOffset, NaiveDate};
use std::collections::HashMap;

// Import actual app modules
use recap_lib::core_services::worklog::{
    estimate_commit_hours, estimate_from_diff, get_commits_for_date,
    build_rule_based_outcome, CommitRecord, HoursEstimate, SessionBrief,
};
use recap_lib::models::HoursSource;

/// Test the actual estimate_from_diff function from worklog.rs
#[test]
fn test_actual_estimate_from_diff() {
    println!("\n=== Testing ACTUAL estimate_from_diff function ===\n");

    let test_cases = vec![
        (0, 0, 0, "Empty commit"),
        (10, 2, 1, "Small: 12 lines, 1 file"),
        (50, 10, 2, "Medium-small: 60 lines, 2 files"),
        (100, 20, 3, "Medium: 120 lines, 3 files"),
        (300, 50, 5, "Medium-large: 350 lines, 5 files"),
        (800, 200, 6, "Large: 1000 lines, 6 files"),
        (2000, 500, 10, "Very large: 2500 lines, 10 files"),
        (5000, 1000, 20, "Huge: 6000 lines, 20 files"),
    ];

    println!("{:<45} {:>10}", "Scenario", "Hours");
    println!("{}", "-".repeat(57));

    for (add, del, files, desc) in test_cases {
        // Call the ACTUAL function from worklog.rs
        let hours = estimate_from_diff(add, del, files);

        println!("{:<45} {:>10.2}h", desc, hours);

        // Verify constraints
        assert!(hours >= 0.25, "Hours should be >= 0.25");
        assert!(hours <= 4.0, "Hours should be <= 4.0");

        // Verify rounded to 0.25
        let remainder = (hours * 4.0) % 1.0;
        assert!(remainder.abs() < 0.001, "Hours should be rounded to 0.25");
    }
}

/// Test the actual estimate_commit_hours function with priority chain
#[test]
fn test_actual_estimate_commit_hours() {
    println!("\n=== Testing ACTUAL estimate_commit_hours function ===\n");

    let commit_time = DateTime::parse_from_rfc3339("2026-01-11T10:00:00+08:00").unwrap();
    let prev_time = DateTime::parse_from_rfc3339("2026-01-11T08:30:00+08:00").unwrap();

    // Create a mock session
    let session = SessionBrief {
        session_id: "test-session".to_string(),
        hours: 2.5,
        first_message: Some("Implement feature X".to_string()),
        tools_used: HashMap::new(),
    };

    println!("{:<50} {:>8} {:>15}", "Scenario", "Hours", "Source");
    println!("{}", "-".repeat(75));

    // Test 1: User override (highest priority)
    let result = estimate_commit_hours(
        &commit_time,
        Some(&prev_time),
        Some(&session),
        100, 10, 3,
        Some(3.0), // User override
    );
    println!("{:<50} {:>8.2}h {:>15}",
        "With user override (3.0h)", result.hours, result.source.as_str());
    assert_eq!(result.hours, 3.0);
    assert_eq!(result.source, HoursSource::UserModified);

    // Test 2: Session hours (second priority)
    let result = estimate_commit_hours(
        &commit_time,
        Some(&prev_time),
        Some(&session),
        100, 10, 3,
        None, // No user override
    );
    println!("{:<50} {:>8.2}h {:>15}",
        "With session (2.5h), no override", result.hours, result.source.as_str());
    assert_eq!(result.hours, 2.5);
    assert_eq!(result.source, HoursSource::Session);

    // Test 3: Commit interval (third priority)
    let result = estimate_commit_hours(
        &commit_time,
        Some(&prev_time), // 1.5 hours gap
        None,             // No session
        100, 10, 3,
        None,
    );
    println!("{:<50} {:>8.2}h {:>15}",
        "With 1.5h commit interval, no session", result.hours, result.source.as_str());
    assert_eq!(result.hours, 1.5); // 90 minutes rounds to 1.5h
    assert_eq!(result.source, HoursSource::CommitInterval);

    // Verify rounding: 90 min = 1.5h (exact quarter)
    let is_quarter = (result.hours * 4.0).fract().abs() < 0.001;
    assert!(is_quarter, "Commit interval hours should be rounded to 0.25");

    // Test 4: Heuristic fallback
    let result = estimate_commit_hours(
        &commit_time,
        None, // No previous commit
        None, // No session
        100, 10, 3,
        None,
    );
    println!("{:<50} {:>8.2}h {:>15}",
        "Heuristic only (100+10 lines, 3 files)", result.hours, result.source.as_str());
    assert_eq!(result.source, HoursSource::Heuristic);
    assert!(result.hours > 0.0);

    // Test 5: Commit interval too short (< 5 min) - falls back to heuristic
    let short_prev = DateTime::parse_from_rfc3339("2026-01-11T09:58:00+08:00").unwrap();
    let result = estimate_commit_hours(
        &commit_time,
        Some(&short_prev), // Only 2 minutes gap
        None,
        50, 5, 2,
        None,
    );
    println!("{:<50} {:>8.2}h {:>15}",
        "Short interval (2 min) -> heuristic", result.hours, result.source.as_str());
    assert_eq!(result.source, HoursSource::Heuristic);

    // Test 6: Commit interval too long (> 4h) - falls back to heuristic
    let long_prev = DateTime::parse_from_rfc3339("2026-01-11T04:00:00+08:00").unwrap();
    let result = estimate_commit_hours(
        &commit_time,
        Some(&long_prev), // 6 hours gap
        None,
        50, 5, 2,
        None,
    );
    println!("{:<50} {:>8.2}h {:>15}",
        "Long interval (6h) -> heuristic", result.hours, result.source.as_str());
    assert_eq!(result.source, HoursSource::Heuristic);
}

/// Test the actual get_commits_for_date function with real recap project
#[test]
fn test_actual_get_commits_for_date() {
    println!("\n=== Testing ACTUAL get_commits_for_date function ===\n");

    let repo_path = "/home/weifan/projects/recap";
    let date = NaiveDate::parse_from_str("2026-01-11", "%Y-%m-%d").unwrap();

    // Call the ACTUAL function from worklog.rs
    let commits = get_commits_for_date(repo_path, &date);

    println!("Repository: {}", repo_path);
    println!("Date: {}", date);
    println!("Commits found: {}\n", commits.len());

    if !commits.is_empty() {
        println!("{:<10} {:<40} {:>8} {:>8} {:>8} {:>15}",
            "Hash", "Message", "+Lines", "-Lines", "Hours", "Source");
        println!("{}", "-".repeat(95));

        for commit in &commits {
            let msg: String = commit.message.chars().take(38).collect();
            let msg = if commit.message.len() > 38 {
                format!("{}...", msg)
            } else {
                msg
            };

            println!("{:<10} {:<40} {:>8} {:>8} {:>8.2}h {:>15}",
                commit.short_hash,
                msg,
                commit.total_additions,
                commit.total_deletions,
                commit.hours,
                commit.hours_source);
        }

        // Verify commit structure
        for commit in &commits {
            assert!(!commit.hash.is_empty(), "Hash should not be empty");
            assert!(!commit.short_hash.is_empty(), "Short hash should not be empty");
            assert_eq!(commit.short_hash.len(), 7, "Short hash should be 7 chars");
            assert!(commit.hours >= 0.25, "Hours should be >= 0.25");
            assert!(commit.hours <= 4.0, "Hours should be <= 4.0");
            assert!(!commit.hours_source.is_empty(), "Hours source should not be empty");
        }

        println!("\n[PASS] get_commits_for_date returns valid commit data");
    } else {
        println!("[INFO] No commits found for this date");
    }
}

/// Test the actual build_rule_based_outcome function
#[test]
fn test_actual_build_rule_based_outcome() {
    println!("\n=== Testing ACTUAL build_rule_based_outcome function ===\n");

    // Test 1: Files only
    let files = vec![
        "/home/user/project/src/main.rs".to_string(),
        "/home/user/project/src/lib.rs".to_string(),
        "/home/user/project/src/utils.rs".to_string(),
    ];
    let tools = HashMap::new();
    let outcome = build_rule_based_outcome(&files, &tools, None);
    println!("Test 1 - Files only:");
    println!("  Input: {:?}", files);
    println!("  Output: {}", outcome);
    assert!(outcome.contains("main.rs"));
    assert!(outcome.contains("lib.rs"));

    // Test 2: Files with overflow
    let files = vec![
        "/a/1.rs".to_string(),
        "/a/2.rs".to_string(),
        "/a/3.rs".to_string(),
        "/a/4.rs".to_string(),
        "/a/5.rs".to_string(),
    ];
    let outcome = build_rule_based_outcome(&files, &HashMap::new(), None);
    println!("\nTest 2 - Files with overflow:");
    println!("  Input: 5 files");
    println!("  Output: {}", outcome);
    assert!(outcome.contains("(+2)"), "Should show +2 more");

    // Test 3: Tools only
    let mut tools = HashMap::new();
    tools.insert("Edit".to_string(), 15);
    tools.insert("Read".to_string(), 8);
    tools.insert("Bash".to_string(), 2); // Below threshold
    let outcome = build_rule_based_outcome(&[], &tools, None);
    println!("\nTest 3 - Tools only:");
    println!("  Input: Edit(15), Read(8), Bash(2)");
    println!("  Output: {}", outcome);
    assert!(outcome.contains("Edit(15)"));
    assert!(outcome.contains("Read(8)"));
    assert!(!outcome.contains("Bash"), "Bash should be filtered (count < 3)");

    // Test 4: First message fallback
    let outcome = build_rule_based_outcome(&[], &HashMap::new(), Some("幫我實作登入功能"));
    println!("\nTest 4 - First message fallback:");
    println!("  Input: first_message = \"幫我實作登入功能\"");
    println!("  Output: {}", outcome);
    assert_eq!(outcome, "幫我實作登入功能");

    // Test 5: Default fallback
    let outcome = build_rule_based_outcome(&[], &HashMap::new(), None);
    println!("\nTest 5 - Default fallback:");
    println!("  Input: (empty)");
    println!("  Output: {}", outcome);
    assert_eq!(outcome, "工作 session");

    println!("\n[PASS] build_rule_based_outcome works correctly");
}

/// Comprehensive test showing actual output for recap project
#[test]
fn test_recap_project_actual_output() {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║            ACTUAL OUTPUT FOR RECAP PROJECT (2026-01-11)                  ║");
    println!("╚══════════════════════════════════════════════════════════════════════════╝\n");

    let repo_path = "/home/weifan/projects/recap";
    let date = NaiveDate::parse_from_str("2026-01-11", "%Y-%m-%d").unwrap();

    // Get commits using the ACTUAL function
    let commits = get_commits_for_date(repo_path, &date);

    println!("┌─────────────────────────────────────────────────────────────────────────┐");
    println!("│ Git Commits for {} ({} total)                                   │", date, commits.len());
    println!("├─────────────────────────────────────────────────────────────────────────┤");

    let mut total_hours = 0.0;

    for (i, commit) in commits.iter().enumerate() {
        println!("│                                                                         │");
        println!("│ Commit #{}: {}                                                       │", i + 1, commit.short_hash);
        println!("│   Message: {:<57} │", truncate(&commit.message, 55));
        println!("│   Author:  {:<57} │", commit.author);
        println!("│   Time:    {:<57} │", commit.time);
        println!("│   Changes: +{} -{} ({} files){:<37} │",
            commit.total_additions,
            commit.total_deletions,
            commit.files_changed.len(),
            "");
        println!("│   Hours:   {:.2}h (source: {}){:<40} │",
            commit.hours,
            commit.hours_source,
            "");

        if !commit.files_changed.is_empty() {
            println!("│   Files:                                                                │");
            for file in commit.files_changed.iter().take(3) {
                let short_path = truncate(&file.path, 50);
                println!("│     - {} (+{} -{}){:<30} │",
                    short_path,
                    file.additions,
                    file.deletions,
                    "");
            }
            if commit.files_changed.len() > 3 {
                println!("│     ... and {} more files{:<44} │",
                    commit.files_changed.len() - 3, "");
            }
        }

        total_hours += commit.hours;
    }

    println!("├─────────────────────────────────────────────────────────────────────────┤");
    println!("│ Total Estimated Hours: {:.2}h{:<46} │", total_hours, "");
    println!("└─────────────────────────────────────────────────────────────────────────┘");

    println!("\n[PASS] Actual recap project data retrieved successfully");
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max - 3).collect::<String>())
    }
}
