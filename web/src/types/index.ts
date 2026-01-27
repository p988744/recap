/**
 * Centralized type exports
 *
 * All shared types should be imported from this file.
 * Single source of truth - no duplicate type definitions.
 */

// Auth types
export type {
  UserResponse,
  AppStatus,
  TokenResponse,
  RegisterRequest,
  LoginRequest,
  UpdateProfileRequest,
} from './auth'

// Config types
export type {
  ConfigResponse,
  UpdateConfigRequest,
  UpdateLlmConfigRequest,
  UpdateJiraConfigRequest,
  MessageResponse,
} from './config'

// Work Items types
export type {
  WorkItem,
  WorkItemWithChildren,
  PaginatedResponse,
  WorkItemFilters,
  CreateWorkItemRequest,
  UpdateWorkItemRequest,
  WorkLogItem,
  JiraIssueGroup,
  ProjectGroup,
  DateGroup,
  GroupedWorkItemsResponse,
  DailyHours,
  JiraMappingStats,
  TempoSyncStats,
  WorkItemStatsResponse,
  TimelineCommit,
  TimelineSession,
  TimelineResponse,
  BatchSyncRequest,
  BatchSyncResponse,
  AggregateRequest,
  AggregateResponse,
  CommitWorklogItem,
  CommitCentricWorklogResponse,
} from './work-items'

// Reports types
export type {
  ReportQuery,
  DailyItems,
  PersonalReport,
  SourceSummary,
  SummaryReport,
  CategorySummary,
  CategoryReport,
  ExportResult,
  TempoReportPeriod,
  TempoReportQuery,
  TempoProjectSummary,
  TempoReport,
  WorkItemSummary,
  DailyReport,
  LegacyPersonalReport,
  WeeklyReport,
  TeamMemberSummary,
  TeamReport,
  PEWorkResult,
  GoalProgress,
  PEReport,
} from './reports'

// Sync types
export type {
  SyncStatus,
  AutoSyncRequest,
  SyncResult,
  AutoSyncResponse,
  AvailableProject,
} from './sync'

// Integration types
export type {
  // Sources
  GitRepoInfo,
  SourcesResponse,
  AddGitRepoResponse,
  SourceModeResponse,
  // GitLab
  GitLabConfigStatus,
  ConfigureGitLabRequest,
  GitLabProject,
  AddGitLabProjectRequest,
  SyncGitLabRequest,
  SyncGitLabResponse,
  SearchGitLabProjectsRequest,
  GitLabProjectInfo,
  // Tempo
  TempoSuccessResponse,
  WorklogEntryRequest,
  WorklogEntryResponse,
  SyncWorklogsRequest,
  SyncWorklogsResponse,
  GetWorklogsRequest,
  ValidateIssueResponse,
  // Claude
  ToolUsage,
  ClaudeSession,
  ClaudeProject,
  ImportSessionsRequest,
  ImportResult,
  SummarizeRequest,
  SummarizeResult,
  SyncProjectsRequest,
  ClaudeSyncResult,
  // Teams (Legacy)
  TeamMember,
  Team,
  // Analyze (Legacy)
  DailyEntry,
  ProjectSummary,
  AnalyzeResponse,
} from './integrations'

// Worklog types
export type {
  GitCommitRef,
  HourlyBreakdownItem,
  ManualWorkItem,
  WorklogDayProject,
  WorklogDay,
  WorklogOverviewResponse,
} from './worklog'

// Project types
export type {
  ProjectInfo,
  ProjectSourceInfo,
  ProjectWorkItemSummary,
  ProjectStats,
  ProjectDetail,
  SetProjectVisibilityRequest,
  ClaudeCodeDirEntry,
  ProjectDirectories,
  AddManualProjectRequest,
  ClaudeSessionPathResponse,
} from './projects'
