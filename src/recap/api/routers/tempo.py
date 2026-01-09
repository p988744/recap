"""
Tempo Router - Jira Tempo integration API
"""

from fastapi import APIRouter, HTTPException

from ...config import Config, ProjectMapping
from ...tempo_api import WorklogUploader, WorklogEntry, JiraClient
from ..models.schemas import (
    SyncRequest,
    SyncResponse,
    WorklogEntryRequest,
    WorklogEntryResponse,
    SuccessResponse,
)

router = APIRouter()


def get_uploader() -> WorklogUploader:
    """Get configured WorklogUploader"""
    config = Config.load()
    if not config.is_configured():
        raise HTTPException(status_code=400, detail="Jira not configured")

    return WorklogUploader(
        jira_url=config.jira_url,
        token=config.get_token(),
        email=config.jira_email or None,
        auth_type=config.auth_type,
        tempo_token=config.tempo_api_token or None,
    )


@router.get("/test", response_model=SuccessResponse)
async def test_connection():
    """Test Tempo/Jira connection"""
    config = Config.load()
    if not config.is_configured():
        raise HTTPException(status_code=400, detail="Jira not configured")

    try:
        uploader = get_uploader()
        success, message = uploader.test_connection()
        if success:
            return SuccessResponse(message=message)
        raise HTTPException(status_code=400, detail=message)
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))


@router.post("/validate-issue/{issue_key}", response_model=SuccessResponse)
async def validate_issue(issue_key: str):
    """Validate that an issue key exists"""
    config = Config.load()
    if not config.is_configured():
        raise HTTPException(status_code=400, detail="Jira not configured")

    try:
        client = JiraClient(
            base_url=config.jira_url,
            token=config.get_token(),
            auth_type=config.auth_type,
            email=config.jira_email,
        )
        valid, summary = client.validate_issue_key(issue_key)
        if valid:
            return SuccessResponse(message=f"{issue_key}: {summary}")
        raise HTTPException(status_code=404, detail=f"Issue not found: {issue_key}")
    except HTTPException:
        raise
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))


@router.post("/sync", response_model=SyncResponse)
async def sync_worklogs(request: SyncRequest):
    """Sync worklogs to Tempo/Jira"""
    config = Config.load()
    if not config.is_configured():
        raise HTTPException(status_code=400, detail="Jira not configured")

    # Determine whether to use Tempo API
    use_tempo = bool(config.tempo_api_token)

    try:
        uploader = get_uploader()
    except Exception as e:
        raise HTTPException(status_code=400, detail=f"Failed to initialize uploader: {e}")

    results = []
    successful = 0
    failed = 0

    for entry_req in request.entries:
        entry = WorklogEntry(
            issue_key=entry_req.issue_key,
            date=entry_req.date,
            time_spent_seconds=entry_req.minutes * 60,
            description=entry_req.description,
        )

        if request.dry_run:
            # Dry run - just validate
            results.append(WorklogEntryResponse(
                issue_key=entry_req.issue_key,
                date=entry_req.date,
                minutes=entry_req.minutes,
                hours=entry_req.minutes / 60,
                description=entry_req.description,
                status="pending",
            ))
            continue

        try:
            result = uploader.upload_worklog(entry, use_tempo=use_tempo)
            results.append(WorklogEntryResponse(
                id=result.get("id") or result.get("tempoWorklogId"),
                issue_key=entry_req.issue_key,
                date=entry_req.date,
                minutes=entry_req.minutes,
                hours=entry_req.minutes / 60,
                description=entry_req.description,
                status="success",
            ))
            successful += 1
        except Exception as e:
            results.append(WorklogEntryResponse(
                issue_key=entry_req.issue_key,
                date=entry_req.date,
                minutes=entry_req.minutes,
                hours=entry_req.minutes / 60,
                description=entry_req.description,
                status="error",
                error_message=str(e),
            ))
            failed += 1

    return SyncResponse(
        success=failed == 0,
        total_entries=len(request.entries),
        successful=successful,
        failed=failed,
        results=results,
        dry_run=request.dry_run,
    )


@router.post("/upload", response_model=WorklogEntryResponse)
async def upload_single_worklog(entry: WorklogEntryRequest):
    """Upload a single worklog entry"""
    config = Config.load()
    if not config.is_configured():
        raise HTTPException(status_code=400, detail="Jira not configured")

    use_tempo = bool(config.tempo_api_token)

    try:
        uploader = get_uploader()
        worklog_entry = WorklogEntry(
            issue_key=entry.issue_key,
            date=entry.date,
            time_spent_seconds=entry.minutes * 60,
            description=entry.description,
        )

        result = uploader.upload_worklog(worklog_entry, use_tempo=use_tempo)

        return WorklogEntryResponse(
            id=result.get("id") or result.get("tempoWorklogId"),
            issue_key=entry.issue_key,
            date=entry.date,
            minutes=entry.minutes,
            hours=entry.minutes / 60,
            description=entry.description,
            status="success",
        )
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))


@router.post("/map/{project_name}", response_model=SuccessResponse)
async def set_project_mapping(project_name: str, jira_id: str):
    """Set project to Jira issue mapping"""
    mapping = ProjectMapping()
    mapping.set(project_name, jira_id)
    return SuccessResponse(message=f"Mapped {project_name} to {jira_id}")
