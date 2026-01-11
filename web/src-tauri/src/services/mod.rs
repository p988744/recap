//! Services module

pub mod excel;
pub mod llm;
pub mod sync;
pub mod tempo;
pub mod worklog;

pub use excel::{ExcelReportGenerator, ExcelWorkItem, ProjectSummary, ReportMetadata};
pub use llm::create_llm_service;
pub use sync::{create_sync_service, sync_claude_projects, ClaudeSyncResult, SyncService};
pub use tempo::{JiraClient, TempoClient, WorklogUploader, WorklogEntry, JiraAuthType};
pub use worklog::{
    CommitRecord, DailyWorklog, FileChange, HoursEstimate, SessionBrief,
    StandaloneSession, estimate_commit_hours, estimate_from_diff, get_commits_for_date,
    build_rule_based_outcome,
};
