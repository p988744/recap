"""Pytest configuration and fixtures."""

import json
import pytest
from pathlib import Path
from datetime import datetime
from unittest.mock import MagicMock, patch


@pytest.fixture
def temp_config_dir(tmp_path):
    """Create a temporary config directory."""
    config_dir = tmp_path / ".tempo-sync"
    config_dir.mkdir()
    return config_dir


@pytest.fixture
def sample_config():
    """Sample configuration data."""
    return {
        "jira_url": "https://jira.example.com",
        "jira_pat": "test-token-12345",
        "jira_email": "",
        "jira_api_token": "",
        "auth_type": "pat",
        "tempo_api_token": "",
        "llm_provider": "ollama",
        "llm_model": "llama3.2",
        "llm_api_key": "",
        "llm_base_url": "",
        "outlook_enabled": False,
        "outlook_client_id": "",
        "outlook_tenant_id": "",
        "outlook_include_meetings": True,
        "outlook_include_leave": True,
    }


@pytest.fixture
def sample_project_mapping():
    """Sample project mapping data."""
    return {
        "tempo-sync": "PROJ-123",
        "web-app": "WEB-456",
        "backend-api": "API-789",
    }


@pytest.fixture
def sample_session_data():
    """Sample Claude session data."""
    return {
        "id": "session-123",
        "startTime": "2025-12-31T09:00:00Z",
        "endTime": "2025-12-31T11:30:00Z",
        "turns": [
            {
                "role": "user",
                "content": "Help me implement user authentication"
            },
            {
                "role": "assistant",
                "content": "I'll help you implement user authentication using JWT..."
            },
            {
                "role": "user",
                "content": "Add password reset functionality"
            },
            {
                "role": "assistant",
                "content": "I'll add password reset with email verification..."
            }
        ]
    }


@pytest.fixture
def sample_worklog_entry():
    """Sample worklog entry."""
    from recap.tempo_api import WorklogEntry
    return WorklogEntry(
        issue_key="PROJ-123",
        date="2025-12-31",
        time_spent_seconds=3600,
        description="Implemented user authentication",
        account_id="user-123"
    )


@pytest.fixture
def mock_jira_response():
    """Mock Jira API response."""
    return {
        "accountId": "user-123",
        "displayName": "Test User",
        "emailAddress": "test@example.com"
    }


@pytest.fixture
def mock_issue_response():
    """Mock Jira issue response."""
    return {
        "key": "PROJ-123",
        "fields": {
            "summary": "Test Issue Summary",
            "status": {"name": "In Progress"},
            "assignee": {"displayName": "Test User"}
        }
    }


@pytest.fixture
def mock_requests_session():
    """Mock requests session for API testing."""
    with patch("requests.Session") as mock_session:
        mock_instance = MagicMock()
        mock_session.return_value = mock_instance
        yield mock_instance
