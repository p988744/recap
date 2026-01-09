"""
GitLab Router - Manage GitLab integration and sync work items
"""

from datetime import datetime, timezone
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, status
from pydantic import BaseModel, HttpUrl
from sqlalchemy import select, delete
from sqlalchemy.ext.asyncio import AsyncSession

from ..models.database import User, GitLabProject as GitLabProjectModel, WorkItem, get_db
from .auth import get_current_active_user
from ...services.gitlab_client import GitLabClient

router = APIRouter()


# Pydantic schemas
class GitLabConfig(BaseModel):
    """GitLab configuration"""
    gitlab_url: HttpUrl
    gitlab_pat: str


class GitLabProjectCreate(BaseModel):
    """Add a GitLab project to track"""
    gitlab_project_id: int


class GitLabProjectResponse(BaseModel):
    """GitLab project response"""
    id: str
    gitlab_project_id: int
    name: str
    path_with_namespace: str
    gitlab_url: str
    default_branch: str
    enabled: bool
    last_synced: Optional[datetime] = None
    created_at: datetime

    class Config:
        from_attributes = True


class GitLabCommitResponse(BaseModel):
    """GitLab commit response"""
    id: str
    short_id: str
    title: str
    message: str
    author_name: str
    author_email: str
    authored_date: datetime
    web_url: str


class GitLabMRResponse(BaseModel):
    """GitLab merge request response"""
    id: int
    iid: int
    title: str
    description: str
    state: str
    author_username: str
    created_at: datetime
    merged_at: Optional[datetime] = None
    web_url: str
    source_branch: str
    target_branch: str


class GitLabRemoteProjectResponse(BaseModel):
    """Remote GitLab project (not yet tracked)"""
    id: int
    name: str
    path_with_namespace: str
    description: Optional[str] = None
    web_url: str
    default_branch: str
    last_activity_at: datetime


class SyncResult(BaseModel):
    """Sync operation result"""
    project_id: str
    commits_synced: int
    merge_requests_synced: int
    work_items_created: int


class MessageResponse(BaseModel):
    """Simple message response"""
    message: str


def get_gitlab_client(user: User) -> GitLabClient:
    """Get GitLab client for user."""
    if not user.gitlab_url or not user.gitlab_pat:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="GitLab not configured. Please set GitLab URL and PAT first."
        )
    return GitLabClient(base_url=user.gitlab_url, access_token=user.gitlab_pat)


@router.post("/config", response_model=MessageResponse)
async def configure_gitlab(
    config: GitLabConfig,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """
    Configure GitLab connection for current user.

    Requires a GitLab URL and Personal Access Token with api scope.
    """
    # Test the connection
    try:
        client = GitLabClient(base_url=str(config.gitlab_url), access_token=config.gitlab_pat)
        user_info = client.test_connection()
    except Exception as e:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"Failed to connect to GitLab: {str(e)}"
        )

    # Update user's GitLab configuration
    current_user.gitlab_url = str(config.gitlab_url)
    current_user.gitlab_pat = config.gitlab_pat
    await db.commit()

    return MessageResponse(
        message=f"GitLab configured successfully. Connected as {user_info.get('username', 'unknown')}"
    )


@router.get("/config/status")
async def get_gitlab_status(
    current_user: User = Depends(get_current_active_user),
):
    """Get GitLab configuration status."""
    return {
        "configured": bool(current_user.gitlab_url and current_user.gitlab_pat),
        "gitlab_url": current_user.gitlab_url,
    }


@router.delete("/config", response_model=MessageResponse)
async def remove_gitlab_config(
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Remove GitLab configuration."""
    current_user.gitlab_url = None
    current_user.gitlab_pat = None
    await db.commit()
    return MessageResponse(message="GitLab configuration removed")


@router.get("/remote-projects", response_model=list[GitLabRemoteProjectResponse])
async def list_remote_projects(
    search: Optional[str] = None,
    page: int = 1,
    per_page: int = 20,
    current_user: User = Depends(get_current_active_user),
):
    """
    List GitLab projects available to the user.

    These are projects on GitLab that can be added for tracking.
    """
    client = get_gitlab_client(current_user)

    try:
        projects = client.get_projects(
            membership=True,
            per_page=per_page,
            page=page,
            search=search,
        )
    except Exception as e:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to fetch projects from GitLab: {str(e)}"
        )

    return [
        GitLabRemoteProjectResponse(
            id=p.id,
            name=p.name,
            path_with_namespace=p.path_with_namespace,
            description=p.description,
            web_url=p.web_url,
            default_branch=p.default_branch,
            last_activity_at=p.last_activity_at,
        )
        for p in projects
    ]


@router.get("/projects", response_model=list[GitLabProjectResponse])
async def list_tracked_projects(
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """List GitLab projects being tracked by the user."""
    result = await db.execute(
        select(GitLabProjectModel)
        .where(GitLabProjectModel.user_id == current_user.id)
        .order_by(GitLabProjectModel.created_at.desc())
    )
    projects = result.scalars().all()
    return projects


@router.post("/projects", response_model=GitLabProjectResponse, status_code=status.HTTP_201_CREATED)
async def add_tracked_project(
    project_data: GitLabProjectCreate,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Add a GitLab project to track."""
    client = get_gitlab_client(current_user)

    # Check if already tracking this project
    result = await db.execute(
        select(GitLabProjectModel)
        .where(
            GitLabProjectModel.user_id == current_user.id,
            GitLabProjectModel.gitlab_project_id == project_data.gitlab_project_id
        )
    )
    existing = result.scalar_one_or_none()
    if existing:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="Project is already being tracked"
        )

    # Fetch project details from GitLab
    try:
        gitlab_project = client.get_project(project_data.gitlab_project_id)
    except Exception as e:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Project not found on GitLab: {str(e)}"
        )

    # Create tracking record
    project = GitLabProjectModel(
        user_id=current_user.id,
        gitlab_project_id=gitlab_project.id,
        name=gitlab_project.name,
        path_with_namespace=gitlab_project.path_with_namespace,
        gitlab_url=gitlab_project.web_url,
        default_branch=gitlab_project.default_branch,
        enabled=True,
    )
    db.add(project)
    await db.commit()
    await db.refresh(project)

    return project


@router.delete("/projects/{project_id}", response_model=MessageResponse)
async def remove_tracked_project(
    project_id: str,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Remove a GitLab project from tracking."""
    result = await db.execute(
        select(GitLabProjectModel)
        .where(
            GitLabProjectModel.id == project_id,
            GitLabProjectModel.user_id == current_user.id
        )
    )
    project = result.scalar_one_or_none()
    if not project:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Project not found"
        )

    await db.delete(project)
    await db.commit()
    return MessageResponse(message="Project removed from tracking")


@router.get("/projects/{project_id}/commits", response_model=list[GitLabCommitResponse])
async def get_project_commits(
    project_id: str,
    since: Optional[datetime] = None,
    until: Optional[datetime] = None,
    page: int = 1,
    per_page: int = 50,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Get commits from a tracked GitLab project."""
    # Verify project belongs to user
    result = await db.execute(
        select(GitLabProjectModel)
        .where(
            GitLabProjectModel.id == project_id,
            GitLabProjectModel.user_id == current_user.id
        )
    )
    project = result.scalar_one_or_none()
    if not project:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Project not found"
        )

    client = get_gitlab_client(current_user)

    try:
        commits = client.get_commits(
            project_id=project.gitlab_project_id,
            since=since,
            until=until,
            per_page=per_page,
            page=page,
        )
    except Exception as e:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to fetch commits: {str(e)}"
        )

    return [
        GitLabCommitResponse(
            id=c.id,
            short_id=c.short_id,
            title=c.title,
            message=c.message,
            author_name=c.author_name,
            author_email=c.author_email,
            authored_date=c.authored_date,
            web_url=c.web_url,
        )
        for c in commits
    ]


@router.get("/projects/{project_id}/merge-requests", response_model=list[GitLabMRResponse])
async def get_project_merge_requests(
    project_id: str,
    state: str = "all",
    since: Optional[datetime] = None,
    until: Optional[datetime] = None,
    page: int = 1,
    per_page: int = 50,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Get merge requests from a tracked GitLab project."""
    # Verify project belongs to user
    result = await db.execute(
        select(GitLabProjectModel)
        .where(
            GitLabProjectModel.id == project_id,
            GitLabProjectModel.user_id == current_user.id
        )
    )
    project = result.scalar_one_or_none()
    if not project:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Project not found"
        )

    client = get_gitlab_client(current_user)

    try:
        merge_requests = client.get_merge_requests(
            project_id=project.gitlab_project_id,
            state=state,
            created_after=since,
            created_before=until,
            per_page=per_page,
            page=page,
        )
    except Exception as e:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to fetch merge requests: {str(e)}"
        )

    return [
        GitLabMRResponse(
            id=mr.id,
            iid=mr.iid,
            title=mr.title,
            description=mr.description,
            state=mr.state,
            author_username=mr.author_username,
            created_at=mr.created_at,
            merged_at=mr.merged_at,
            web_url=mr.web_url,
            source_branch=mr.source_branch,
            target_branch=mr.target_branch,
        )
        for mr in merge_requests
    ]


@router.post("/projects/{project_id}/sync", response_model=SyncResult)
async def sync_project(
    project_id: str,
    since: Optional[datetime] = None,
    until: Optional[datetime] = None,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """
    Sync GitLab commits and merge requests to work items.

    Creates WorkItem entries for each commit and merged MR.
    """
    # Verify project belongs to user
    result = await db.execute(
        select(GitLabProjectModel)
        .where(
            GitLabProjectModel.id == project_id,
            GitLabProjectModel.user_id == current_user.id
        )
    )
    project = result.scalar_one_or_none()
    if not project:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Project not found"
        )

    client = get_gitlab_client(current_user)

    # Default to last 7 days if no date range specified
    if not since:
        from datetime import timedelta
        since = datetime.now(timezone.utc) - timedelta(days=7)
    if not until:
        until = datetime.now(timezone.utc)

    try:
        commits = client.get_commits(
            project_id=project.gitlab_project_id,
            since=since,
            until=until,
            per_page=100,
        )
        merge_requests = client.get_merge_requests(
            project_id=project.gitlab_project_id,
            state="merged",
            created_after=since,
            created_before=until,
            per_page=100,
        )
    except Exception as e:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to fetch from GitLab: {str(e)}"
        )

    work_items_created = 0

    # Create work items for commits
    for commit in commits:
        # Check if work item already exists
        result = await db.execute(
            select(WorkItem)
            .where(
                WorkItem.user_id == current_user.id,
                WorkItem.source == "gitlab",
                WorkItem.source_id == commit.id
            )
        )
        existing = result.scalar_one_or_none()
        if existing:
            continue

        work_item = WorkItem(
            user_id=current_user.id,
            source="gitlab",
            source_id=commit.id,
            title=f"Commit: {commit.title}",
            description=commit.message,
            hours=0.5,  # Default estimate for commits
            date=commit.authored_date.date(),
            source_url=commit.web_url,
        )
        db.add(work_item)
        work_items_created += 1

    # Create work items for merged MRs
    for mr in merge_requests:
        # Check if work item already exists
        result = await db.execute(
            select(WorkItem)
            .where(
                WorkItem.user_id == current_user.id,
                WorkItem.source == "gitlab",
                WorkItem.source_id == f"mr-{mr.id}"
            )
        )
        existing = result.scalar_one_or_none()
        if existing:
            continue

        work_item = WorkItem(
            user_id=current_user.id,
            source="gitlab",
            source_id=f"mr-{mr.id}",
            title=f"MR: {mr.title}",
            description=mr.description,
            hours=2.0,  # Default estimate for MRs
            date=(mr.merged_at or mr.created_at).date(),
            source_url=mr.web_url,
        )
        db.add(work_item)
        work_items_created += 1

    # Update last synced time
    project.last_synced = datetime.now(timezone.utc)
    await db.commit()

    return SyncResult(
        project_id=project_id,
        commits_synced=len(commits),
        merge_requests_synced=len(merge_requests),
        work_items_created=work_items_created,
    )


@router.post("/sync-all", response_model=list[SyncResult])
async def sync_all_projects(
    since: Optional[datetime] = None,
    until: Optional[datetime] = None,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Sync all tracked GitLab projects."""
    result = await db.execute(
        select(GitLabProjectModel)
        .where(
            GitLabProjectModel.user_id == current_user.id,
            GitLabProjectModel.enabled == True
        )
    )
    projects = result.scalars().all()

    results = []
    for project in projects:
        try:
            sync_result = await sync_project(
                project_id=project.id,
                since=since,
                until=until,
                current_user=current_user,
                db=db,
            )
            results.append(sync_result)
        except HTTPException:
            # Skip projects that fail to sync
            continue

    return results
