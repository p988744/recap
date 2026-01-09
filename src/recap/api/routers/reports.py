"""
Reports Router - Generate personal, team, and PE reports
"""

import json
from datetime import date, datetime, timedelta
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, Query, status
from fastapi.responses import StreamingResponse
from pydantic import BaseModel
from sqlalchemy import select, and_, func
from sqlalchemy.ext.asyncio import AsyncSession

from ..models.database import User, WorkItem, YearlyGoal, Department, get_db
from .auth import get_current_active_user, get_current_admin_user

router = APIRouter()


# Pydantic schemas
class WorkItemSummary(BaseModel):
    """Summarized work item for reports"""
    id: str
    title: str
    hours: float
    date: date
    jira_issue_key: Optional[str] = None
    category: Optional[str] = None
    source: str


class DailyReport(BaseModel):
    """Daily work report"""
    date: date
    total_hours: float
    items: list[WorkItemSummary]


class WeeklyReport(BaseModel):
    """Weekly work report"""
    start_date: date
    end_date: date
    total_hours: float
    daily_breakdown: list[DailyReport]
    category_breakdown: dict[str, float]
    jira_issues: dict[str, float]


class PersonalReport(BaseModel):
    """Personal report for a date range"""
    user_name: str
    user_email: str
    start_date: date
    end_date: date
    total_hours: float
    work_items: list[WorkItemSummary]
    daily_breakdown: list[DailyReport]
    category_breakdown: dict[str, float]
    jira_issues: dict[str, float]
    source_breakdown: dict[str, float]


class TeamMemberSummary(BaseModel):
    """Summary for a team member"""
    user_id: str
    user_name: str
    total_hours: float
    work_item_count: int
    category_breakdown: dict[str, float]


class TeamReport(BaseModel):
    """Team report for managers"""
    department_name: str
    start_date: date
    end_date: date
    total_hours: float
    member_count: int
    members: list[TeamMemberSummary]
    category_breakdown: dict[str, float]


class GoalProgress(BaseModel):
    """Progress on a yearly goal"""
    goal_id: str
    goal_title: str
    category: str
    weight: float
    work_item_count: int
    total_hours: float
    work_items: list[WorkItemSummary]


class PEWorkResult(BaseModel):
    """Work result item for PE report"""
    title: str
    period: str
    result_description: str
    weight: float


class PEReport(BaseModel):
    """PE Performance Evaluation report"""
    user_name: str
    department: Optional[str] = None
    title: Optional[str] = None
    evaluation_period: str
    work_results: list[PEWorkResult]
    skills: list[dict]
    goal_progress: list[GoalProgress]
    total_hours: float
    jira_issues_count: int
    commits_count: int
    merge_requests_count: int


def to_work_item_summary(item: WorkItem) -> WorkItemSummary:
    """Convert WorkItem to summary format."""
    return WorkItemSummary(
        id=item.id,
        title=item.title,
        hours=item.hours,
        date=item.date.date() if isinstance(item.date, datetime) else item.date,
        jira_issue_key=item.jira_issue_key,
        category=item.category,
        source=item.source,
    )


@router.get("/personal", response_model=PersonalReport)
async def get_personal_report(
    start_date: date = Query(..., description="Report start date"),
    end_date: date = Query(..., description="Report end date"),
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """
    Generate a personal work report for the specified date range.
    """
    # Get work items in date range
    result = await db.execute(
        select(WorkItem)
        .where(
            WorkItem.user_id == current_user.id,
            WorkItem.date >= start_date,
            WorkItem.date <= end_date
        )
        .order_by(WorkItem.date.desc())
    )
    items = result.scalars().all()

    # Calculate summaries
    work_items = [to_work_item_summary(item) for item in items]
    total_hours = sum(item.hours for item in items)

    # Daily breakdown
    daily_data: dict[date, list] = {}
    for item in items:
        item_date = item.date.date() if isinstance(item.date, datetime) else item.date
        if item_date not in daily_data:
            daily_data[item_date] = []
        daily_data[item_date].append(to_work_item_summary(item))

    daily_breakdown = [
        DailyReport(
            date=d,
            total_hours=sum(i.hours for i in summaries),
            items=summaries
        )
        for d, summaries in sorted(daily_data.items(), reverse=True)
    ]

    # Category breakdown
    category_breakdown = {}
    for item in items:
        cat = item.category or "uncategorized"
        category_breakdown[cat] = category_breakdown.get(cat, 0) + item.hours

    # Jira issues
    jira_issues = {}
    for item in items:
        if item.jira_issue_key:
            jira_issues[item.jira_issue_key] = jira_issues.get(item.jira_issue_key, 0) + item.hours

    # Source breakdown
    source_breakdown = {}
    for item in items:
        source_breakdown[item.source] = source_breakdown.get(item.source, 0) + item.hours

    return PersonalReport(
        user_name=current_user.name,
        user_email=current_user.email,
        start_date=start_date,
        end_date=end_date,
        total_hours=total_hours,
        work_items=work_items,
        daily_breakdown=daily_breakdown,
        category_breakdown=category_breakdown,
        jira_issues=jira_issues,
        source_breakdown=source_breakdown,
    )


@router.get("/weekly", response_model=WeeklyReport)
async def get_weekly_report(
    week_start: Optional[date] = None,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """
    Generate a weekly report.

    If week_start is not provided, uses the current week.
    """
    if week_start is None:
        today = date.today()
        # Get Monday of current week
        week_start = today - timedelta(days=today.weekday())

    week_end = week_start + timedelta(days=6)

    # Get work items for the week
    result = await db.execute(
        select(WorkItem)
        .where(
            WorkItem.user_id == current_user.id,
            WorkItem.date >= week_start,
            WorkItem.date <= week_end
        )
        .order_by(WorkItem.date.desc())
    )
    items = result.scalars().all()

    total_hours = sum(item.hours for item in items)

    # Daily breakdown
    daily_data: dict[date, list] = {}
    for item in items:
        item_date = item.date.date() if isinstance(item.date, datetime) else item.date
        if item_date not in daily_data:
            daily_data[item_date] = []
        daily_data[item_date].append(to_work_item_summary(item))

    daily_breakdown = [
        DailyReport(
            date=d,
            total_hours=sum(i.hours for i in summaries),
            items=summaries
        )
        for d, summaries in sorted(daily_data.items(), reverse=True)
    ]

    # Category breakdown
    category_breakdown = {}
    for item in items:
        cat = item.category or "uncategorized"
        category_breakdown[cat] = category_breakdown.get(cat, 0) + item.hours

    # Jira issues
    jira_issues = {}
    for item in items:
        if item.jira_issue_key:
            jira_issues[item.jira_issue_key] = jira_issues.get(item.jira_issue_key, 0) + item.hours

    return WeeklyReport(
        start_date=week_start,
        end_date=week_end,
        total_hours=total_hours,
        daily_breakdown=daily_breakdown,
        category_breakdown=category_breakdown,
        jira_issues=jira_issues,
    )


@router.get("/team", response_model=TeamReport)
async def get_team_report(
    start_date: date = Query(..., description="Report start date"),
    end_date: date = Query(..., description="Report end date"),
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """
    Generate a team report for department managers.

    Requires manager role in a department.
    """
    # Check if user is a manager
    if not current_user.is_admin:
        result = await db.execute(
            select(Department)
            .where(Department.manager_id == current_user.id)
        )
        managed_dept = result.scalar_one_or_none()
        if not managed_dept:
            raise HTTPException(
                status_code=status.HTTP_403_FORBIDDEN,
                detail="Not authorized to view team report. Manager role required."
            )
        department_id = managed_dept.id
        department_name = managed_dept.name
    else:
        # Admin can see all, but needs department_id parameter
        if not current_user.department_id:
            # Get first department or show message
            result = await db.execute(select(Department).limit(1))
            dept = result.scalar_one_or_none()
            if not dept:
                raise HTTPException(
                    status_code=status.HTTP_404_NOT_FOUND,
                    detail="No departments found"
                )
            department_id = dept.id
            department_name = dept.name
        else:
            result = await db.execute(
                select(Department).where(Department.id == current_user.department_id)
            )
            dept = result.scalar_one_or_none()
            department_id = dept.id if dept else None
            department_name = dept.name if dept else "Unknown"

    # Get team members
    result = await db.execute(
        select(User)
        .where(User.department_id == department_id)
    )
    members = result.scalars().all()

    team_total_hours = 0
    team_category_breakdown = {}
    member_summaries = []

    for member in members:
        # Get work items for this member
        result = await db.execute(
            select(WorkItem)
            .where(
                WorkItem.user_id == member.id,
                WorkItem.date >= start_date,
                WorkItem.date <= end_date
            )
        )
        items = result.scalars().all()

        member_hours = sum(item.hours for item in items)
        team_total_hours += member_hours

        # Member category breakdown
        member_categories = {}
        for item in items:
            cat = item.category or "uncategorized"
            member_categories[cat] = member_categories.get(cat, 0) + item.hours
            team_category_breakdown[cat] = team_category_breakdown.get(cat, 0) + item.hours

        member_summaries.append(TeamMemberSummary(
            user_id=member.id,
            user_name=member.name,
            total_hours=member_hours,
            work_item_count=len(items),
            category_breakdown=member_categories,
        ))

    return TeamReport(
        department_name=department_name,
        start_date=start_date,
        end_date=end_date,
        total_hours=team_total_hours,
        member_count=len(members),
        members=member_summaries,
        category_breakdown=team_category_breakdown,
    )


@router.get("/pe", response_model=PEReport)
async def get_pe_report(
    year: int = Query(..., description="Evaluation year"),
    half: int = Query(1, ge=1, le=2, description="1=First half, 2=Second half"),
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """
    Generate PE (Performance Evaluation) report.

    Follows the PE template structure with:
    - Work results (工作成果)
    - Skills development (技能發展)
    - Goal progress (年度目標達成)
    """
    # Calculate date range for the half year
    if half == 1:
        start_date = date(year, 1, 1)
        end_date = date(year, 6, 30)
        period_str = f"{year} 上半年 (1月-6月)"
    else:
        start_date = date(year, 7, 1)
        end_date = date(year, 12, 31)
        period_str = f"{year} 下半年 (7月-12月)"

    # Get work items
    result = await db.execute(
        select(WorkItem)
        .where(
            WorkItem.user_id == current_user.id,
            WorkItem.date >= start_date,
            WorkItem.date <= end_date
        )
        .order_by(WorkItem.date.desc())
    )
    items = result.scalars().all()

    # Get yearly goals
    result = await db.execute(
        select(YearlyGoal)
        .where(
            YearlyGoal.user_id == current_user.id,
            YearlyGoal.year == year
        )
    )
    goals = result.scalars().all()

    # Get department name
    department_name = None
    if current_user.department_id:
        result = await db.execute(
            select(Department).where(Department.id == current_user.department_id)
        )
        dept = result.scalar_one_or_none()
        if dept:
            department_name = dept.name

    # Calculate statistics
    total_hours = sum(item.hours for item in items)
    jira_issues = set(item.jira_issue_key for item in items if item.jira_issue_key)
    commits = [item for item in items if item.source == "gitlab" and "commit" in (item.source_id or "").lower()]
    mrs = [item for item in items if item.source == "gitlab" and "mr-" in (item.source_id or "").lower()]

    # Group work items by category for work results
    category_items: dict[str, list] = {}
    for item in items:
        cat = item.category or "其他"
        if cat not in category_items:
            category_items[cat] = []
        category_items[cat].append(item)

    # Generate work results (工作成果) - max 10 items
    work_results = []
    for cat, cat_items in sorted(category_items.items(), key=lambda x: -sum(i.hours for i in x[1])):
        if len(work_results) >= 10:
            break
        cat_hours = sum(item.hours for item in cat_items)
        work_results.append(PEWorkResult(
            title=cat,
            period=f"{start_date.strftime('%m/%d')} - {end_date.strftime('%m/%d')}",
            result_description=f"完成 {len(cat_items)} 項工作，共 {cat_hours:.1f} 小時",
            weight=round(cat_hours / total_hours, 2) if total_hours > 0 else 0,
        ))

    # Skills based on sources and technologies (simplified)
    skills = []
    source_counts = {}
    for item in items:
        source_counts[item.source] = source_counts.get(item.source, 0) + 1

    if source_counts.get("gitlab", 0) > 0:
        skills.append({
            "name": "版本控制與協作",
            "description": f"使用 GitLab 進行版本控制，產出 {source_counts.get('gitlab', 0)} 項工作記錄"
        })
    if source_counts.get("claude_code", 0) > 0:
        skills.append({
            "name": "AI 輔助開發",
            "description": f"使用 Claude Code 進行 AI 輔助開發，完成 {source_counts.get('claude_code', 0)} 個 session"
        })
    if len(jira_issues) > 0:
        skills.append({
            "name": "專案管理",
            "description": f"追蹤並完成 {len(jira_issues)} 個 Jira issue"
        })

    # Goal progress
    goal_progress = []
    for goal in goals:
        # Find work items linked to this goal
        goal_items = [item for item in items if item.yearly_goal_id == goal.id]
        goal_hours = sum(item.hours for item in goal_items)
        goal_progress.append(GoalProgress(
            goal_id=goal.id,
            goal_title=goal.title,
            category=goal.category or "其他",
            weight=goal.weight,
            work_item_count=len(goal_items),
            total_hours=goal_hours,
            work_items=[to_work_item_summary(item) for item in goal_items],
        ))

    return PEReport(
        user_name=current_user.name,
        department=department_name,
        title=current_user.title,
        evaluation_period=period_str,
        work_results=work_results,
        skills=skills,
        goal_progress=goal_progress,
        total_hours=total_hours,
        jira_issues_count=len(jira_issues),
        commits_count=len(commits),
        merge_requests_count=len(mrs),
    )


@router.get("/export/markdown")
async def export_markdown_report(
    start_date: date = Query(..., description="Report start date"),
    end_date: date = Query(..., description="Report end date"),
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db)
):
    """Export personal report as Markdown."""
    # Get the report data
    result = await db.execute(
        select(WorkItem)
        .where(
            WorkItem.user_id == current_user.id,
            WorkItem.date >= start_date,
            WorkItem.date <= end_date
        )
        .order_by(WorkItem.date.desc())
    )
    items = result.scalars().all()

    total_hours = sum(item.hours for item in items)

    # Generate markdown
    lines = [
        f"# 工作報告",
        f"",
        f"**姓名:** {current_user.name}",
        f"**期間:** {start_date} - {end_date}",
        f"**總時數:** {total_hours:.1f} 小時",
        f"**工作項目數:** {len(items)}",
        f"",
        f"---",
        f"",
        f"## 工作項目明細",
        f"",
    ]

    # Group by date
    daily_items: dict[date, list] = {}
    for item in items:
        item_date = item.date.date() if isinstance(item.date, datetime) else item.date
        if item_date not in daily_items:
            daily_items[item_date] = []
        daily_items[item_date].append(item)

    for d, d_items in sorted(daily_items.items(), reverse=True):
        lines.append(f"### {d}")
        lines.append("")
        for item in d_items:
            jira_str = f" [{item.jira_issue_key}]" if item.jira_issue_key else ""
            lines.append(f"- **{item.title}**{jira_str} - {item.hours:.1f}h")
            if item.description:
                lines.append(f"  - {item.description[:100]}{'...' if len(item.description) > 100 else ''}")
        lines.append("")

    # Category summary
    lines.append("## 類別統計")
    lines.append("")
    category_hours = {}
    for item in items:
        cat = item.category or "未分類"
        category_hours[cat] = category_hours.get(cat, 0) + item.hours

    for cat, hours in sorted(category_hours.items(), key=lambda x: -x[1]):
        lines.append(f"- {cat}: {hours:.1f}h ({hours/total_hours*100:.1f}%)" if total_hours > 0 else f"- {cat}: {hours:.1f}h")

    markdown_content = "\n".join(lines)

    return StreamingResponse(
        iter([markdown_content]),
        media_type="text/markdown",
        headers={
            "Content-Disposition": f"attachment; filename=report_{start_date}_{end_date}.md"
        }
    )
