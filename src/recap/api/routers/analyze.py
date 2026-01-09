"""
Analyze Router - Work analysis API
"""

from datetime import datetime, timedelta

from fastapi import APIRouter, HTTPException, Query

from ...config import Config, ProjectMapping
from ...session_parser import WorklogHelper, get_week_range, get_last_week_range
from ..models.schemas import (
    AnalyzeRequest,
    AnalyzeResponse,
    ProjectSummaryResponse,
    DailyEntryResponse,
)

router = APIRouter()


def worklog_to_response(worklog, mode: str, mapping: ProjectMapping) -> AnalyzeResponse:
    """Convert WeeklyWorklog to AnalyzeResponse"""
    projects = []

    for project in worklog.get_project_summaries():
        daily_entries = []
        for entry in sorted(project.daily_entries, key=lambda e: e.date):
            daily_entries.append(DailyEntryResponse(
                date=entry.date,
                minutes=entry.minutes,
                hours=entry.minutes / 60,
                todos=entry.todos,
                summaries=entry.summaries,
                description=entry.get_description(project.project_name),
            ))

        # Get Jira ID suggestions
        suggestions = mapping.get_suggestions(project.project_name)

        projects.append(ProjectSummaryResponse(
            project_name=project.project_name,
            project_path=project.project_path,
            total_minutes=project.total_minutes,
            total_hours=project.total_hours,
            daily_entries=daily_entries,
            jira_id=project.jira_id or mapping.get(project.project_name),
            jira_id_suggestions=suggestions,
        ))

    return AnalyzeResponse(
        start_date=worklog.start_date,
        end_date=worklog.end_date,
        total_minutes=worklog.total_minutes,
        total_hours=worklog.total_minutes / 60,
        dates_covered=worklog.dates_covered,
        projects=projects,
        mode=mode,
    )


@router.post("", response_model=AnalyzeResponse)
async def analyze_work(request: AnalyzeRequest):
    """Analyze work for a date range"""
    try:
        # Validate dates
        start = datetime.strptime(request.start_date, "%Y-%m-%d")
        end = datetime.strptime(request.end_date, "%Y-%m-%d")
        if start > end:
            raise HTTPException(status_code=400, detail="start_date must be before end_date")
    except ValueError as e:
        raise HTTPException(status_code=400, detail=f"Invalid date format: {e}")

    helper = WorklogHelper(use_git=request.use_git)
    mapping = ProjectMapping()

    worklog = helper.analyze_range(request.start_date, request.end_date)

    return worklog_to_response(worklog, helper.mode, mapping)


@router.get("/week", response_model=AnalyzeResponse)
async def analyze_this_week(use_git: bool = Query(None, description="Override mode to use Git")):
    """Analyze work for the current week"""
    start_date, end_date = get_week_range()
    helper = WorklogHelper(use_git=use_git)
    mapping = ProjectMapping()

    worklog = helper.analyze_range(start_date, end_date)

    return worklog_to_response(worklog, helper.mode, mapping)


@router.get("/last-week", response_model=AnalyzeResponse)
async def analyze_last_week(use_git: bool = Query(None, description="Override mode to use Git")):
    """Analyze work for the last week"""
    start_date, end_date = get_last_week_range()
    helper = WorklogHelper(use_git=use_git)
    mapping = ProjectMapping()

    worklog = helper.analyze_range(start_date, end_date)

    return worklog_to_response(worklog, helper.mode, mapping)


@router.get("/days/{days}", response_model=AnalyzeResponse)
async def analyze_recent_days(
    days: int,
    use_git: bool = Query(None, description="Override mode to use Git"),
):
    """Analyze work for the last N days"""
    if days < 1 or days > 90:
        raise HTTPException(status_code=400, detail="days must be between 1 and 90")

    end = datetime.now()
    start = end - timedelta(days=days - 1)

    start_date = start.strftime("%Y-%m-%d")
    end_date = end.strftime("%Y-%m-%d")

    helper = WorklogHelper(use_git=use_git)
    mapping = ProjectMapping()

    worklog = helper.analyze_range(start_date, end_date)

    return worklog_to_response(worklog, helper.mode, mapping)


@router.get("/dates", response_model=list[str])
async def list_available_dates(
    limit: int = Query(30, description="Maximum number of dates to return"),
    use_git: bool = Query(None, description="Override mode to use Git"),
):
    """List available dates with work records"""
    helper = WorklogHelper(use_git=use_git)
    return helper.list_dates(limit=limit)
