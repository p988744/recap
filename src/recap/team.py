"""
團隊報表產生模組

支援:
- 從 Jira 群組取得團隊成員
- 取得團隊成員的 Tempo 工時記錄
- 依 Issue 類型分類統計
- 產生團隊工時報表
"""

from dataclasses import dataclass, field
from datetime import datetime
from typing import Optional, Callable

from .config import Config, TeamConfig, TeamMemberInfo
from .tempo_api import JiraClient, TempoClient


@dataclass
class WorklogEntry:
    """工時記錄項目"""
    issue_key: str
    issue_type: str          # Task, Bug, Story, etc.
    date: str                # YYYY-MM-DD
    time_spent_seconds: int
    description: str
    author_id: str
    author_name: str


@dataclass
class MemberWorklogSummary:
    """單一成員的工時彙總"""
    member: TeamMemberInfo
    total_seconds: int = 0
    entries: list[WorklogEntry] = field(default_factory=list)
    by_issue_type: dict[str, int] = field(default_factory=dict)   # issue_type -> total_seconds
    by_date: dict[str, int] = field(default_factory=dict)         # date -> total_seconds
    by_issue: dict[str, int] = field(default_factory=dict)        # issue_key -> total_seconds

    @property
    def total_hours(self) -> float:
        """總工時（小時）"""
        return self.total_seconds / 3600


@dataclass
class TeamReportData:
    """完整的團隊報表資料"""
    team_name: str
    jira_group: str
    start_date: str
    end_date: str
    generated_at: str
    members: list[MemberWorklogSummary] = field(default_factory=list)

    @property
    def total_hours(self) -> float:
        """團隊總工時"""
        return sum(m.total_seconds for m in self.members) / 3600

    @property
    def by_issue_type_total(self) -> dict[str, float]:
        """依 Issue 類型的工時統計（小時）"""
        totals: dict[str, float] = {}
        for member in self.members:
            for issue_type, seconds in member.by_issue_type.items():
                totals[issue_type] = totals.get(issue_type, 0) + seconds / 3600
        return totals

    @property
    def by_date_total(self) -> dict[str, float]:
        """依日期的工時統計（小時）"""
        totals: dict[str, float] = {}
        for member in self.members:
            for date, seconds in member.by_date.items():
                totals[date] = totals.get(date, 0) + seconds / 3600
        return totals

    def get_all_issue_types(self) -> list[str]:
        """取得所有 Issue 類型（排序）"""
        types = set()
        for member in self.members:
            types.update(member.by_issue_type.keys())
        return sorted(types)

    def get_all_dates(self) -> list[str]:
        """取得所有日期（排序）"""
        dates = set()
        for member in self.members:
            dates.update(member.by_date.keys())
        return sorted(dates)


class TeamReportGenerator:
    """團隊報表產生器"""

    def __init__(self, config: Config):
        """
        初始化報表產生器

        Args:
            config: 應用程式配置
        """
        self.config = config
        self.jira = JiraClient(
            config.jira_url,
            config.get_token(),
            config.jira_email,
            config.auth_type
        )
        # Tempo API：優先使用 Tempo Token，若無則使用 Jira PAT（適用於 Jira Server + Tempo 插件）
        tempo_token = config.tempo_api_token or config.get_token()
        self.tempo = TempoClient(config.jira_url, tempo_token)
        self._issue_type_cache: dict[str, str] = {}

    def get_team_members_from_jira_group(self, jira_group: str) -> list[TeamMemberInfo]:
        """
        從 Jira 群組取得成員列表

        Args:
            jira_group: Jira 群組名稱

        Returns:
            成員列表
        """
        members_data = self.jira.get_group_members(jira_group)
        members = []

        for m in members_data:
            # Jira Server 可能使用 'name' 或 'key' 而不是 'accountId'
            account_id = m.get("accountId") or m.get("name") or m.get("key", "")
            display_name = m.get("displayName", m.get("name", "Unknown"))
            email = m.get("emailAddress", "")

            members.append(TeamMemberInfo(
                account_id=account_id,
                display_name=display_name,
                email=email
            ))

        return members

    def get_team_members_from_tempo(self, tempo_team_id: int) -> list[TeamMemberInfo]:
        """
        從 Tempo 團隊取得成員列表

        Args:
            tempo_team_id: Tempo 團隊 ID

        Returns:
            成員列表
        """
        if not self.tempo:
            raise ValueError("需要設定 Tempo API Token")

        members_data = self.tempo.get_team_members(tempo_team_id)
        members = []

        for m in members_data:
            # Tempo API 回傳的成員結構
            member_info = m.get("member", m)
            account_id = member_info.get("key") or member_info.get("name", "")
            display_name = member_info.get("displayName", member_info.get("name", "Unknown"))
            email = member_info.get("email", "")

            members.append(TeamMemberInfo(
                account_id=account_id,
                display_name=display_name,
                email=email
            ))

        return members

    def get_team_members(self, team_info) -> list[TeamMemberInfo]:
        """
        取得團隊成員（優先使用 Tempo 團隊，其次 Jira 群組）

        Args:
            team_info: TeamInfo 物件

        Returns:
            成員列表
        """
        if team_info.tempo_team_id:
            return self.get_team_members_from_tempo(team_info.tempo_team_id)
        elif team_info.jira_group:
            return self.get_team_members_from_jira_group(team_info.jira_group)
        else:
            raise ValueError("團隊未設定 Tempo 團隊 ID 或 Jira 群組")

    def generate_report(
        self,
        team_name: str,
        start_date: str,
        end_date: str,
        progress_callback: Optional[Callable[[str, int, int], None]] = None
    ) -> TeamReportData:
        """
        產生團隊工時報表

        Args:
            team_name: 團隊名稱
            start_date: 開始日期 (YYYY-MM-DD)
            end_date: 結束日期 (YYYY-MM-DD)
            progress_callback: 進度回調 (member_name, current, total)

        Returns:
            團隊報表資料
        """
        team_config = TeamConfig()
        team_info = team_config.get_team(team_name)

        if not team_info:
            raise ValueError(f"找不到團隊 '{team_name}'")

        if not self.tempo:
            raise ValueError("需要設定 Tempo API Token 才能使用團隊報表功能")

        # 取得成員列表
        members = self.get_team_members(team_info)

        # 更新成員快取
        team_config.update_members(
            team_name,
            members,
            datetime.now().isoformat()
        )

        # 收集所有 issue keys 用於批次查詢類型
        all_issue_keys: set[str] = set()
        member_worklogs: dict[str, list[dict]] = {}

        # 取得每個成員的 worklogs
        total_members = len(members)
        for i, member in enumerate(members):
            if progress_callback:
                progress_callback(member.display_name, i + 1, total_members)

            try:
                worklogs = self.tempo.get_worklogs_for_user(
                    member.account_id,
                    start_date,
                    end_date
                )
                member_worklogs[member.account_id] = worklogs

                # 收集 issue keys
                for wl in worklogs:
                    issue_key = wl.get("issue", {}).get("key")
                    if issue_key:
                        all_issue_keys.add(issue_key)
            except Exception:
                # 如果取得某成員的 worklog 失敗，繼續處理其他成員
                member_worklogs[member.account_id] = []

        # 批次取得 issue 類型
        issue_types = self.jira.batch_get_issue_types(list(all_issue_keys))
        self._issue_type_cache.update(issue_types)

        # 處理每個成員的資料
        member_summaries = []
        for member in members:
            worklogs = member_worklogs.get(member.account_id, [])
            summary = self._process_member_worklogs(member, worklogs)
            member_summaries.append(summary)

        # 依工時排序（多到少）
        member_summaries.sort(key=lambda x: x.total_seconds, reverse=True)

        return TeamReportData(
            team_name=team_name,
            jira_group=team_info.jira_group,
            start_date=start_date,
            end_date=end_date,
            generated_at=datetime.now().isoformat(),
            members=member_summaries
        )

    def _process_member_worklogs(
        self,
        member: TeamMemberInfo,
        worklogs: list[dict]
    ) -> MemberWorklogSummary:
        """處理單一成員的 worklogs"""
        summary = MemberWorklogSummary(member=member)

        for wl in worklogs:
            issue_key = wl.get("issue", {}).get("key", "Unknown")
            issue_type = self._issue_type_cache.get(issue_key, "Unknown")
            date = wl.get("startDate", wl.get("dateStarted", ""))[:10]
            seconds = wl.get("timeSpentSeconds", 0)
            description = wl.get("description", "")

            entry = WorklogEntry(
                issue_key=issue_key,
                issue_type=issue_type,
                date=date,
                time_spent_seconds=seconds,
                description=description,
                author_id=member.account_id,
                author_name=member.display_name
            )

            summary.entries.append(entry)
            summary.total_seconds += seconds

            # 依 issue 類型統計
            summary.by_issue_type[issue_type] = (
                summary.by_issue_type.get(issue_type, 0) + seconds
            )

            # 依日期統計
            summary.by_date[date] = summary.by_date.get(date, 0) + seconds

            # 依 issue 統計
            summary.by_issue[issue_key] = (
                summary.by_issue.get(issue_key, 0) + seconds
            )

        return summary
