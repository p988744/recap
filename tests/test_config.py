"""Tests for config module."""

import json
import pytest
from pathlib import Path
from unittest.mock import patch

from recap.config import Config, ProjectMapping, CONFIG_DIR


class TestConfig:
    """Tests for Config class."""

    def test_default_values(self):
        """Test default configuration values."""
        config = Config()

        assert config.jira_url == "https://ims.eland.com.tw"
        assert config.jira_pat == ""
        assert config.auth_type == "pat"
        assert config.llm_provider == "ollama"
        assert config.outlook_enabled is False

    def test_is_configured_with_pat(self):
        """Test is_configured returns True when PAT is set."""
        config = Config(jira_pat="test-token")
        assert config.is_configured() is True

    def test_is_configured_without_pat(self):
        """Test is_configured returns False when PAT is empty."""
        config = Config()
        assert config.is_configured() is False

    def test_is_configured_with_basic_auth(self):
        """Test is_configured with basic auth (email + token)."""
        config = Config(
            auth_type="basic",
            jira_email="test@example.com",
            jira_api_token="api-token"
        )
        assert config.is_configured() is True

    def test_is_configured_basic_auth_incomplete(self):
        """Test is_configured returns False when basic auth is incomplete."""
        config = Config(
            auth_type="basic",
            jira_email="test@example.com"
            # Missing jira_api_token
        )
        assert config.is_configured() is False

    def test_get_token_pat(self):
        """Test get_token returns PAT for PAT auth type."""
        config = Config(auth_type="pat", jira_pat="my-pat-token")
        assert config.get_token() == "my-pat-token"

    def test_get_token_basic(self):
        """Test get_token returns API token for basic auth type."""
        config = Config(
            auth_type="basic",
            jira_api_token="my-api-token"
        )
        assert config.get_token() == "my-api-token"

    def test_has_llm_config_ollama(self):
        """Test has_llm_config returns True for Ollama (no API key needed)."""
        config = Config(llm_provider="ollama")
        assert config.has_llm_config() is True

    def test_has_llm_config_openai_compatible_with_url(self):
        """Test has_llm_config for OpenAI compatible with base URL."""
        config = Config(
            llm_provider="openai-compatible",
            llm_base_url="https://api.example.com"
        )
        assert config.has_llm_config() is True

    def test_has_llm_config_openai_compatible_without_url(self):
        """Test has_llm_config returns False without base URL."""
        config = Config(llm_provider="openai-compatible")
        assert config.has_llm_config() is False

    def test_has_llm_config_openai_with_key(self):
        """Test has_llm_config for OpenAI with API key."""
        config = Config(
            llm_provider="openai",
            llm_api_key="sk-test-key"
        )
        assert config.has_llm_config() is True

    def test_has_llm_config_openai_without_key(self):
        """Test has_llm_config returns False for OpenAI without key."""
        config = Config(llm_provider="openai")
        assert config.has_llm_config() is False

    def test_save_and_load(self, temp_config_dir):
        """Test saving and loading configuration."""
        # Patch CONFIG_DIR and CONFIG_FILE
        config_file = temp_config_dir / "config.json"

        with patch("recap.config.CONFIG_DIR", temp_config_dir), \
             patch("recap.config.CONFIG_FILE", config_file):

            # Create and save config
            original = Config(
                jira_url="https://test.jira.com",
                jira_pat="test-pat",
                llm_provider="openai",
                llm_api_key="sk-test"
            )
            original.save()

            # Verify file was created
            assert config_file.exists()

            # Load and verify
            loaded = Config.load()
            assert loaded.jira_url == "https://test.jira.com"
            assert loaded.jira_pat == "test-pat"
            assert loaded.llm_provider == "openai"
            assert loaded.llm_api_key == "sk-test"


class TestProjectMapping:
    """Tests for ProjectMapping class."""

    def test_set_and_get(self, temp_config_dir):
        """Test setting and getting project mappings."""
        mapping_file = temp_config_dir / "project_mapping.json"

        with patch("recap.config.CONFIG_DIR", temp_config_dir), \
             patch("recap.config.MAPPING_FILE", mapping_file):

            mapping = ProjectMapping()
            mapping.mappings = {}  # Reset

            # Set mapping
            mapping.set("my-project", "PROJ-123")

            # Get mapping
            assert mapping.get("my-project") == "PROJ-123"
            assert mapping.get("nonexistent") is None

    def test_get_suggestions_exact_match(self, temp_config_dir):
        """Test get_suggestions with exact match."""
        mapping_file = temp_config_dir / "project_mapping.json"

        with patch("recap.config.CONFIG_DIR", temp_config_dir), \
             patch("recap.config.MAPPING_FILE", mapping_file):

            mapping = ProjectMapping()
            mapping.mappings = {
                "tempo-sync": "PROJ-123",
                "tempo-api": "PROJ-456"
            }

            suggestions = mapping.get_suggestions("tempo-sync")
            assert "PROJ-123" in suggestions

    def test_get_suggestions_partial_match(self, temp_config_dir):
        """Test get_suggestions with partial match."""
        mapping_file = temp_config_dir / "project_mapping.json"

        with patch("recap.config.CONFIG_DIR", temp_config_dir), \
             patch("recap.config.MAPPING_FILE", mapping_file):

            mapping = ProjectMapping()
            mapping.mappings = {
                "tempo-sync": "PROJ-123",
                "tempo-api": "PROJ-456",
                "other-project": "OTHER-789"
            }

            suggestions = mapping.get_suggestions("tempo")
            assert len(suggestions) >= 2
            assert "PROJ-123" in suggestions
            assert "PROJ-456" in suggestions

    def test_get_suggestions_limit(self, temp_config_dir):
        """Test get_suggestions returns max 5 results."""
        mapping_file = temp_config_dir / "project_mapping.json"

        with patch("recap.config.CONFIG_DIR", temp_config_dir), \
             patch("recap.config.MAPPING_FILE", mapping_file):

            mapping = ProjectMapping()
            mapping.mappings = {
                f"project-{i}": f"PROJ-{i}" for i in range(10)
            }

            suggestions = mapping.get_suggestions("project")
            assert len(suggestions) <= 5
