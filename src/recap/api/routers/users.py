"""
Users Router - User profile and settings management
"""

from datetime import datetime
from typing import List, Optional

from fastapi import APIRouter, Depends, HTTPException, status
from pydantic import BaseModel, EmailStr
from sqlalchemy import select, delete
from sqlalchemy.ext.asyncio import AsyncSession

from ..models.database import User, YearlyGoal, Department, get_db
from .auth import get_current_active_user

router = APIRouter()


# Pydantic schemas
class ProfileUpdate(BaseModel):
    """User profile update request"""
    name: Optional[str] = None
    employee_id: Optional[str] = None
    title: Optional[str] = None
    jira_email: Optional[str] = None
    jira_account_id: Optional[str] = None


class GitLabPATUpdate(BaseModel):
    """GitLab PAT update request"""
    gitlab_url: str  # e.g., "https://gitlab.example.com"
    gitlab_pat: str  # Personal Access Token


class ProfileResponse(BaseModel):
    """User profile response"""
    id: str
    email: str
    name: str
    employee_id: Optional[str] = None
    department_id: Optional[str] = None
    department_name: Optional[str] = None
    title: Optional[str] = None
    gitlab_url: Optional[str] = None
    gitlab_configured: bool = False
    jira_email: Optional[str] = None
    jira_account_id: Optional[str] = None
    is_admin: bool
    created_at: datetime

    class Config:
        from_attributes = True


class YearlyGoalCreate(BaseModel):
    """Create yearly goal request"""
    year: int
    title: str
    description: Optional[str] = None
    category: Optional[str] = None  # PE 分類
    weight: float = 0.1
    department_goal_id: Optional[str] = None


class YearlyGoalUpdate(BaseModel):
    """Update yearly goal request"""
    title: Optional[str] = None
    description: Optional[str] = None
    category: Optional[str] = None
    weight: Optional[float] = None
    department_goal_id: Optional[str] = None


class YearlyGoalResponse(BaseModel):
    """Yearly goal response"""
    id: str
    user_id: str
    department_goal_id: Optional[str] = None
    year: int
    title: str
    description: Optional[str] = None
    category: Optional[str] = None
    weight: float
    created_at: datetime
    updated_at: datetime

    class Config:
        from_attributes = True


class MessageResponse(BaseModel):
    """Simple message response"""
    message: str
    success: bool = True


# Profile endpoints
@router.get("/profile", response_model=ProfileResponse)
async def get_profile(
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Get current user's profile"""
    department_name = None
    if current_user.department_id:
        result = await db.execute(
            select(Department).where(Department.id == current_user.department_id)
        )
        department = result.scalar_one_or_none()
        if department:
            department_name = department.name

    return ProfileResponse(
        id=current_user.id,
        email=current_user.email,
        name=current_user.name,
        employee_id=current_user.employee_id,
        department_id=current_user.department_id,
        department_name=department_name,
        title=current_user.title,
        gitlab_url=current_user.gitlab_url,
        gitlab_configured=bool(current_user.gitlab_pat),
        jira_email=current_user.jira_email,
        jira_account_id=current_user.jira_account_id,
        is_admin=current_user.is_admin,
        created_at=current_user.created_at,
    )


@router.patch("/profile", response_model=ProfileResponse)
async def update_profile(
    profile_data: ProfileUpdate,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Update current user's profile"""
    update_data = profile_data.model_dump(exclude_unset=True)

    for field, value in update_data.items():
        setattr(current_user, field, value)

    current_user.updated_at = datetime.utcnow()
    await db.commit()
    await db.refresh(current_user)

    # Get department name
    department_name = None
    if current_user.department_id:
        result = await db.execute(
            select(Department).where(Department.id == current_user.department_id)
        )
        department = result.scalar_one_or_none()
        if department:
            department_name = department.name

    return ProfileResponse(
        id=current_user.id,
        email=current_user.email,
        name=current_user.name,
        employee_id=current_user.employee_id,
        department_id=current_user.department_id,
        department_name=department_name,
        title=current_user.title,
        gitlab_url=current_user.gitlab_url,
        gitlab_configured=bool(current_user.gitlab_pat),
        jira_email=current_user.jira_email,
        jira_account_id=current_user.jira_account_id,
        is_admin=current_user.is_admin,
        created_at=current_user.created_at,
    )


@router.put("/gitlab-pat", response_model=MessageResponse)
async def update_gitlab_pat(
    pat_data: GitLabPATUpdate,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Update GitLab PAT for current user"""
    # TODO: Encrypt PAT before storing
    current_user.gitlab_url = pat_data.gitlab_url.rstrip("/")
    current_user.gitlab_pat = pat_data.gitlab_pat
    current_user.updated_at = datetime.utcnow()

    await db.commit()

    return MessageResponse(message="GitLab PAT updated successfully")


@router.delete("/gitlab-pat", response_model=MessageResponse)
async def delete_gitlab_pat(
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Remove GitLab PAT for current user"""
    current_user.gitlab_url = None
    current_user.gitlab_pat = None
    current_user.updated_at = datetime.utcnow()

    await db.commit()

    return MessageResponse(message="GitLab PAT removed successfully")


# Yearly goals endpoints
@router.get("/yearly-goals", response_model=List[YearlyGoalResponse])
async def list_yearly_goals(
    year: Optional[int] = None,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """List current user's yearly goals"""
    query = select(YearlyGoal).where(YearlyGoal.user_id == current_user.id)

    if year:
        query = query.where(YearlyGoal.year == year)

    query = query.order_by(YearlyGoal.year.desc(), YearlyGoal.created_at)

    result = await db.execute(query)
    goals = result.scalars().all()

    return goals


@router.post("/yearly-goals", response_model=YearlyGoalResponse, status_code=status.HTTP_201_CREATED)
async def create_yearly_goal(
    goal_data: YearlyGoalCreate,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Create a new yearly goal"""
    goal = YearlyGoal(
        user_id=current_user.id,
        year=goal_data.year,
        title=goal_data.title,
        description=goal_data.description,
        category=goal_data.category,
        weight=goal_data.weight,
        department_goal_id=goal_data.department_goal_id,
    )

    db.add(goal)
    await db.commit()
    await db.refresh(goal)

    return goal


@router.get("/yearly-goals/{goal_id}", response_model=YearlyGoalResponse)
async def get_yearly_goal(
    goal_id: str,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Get a specific yearly goal"""
    result = await db.execute(
        select(YearlyGoal).where(
            YearlyGoal.id == goal_id,
            YearlyGoal.user_id == current_user.id
        )
    )
    goal = result.scalar_one_or_none()

    if not goal:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Goal not found"
        )

    return goal


@router.patch("/yearly-goals/{goal_id}", response_model=YearlyGoalResponse)
async def update_yearly_goal(
    goal_id: str,
    goal_data: YearlyGoalUpdate,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Update a yearly goal"""
    result = await db.execute(
        select(YearlyGoal).where(
            YearlyGoal.id == goal_id,
            YearlyGoal.user_id == current_user.id
        )
    )
    goal = result.scalar_one_or_none()

    if not goal:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Goal not found"
        )

    update_data = goal_data.model_dump(exclude_unset=True)
    for field, value in update_data.items():
        setattr(goal, field, value)

    goal.updated_at = datetime.utcnow()
    await db.commit()
    await db.refresh(goal)

    return goal


@router.delete("/yearly-goals/{goal_id}", response_model=MessageResponse)
async def delete_yearly_goal(
    goal_id: str,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Delete a yearly goal"""
    result = await db.execute(
        select(YearlyGoal).where(
            YearlyGoal.id == goal_id,
            YearlyGoal.user_id == current_user.id
        )
    )
    goal = result.scalar_one_or_none()

    if not goal:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Goal not found"
        )

    await db.delete(goal)
    await db.commit()

    return MessageResponse(message="Goal deleted successfully")
