"""
Jira & Tempo API 整合模組

支援:
- Jira Server PAT (Personal Access Token) 認證
- Jira Cloud Basic Auth
- Tempo Timesheets API
"""

import requests
from datetime import datetime
from dataclasses import dataclass
from typing import Optional
import base64


@dataclass
class WorklogEntry:
    """要上傳的 worklog 項目"""
    issue_key: str          # e.g., "PROJ-123"
    date: str              # e.g., "2025-12-31"
    time_spent_seconds: int
    description: str
    account_id: Optional[str] = None  # Jira user account ID


class JiraClient:
    """Jira REST API 客戶端"""

    def __init__(self, base_url: str, token: str, email: Optional[str] = None,
                 auth_type: str = "pat"):
        """
        初始化 Jira 客戶端

        Args:
            base_url: Jira URL (e.g., https://ims.eland.com.tw)
            token: PAT 或 API Token
            email: Email (僅 Cloud Basic Auth 需要)
            auth_type: 認證類型 - "pat" (Server) 或 "basic" (Cloud)
        """
        self.base_url = base_url.rstrip('/')
        self.session = requests.Session()

        if auth_type == "pat":
            # Jira Server PAT: Bearer token
            self.session.headers.update({
                "Authorization": f"Bearer {token}",
            })
        else:
            # Jira Cloud: Basic auth (email:token)
            auth_string = base64.b64encode(f"{email}:{token}".encode()).decode()
            self.session.headers.update({
                "Authorization": f"Basic {auth_string}",
            })

        self.session.headers.update({
            "Content-Type": "application/json",
            "Accept": "application/json"
        })

    def get_myself(self) -> dict:
        """獲取當前用戶資訊"""
        resp = self.session.get(f"{self.base_url}/rest/api/2/myself")
        resp.raise_for_status()
        return resp.json()

    def get_issue(self, issue_key: str) -> Optional[dict]:
        """獲取 issue 資訊"""
        try:
            resp = self.session.get(f"{self.base_url}/rest/api/2/issue/{issue_key}")
            resp.raise_for_status()
            return resp.json()
        except requests.exceptions.HTTPError as e:
            if e.response.status_code == 404:
                return None
            raise

    def validate_issue_key(self, issue_key: str) -> tuple[bool, str]:
        """驗證 issue key 是否有效"""
        issue = self.get_issue(issue_key)
        if issue:
            summary = issue.get('fields', {}).get('summary', 'Unknown')
            return True, summary
        return False, "Issue not found"

    def add_worklog(self, entry: WorklogEntry) -> dict:
        """添加 worklog 到 Jira issue (使用 Jira 原生 worklog API)"""
        url = f"{self.base_url}/rest/api/2/issue/{entry.issue_key}/worklog"

        payload = {
            "timeSpentSeconds": entry.time_spent_seconds,
            "comment": entry.description,
            "started": self._format_jira_datetime(entry.date)
        }

        resp = self.session.post(url, json=payload)
        resp.raise_for_status()
        return resp.json()

    def _format_jira_datetime(self, date_str: str) -> str:
        """格式化日期為 Jira 接受的格式"""
        # Jira 需要 ISO 8601 格式: 2025-12-31T09:00:00.000+0800
        dt = datetime.strptime(date_str, "%Y-%m-%d")
        return dt.strftime("%Y-%m-%dT09:00:00.000+0800")


class TempoClient:
    """Tempo Timesheets API 客戶端"""

    def __init__(self, base_url: str, api_token: str):
        """
        初始化 Tempo 客戶端

        對於 self-hosted Jira + Tempo，API endpoint 通常是:
        - {jira_url}/rest/tempo-timesheets/4/worklogs
        """
        self.base_url = base_url.rstrip('/')
        self.session = requests.Session()
        self.session.headers.update({
            "Authorization": f"Bearer {api_token}",
            "Content-Type": "application/json",
            "Accept": "application/json"
        })

    def get_worklogs(self, date_from: str, date_to: str) -> list[dict]:
        """獲取指定日期範圍的 worklogs"""
        url = f"{self.base_url}/rest/tempo-timesheets/4/worklogs"
        params = {
            "dateFrom": date_from,
            "dateTo": date_to
        }
        resp = self.session.get(url, params=params)
        resp.raise_for_status()
        return resp.json()

    def create_worklog(self, entry: WorklogEntry) -> dict:
        """創建 worklog"""
        url = f"{self.base_url}/rest/tempo-timesheets/4/worklogs"

        payload = {
            "issueKey": entry.issue_key,
            "timeSpentSeconds": entry.time_spent_seconds,
            "startDate": entry.date,
            "startTime": "09:00:00",
            "description": entry.description,
            "authorAccountId": entry.account_id
        }

        resp = self.session.post(url, json=payload)
        resp.raise_for_status()
        return resp.json()


class WorklogUploader:
    """Worklog 上傳管理器"""

    def __init__(self, jira_url: str, token: str, email: Optional[str] = None,
                 auth_type: str = "pat", tempo_token: Optional[str] = None):
        """
        初始化上傳管理器

        Args:
            jira_url: Jira URL
            token: PAT 或 API Token
            email: Email (僅 Cloud 需要)
            auth_type: "pat" (Server) 或 "basic" (Cloud)
            tempo_token: Tempo API Token (可選)
        """
        self.jira = JiraClient(jira_url, token, email, auth_type)
        self.tempo = TempoClient(jira_url, tempo_token) if tempo_token else None
        self._account_id: Optional[str] = None

    @property
    def account_id(self) -> str:
        """獲取當前用戶的 account ID"""
        if not self._account_id:
            user = self.jira.get_myself()
            # Jira Server 可能使用 'name' 或 'key' 而不是 'accountId'
            self._account_id = user.get('accountId') or user.get('name') or user.get('key')
        return self._account_id

    def validate_issue(self, issue_key: str) -> tuple[bool, str]:
        """驗證 issue"""
        return self.jira.validate_issue_key(issue_key)

    def upload_worklog(self, entry: WorklogEntry, use_tempo: bool = False) -> dict:
        """上傳 worklog"""
        if not entry.account_id:
            entry.account_id = self.account_id

        if use_tempo and self.tempo:
            return self.tempo.create_worklog(entry)
        else:
            return self.jira.add_worklog(entry)

    def test_connection(self) -> tuple[bool, str]:
        """測試連接"""
        try:
            user = self.jira.get_myself()
            display_name = user.get('displayName', user.get('name', 'Unknown'))
            return True, f"Connected as: {display_name}"
        except requests.exceptions.HTTPError as e:
            return False, f"HTTP Error: {e.response.status_code} - {e.response.text}"
        except Exception as e:
            return False, f"Connection failed: {str(e)}"
