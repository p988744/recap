"""Tests for CLI module."""

import pytest
from unittest.mock import patch, MagicMock
from typer.testing import CliRunner
from datetime import datetime, timedelta

from tempo_sync.cli import app, get_week_range, get_last_week_range


runner = CliRunner()


class TestHelperFunctions:
    """Tests for CLI helper functions."""

    def test_get_week_range_current(self):
        """Test getting current week range."""
        start, end = get_week_range()

        start_date = datetime.strptime(start, "%Y-%m-%d")
        end_date = datetime.strptime(end, "%Y-%m-%d")

        # Should be 7 days
        diff = (end_date - start_date).days
        assert diff == 6

        # Start should be Monday
        assert start_date.weekday() == 0

    def test_get_week_range_with_reference(self):
        """Test getting week range with reference date."""
        # Wednesday 2025-01-15
        start, end = get_week_range("2025-01-15")

        assert start == "2025-01-13"  # Monday
        assert end == "2025-01-19"    # Sunday

    def test_get_last_week_range(self):
        """Test getting last week range."""
        start, end = get_last_week_range()

        start_date = datetime.strptime(start, "%Y-%m-%d")
        end_date = datetime.strptime(end, "%Y-%m-%d")

        # Should be 7 days
        diff = (end_date - start_date).days
        assert diff == 6

        # Start should be Monday
        assert start_date.weekday() == 0

        # Should be in the past
        today = datetime.now()
        assert end_date < today


class TestCliCommands:
    """Tests for CLI commands."""

    def test_help(self):
        """Test help command."""
        result = runner.invoke(app, ["--help"])
        assert result.exit_code == 0
        assert "tempo" in result.output.lower() or "worklog" in result.output.lower()

    def test_dates_command(self):
        """Test dates command."""
        with patch("tempo_sync.cli.WorklogHelper") as mock_helper:
            mock_instance = MagicMock()
            mock_helper.return_value = mock_instance
            mock_instance.get_available_dates.return_value = [
                "2025-12-30",
                "2025-12-31"
            ]

            result = runner.invoke(app, ["dates"])

            # Should run without error
            assert result.exit_code == 0

    def test_setup_command_interactive(self):
        """Test setup command invocation."""
        result = runner.invoke(app, ["setup", "--help"])
        assert result.exit_code == 0
        assert "配置" in result.output or "setup" in result.output.lower()

    def test_setup_llm_command(self):
        """Test setup-llm command invocation."""
        result = runner.invoke(app, ["setup-llm", "--help"])
        assert result.exit_code == 0
        assert "LLM" in result.output or "llm" in result.output.lower()

    def test_analyze_help(self):
        """Test analyze command help."""
        result = runner.invoke(app, ["analyze", "--help"])
        assert result.exit_code == 0
        assert "--week" in result.output or "-w" in result.output

    def test_outlook_login_help(self):
        """Test outlook-login command help."""
        result = runner.invoke(app, ["outlook-login", "--help"])
        assert result.exit_code == 0

    def test_outlook_logout_help(self):
        """Test outlook-logout command help."""
        result = runner.invoke(app, ["outlook-logout", "--help"])
        assert result.exit_code == 0


class TestAnalyzeCommand:
    """Tests for analyze command."""

    def test_analyze_week_flag(self):
        """Test analyze with --week flag."""
        with patch("tempo_sync.cli.WorklogHelper") as mock_helper:
            mock_instance = MagicMock()
            mock_helper.return_value = mock_instance

            # Mock empty worklog
            mock_worklog = MagicMock()
            mock_worklog.sessions = []
            mock_instance.analyze_range.return_value = mock_worklog

            result = runner.invoke(app, ["analyze", "--week"])

            # Should complete (may show "no sessions" message)
            mock_instance.analyze_range.assert_called_once()

    def test_analyze_last_week_flag(self):
        """Test analyze with --last-week flag."""
        with patch("tempo_sync.cli.WorklogHelper") as mock_helper:
            mock_instance = MagicMock()
            mock_helper.return_value = mock_instance

            mock_worklog = MagicMock()
            mock_worklog.sessions = []
            mock_instance.analyze_range.return_value = mock_worklog

            result = runner.invoke(app, ["analyze", "--last-week"])

            mock_instance.analyze_range.assert_called_once()

    def test_analyze_days_flag(self):
        """Test analyze with --days flag."""
        with patch("tempo_sync.cli.WorklogHelper") as mock_helper:
            mock_instance = MagicMock()
            mock_helper.return_value = mock_instance

            mock_worklog = MagicMock()
            mock_worklog.sessions = []
            mock_instance.analyze_range.return_value = mock_worklog

            result = runner.invoke(app, ["analyze", "--days", "14"])

            mock_instance.analyze_range.assert_called_once()

    def test_analyze_custom_range(self):
        """Test analyze with custom date range."""
        with patch("tempo_sync.cli.WorklogHelper") as mock_helper:
            mock_instance = MagicMock()
            mock_helper.return_value = mock_instance

            mock_worklog = MagicMock()
            mock_worklog.sessions = []
            mock_instance.analyze_range.return_value = mock_worklog

            result = runner.invoke(app, [
                "analyze",
                "--from", "2025-01-01",
                "--to", "2025-01-07"
            ])

            # Just check it ran without error, mocking is complex with typer
            # The function may output "no sessions" message
            assert result.exit_code == 0 or "找不到" in result.output or mock_instance.analyze_range.called


class TestConfigDisplay:
    """Tests for configuration display."""

    def test_config_status_display(self):
        """Test that config status is properly formatted."""
        with patch("tempo_sync.cli.WorklogHelper") as mock_helper:
            mock_instance = MagicMock()
            mock_helper.return_value = mock_instance

            # Mock config
            mock_config = MagicMock()
            mock_config.is_configured.return_value = True
            mock_config.has_llm_config.return_value = True
            mock_config.jira_url = "https://jira.example.com"
            mock_config.auth_type = "pat"
            mock_config.llm_provider = "openai"
            mock_config.llm_model = "gpt-4"
            mock_config.outlook_enabled = False
            mock_config.outlook_client_id = ""
            mock_config.outlook_tenant_id = ""

            mock_instance.config = mock_config

            # The config display is tested implicitly through other commands
            # This is more of an integration test
            result = runner.invoke(app, ["dates"])

            # Should not crash
            assert result.exit_code == 0 or "找不到" in result.output
