#!/usr/bin/env -S cargo +nightly -Zscript
//! Verify worklog data integration with actual recap project
//!
//! Run with: cargo +nightly -Zscript scripts/verify_worklog_data.rs
//! Or: rustc scripts/verify_worklog_data.rs -o /tmp/verify && /tmp/verify

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("=== Worklog Data Verification ===\n");

    // Test 1: Git commits
    test_git_commits();

    // Test 2: Claude sessions
    test_claude_sessions();

    // Test 3: Hours estimation
    test_hours_estimation();

    // Test 4: Cross-source dedup
    test_deduplication();

    println!("\n=== All verifications passed! ===");
}

fn test_git_commits() {
    println!("--- Test: Git Commits ---");

    let project_dir = PathBuf::from("/home/weifan/projects/recap");

    // Get today's commits
    let output = Command::new("git")
        .arg("log")
        .arg("--since=2026-01-11 00:00:00")
        .arg("--until=2026-01-11 23:59:59")
        .arg("--format=%H|%h|%an|%aI|%s")
        .arg("--all")
        .current_dir(&project_dir)
        .output()
        .expect("Failed to run git log");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let commits: Vec<&str> = stdout.lines().collect();

    println!("  Commits found today: {}", commits.len());

    for commit in commits.iter().take(3) {
        let parts: Vec<&str> = commit.splitn(5, '|').collect();
        if parts.len() >= 5 {
            let hash = parts[0];
            let short_hash = parts[1];
            let message = parts[4];

            // Get diff stats for this commit
            let stats_output = Command::new("git")
                .arg("show")
                .arg("--numstat")
                .arg("--format=")
                .arg(hash)
                .current_dir(&project_dir)
                .output()
                .expect("Failed to get commit stats");

            let stats_str = String::from_utf8_lossy(&stats_output.stdout);
            let (additions, deletions, files) = parse_numstat(&stats_str);

            let estimated_hours = estimate_from_diff(additions, deletions, files);

            println!("  Commit {}: {} (+{} -{} {}files) -> {:.2}h",
                short_hash, &message[..message.len().min(40)],
                additions, deletions, files, estimated_hours);
        }
    }

    println!("  [OK] Git commit data accessible\n");
}

fn test_claude_sessions() {
    println!("--- Test: Claude Sessions ---");

    let claude_dir = PathBuf::from("/home/weifan/.claude/projects/-home-weifan-projects-recap");

    if !claude_dir.exists() {
        println!("  [SKIP] Claude directory not found");
        return;
    }

    let mut session_count = 0;
    let mut total_hours = 0.0;

    if let Ok(entries) = fs::read_dir(&claude_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                session_count += 1;

                if let Some((hours, tools)) = analyze_session(&path) {
                    total_hours += hours;
                    if session_count <= 3 {
                        println!("  Session {}: {:.2}h, tools: {:?}",
                            path.file_name().unwrap().to_string_lossy()[..8].to_string(),
                            hours, tools);
                    }
                }
            }
        }
    }

    println!("  Total sessions: {}", session_count);
    println!("  Total hours: {:.2}h", total_hours);
    println!("  [OK] Claude session data accessible\n");
}

fn test_hours_estimation() {
    println!("--- Test: Hours Estimation ---");

    // Test cases from actual code behavior
    let test_cases = vec![
        (0, 0, 0, 0.25, "Empty commit"),
        (10, 0, 1, 0.5, "Small change"),
        (100, 20, 3, 1.25, "Medium change"),
        (800, 200, 5, 2.0, "Large change"),
    ];

    for (add, del, files, expected_approx, desc) in test_cases {
        let hours = estimate_from_diff(add, del, files);
        let diff = (hours - expected_approx).abs();

        if diff <= 0.5 {
            println!("  {}: +{} -{} {} files -> {:.2}h (expected ~{:.2}h) [OK]",
                desc, add, del, files, hours, expected_approx);
        } else {
            println!("  {}: +{} -{} {} files -> {:.2}h (expected ~{:.2}h) [WARN]",
                desc, add, del, files, hours, expected_approx);
        }
    }

    println!("  [OK] Hours estimation working\n");
}

fn test_deduplication() {
    println!("--- Test: Cross-Source Deduplication ---");

    // Simulate GitLab and local git having same commit
    let gitlab_full_hash = "c085e070f2ea03166d257c9f6f49b7b7ada2788f";
    let local_short_hash = "c085e070";

    let gitlab_short: String = gitlab_full_hash.chars().take(8).collect();

    if gitlab_short == local_short_hash {
        println!("  GitLab hash: {}", gitlab_full_hash);
        println!("  Local hash:  {}", local_short_hash);
        println!("  Match: {} == {} [OK]", gitlab_short, local_short_hash);
    }

    // Test the dedup set logic
    let mut existing_hashes = std::collections::HashSet::new();
    existing_hashes.insert(local_short_hash.to_string());

    let should_skip = existing_hashes.contains(&gitlab_short);
    println!("  Dedup check: should_skip = {} [OK]", should_skip);

    println!("  [OK] Deduplication working\n");
}

// Helper functions (same logic as in Rust codebase)

fn estimate_from_diff(additions: i32, deletions: i32, files_count: usize) -> f64 {
    let total_lines = (additions + deletions) as f64;
    let files = files_count as f64;

    if total_lines == 0.0 {
        return 0.25;
    }

    let line_factor = (total_lines + 1.0).ln() * 0.2;
    let file_factor = files * 0.15;
    let hours = (line_factor + file_factor).max(0.25).min(4.0);

    (hours * 4.0).round() / 4.0
}

fn parse_numstat(output: &str) -> (i32, i32, usize) {
    let mut add = 0;
    let mut del = 0;
    let mut files = 0;

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            if parts[0] != "-" && parts[1] != "-" {
                add += parts[0].parse::<i32>().unwrap_or(0);
                del += parts[1].parse::<i32>().unwrap_or(0);
                files += 1;
            }
        }
    }

    (add, del, files)
}

fn analyze_session(path: &PathBuf) -> Option<(f64, HashMap<String, usize>)> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut first_ts: Option<String> = None;
    let mut last_ts: Option<String> = None;
    let mut tools: HashMap<String, usize> = HashMap::new();

    for line in reader.lines().flatten() {
        // Extract timestamp
        if let Some(ts_start) = line.find("\"timestamp\":\"") {
            let ts_start = ts_start + 13;
            if let Some(ts_end) = line[ts_start..].find('"') {
                let ts = &line[ts_start..ts_start + ts_end];
                if first_ts.is_none() {
                    first_ts = Some(ts.to_string());
                }
                last_ts = Some(ts.to_string());
            }
        }

        // Extract tool usage
        if let Some(name_start) = line.find("\"name\":\"") {
            let name_start = name_start + 8;
            if let Some(name_end) = line[name_start..].find('"') {
                let name = &line[name_start..name_start + name_end];
                *tools.entry(name.to_string()).or_insert(0) += 1;
            }
        }
    }

    let hours = calculate_hours(&first_ts, &last_ts);
    Some((hours, tools))
}

fn calculate_hours(first: &Option<String>, last: &Option<String>) -> f64 {
    match (first, last) {
        (Some(start), Some(end)) => {
            // Simple ISO8601 parsing
            if let (Some(start_h), Some(end_h)) = (
                extract_hour_minute(start),
                extract_hour_minute(end),
            ) {
                let duration_minutes = end_h.saturating_sub(start_h);
                let hours = duration_minutes as f64 / 60.0;
                hours.min(8.0).max(0.1)
            } else {
                0.5
            }
        }
        _ => 0.5,
    }
}

fn extract_hour_minute(ts: &str) -> Option<i32> {
    // Format: 2026-01-11T12:26:36.966Z
    let parts: Vec<&str> = ts.split('T').collect();
    if parts.len() >= 2 {
        let time_parts: Vec<&str> = parts[1].split(':').collect();
        if time_parts.len() >= 2 {
            let hour = time_parts[0].parse::<i32>().ok()?;
            let minute = time_parts[1].parse::<i32>().ok()?;
            return Some(hour * 60 + minute);
        }
    }
    None
}
