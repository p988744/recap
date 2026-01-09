"""
Config Router - Configuration management API
"""

from fastapi import APIRouter, HTTPException

from ...config import Config, ProjectMapping, TeamConfig
from ..models.schemas import (
    ConfigResponse,
    ConfigUpdate,
    JiraConfigUpdate,
    LLMConfigUpdate,
    ProjectMappingResponse,
    ProjectMappingUpdate,
    SuccessResponse,
    TeamListResponse,
    TeamResponse,
    TeamMemberResponse,
)

router = APIRouter()


@router.get("", response_model=ConfigResponse)
async def get_config():
    """Get current configuration (without secrets)"""
    config = Config.load()
    return ConfigResponse(
        jira_url=config.jira_url,
        auth_type=config.auth_type,
        jira_configured=config.is_configured(),
        tempo_configured=bool(config.tempo_api_token),
        llm_provider=config.llm_provider,
        llm_model=config.llm_model or "(default)",
        llm_configured=config.has_llm_config(),
        daily_work_hours=config.daily_work_hours,
        normalize_hours=config.normalize_hours,
        use_git_mode=config.use_git_mode,
        git_repos=config.git_repos,
        outlook_enabled=config.outlook_enabled,
    )


@router.patch("", response_model=ConfigResponse)
async def update_config(update: ConfigUpdate):
    """Update configuration"""
    config = Config.load()

    # Update Jira settings
    if update.jira:
        if update.jira.jira_url is not None:
            config.jira_url = update.jira.jira_url
        if update.jira.jira_pat is not None:
            config.jira_pat = update.jira.jira_pat
        if update.jira.jira_email is not None:
            config.jira_email = update.jira.jira_email
        if update.jira.jira_api_token is not None:
            config.jira_api_token = update.jira.jira_api_token
        if update.jira.auth_type is not None:
            config.auth_type = update.jira.auth_type
        if update.jira.tempo_api_token is not None:
            config.tempo_api_token = update.jira.tempo_api_token

    # Update LLM settings
    if update.llm:
        if update.llm.llm_provider is not None:
            config.llm_provider = update.llm.llm_provider
        if update.llm.llm_model is not None:
            config.llm_model = update.llm.llm_model
        if update.llm.llm_api_key is not None:
            config.llm_api_key = update.llm.llm_api_key
        if update.llm.llm_base_url is not None:
            config.llm_base_url = update.llm.llm_base_url

    # Update other settings
    if update.daily_work_hours is not None:
        config.daily_work_hours = update.daily_work_hours
    if update.normalize_hours is not None:
        config.normalize_hours = update.normalize_hours
    if update.use_git_mode is not None:
        config.use_git_mode = update.use_git_mode

    config.save()

    return ConfigResponse(
        jira_url=config.jira_url,
        auth_type=config.auth_type,
        jira_configured=config.is_configured(),
        tempo_configured=bool(config.tempo_api_token),
        llm_provider=config.llm_provider,
        llm_model=config.llm_model or "(default)",
        llm_configured=config.has_llm_config(),
        daily_work_hours=config.daily_work_hours,
        normalize_hours=config.normalize_hours,
        use_git_mode=config.use_git_mode,
        git_repos=config.git_repos,
        outlook_enabled=config.outlook_enabled,
    )


@router.post("/jira", response_model=SuccessResponse)
async def update_jira_config(update: JiraConfigUpdate):
    """Update Jira configuration"""
    config = Config.load()

    if update.jira_url is not None:
        config.jira_url = update.jira_url
    if update.jira_pat is not None:
        config.jira_pat = update.jira_pat
    if update.jira_email is not None:
        config.jira_email = update.jira_email
    if update.jira_api_token is not None:
        config.jira_api_token = update.jira_api_token
    if update.auth_type is not None:
        config.auth_type = update.auth_type
    if update.tempo_api_token is not None:
        config.tempo_api_token = update.tempo_api_token

    config.save()
    return SuccessResponse(message="Jira configuration updated")


@router.post("/llm", response_model=SuccessResponse)
async def update_llm_config(update: LLMConfigUpdate):
    """Update LLM configuration"""
    config = Config.load()

    if update.llm_provider is not None:
        config.llm_provider = update.llm_provider
    if update.llm_model is not None:
        config.llm_model = update.llm_model
    if update.llm_api_key is not None:
        config.llm_api_key = update.llm_api_key
    if update.llm_base_url is not None:
        config.llm_base_url = update.llm_base_url

    config.save()
    return SuccessResponse(message="LLM configuration updated")


@router.get("/test-jira", response_model=SuccessResponse)
async def test_jira_connection():
    """Test Jira connection"""
    config = Config.load()
    if not config.is_configured():
        raise HTTPException(status_code=400, detail="Jira not configured")

    try:
        from ...tempo_api import JiraClient
        client = JiraClient(
            base_url=config.jira_url,
            token=config.get_token(),
            auth_type=config.auth_type,
            email=config.jira_email,
        )
        user = client.get_myself()
        return SuccessResponse(message=f"Connected as {user.get('displayName', 'Unknown')}")
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))


# ============================================================
# Project Mappings
# ============================================================

@router.get("/mappings", response_model=ProjectMappingResponse)
async def get_project_mappings():
    """Get all project to Jira issue mappings"""
    mapping = ProjectMapping()
    return ProjectMappingResponse(mappings=mapping.mappings)


@router.post("/mappings", response_model=SuccessResponse)
async def set_project_mapping(update: ProjectMappingUpdate):
    """Set a project to Jira issue mapping"""
    mapping = ProjectMapping()
    mapping.set(update.project_name, update.jira_id)
    return SuccessResponse(message=f"Mapped {update.project_name} to {update.jira_id}")


# ============================================================
# Teams
# ============================================================

@router.get("/teams", response_model=TeamListResponse)
async def list_teams():
    """List all configured teams"""
    team_config = TeamConfig()
    teams = []
    for team_info in team_config.list_teams():
        teams.append(TeamResponse(
            name=team_info.name,
            jira_group=team_info.jira_group,
            tempo_team_id=team_info.tempo_team_id,
            members=[
                TeamMemberResponse(
                    account_id=m.account_id,
                    display_name=m.display_name,
                    email=m.email,
                )
                for m in team_info.members
            ],
            member_count=len(team_info.members),
            last_synced=team_info.last_synced,
        ))
    return TeamListResponse(teams=teams, total=len(teams))


@router.get("/teams/{team_name}", response_model=TeamResponse)
async def get_team(team_name: str):
    """Get a specific team"""
    team_config = TeamConfig()
    team_info = team_config.get_team(team_name)
    if not team_info:
        raise HTTPException(status_code=404, detail=f"Team '{team_name}' not found")

    return TeamResponse(
        name=team_info.name,
        jira_group=team_info.jira_group,
        tempo_team_id=team_info.tempo_team_id,
        members=[
            TeamMemberResponse(
                account_id=m.account_id,
                display_name=m.display_name,
                email=m.email,
            )
            for m in team_info.members
        ],
        member_count=len(team_info.members),
        last_synced=team_info.last_synced,
    )
