"""
WorkItem Router - CRUD operations for work items with Jira mapping
"""

import json
from datetime import date, datetime
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, Query, status
from pydantic import BaseModel
from sqlalchemy import select, and_, or_, func
from sqlalchemy.ext.asyncio import AsyncSession

from ..models.database import User, WorkItem, YearlyGoal, get_db
from .auth import get_current_active_user

router = APIRouter()


# Pydantic schemas
class WorkItemCreate(BaseModel):
    """Create a new work item"""
    title: str
    description: Optional[str] = None
    hours: float = 0
    date: date
    source: str = "manual"
    source_id: Optional[str] = None
    source_url: Optional[str] = None
    jira_issue_key: Optional[str] = None
    jira_issue_title: Optional[str] = None
    category: Optional[str] = None
    tags: Optional[list[str]] = None
    yearly_goal_id: Optional[str] = None


class WorkItemUpdate(BaseModel):
    """Update work item fields"""
    title: Optional[str] = None
    description: Optional[str] = None
    hours: Optional[float] = None
    date: Optional[date] = None
    jira_issue_key: Optional[str] = None
    jira_issue_title: Optional[str] = None
    category: Optional[str] = None
    tags: Optional[list[str]] = None
    yearly_goal_id: Optional[str] = None
    synced_to_tempo: Optional[bool] = None
    tempo_worklog_id: Optional[str] = None


class JiraMapping(BaseModel):
    """Map work item to Jira issue"""
    jira_issue_key: str
    jira_issue_title: Optional[str] = None


class WorkItemResponse(BaseModel):
    """Work item response"""
    id: str
    user_id: str
    source: str
    source_id: Optional[str] = None
    source_url: Optional[str] = None
    title: str
    description: Optional[str] = None
    hours: float
    date: date
    jira_issue_key: Optional[str] = None
    jira_issue_suggested: Optional[str] = None
    jira_issue_title: Optional[str] = None
    category: Optional[str] = None
    tags: Optional[list[str]] = None
    yearly_goal_id: Optional[str] = None
    synced_to_tempo: bool
    tempo_worklog_id: Optional[str] = None
    synced_at: Optional[datetime] = None
    created_at: datetime
    updated_at: datetime

    class Config:
        from_attributes = True


class WorkItemListResponse(BaseModel):
    """Paginated work item list"""
    items: list[WorkItemResponse]
    total: int
    page: int
    per_page: int
    pages: int


class BatchMapRequest(BaseModel):
    """Batch map work items to Jira"""
    work_item_ids: list[str]
    jira_issue_key: str
    jira_issue_title: Optional[str] = None


class MessageResponse(BaseModel):
    """Simple message response"""
    message: str
    count: int = 0


def parse_tags(tags_json: Optional[str]) -> Optional[list[str]]:
    """Parse tags from JSON string."""
    if not tags_json:
        return None
    try:
        return json.loads(tags_json)
    except json.JSONDecodeError:
        return None


def to_response(item: WorkItem) -> WorkItemResponse:
    """Convert WorkItem model to response."""
    return WorkItemResponse(
        id=item.id,
        user_id=item.user_id,
        source=item.source,
        source_id=item.source_id,
        source_url=item.source_url,
        title=item.title,
        description=item.description,
        hours=item.hours,
        date=item.date.date() if isinstance(item.date, datetime) else item.date,
        jira_issue_key=item.jira_issue_key,
        jira_issue_suggested=item.jira_issue_suggested,
        jira_issue_title=item.jira_issue_title,
        category=item.category,
        tags=parse_tags(item.tags),
        yearly_goal_id=item.yearly_goal_id,
        synced_to_tempo=item.synced_to_tempo,
        tempo_worklog_id=item.tempo_worklog_id,
        synced_at=item.synced_at,
        created_at=item.created_at,
        updated_at=item.updated_at,
    )


@router.get("", response_model=WorkItemListResponse)
async def list_work_items(
    page: int = Query(1, ge=1),
    per_page: int = Query(20, ge=1, le=100),
    source: Optional[str] = None,
    category: Optional[str] = None,
    jira_mapped: Optional[bool] = None,
    synced_to_tempo: Optional[bool] = None,
    start_date: Optional[date] = None,
    end_date: Optional[date] = None,
    search: Optional[str] = None,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """
    List work items with filtering and pagination.

    Filters:
    - source: Filter by source (gitlab, claude_code, manual)
    - category: Filter by PE category
    - jira_mapped: Filter by whether Jira issue is mapped
    - synced_to_tempo: Filter by Tempo sync status
    - start_date/end_date: Filter by date range
    - search: Search in title and description
    """
    # Build query
    query = select(WorkItem).where(WorkItem.user_id == current_user.id)

    # Apply filters
    if source:
        query = query.where(WorkItem.source == source)
    if category:
        query = query.where(WorkItem.category == category)
    if jira_mapped is not None:
        if jira_mapped:
            query = query.where(WorkItem.jira_issue_key.isnot(None))
        else:
            query = query.where(WorkItem.jira_issue_key.is_(None))
    if synced_to_tempo is not None:
        query = query.where(WorkItem.synced_to_tempo == synced_to_tempo)
    if start_date:
        query = query.where(WorkItem.date >= start_date)
    if end_date:
        query = query.where(WorkItem.date <= end_date)
    if search:
        search_pattern = f"%{search}%"
        query = query.where(
            or_(
                WorkItem.title.ilike(search_pattern),
                WorkItem.description.ilike(search_pattern)
            )
        )

    # Get total count
    count_query = select(func.count()).select_from(query.subquery())
    total = (await db.execute(count_query)).scalar() or 0

    # Apply pagination and ordering
    offset = (page - 1) * per_page
    query = query.order_by(WorkItem.date.desc(), WorkItem.created_at.desc())
    query = query.offset(offset).limit(per_page)

    # Execute query
    result = await db.execute(query)
    items = result.scalars().all()

    # Calculate pages
    pages = (total + per_page - 1) // per_page if total > 0 else 1

    return WorkItemListResponse(
        items=[to_response(item) for item in items],
        total=total,
        page=page,
        per_page=per_page,
        pages=pages,
    )


@router.post("", response_model=WorkItemResponse, status_code=status.HTTP_201_CREATED)
async def create_work_item(
    item_data: WorkItemCreate,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Create a new work item manually."""
    # Validate yearly goal if provided
    if item_data.yearly_goal_id:
        result = await db.execute(
            select(YearlyGoal)
            .where(
                YearlyGoal.id == item_data.yearly_goal_id,
                YearlyGoal.user_id == current_user.id
            )
        )
        goal = result.scalar_one_or_none()
        if not goal:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail="Yearly goal not found"
            )

    work_item = WorkItem(
        user_id=current_user.id,
        source=item_data.source,
        source_id=item_data.source_id,
        source_url=item_data.source_url,
        title=item_data.title,
        description=item_data.description,
        hours=item_data.hours,
        date=item_data.date,
        jira_issue_key=item_data.jira_issue_key,
        jira_issue_title=item_data.jira_issue_title,
        category=item_data.category,
        tags=json.dumps(item_data.tags) if item_data.tags else None,
        yearly_goal_id=item_data.yearly_goal_id,
    )

    db.add(work_item)
    await db.commit()
    await db.refresh(work_item)

    return to_response(work_item)


@router.get("/{item_id}", response_model=WorkItemResponse)
async def get_work_item(
    item_id: str,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Get a single work item by ID."""
    result = await db.execute(
        select(WorkItem)
        .where(
            WorkItem.id == item_id,
            WorkItem.user_id == current_user.id
        )
    )
    item = result.scalar_one_or_none()
    if not item:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Work item not found"
        )
    return to_response(item)


@router.patch("/{item_id}", response_model=WorkItemResponse)
async def update_work_item(
    item_id: str,
    updates: WorkItemUpdate,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Update a work item."""
    result = await db.execute(
        select(WorkItem)
        .where(
            WorkItem.id == item_id,
            WorkItem.user_id == current_user.id
        )
    )
    item = result.scalar_one_or_none()
    if not item:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Work item not found"
        )

    # Validate yearly goal if being updated
    if updates.yearly_goal_id is not None:
        if updates.yearly_goal_id:
            result = await db.execute(
                select(YearlyGoal)
                .where(
                    YearlyGoal.id == updates.yearly_goal_id,
                    YearlyGoal.user_id == current_user.id
                )
            )
            goal = result.scalar_one_or_none()
            if not goal:
                raise HTTPException(
                    status_code=status.HTTP_404_NOT_FOUND,
                    detail="Yearly goal not found"
                )

    # Apply updates
    update_data = updates.model_dump(exclude_unset=True)
    if "tags" in update_data:
        update_data["tags"] = json.dumps(update_data["tags"]) if update_data["tags"] else None

    for key, value in update_data.items():
        setattr(item, key, value)

    await db.commit()
    await db.refresh(item)

    return to_response(item)


@router.delete("/{item_id}", response_model=MessageResponse)
async def delete_work_item(
    item_id: str,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Delete a work item."""
    result = await db.execute(
        select(WorkItem)
        .where(
            WorkItem.id == item_id,
            WorkItem.user_id == current_user.id
        )
    )
    item = result.scalar_one_or_none()
    if not item:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Work item not found"
        )

    await db.delete(item)
    await db.commit()

    return MessageResponse(message="Work item deleted", count=1)


@router.post("/{item_id}/map-jira", response_model=WorkItemResponse)
async def map_jira_issue(
    item_id: str,
    mapping: JiraMapping,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Map a work item to a Jira issue."""
    result = await db.execute(
        select(WorkItem)
        .where(
            WorkItem.id == item_id,
            WorkItem.user_id == current_user.id
        )
    )
    item = result.scalar_one_or_none()
    if not item:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Work item not found"
        )

    item.jira_issue_key = mapping.jira_issue_key
    if mapping.jira_issue_title:
        item.jira_issue_title = mapping.jira_issue_title

    await db.commit()
    await db.refresh(item)

    return to_response(item)


@router.post("/batch-map-jira", response_model=MessageResponse)
async def batch_map_jira(
    request: BatchMapRequest,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Batch map multiple work items to the same Jira issue."""
    result = await db.execute(
        select(WorkItem)
        .where(
            WorkItem.id.in_(request.work_item_ids),
            WorkItem.user_id == current_user.id
        )
    )
    items = result.scalars().all()

    if len(items) != len(request.work_item_ids):
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Some work items not found"
        )

    for item in items:
        item.jira_issue_key = request.jira_issue_key
        if request.jira_issue_title:
            item.jira_issue_title = request.jira_issue_title

    await db.commit()

    return MessageResponse(
        message=f"Mapped {len(items)} work items to {request.jira_issue_key}",
        count=len(items)
    )


@router.get("/stats/summary")
async def get_work_items_summary(
    start_date: Optional[date] = None,
    end_date: Optional[date] = None,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """
    Get summary statistics for work items.

    Returns:
    - Total hours by source
    - Total hours by category
    - Jira mapping status
    - Tempo sync status
    """
    # Base query
    query = select(WorkItem).where(WorkItem.user_id == current_user.id)

    if start_date:
        query = query.where(WorkItem.date >= start_date)
    if end_date:
        query = query.where(WorkItem.date <= end_date)

    result = await db.execute(query)
    items = result.scalars().all()

    # Calculate statistics
    total_items = len(items)
    total_hours = sum(item.hours for item in items)

    hours_by_source = {}
    hours_by_category = {}
    jira_mapped_count = 0
    tempo_synced_count = 0

    for item in items:
        # By source
        source = item.source or "unknown"
        hours_by_source[source] = hours_by_source.get(source, 0) + item.hours

        # By category
        category = item.category or "uncategorized"
        hours_by_category[category] = hours_by_category.get(category, 0) + item.hours

        # Jira mapping
        if item.jira_issue_key:
            jira_mapped_count += 1

        # Tempo sync
        if item.synced_to_tempo:
            tempo_synced_count += 1

    return {
        "total_items": total_items,
        "total_hours": total_hours,
        "hours_by_source": hours_by_source,
        "hours_by_category": hours_by_category,
        "jira_mapping": {
            "mapped": jira_mapped_count,
            "unmapped": total_items - jira_mapped_count,
            "percentage": (jira_mapped_count / total_items * 100) if total_items > 0 else 0,
        },
        "tempo_sync": {
            "synced": tempo_synced_count,
            "not_synced": total_items - tempo_synced_count,
            "percentage": (tempo_synced_count / total_items * 100) if total_items > 0 else 0,
        },
    }
