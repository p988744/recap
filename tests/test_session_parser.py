"""Tests for session_parser module."""

import pytest
from datetime import datetime
from unittest.mock import patch, MagicMock

from recap.session_parser import (
    WorkSession,
    DailyProjectEntry,
    ProjectSummary,
    WeeklyWorklog,
    ClaudeSessionParser,
    GitSessionParser,
    WorklogHelper,
)


class TestWorkSession:
    """Tests for WorkSession dataclass."""

    def test_create_session(self):
        """Test creating a work session."""
        session = WorkSession(
            project_path="/path/to/project",
            project_name="my-project",
            session_id="session-123",
            start_time=datetime(2025, 12, 31, 9, 0),
            end_time=datetime(2025, 12, 31, 11, 30),
            duration_minutes=150,
            date="2025-12-31",
            summary=["Implemented feature X"],
            todos=["Add tests", "Update docs"]
        )

        assert session.project_name == "my-project"
        assert session.duration_minutes == 150
        assert session.date == "2025-12-31"
        assert len(session.todos) == 2

    def test_default_values(self):
        """Test default values for optional fields."""
        session = WorkSession(
            project_path="/path",
            project_name="project",
            session_id="id",
            start_time=datetime.now(),
            end_time=datetime.now(),
            duration_minutes=60,
            date="2025-12-31"
        )

        assert session.summary == []
        assert session.todos == []
        assert session.jira_id is None


class TestDailyProjectEntry:
    """Tests for DailyProjectEntry dataclass."""

    def test_get_description_with_todos(self):
        """Test description generation with todos."""
        entry = DailyProjectEntry(
            date="2025-12-31",
            minutes=120,
            todos=["Add login", "Fix bug", "Update UI"],
            summaries=[]
        )

        description = entry.get_description("my-project")
        assert "完成:" in description
        assert "Add login" in description

    def test_get_description_with_summaries(self):
        """Test description generation with summaries when no todos."""
        entry = DailyProjectEntry(
            date="2025-12-31",
            minutes=120,
            todos=[],
            summaries=["Worked on authentication module"]
        )

        description = entry.get_description("my-project")
        assert "authentication" in description

    def test_get_description_fallback(self):
        """Test fallback description when no todos or summaries."""
        entry = DailyProjectEntry(
            date="2025-12-31",
            minutes=120,
            todos=[],
            summaries=[]
        )

        description = entry.get_description("my-project")
        assert "my-project" in description


class TestProjectSummary:
    """Tests for ProjectSummary dataclass."""

    def test_total_hours(self):
        """Test total hours calculation."""
        summary = ProjectSummary(
            project_name="project",
            project_path="/path",
            total_minutes=150,
            daily_entries=[]
        )

        assert summary.total_hours == 2.5

    def test_get_daily_breakdown(self):
        """Test daily breakdown string generation."""
        summary = ProjectSummary(
            project_name="project",
            project_path="/path",
            total_minutes=240,
            daily_entries=[
                DailyProjectEntry(
                    date="2025-12-30",
                    minutes=120,
                    todos=[],
                    summaries=[]
                ),
                DailyProjectEntry(
                    date="2025-12-31",
                    minutes=120,
                    todos=[],
                    summaries=[]
                ),
            ]
        )

        breakdown = summary.get_daily_breakdown()
        assert "2025-12-30" in breakdown
        assert "2025-12-31" in breakdown
        assert "2.0h" in breakdown


class TestWeeklyWorklog:
    """Tests for WeeklyWorklog dataclass."""

    def test_total_minutes(self):
        """Test total minutes calculation."""
        worklog = WeeklyWorklog(
            start_date="2025-12-25",
            end_date="2025-12-31",
            sessions=[
                WorkSession(
                    project_path="/p1",
                    project_name="project1",
                    session_id="s1",
                    start_time=datetime.now(),
                    end_time=datetime.now(),
                    duration_minutes=60,
                    date="2025-12-30"
                ),
                WorkSession(
                    project_path="/p2",
                    project_name="project2",
                    session_id="s2",
                    start_time=datetime.now(),
                    end_time=datetime.now(),
                    duration_minutes=90,
                    date="2025-12-31"
                ),
            ]
        )

        assert worklog.total_minutes == 150

    def test_dates_covered(self):
        """Test dates covered extraction."""
        worklog = WeeklyWorklog(
            start_date="2025-12-25",
            end_date="2025-12-31",
            sessions=[
                WorkSession(
                    project_path="/p1",
                    project_name="project1",
                    session_id="s1",
                    start_time=datetime.now(),
                    end_time=datetime.now(),
                    duration_minutes=60,
                    date="2025-12-30"
                ),
                WorkSession(
                    project_path="/p1",
                    project_name="project1",
                    session_id="s2",
                    start_time=datetime.now(),
                    end_time=datetime.now(),
                    duration_minutes=60,
                    date="2025-12-30"
                ),
                WorkSession(
                    project_path="/p2",
                    project_name="project2",
                    session_id="s3",
                    start_time=datetime.now(),
                    end_time=datetime.now(),
                    duration_minutes=90,
                    date="2025-12-31"
                ),
            ]
        )

        dates = worklog.dates_covered
        assert len(dates) == 2
        assert "2025-12-30" in dates
        assert "2025-12-31" in dates

    def test_get_project_summaries(self):
        """Test project summaries generation."""
        worklog = WeeklyWorklog(
            start_date="2025-12-25",
            end_date="2025-12-31",
            sessions=[
                WorkSession(
                    project_path="/p1",
                    project_name="project1",
                    session_id="s1",
                    start_time=datetime.now(),
                    end_time=datetime.now(),
                    duration_minutes=60,
                    date="2025-12-30"
                ),
                WorkSession(
                    project_path="/p1",
                    project_name="project1",
                    session_id="s2",
                    start_time=datetime.now(),
                    end_time=datetime.now(),
                    duration_minutes=90,
                    date="2025-12-31"
                ),
                WorkSession(
                    project_path="/p2",
                    project_name="project2",
                    session_id="s3",
                    start_time=datetime.now(),
                    end_time=datetime.now(),
                    duration_minutes=120,
                    date="2025-12-31"
                ),
            ]
        )

        summaries = worklog.get_project_summaries()

        assert len(summaries) == 2

        project1 = next(s for s in summaries if s.project_name == "project1")
        project2 = next(s for s in summaries if s.project_name == "project2")

        assert project1.total_minutes == 150
        assert len(project1.daily_entries) == 2

        assert project2.total_minutes == 120
        assert len(project2.daily_entries) == 1

    def test_empty_worklog(self):
        """Test empty worklog."""
        worklog = WeeklyWorklog(
            start_date="2025-12-25",
            end_date="2025-12-31",
            sessions=[]
        )

        assert worklog.total_minutes == 0
        assert worklog.dates_covered == []
        assert worklog.get_project_summaries() == []


class TestClaudeSessionParser:
    """Tests for ClaudeSessionParser class."""

    def test_init_with_default_path(self):
        """Test initialization with default Claude directory."""
        parser = ClaudeSessionParser()
        assert parser.claude_dir is not None
        assert ".claude" in str(parser.claude_dir)

    def test_init_with_custom_path(self, tmp_path):
        """Test initialization with custom Claude directory."""
        custom_dir = tmp_path / ".claude"
        custom_dir.mkdir()

        parser = ClaudeSessionParser(str(custom_dir))
        assert parser.claude_dir == custom_dir

    def test_projects_dir(self, tmp_path):
        """Test projects directory is set correctly."""
        custom_dir = tmp_path / ".claude"
        custom_dir.mkdir()

        parser = ClaudeSessionParser(str(custom_dir))
        assert parser.projects_dir == custom_dir / "projects"

    def test_get_available_dates_empty(self, tmp_path):
        """Test getting available dates when no sessions exist."""
        custom_dir = tmp_path / ".claude"
        custom_dir.mkdir()
        (custom_dir / "projects").mkdir()

        parser = ClaudeSessionParser(str(custom_dir))
        dates = parser.get_available_dates()

        assert isinstance(dates, list)
        assert len(dates) == 0


class TestGitSessionParser:
    """Tests for GitSessionParser class."""

    def test_init_empty(self):
        """Test initialization with no repos."""
        parser = GitSessionParser()
        assert parser.repo_paths == []

    def test_init_with_repos(self, tmp_path):
        """Test initialization with repo paths."""
        repo1 = tmp_path / "repo1"
        repo1.mkdir()
        (repo1 / ".git").mkdir()

        parser = GitSessionParser([str(repo1)])
        assert len(parser.repo_paths) == 1

    def test_add_repo_valid(self, tmp_path):
        """Test adding a valid git repository."""
        repo = tmp_path / "my-repo"
        repo.mkdir()
        (repo / ".git").mkdir()

        parser = GitSessionParser()
        result = parser.add_repo(str(repo))

        assert result is True
        assert len(parser.repo_paths) == 1

    def test_add_repo_invalid(self, tmp_path):
        """Test adding an invalid path (not a git repo)."""
        not_repo = tmp_path / "not-a-repo"
        not_repo.mkdir()

        parser = GitSessionParser()
        result = parser.add_repo(str(not_repo))

        assert result is False
        assert len(parser.repo_paths) == 0

    def test_add_repo_nonexistent(self, tmp_path):
        """Test adding a nonexistent path."""
        parser = GitSessionParser()
        result = parser.add_repo(str(tmp_path / "nonexistent"))

        assert result is False
        assert len(parser.repo_paths) == 0

    def test_parse_date_range_empty(self, tmp_path):
        """Test parsing when no repos are configured."""
        parser = GitSessionParser()
        worklog = parser.parse_date_range("2025-01-01", "2025-01-07")

        assert worklog.sessions == []
        assert worklog.start_date == "2025-01-01"
        assert worklog.end_date == "2025-01-07"

    def test_parse_date_single_day(self, tmp_path):
        """Test parse_date convenience method."""
        parser = GitSessionParser()
        worklog = parser.parse_date("2025-01-01")

        assert worklog.start_date == "2025-01-01"
        assert worklog.end_date == "2025-01-01"


class TestWorklogHelperModes:
    """Tests for WorklogHelper git/claude mode switching."""

    def test_default_claude_mode(self):
        """Test default mode is Claude."""
        with patch("recap.session_parser.Config.load") as mock_load:
            mock_config = MagicMock()
            mock_config.use_git_mode = False
            mock_config.git_repos = []
            mock_load.return_value = mock_config

            helper = WorklogHelper()
            assert helper.mode == "claude"
            assert isinstance(helper.parser, ClaudeSessionParser)

    def test_git_mode_from_config(self):
        """Test git mode from config."""
        with patch("recap.session_parser.Config.load") as mock_load:
            mock_config = MagicMock()
            mock_config.use_git_mode = True
            mock_config.git_repos = []
            mock_load.return_value = mock_config

            helper = WorklogHelper()
            assert helper.mode == "git"
            assert isinstance(helper.parser, GitSessionParser)

    def test_git_mode_override(self):
        """Test git mode can be overridden."""
        with patch("recap.session_parser.Config.load") as mock_load:
            mock_config = MagicMock()
            mock_config.use_git_mode = False
            mock_config.git_repos = []
            mock_load.return_value = mock_config

            helper = WorklogHelper(use_git=True)
            assert helper.mode == "git"

    def test_claude_mode_override(self):
        """Test claude mode can be overridden."""
        with patch("recap.session_parser.Config.load") as mock_load:
            mock_config = MagicMock()
            mock_config.use_git_mode = True
            mock_config.git_repos = []
            mock_load.return_value = mock_config

            helper = WorklogHelper(use_git=False)
            assert helper.mode == "claude"

    def test_add_git_repo_in_git_mode(self, tmp_path):
        """Test adding repo in git mode."""
        repo = tmp_path / "repo"
        repo.mkdir()
        (repo / ".git").mkdir()

        with patch("recap.session_parser.Config.load") as mock_load:
            mock_config = MagicMock()
            mock_config.use_git_mode = True
            mock_config.git_repos = []
            mock_load.return_value = mock_config

            helper = WorklogHelper()
            result = helper.add_git_repo(str(repo))
            assert result is True

    def test_add_git_repo_in_claude_mode(self, tmp_path):
        """Test adding repo in claude mode does nothing."""
        repo = tmp_path / "repo"
        repo.mkdir()
        (repo / ".git").mkdir()

        with patch("recap.session_parser.Config.load") as mock_load:
            mock_config = MagicMock()
            mock_config.use_git_mode = False
            mock_config.git_repos = []
            mock_load.return_value = mock_config

            helper = WorklogHelper()
            result = helper.add_git_repo(str(repo))
            assert result is False
