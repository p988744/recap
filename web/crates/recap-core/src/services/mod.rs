//! Services module

pub mod excel;
pub mod llm;
pub mod session_parser;
pub mod sync;
pub mod tempo;
pub mod worklog;

pub use excel::{ExcelReportGenerator, ExcelWorkItem, ProjectSummary, ReportMetadata};
pub use llm::create_llm_service;
pub use sync::{
    create_sync_service, resolve_git_root, sync_claude_projects, sync_discovered_projects,
    ClaudeSyncResult, DiscoveredProject, SyncService,
};
pub use tempo::{JiraClient, TempoClient, WorklogUploader, WorklogEntry, JiraAuthType};
pub use worklog::{
    CommitRecord, DailyWorklog, FileChange, HoursEstimate, SessionBrief,
    StandaloneSession, TimelineCommit, estimate_commit_hours, estimate_from_diff,
    get_commits_for_date, get_commits_in_time_range, calculate_session_hours,
    build_rule_based_outcome,
};
pub use session_parser::{
    extract_cwd, generate_daily_hash, is_meaningful_message, extract_tool_detail,
    parse_session_fast, parse_session_full,
    SessionMetadata, ParsedSession, ToolUsage,
};
