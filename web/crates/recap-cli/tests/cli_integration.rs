//! Integration tests for recap-cli
//!
//! These tests verify the CLI commands work end-to-end.

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a Command for the recap binary
fn recap() -> Command {
    Command::cargo_bin("recap").unwrap()
}

// =============================================================================
// Help and Version Tests
// =============================================================================

#[test]
fn test_cli_help() {
    recap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("recap"))
        .stdout(predicate::str::contains("COMMAND").or(predicate::str::contains("Commands")));
}

#[test]
fn test_cli_version() {
    recap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("recap"));
}

// =============================================================================
// Work Command Tests
// =============================================================================

#[test]
fn test_work_help() {
    recap()
        .args(["work", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("work"));
}

#[test]
fn test_work_list_help() {
    recap()
        .args(["work", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"));
}

#[test]
fn test_work_add_help() {
    recap()
        .args(["work", "add", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("add"));
}

// =============================================================================
// Report Command Tests
// =============================================================================

#[test]
fn test_report_help() {
    recap()
        .args(["report", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("report"));
}

#[test]
fn test_report_summary_help() {
    recap()
        .args(["report", "summary", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("summary"));
}

#[test]
fn test_report_export_help() {
    recap()
        .args(["report", "export", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("export"));
}

// =============================================================================
// Sync Command Tests
// =============================================================================

#[test]
fn test_sync_help() {
    recap()
        .args(["sync", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sync"));
}

#[test]
fn test_sync_run_help() {
    recap()
        .args(["sync", "run", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("run"));
}

#[test]
fn test_sync_status_help() {
    recap()
        .args(["sync", "status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status"));
}

// =============================================================================
// Source Command Tests
// =============================================================================

#[test]
fn test_source_help() {
    recap()
        .args(["source", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("source"));
}

#[test]
fn test_source_list_help() {
    recap()
        .args(["source", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"));
}

#[test]
fn test_source_add_help() {
    recap()
        .args(["source", "add", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("add"));
}

// =============================================================================
// Config Command Tests
// =============================================================================

#[test]
fn test_config_help() {
    recap()
        .args(["config", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config"));
}

#[test]
fn test_config_show_help() {
    recap()
        .args(["config", "show", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("show"));
}

#[test]
fn test_config_get_help() {
    recap()
        .args(["config", "get", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("get"));
}

#[test]
fn test_config_set_help() {
    recap()
        .args(["config", "set", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("set"));
}

// =============================================================================
// Dashboard Command Tests
// =============================================================================

#[test]
fn test_dashboard_help() {
    recap()
        .args(["dashboard", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dashboard"));
}

#[test]
fn test_dashboard_stats_help() {
    recap()
        .args(["dashboard", "stats", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("stats"));
}

#[test]
fn test_dashboard_timeline_help() {
    recap()
        .args(["dashboard", "timeline", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("timeline"));
}

#[test]
fn test_dashboard_heatmap_help() {
    recap()
        .args(["dashboard", "heatmap", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("heatmap"));
}

#[test]
fn test_dashboard_projects_help() {
    recap()
        .args(["dashboard", "projects", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("projects"));
}

// =============================================================================
// Tempo Command Tests
// =============================================================================

#[test]
fn test_tempo_help() {
    recap()
        .args(["tempo", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tempo"));
}

#[test]
fn test_tempo_generate_help() {
    recap()
        .args(["tempo", "generate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("generate"));
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_invalid_command() {
    recap()
        .arg("invalid-command-that-does-not-exist")
        .assert()
        .failure();
}

#[test]
fn test_work_invalid_subcommand() {
    recap()
        .args(["work", "invalid-subcommand"])
        .assert()
        .failure();
}

// =============================================================================
// Format Flag Tests
// =============================================================================

#[test]
fn test_work_list_format_json_accepted() {
    // Just verify the format flag is accepted
    recap()
        .args(["work", "list", "--format", "json", "--help"])
        .assert()
        .success();
}

#[test]
fn test_work_list_format_table_accepted() {
    // Just verify the format flag is accepted
    recap()
        .args(["work", "list", "--format", "table", "--help"])
        .assert()
        .success();
}

// =============================================================================
// Date Argument Tests
// =============================================================================

#[test]
fn test_report_summary_date_args_accepted() {
    recap()
        .args(["report", "summary", "--start", "2025-01-01", "--end", "2025-01-31", "--help"])
        .assert()
        .success();
}

#[test]
fn test_dashboard_stats_date_args_accepted() {
    // dashboard stats uses --start and --end, not --days
    recap()
        .args(["dashboard", "stats", "--start", "2025-01-01", "--end", "2025-01-31", "--help"])
        .assert()
        .success();
}
