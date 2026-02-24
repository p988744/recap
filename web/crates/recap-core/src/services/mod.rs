//! Services module

pub mod compaction;
pub mod excel;
pub mod http_export;
pub mod llm;
pub mod llm_batch;
pub mod llm_pricing;
pub mod llm_usage;
pub mod session_parser;
pub mod snapshot;
pub mod sources;
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
    get_commits_for_date, get_commits_in_time_range, get_git_user_email,
    calculate_session_hours, build_rule_based_outcome,
};
pub use session_parser::{
    extract_cwd, generate_daily_hash, is_meaningful_message, extract_tool_detail,
    parse_session_fast, parse_session_full,
    SessionMetadata, ParsedSession, ToolUsage,
};
pub use snapshot::{
    capture_snapshots_for_project, parse_session_into_hourly_buckets,
    save_hourly_snapshots, CommitSnapshot, HourlyBucket, SnapshotCaptureResult,
    ToolCallRecord,
};
pub use compaction::{
    compact_daily, compact_hourly, compact_period, run_compaction_cycle,
    CompactionResult, ForceRecompactOptions, ForceRecompactResult,
    // Batch mode
    collect_pending_hourly, prepare_hourly_batch_requests, save_batch_results_as_summaries,
    submit_hourly_batch, process_completed_batch,
    PendingHourlyCompaction, BatchCompactionSubmitResult, BatchCompactionProcessResult,
};
pub use llm::{LlmUsageRecord, parse_error_usage};
pub use llm_pricing::estimate_cost;
pub use llm_usage::{
    save_usage_log, get_usage_stats, get_usage_by_day, get_usage_by_model, get_usage_logs,
    LlmUsageStats, DailyUsage, ModelUsage, LlmUsageLog,
};
pub use llm_batch::{
    LlmBatchService, BatchJob, BatchRequest, BatchJobStatus, BatchSubmitResult, BatchProcessResult,
    HourlyCompactionRequest,
};
pub use sources::{
    SyncSource, SourceProject, SourceSyncResult, WorkItemParams,
    ClaudeSource, SyncConfig,
    get_enabled_sources, upsert_work_item, UpsertResult,
};
