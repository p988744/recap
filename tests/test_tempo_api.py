"""Tests for tempo_api module."""

import pytest
from unittest.mock import MagicMock, patch
from datetime import datetime

from recap.tempo_api import WorklogEntry, JiraClient, TempoClient, WorklogUploader


class TestWorklogEntry:
    """Tests for WorklogEntry dataclass."""

    def test_create_entry(self):
        """Test creating a worklog entry."""
        entry = WorklogEntry(
            issue_key="PROJ-123",
            date="2025-12-31",
            time_spent_seconds=3600,
            description="Test work"
        )

        assert entry.issue_key == "PROJ-123"
        assert entry.date == "2025-12-31"
        assert entry.time_spent_seconds == 3600
        assert entry.description == "Test work"
        assert entry.account_id is None

    def test_create_entry_with_account_id(self):
        """Test creating a worklog entry with account ID."""
        entry = WorklogEntry(
            issue_key="PROJ-123",
            date="2025-12-31",
            time_spent_seconds=7200,
            description="More work",
            account_id="user-123"
        )

        assert entry.account_id == "user-123"


class TestJiraClient:
    """Tests for JiraClient class."""

    def test_init_pat_auth(self):
        """Test initialization with PAT authentication."""
        with patch("requests.Session") as mock_session:
            mock_instance = MagicMock()
            mock_session.return_value = mock_instance

            client = JiraClient(
                base_url="https://jira.example.com",
                token="test-pat",
                auth_type="pat"
            )

            assert client.base_url == "https://jira.example.com"
            mock_instance.headers.update.assert_called()

    def test_init_basic_auth(self):
        """Test initialization with Basic authentication."""
        with patch("requests.Session") as mock_session:
            mock_instance = MagicMock()
            mock_session.return_value = mock_instance

            client = JiraClient(
                base_url="https://jira.example.com",
                token="api-token",
                email="user@example.com",
                auth_type="basic"
            )

            assert client.base_url == "https://jira.example.com"

    def test_base_url_strips_trailing_slash(self):
        """Test that trailing slash is stripped from base URL."""
        with patch("requests.Session"):
            client = JiraClient(
                base_url="https://jira.example.com/",
                token="test-pat"
            )

            assert client.base_url == "https://jira.example.com"

    def test_format_jira_datetime(self):
        """Test date formatting for Jira API."""
        with patch("requests.Session"):
            client = JiraClient(
                base_url="https://jira.example.com",
                token="test-pat"
            )

            formatted = client._format_jira_datetime("2025-12-31")
            assert formatted == "2025-12-31T09:00:00.000+0800"

    def test_get_myself(self, mock_jira_response):
        """Test getting current user info."""
        with patch("requests.Session") as mock_session:
            mock_instance = MagicMock()
            mock_session.return_value = mock_instance

            mock_response = MagicMock()
            mock_response.json.return_value = mock_jira_response
            mock_instance.get.return_value = mock_response

            client = JiraClient(
                base_url="https://jira.example.com",
                token="test-pat"
            )

            result = client.get_myself()

            assert result["displayName"] == "Test User"
            mock_instance.get.assert_called_once()

    def test_validate_issue_key_success(self, mock_issue_response):
        """Test validating existing issue."""
        with patch("requests.Session") as mock_session:
            mock_instance = MagicMock()
            mock_session.return_value = mock_instance

            mock_response = MagicMock()
            mock_response.json.return_value = mock_issue_response
            mock_instance.get.return_value = mock_response

            client = JiraClient(
                base_url="https://jira.example.com",
                token="test-pat"
            )

            valid, summary = client.validate_issue_key("PROJ-123")

            assert valid is True
            assert summary == "Test Issue Summary"

    def test_validate_issue_key_not_found(self):
        """Test validating non-existent issue."""
        with patch("requests.Session") as mock_session:
            mock_instance = MagicMock()
            mock_session.return_value = mock_instance

            import requests
            mock_response = MagicMock()
            mock_response.status_code = 404
            error = requests.exceptions.HTTPError()
            error.response = mock_response
            mock_instance.get.side_effect = error

            client = JiraClient(
                base_url="https://jira.example.com",
                token="test-pat"
            )

            valid, message = client.validate_issue_key("INVALID-999")

            assert valid is False
            assert message == "Issue not found"


class TestTempoClient:
    """Tests for TempoClient class."""

    def test_init(self):
        """Test initialization."""
        with patch("requests.Session") as mock_session:
            mock_instance = MagicMock()
            mock_session.return_value = mock_instance

            client = TempoClient(
                base_url="https://jira.example.com",
                api_token="tempo-token"
            )

            assert client.base_url == "https://jira.example.com"

    def test_create_worklog_payload(self, sample_worklog_entry):
        """Test worklog creation payload structure."""
        with patch("requests.Session") as mock_session:
            mock_instance = MagicMock()
            mock_session.return_value = mock_instance

            mock_response = MagicMock()
            mock_response.json.return_value = {"id": 123}
            mock_instance.post.return_value = mock_response

            client = TempoClient(
                base_url="https://jira.example.com",
                api_token="tempo-token"
            )

            client.create_worklog(sample_worklog_entry)

            # Verify the POST was called with correct structure
            mock_instance.post.assert_called_once()
            call_args = mock_instance.post.call_args

            assert "issueKey" in str(call_args) or call_args[1]["json"]["issueKey"]


class TestWorklogUploader:
    """Tests for WorklogUploader class."""

    def test_init_without_tempo(self):
        """Test initialization without Tempo token."""
        with patch("requests.Session"):
            uploader = WorklogUploader(
                jira_url="https://jira.example.com",
                token="test-pat"
            )

            assert uploader.tempo is None

    def test_init_with_tempo(self):
        """Test initialization with Tempo token."""
        with patch("requests.Session"):
            uploader = WorklogUploader(
                jira_url="https://jira.example.com",
                token="test-pat",
                tempo_token="tempo-token"
            )

            assert uploader.tempo is not None

    def test_account_id_cached(self, mock_jira_response):
        """Test that account ID is cached after first fetch."""
        with patch("requests.Session") as mock_session:
            mock_instance = MagicMock()
            mock_session.return_value = mock_instance

            mock_response = MagicMock()
            mock_response.json.return_value = mock_jira_response
            mock_instance.get.return_value = mock_response

            uploader = WorklogUploader(
                jira_url="https://jira.example.com",
                token="test-pat"
            )

            # Access account_id twice
            _ = uploader.account_id
            _ = uploader.account_id

            # get_myself should only be called once due to caching
            assert mock_instance.get.call_count == 1

    def test_validate_issue(self, mock_issue_response):
        """Test issue validation through uploader."""
        with patch("requests.Session") as mock_session:
            mock_instance = MagicMock()
            mock_session.return_value = mock_instance

            mock_response = MagicMock()
            mock_response.json.return_value = mock_issue_response
            mock_instance.get.return_value = mock_response

            uploader = WorklogUploader(
                jira_url="https://jira.example.com",
                token="test-pat"
            )

            valid, summary = uploader.validate_issue("PROJ-123")

            assert valid is True
            assert summary == "Test Issue Summary"

    def test_test_connection_success(self, mock_jira_response):
        """Test successful connection test."""
        with patch("requests.Session") as mock_session:
            mock_instance = MagicMock()
            mock_session.return_value = mock_instance

            mock_response = MagicMock()
            mock_response.json.return_value = mock_jira_response
            mock_instance.get.return_value = mock_response

            uploader = WorklogUploader(
                jira_url="https://jira.example.com",
                token="test-pat"
            )

            success, message = uploader.test_connection()

            assert success is True
            assert "Test User" in message

    def test_test_connection_failure(self):
        """Test failed connection test."""
        with patch("requests.Session") as mock_session:
            mock_instance = MagicMock()
            mock_session.return_value = mock_instance

            mock_instance.get.side_effect = Exception("Connection refused")

            uploader = WorklogUploader(
                jira_url="https://jira.example.com",
                token="test-pat"
            )

            success, message = uploader.test_connection()

            assert success is False
            assert "Connection refused" in message
