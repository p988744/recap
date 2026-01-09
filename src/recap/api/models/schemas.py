"""
Pydantic schemas for the Recap API
"""

from datetime import datetime
from typing import Optional
from pydantic import BaseModel, Field


# ============================================================
# Config Schemas
# ============================================================

class JiraConfigUpdate(BaseModel):
    """Jira configuration update request"""
    jira_url: Optional[str] = None
    jira_pat: Optional[str] = None
    jira_email: Optional[str] = None
    jira_api_token: Optional[str] = None
    auth_type: Optional[str] = None  # "pat" or "basic"
    tempo_api_token: Optional[str] = None


class LLMConfigUpdate(BaseModel):
    """LLM configuration update request"""
    llm_provider: Optional[str] = None  # anthropic, openai, gemini, ollama, openai-compatible
    llm_model: Optional[str] = None
    llm_api_key: Optional[str] = None
    llm_base_url: Optional[str] = None


class ConfigUpdate(BaseModel):
    """Full configuration update request"""
    jira: Optional[JiraConfigUpdate] = None
    llm: Optional[LLMConfigUpdate] = None
    daily_work_hours: Optional[float] = None
    normalize_hours: Optional[bool] = None
    use_git_mode: Optional[bool] = None


class ConfigResponse(BaseModel):
    """Configuration response (safe version without secrets)"""
    jira_url: str
    auth_type: str
    jira_configured: bool
    tempo_configured: bool
    llm_provider: str
    llm_model: str
    llm_configured: bool
    daily_work_hours: float
    normalize_hours: bool
    use_git_mode: bool
    git_repos: list[str]
    outlook_enabled: bool


# ============================================================
# Sources Schemas
# ============================================================

class GitRepoInfo(BaseModel):
    """Git repository information"""
    path: str
    name: str
    valid: bool
    last_commit: Optional[str] = None
    last_commit_date: Optional[datetime] = None


class SourcesResponse(BaseModel):
    """Data sources response"""
    mode: str  # "git" or "claude"
    git_repos: list[GitRepoInfo]
    claude_connected: bool
    claude_path: Optional[str] = None
    outlook_enabled: bool


class AddGitRepoRequest(BaseModel):
    """Request to add a Git repository"""
    path: str


# ============================================================
# Analyze Schemas
# ============================================================

class WorkSessionResponse(BaseModel):
    """Single work session"""
    project_path: str
    project_name: str
    session_id: str
    start_time: datetime
    end_time: datetime
    duration_minutes: int
    date: str
    summary: list[str]
    todos: list[str]
    jira_id: Optional[str] = None


class DailyEntryResponse(BaseModel):
    """Daily entry for a project"""
    date: str
    minutes: int
    hours: float
    todos: list[str]
    summaries: list[str]
    description: str


class ProjectSummaryResponse(BaseModel):
    """Project summary with daily breakdown"""
    project_name: str
    project_path: str
    total_minutes: int
    total_hours: float
    daily_entries: list[DailyEntryResponse]
    jira_id: Optional[str] = None
    jira_id_suggestions: list[str] = Field(default_factory=list)


class AnalyzeRequest(BaseModel):
    """Request for work analysis"""
    start_date: str  # YYYY-MM-DD
    end_date: str    # YYYY-MM-DD
    use_git: Optional[bool] = None  # Override mode


class AnalyzeResponse(BaseModel):
    """Work analysis response"""
    start_date: str
    end_date: str
    total_minutes: int
    total_hours: float
    dates_covered: list[str]
    projects: list[ProjectSummaryResponse]
    mode: str  # "git" or "claude"


# ============================================================
# Tempo Schemas
# ============================================================

class WorklogEntryRequest(BaseModel):
    """Single worklog entry to upload"""
    issue_key: str
    date: str  # YYYY-MM-DD
    minutes: int
    description: str


class WorklogEntryResponse(BaseModel):
    """Worklog entry response"""
    id: Optional[int] = None
    issue_key: str
    date: str
    minutes: int
    hours: float
    description: str
    status: str  # "success", "error", "pending"
    error_message: Optional[str] = None


class SyncRequest(BaseModel):
    """Request to sync worklogs to Tempo"""
    entries: list[WorklogEntryRequest]
    dry_run: bool = False


class SyncResponse(BaseModel):
    """Sync operation response"""
    success: bool
    total_entries: int
    successful: int
    failed: int
    results: list[WorklogEntryResponse]
    dry_run: bool


# ============================================================
# Team Schemas
# ============================================================

class TeamMemberResponse(BaseModel):
    """Team member information"""
    account_id: str
    display_name: str
    email: str = ""


class TeamResponse(BaseModel):
    """Team information"""
    name: str
    jira_group: str = ""
    tempo_team_id: Optional[int] = None
    members: list[TeamMemberResponse]
    member_count: int
    last_synced: Optional[str] = None


class TeamListResponse(BaseModel):
    """List of teams"""
    teams: list[TeamResponse]
    total: int


# ============================================================
# Project Mapping Schemas
# ============================================================

class ProjectMappingUpdate(BaseModel):
    """Update project to Jira issue mapping"""
    project_name: str
    jira_id: str


class ProjectMappingResponse(BaseModel):
    """Project mapping response"""
    mappings: dict[str, str]  # project_name -> jira_id


# ============================================================
# General Response Schemas
# ============================================================

class SuccessResponse(BaseModel):
    """Generic success response"""
    success: bool = True
    message: str = ""


class ErrorResponse(BaseModel):
    """Error response"""
    success: bool = False
    error: str
    detail: Optional[str] = None
