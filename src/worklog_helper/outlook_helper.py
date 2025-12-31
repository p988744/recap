"""
Outlook 行事曆整合模組

使用 Microsoft Graph API 取得 Microsoft 365 行事曆資料
"""

import json
from datetime import datetime, timedelta
from pathlib import Path
from dataclasses import dataclass
from typing import Optional

# Microsoft Graph API endpoints
GRAPH_API_ENDPOINT = "https://graph.microsoft.com/v1.0"
SCOPES = ["Calendars.Read"]

# Token cache path
TOKEN_CACHE_FILE = Path.home() / ".worklog-helper" / "outlook_token_cache.json"


@dataclass
class CalendarEvent:
    """行事曆事件"""
    subject: str
    start: datetime
    end: datetime
    is_all_day: bool
    duration_minutes: int
    categories: list[str]
    is_meeting: bool  # 有其他與會者
    organizer: str
    location: str

    @property
    def is_leave(self) -> bool:
        """判斷是否為請假"""
        leave_keywords = ["請假", "休假", "leave", "vacation", "pto", "off"]
        subject_lower = self.subject.lower()
        return self.is_all_day or any(kw in subject_lower for kw in leave_keywords)

    def get_description(self) -> str:
        """取得事件描述"""
        if self.is_leave:
            return f"[請假] {self.subject}"
        elif self.is_meeting:
            return f"[會議] {self.subject}"
        else:
            return self.subject


class OutlookClient:
    """Microsoft Graph API 客戶端"""

    def __init__(self, client_id: str = None, tenant_id: str = None):
        """
        初始化 Outlook 客戶端

        Args:
            client_id: Azure AD 應用程式 ID
            tenant_id: Azure AD 租戶 ID (若為 None 則使用 organizations)
        """
        try:
            import msal
        except ImportError:
            raise ImportError("請安裝 outlook 依賴: pip install worklog-helper[outlook]")

        self.client_id = client_id or "04b07795-8ddb-461a-bbee-02f9e1bf7b46"
        # 使用租戶 ID 或 organizations (適用於工作/學校帳戶)
        if tenant_id:
            self.authority = f"https://login.microsoftonline.com/{tenant_id}"
        else:
            self.authority = "https://login.microsoftonline.com/organizations"
        self._msal_app = None
        self._token_cache = None
        self._access_token = None

    def _get_msal_app(self):
        """取得 MSAL 應用程式實例"""
        import msal

        if self._msal_app is None:
            self._token_cache = msal.SerializableTokenCache()

            # 載入快取的 token
            if TOKEN_CACHE_FILE.exists():
                self._token_cache.deserialize(TOKEN_CACHE_FILE.read_text())

            self._msal_app = msal.PublicClientApplication(
                self.client_id,
                authority=self.authority,
                token_cache=self._token_cache
            )

        return self._msal_app

    def _save_token_cache(self):
        """儲存 token 快取"""
        if self._token_cache and self._token_cache.has_state_changed:
            TOKEN_CACHE_FILE.parent.mkdir(parents=True, exist_ok=True)
            TOKEN_CACHE_FILE.write_text(self._token_cache.serialize())
            TOKEN_CACHE_FILE.chmod(0o600)

    def get_access_token(self) -> Optional[str]:
        """取得存取權杖 (靜默方式，不互動)"""
        app = self._get_msal_app()

        # 嘗試從快取取得 token
        accounts = app.get_accounts()
        if accounts:
            result = app.acquire_token_silent(SCOPES, account=accounts[0])
            if result and "access_token" in result:
                self._access_token = result["access_token"]
                self._save_token_cache()
                return self._access_token

        return None

    def authenticate_device_flow(self) -> tuple[bool, str]:
        """
        使用裝置碼流程進行認證

        Returns:
            (success, message)
        """
        app = self._get_msal_app()

        # 啟動裝置碼流程
        flow = app.initiate_device_flow(scopes=SCOPES)

        if "user_code" not in flow:
            return False, f"無法啟動認證: {flow.get('error_description', 'Unknown error')}"

        # 返回認證指示
        message = flow["message"]

        # 等待使用者完成認證
        result = app.acquire_token_by_device_flow(flow)

        if "access_token" in result:
            self._access_token = result["access_token"]
            self._save_token_cache()
            return True, f"認證成功: {result.get('id_token_claims', {}).get('preferred_username', 'User')}"
        else:
            return False, f"認證失敗: {result.get('error_description', 'Unknown error')}"

    def get_device_flow_prompt(self) -> tuple[dict, str]:
        """
        取得裝置碼流程的提示訊息

        Returns:
            (flow, message) - flow 用於後續認證，message 顯示給使用者
        """
        app = self._get_msal_app()
        flow = app.initiate_device_flow(scopes=SCOPES)

        if "user_code" not in flow:
            raise Exception(f"無法啟動認證: {flow.get('error_description', 'Unknown error')}")

        return flow, flow["message"]

    def complete_device_flow(self, flow: dict) -> tuple[bool, str]:
        """
        完成裝置碼流程認證

        Args:
            flow: 從 get_device_flow_prompt 取得的 flow

        Returns:
            (success, message)
        """
        app = self._get_msal_app()
        result = app.acquire_token_by_device_flow(flow)

        if "access_token" in result:
            self._access_token = result["access_token"]
            self._save_token_cache()
            username = result.get("id_token_claims", {}).get("preferred_username", "User")
            return True, username
        else:
            return False, result.get("error_description", "Unknown error")

    def is_authenticated(self) -> bool:
        """檢查是否已認證"""
        return self.get_access_token() is not None

    def get_current_user(self) -> Optional[str]:
        """取得目前登入的使用者"""
        app = self._get_msal_app()
        accounts = app.get_accounts()
        if accounts:
            return accounts[0].get("username", "Unknown")
        return None

    def logout(self):
        """登出並清除快取"""
        if TOKEN_CACHE_FILE.exists():
            TOKEN_CACHE_FILE.unlink()
        self._msal_app = None
        self._token_cache = None
        self._access_token = None

    def get_calendar_events(
        self,
        start_date: str,
        end_date: str,
        include_meetings: bool = True,
        include_all_day: bool = True
    ) -> list[CalendarEvent]:
        """
        取得行事曆事件

        Args:
            start_date: 開始日期 (YYYY-MM-DD)
            end_date: 結束日期 (YYYY-MM-DD)
            include_meetings: 包含會議
            include_all_day: 包含全天事件 (請假)

        Returns:
            CalendarEvent 列表
        """
        import requests

        token = self.get_access_token()
        if not token:
            raise Exception("未認證，請先執行 worklog outlook-login")

        # 轉換日期格式
        start_dt = datetime.strptime(start_date, "%Y-%m-%d")
        end_dt = datetime.strptime(end_date, "%Y-%m-%d") + timedelta(days=1)

        # 呼叫 Graph API
        url = f"{GRAPH_API_ENDPOINT}/me/calendarview"
        headers = {
            "Authorization": f"Bearer {token}",
            "Prefer": 'outlook.timezone="Asia/Taipei"'
        }
        params = {
            "startDateTime": start_dt.isoformat(),
            "endDateTime": end_dt.isoformat(),
            "$select": "subject,start,end,isAllDay,categories,organizer,attendees,location",
            "$orderby": "start/dateTime",
            "$top": 100
        }

        response = requests.get(url, headers=headers, params=params)
        response.raise_for_status()

        events = []
        for item in response.json().get("value", []):
            # 解析開始和結束時間
            start_str = item["start"]["dateTime"]
            end_str = item["end"]["dateTime"]

            # 處理時區
            if "Z" in start_str:
                start = datetime.fromisoformat(start_str.replace("Z", "+00:00"))
                end = datetime.fromisoformat(end_str.replace("Z", "+00:00"))
            else:
                start = datetime.fromisoformat(start_str.split(".")[0])
                end = datetime.fromisoformat(end_str.split(".")[0])

            is_all_day = item.get("isAllDay", False)
            duration = int((end - start).total_seconds() / 60)

            # 判斷是否為會議 (有其他與會者)
            attendees = item.get("attendees", [])
            is_meeting = len(attendees) > 0

            # 過濾
            if is_all_day and not include_all_day:
                continue
            if is_meeting and not include_meetings:
                continue

            event = CalendarEvent(
                subject=item.get("subject", ""),
                start=start,
                end=end,
                is_all_day=is_all_day,
                duration_minutes=duration if not is_all_day else 480,  # 全天事件預設 8 小時
                categories=item.get("categories", []),
                is_meeting=is_meeting,
                organizer=item.get("organizer", {}).get("emailAddress", {}).get("name", ""),
                location=item.get("location", {}).get("displayName", "")
            )
            events.append(event)

        return events

    def get_events_by_date(
        self,
        start_date: str,
        end_date: str
    ) -> dict[str, list[CalendarEvent]]:
        """
        按日期分組取得事件

        Returns:
            {date: [events]}
        """
        events = self.get_calendar_events(start_date, end_date)

        by_date: dict[str, list[CalendarEvent]] = {}
        for event in events:
            date_str = event.start.strftime("%Y-%m-%d")
            if date_str not in by_date:
                by_date[date_str] = []
            by_date[date_str].append(event)

        return by_date
