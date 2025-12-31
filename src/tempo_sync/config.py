"""
配置管理模組
"""

import json
from pathlib import Path
from dataclasses import dataclass, asdict
from typing import Optional


CONFIG_DIR = Path.home() / ".tempo-sync"
CONFIG_FILE = CONFIG_DIR / "config.json"
MAPPING_FILE = CONFIG_DIR / "project_mapping.json"


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
    # Outlook 配置
    outlook_enabled: bool = False         # 是否啟用 Outlook 整合
    outlook_client_id: str = ""           # Azure AD 應用程式 ID
    outlook_tenant_id: str = ""           # Azure AD 租戶 ID
    outlook_include_meetings: bool = True # 包含會議
    outlook_include_leave: bool = True    # 包含請假

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
