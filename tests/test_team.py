"""Tests for team module."""

import pytest
from datetime import datetime
from unittest.mock import patch, MagicMock
import tempfile
import json
from pathlib import Path

from recap.config import TeamConfig, TeamInfo, TeamMemberInfo
from recap.team import (
    WorklogEntry, MemberWorklogSummary, TeamReportData, TeamReportGenerator
)
from recap.cli import get_month_range, get_last_month_range


class TestTeamConfig:
    """Tests for TeamConfig class."""

    def test_add_team(self, tmp_path, monkeypatch):
        """Test adding a team."""
        teams_file = tmp_path / "teams.json"
        monkeypatch.setattr("recap.config.TEAMS_FILE", teams_file)

        team_config = TeamConfig()
        result = team_config.add_team("frontend", "frontend-developers")

        assert result is True
        assert "frontend" in team_config.teams
        assert team_config.teams["frontend"].jira_group == "frontend-developers"
        assert teams_file.exists()

    def test_add_duplicate_team(self, tmp_path, monkeypatch):
        """Test adding a duplicate team."""
        teams_file = tmp_path / "teams.json"
        monkeypatch.setattr("recap.config.TEAMS_FILE", teams_file)

        team_config = TeamConfig()
        team_config.add_team("frontend", "frontend-developers")
        result = team_config.add_team("frontend", "another-group")

        assert result is False

    def test_remove_team(self, tmp_path, monkeypatch):
        """Test removing a team."""
        teams_file = tmp_path / "teams.json"
        monkeypatch.setattr("recap.config.TEAMS_FILE", teams_file)

        team_config = TeamConfig()
        team_config.add_team("frontend", "frontend-developers")
        result = team_config.remove_team("frontend")

        assert result is True
        assert "frontend" not in team_config.teams

    def test_remove_nonexistent_team(self, tmp_path, monkeypatch):
        """Test removing a nonexistent team."""
        teams_file = tmp_path / "teams.json"
        monkeypatch.setattr("recap.config.TEAMS_FILE", teams_file)

        team_config = TeamConfig()
        result = team_config.remove_team("nonexistent")

        assert result is False

    def test_get_team(self, tmp_path, monkeypatch):
        """Test getting a team."""
        teams_file = tmp_path / "teams.json"
        monkeypatch.setattr("recap.config.TEAMS_FILE", teams_file)

        team_config = TeamConfig()
        team_config.add_team("frontend", "frontend-developers")
        team = team_config.get_team("frontend")

        assert team is not None
        assert team.name == "frontend"
        assert team.jira_group == "frontend-developers"

    def test_list_teams(self, tmp_path, monkeypatch):
        """Test listing teams."""
        teams_file = tmp_path / "teams.json"
        monkeypatch.setattr("recap.config.TEAMS_FILE", teams_file)

        team_config = TeamConfig()
        team_config.add_team("frontend", "frontend-developers")
        team_config.add_team("backend", "backend-developers")
        teams = team_config.list_teams()

        assert len(teams) == 2

    def test_update_members(self, tmp_path, monkeypatch):
        """Test updating team members cache."""
        teams_file = tmp_path / "teams.json"
        monkeypatch.setattr("recap.config.TEAMS_FILE", teams_file)

        team_config = TeamConfig()
        team_config.add_team("frontend", "frontend-developers")

        members = [
            TeamMemberInfo(account_id="user1", display_name="User 1", email="user1@test.com"),
            TeamMemberInfo(account_id="user2", display_name="User 2", email="user2@test.com"),
        ]
        synced_at = datetime.now().isoformat()
        team_config.update_members("frontend", members, synced_at)

        team = team_config.get_team("frontend")
        assert len(team.members) == 2
        assert team.last_synced == synced_at


class TestWorklogEntry:
    """Tests for WorklogEntry class."""

    def test_create_entry(self):
        """Test creating a worklog entry."""
        entry = WorklogEntry(
            issue_key="PROJ-123",
            issue_type="Task",
            date="2025-01-15",
            time_spent_seconds=3600,
            description="Test work",
            author_id="user1",
            author_name="User 1"
        )

        assert entry.issue_key == "PROJ-123"
        assert entry.issue_type == "Task"
        assert entry.time_spent_seconds == 3600


class TestMemberWorklogSummary:
    """Tests for MemberWorklogSummary class."""

    def test_total_hours(self):
        """Test total hours calculation."""
        member = TeamMemberInfo(account_id="user1", display_name="User 1")
        summary = MemberWorklogSummary(member=member, total_seconds=7200)  # 2 hours

        assert summary.total_hours == 2.0

    def test_total_hours_empty(self):
        """Test total hours with no worklogs."""
        member = TeamMemberInfo(account_id="user1", display_name="User 1")
        summary = MemberWorklogSummary(member=member)

        assert summary.total_hours == 0.0


class TestTeamReportData:
    """Tests for TeamReportData class."""

    def test_total_hours(self):
        """Test team total hours calculation."""
        member1 = TeamMemberInfo(account_id="user1", display_name="User 1")
        member2 = TeamMemberInfo(account_id="user2", display_name="User 2")

        summary1 = MemberWorklogSummary(member=member1, total_seconds=7200)
        summary2 = MemberWorklogSummary(member=member2, total_seconds=3600)

        report = TeamReportData(
            team_name="Test Team",
            jira_group="test-group",
            start_date="2025-01-01",
            end_date="2025-01-07",
            generated_at="2025-01-08T10:00:00",
            members=[summary1, summary2]
        )

        assert report.total_hours == 3.0  # 2 + 1 hours

    def test_by_issue_type_total(self):
        """Test issue type aggregation."""
        member1 = TeamMemberInfo(account_id="user1", display_name="User 1")
        summary1 = MemberWorklogSummary(
            member=member1,
            total_seconds=7200,
            by_issue_type={"Task": 3600, "Bug": 3600}
        )

        member2 = TeamMemberInfo(account_id="user2", display_name="User 2")
        summary2 = MemberWorklogSummary(
            member=member2,
            total_seconds=3600,
            by_issue_type={"Task": 1800, "Story": 1800}
        )

        report = TeamReportData(
            team_name="Test Team",
            jira_group="test-group",
            start_date="2025-01-01",
            end_date="2025-01-07",
            generated_at="2025-01-08T10:00:00",
            members=[summary1, summary2]
        )

        by_type = report.by_issue_type_total
        assert by_type["Task"] == 1.5  # (3600 + 1800) / 3600
        assert by_type["Bug"] == 1.0
        assert by_type["Story"] == 0.5

    def test_get_all_issue_types(self):
        """Test getting all issue types."""
        member = TeamMemberInfo(account_id="user1", display_name="User 1")
        summary = MemberWorklogSummary(
            member=member,
            by_issue_type={"Task": 3600, "Bug": 1800, "Story": 900}
        )

        report = TeamReportData(
            team_name="Test Team",
            jira_group="test-group",
            start_date="2025-01-01",
            end_date="2025-01-07",
            generated_at="2025-01-08T10:00:00",
            members=[summary]
        )

        types = report.get_all_issue_types()
        assert types == ["Bug", "Story", "Task"]  # sorted

    def test_get_all_dates(self):
        """Test getting all dates."""
        member = TeamMemberInfo(account_id="user1", display_name="User 1")
        summary = MemberWorklogSummary(
            member=member,
            by_date={"2025-01-03": 3600, "2025-01-01": 1800, "2025-01-02": 900}
        )

        report = TeamReportData(
            team_name="Test Team",
            jira_group="test-group",
            start_date="2025-01-01",
            end_date="2025-01-07",
            generated_at="2025-01-08T10:00:00",
            members=[summary]
        )

        dates = report.get_all_dates()
        assert dates == ["2025-01-01", "2025-01-02", "2025-01-03"]  # sorted


class TestMonthRangeHelpers:
    """Tests for month range helper functions."""

    def test_get_month_range_current(self):
        """Test getting current month range."""
        start, end = get_month_range()

        start_date = datetime.strptime(start, "%Y-%m-%d")
        end_date = datetime.strptime(end, "%Y-%m-%d")

        # Start should be day 1
        assert start_date.day == 1
        # End should be the same month
        assert end_date.month == start_date.month

    def test_get_month_range_with_reference(self):
        """Test getting month range with reference date."""
        start, end = get_month_range("2025-01-15")

        assert start == "2025-01-01"
        assert end == "2025-01-31"

    def test_get_month_range_february_leap_year(self):
        """Test February in a leap year."""
        start, end = get_month_range("2024-02-15")

        assert start == "2024-02-01"
        assert end == "2024-02-29"  # 2024 is a leap year

    def test_get_month_range_february_non_leap_year(self):
        """Test February in a non-leap year."""
        start, end = get_month_range("2025-02-15")

        assert start == "2025-02-01"
        assert end == "2025-02-28"

    def test_get_month_range_december(self):
        """Test December month range."""
        start, end = get_month_range("2025-12-15")

        assert start == "2025-12-01"
        assert end == "2025-12-31"

    def test_get_last_month_range(self):
        """Test getting last month range."""
        start, end = get_last_month_range()

        start_date = datetime.strptime(start, "%Y-%m-%d")
        end_date = datetime.strptime(end, "%Y-%m-%d")
        today = datetime.now()

        # Should be in a month before current
        if today.month == 1:
            assert start_date.month == 12
            assert start_date.year == today.year - 1
        else:
            assert start_date.month == today.month - 1


class TestTeamCLICommands:
    """Tests for team CLI commands."""

    def test_team_list_help(self):
        """Test team-list command help."""
        from typer.testing import CliRunner
        from recap.cli import app

        runner = CliRunner()
        result = runner.invoke(app, ["team-list", "--help"])

        assert result.exit_code == 0
        assert "列出所有已設定的團隊" in result.output

    def test_team_add_help(self):
        """Test team-add command help."""
        from typer.testing import CliRunner
        from recap.cli import app

        runner = CliRunner()
        result = runner.invoke(app, ["team-add", "--help"])

        assert result.exit_code == 0
        assert "新增團隊" in result.output

    def test_team_remove_help(self):
        """Test team-remove command help."""
        from typer.testing import CliRunner
        from recap.cli import app

        runner = CliRunner()
        result = runner.invoke(app, ["team-remove", "--help"])

        assert result.exit_code == 0
        assert "移除團隊設定" in result.output

    def test_team_report_help(self):
        """Test team-report command help."""
        from typer.testing import CliRunner
        from recap.cli import app

        runner = CliRunner()
        result = runner.invoke(app, ["team-report", "--help"])

        assert result.exit_code == 0
        assert "產生團隊工時報表" in result.output
        assert "--week" in result.output
        assert "--month" in result.output
        assert "--output" in result.output
