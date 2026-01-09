//! Services module

pub mod excel;
pub mod llm;
pub mod sync;
pub mod tempo;

pub use excel::{ExcelReportGenerator, ExcelWorkItem, ProjectSummary, ReportMetadata};
pub use llm::create_llm_service;
pub use sync::{create_sync_service, sync_claude_projects, ClaudeSyncResult, SyncService};
pub use tempo::{JiraClient, TempoClient, WorklogUploader, WorklogEntry, JiraAuthType};
