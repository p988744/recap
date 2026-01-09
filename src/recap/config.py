"""
配置管理模組
"""

import json
from pathlib import Path
from dataclasses import dataclass, asdict, field
from typing import Optional


CONFIG_DIR = Path.home() / ".recap"
CONFIG_FILE = CONFIG_DIR / "config.json"
MAPPING_FILE = CONFIG_DIR / "project_mapping.json"
TEAMS_FILE = CONFIG_DIR / "teams.json"


@dataclass
class Config:
    """應用程式配置"""
    jira_url: str = "https://ims.eland.com.tw"
    jira_pat: str = ""                    # Personal Access Token (Server)
    jira_email: str = ""                  # Email (Cloud only)
    jira_api_token: str = ""              # API Token (Cloud only)
    auth_type: str = "pat"                # "pat" (Server) or "basic" (Cloud)
    tempo_api_token: str = ""             # Tempo API Token (optional)
    # LLM 配置
    llm_provider: str = "ollama"          # anthropic, openai, gemini, ollama, openai-compatible
    llm_model: str = ""                   # 模型名稱，空則使用預設
    llm_api_key: str = ""                 # LLM API Key
    llm_base_url: str = ""                # OpenAI 相容端點 URL
    # 工時正規化配置
    daily_work_hours: float = 8.0         # 每日標準工時（小時）
    normalize_hours: bool = True          # 是否將每日工時正規化為標準工時
    # Git 模式配置
    use_git_mode: bool = False            # 是否使用 Git 模式（無需 Claude Code）
    git_repos: list[str] = field(default_factory=list)  # Git 倉庫路徑列表
    # Outlook 配置
    outlook_enabled: bool = False         # 是否啟用 Outlook 整合
    outlook_client_id: str = ""           # Azure AD 應用程式 ID
    outlook_tenant_id: str = ""           # Azure AD 租戶 ID
    outlook_include_meetings: bool = True # 包含會議
    outlook_include_leave: bool = True    # 包含請假
    # GitLab 配置
    gitlab_url: str = ""                  # GitLab Server URL
    gitlab_pat: str = ""                  # GitLab Personal Access Token
    gitlab_projects: list[int] = field(default_factory=list)  # 追蹤的專案 ID 列表

    @classmethod
    def load(cls) -> "Config":
        """載入配置"""
        if CONFIG_FILE.exists():
            try:
                with open(CONFIG_FILE) as f:
                    data = json.load(f)
                    return cls(**{k: v for k, v in data.items() if k in cls.__dataclass_fields__})
            except Exception:
                pass
        return cls()

    def save(self):
        """儲存配置"""
        CONFIG_DIR.mkdir(parents=True, exist_ok=True)
        with open(CONFIG_FILE, 'w') as f:
            json.dump(asdict(self), f, indent=2)
        # 設定檔案權限為僅擁有者可讀寫
        CONFIG_FILE.chmod(0o600)

    def is_configured(self) -> bool:
        """檢查是否已配置必要項目"""
        if self.auth_type == "pat":
            return bool(self.jira_pat)
        else:
            return bool(self.jira_email and self.jira_api_token)

    def get_token(self) -> str:
        """獲取認證 token"""
        if self.auth_type == "pat":
            return self.jira_pat
        return self.jira_api_token

    def get_llm_config(self):
        """獲取 LLM 配置"""
        from .llm_helper import LLMConfig
        return LLMConfig(
            provider=self.llm_provider,
            model=self.llm_model,
            api_key=self.llm_api_key,
            openai_base_url=self.llm_base_url,
        )

    def has_llm_config(self) -> bool:
        """檢查是否已配置 LLM"""
        if self.llm_provider == "ollama":
            return True  # Ollama 不需要 API key
        if self.llm_provider == "openai-compatible":
            return bool(self.llm_base_url)
        return bool(self.llm_api_key)

    def is_gitlab_configured(self) -> bool:
        """檢查是否已配置 GitLab"""
        return bool(self.gitlab_url and self.gitlab_pat)


class ProjectMapping:
    """專案到 Jira Issue 的映射管理"""

    def __init__(self):
        self.mappings: dict[str, str] = {}  # project_name -> jira_id
        self.load()

    def load(self):
        """載入映射"""
        if MAPPING_FILE.exists():
            try:
                with open(MAPPING_FILE) as f:
                    self.mappings = json.load(f)
            except Exception:
                self.mappings = {}

    def save(self):
        """儲存映射"""
        CONFIG_DIR.mkdir(parents=True, exist_ok=True)
        with open(MAPPING_FILE, 'w') as f:
            json.dump(self.mappings, f, indent=2, ensure_ascii=False)

    def get(self, project_name: str) -> Optional[str]:
        """獲取專案的 Jira ID"""
        return self.mappings.get(project_name)

    def set(self, project_name: str, jira_id: str):
        """設定專案的 Jira ID"""
        self.mappings[project_name] = jira_id
        self.save()

    def get_suggestions(self, project_name: str) -> list[str]:
        """獲取相似專案的 Jira ID 建議"""
        suggestions = []
        # 先找完全匹配
        if project_name in self.mappings:
            suggestions.append(self.mappings[project_name])
        # 再找部分匹配
        for name, jira_id in self.mappings.items():
            if project_name.lower() in name.lower() or name.lower() in project_name.lower():
                if jira_id not in suggestions:
                    suggestions.append(jira_id)
        return suggestions[:5]


@dataclass
class TeamMemberInfo:
    """團隊成員資訊"""
    account_id: str
    display_name: str
    email: str = ""


@dataclass
class TeamInfo:
    """團隊配置資訊"""
    name: str                                    # 使用者定義的團隊名稱
    jira_group: str = ""                         # Jira 群組名稱（舊方式）
    tempo_team_id: Optional[int] = None          # Tempo 團隊 ID（新方式，優先使用）
    members: list[TeamMemberInfo] = field(default_factory=list)  # 快取的成員列表
    last_synced: Optional[str] = None            # 上次同步時間 (ISO format)


class TeamConfig:
    """團隊配置管理"""

    def __init__(self):
        self.teams: dict[str, TeamInfo] = {}  # team_name -> TeamInfo
        self.load()

    def load(self):
        """載入團隊配置"""
        if TEAMS_FILE.exists():
            try:
                with open(TEAMS_FILE) as f:
                    data = json.load(f)
                    for name, info in data.items():
                        members = [
                            TeamMemberInfo(**m) for m in info.get("members", [])
                        ]
                        self.teams[name] = TeamInfo(
                            name=name,
                            jira_group=info.get("jira_group", ""),
                            tempo_team_id=info.get("tempo_team_id"),
                            members=members,
                            last_synced=info.get("last_synced"),
                        )
            except Exception:
                self.teams = {}

    def save(self):
        """儲存團隊配置"""
        CONFIG_DIR.mkdir(parents=True, exist_ok=True)
        data = {}
        for name, info in self.teams.items():
            data[name] = {
                "jira_group": info.jira_group,
                "tempo_team_id": info.tempo_team_id,
                "members": [asdict(m) for m in info.members],
                "last_synced": info.last_synced,
            }
        with open(TEAMS_FILE, 'w') as f:
            json.dump(data, f, indent=2, ensure_ascii=False)

    def add_team(self, name: str, jira_group: str = "", tempo_team_id: Optional[int] = None) -> bool:
        """新增團隊"""
        if name in self.teams:
            return False
        self.teams[name] = TeamInfo(name=name, jira_group=jira_group, tempo_team_id=tempo_team_id)
        self.save()
        return True

    def remove_team(self, name: str) -> bool:
        """移除團隊"""
        if name not in self.teams:
            return False
        del self.teams[name]
        self.save()
        return True

    def get_team(self, name: str) -> Optional[TeamInfo]:
        """取得團隊資訊"""
        return self.teams.get(name)

    def list_teams(self) -> list[TeamInfo]:
        """列出所有團隊"""
        return list(self.teams.values())

    def update_members(self, name: str, members: list[TeamMemberInfo], synced_at: str):
        """更新團隊成員快取"""
        if name in self.teams:
            self.teams[name].members = members
            self.teams[name].last_synced = synced_at
            self.save()
