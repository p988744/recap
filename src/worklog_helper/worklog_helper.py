#!/usr/bin/env python3
"""
Worklog Helper - 從 Claude Code session 自動生成 Jira Tempo worklog

工作流程：
1. 盤點：解析 Claude session，按日期/專案分組
2. 對應：用戶指定每個項目的 Jira ID
3. 上傳：確認後透過 Tempo API 上傳

支持：
- 單日模式：分析單一日期
- 週間模式：分析一週內的工作，按專案彙總每日時數
"""

import json
import sys
from datetime import datetime, timedelta
from pathlib import Path
from collections import defaultdict
from dataclasses import dataclass, field, asdict
from typing import Optional

from .config import Config, ProjectMapping
from .tempo_api import WorklogUploader, WorklogEntry


@dataclass
class WorkSession:
    """代表一個工作 session"""
    project_path: str
    project_name: str
    session_id: str
    start_time: datetime
    end_time: datetime
    duration_minutes: int
    date: str  # YYYY-MM-DD
    summary: list[str] = field(default_factory=list)
    todos: list[str] = field(default_factory=list)
    jira_id: Optional[str] = None


@dataclass
class DailyProjectEntry:
    """單日單專案的工作記錄"""
    date: str
    minutes: int
    todos: list[str]
    summaries: list[str]

    def get_description(self, project_name: str) -> str:
        """生成描述"""
        if self.todos:
            return "完成: " + ", ".join(self.todos[:3])
        if self.summaries:
            return self.summaries[0][:60]
        return f"Work on {project_name}"


@dataclass
class ProjectSummary:
    """專案的週間彙總"""
    project_name: str
    project_path: str
    total_minutes: int
    daily_entries: list[DailyProjectEntry]  # 每日記錄
    jira_id: Optional[str] = None

    @property
    def total_hours(self) -> float:
        return self.total_minutes / 60

    def get_daily_breakdown(self) -> str:
        """獲取每日明細"""
        lines = []
        for entry in sorted(self.daily_entries, key=lambda e: e.date):
            hours = entry.minutes / 60
            lines.append(f"      {entry.date}: {hours:.1f}h")
        return "\n".join(lines)


@dataclass
class WeeklyWorklog:
    """一週的工作記錄"""
    start_date: str
    end_date: str
    sessions: list[WorkSession] = field(default_factory=list)

    @property
    def total_minutes(self) -> int:
        return sum(s.duration_minutes for s in self.sessions)

    @property
    def dates_covered(self) -> list[str]:
        """涵蓋的日期列表"""
        return sorted(set(s.date for s in self.sessions))

    def get_project_summaries(self) -> list[ProjectSummary]:
        """按專案彙總，包含每日明細"""
        # 先按專案分組
        by_project: dict[str, list[WorkSession]] = defaultdict(list)
        for session in self.sessions:
            by_project[session.project_name].append(session)

        summaries = []
        for project_name, sessions in by_project.items():
            # 再按日期分組
            by_date: dict[str, list[WorkSession]] = defaultdict(list)
            for s in sessions:
                by_date[s.date].append(s)

            daily_entries = []
            for date, day_sessions in by_date.items():
                all_todos = []
                all_summaries = []
                for s in day_sessions:
                    all_todos.extend(s.todos)
                    all_summaries.extend(s.summary)

                daily_entries.append(DailyProjectEntry(
                    date=date,
                    minutes=sum(s.duration_minutes for s in day_sessions),
                    todos=list(set(all_todos))[:5],
                    summaries=all_summaries[:3]
                ))

            summaries.append(ProjectSummary(
                project_name=project_name,
                project_path=sessions[0].project_path,
                total_minutes=sum(s.duration_minutes for s in sessions),
                daily_entries=daily_entries,
                jira_id=sessions[0].jira_id
            ))

        return sorted(summaries, key=lambda s: s.total_minutes, reverse=True)


class ClaudeSessionParser:
    """解析 Claude Code session 數據"""

    def __init__(self, claude_dir: str = "~/.claude"):
        self.claude_dir = Path(claude_dir).expanduser()
        self.projects_dir = self.claude_dir / "projects"

    def get_available_dates(self) -> list[str]:
        """獲取有 session 數據的日期列表"""
        dates = set()
        for project_dir in self.projects_dir.iterdir():
            if not project_dir.is_dir():
                continue
            for session_file in project_dir.glob("*.jsonl"):
                if session_file.name.startswith("agent-"):
                    continue
                try:
                    with open(session_file, 'r') as f:
                        for line in f:
                            data = json.loads(line)
                            if 'timestamp' in data:
                                dt = datetime.fromisoformat(data['timestamp'].replace('Z', '+00:00'))
                                dates.add(dt.strftime('%Y-%m-%d'))
                                break
                except Exception:
                    continue
        return sorted(dates, reverse=True)

    def parse_date_range(self, start_date: str, end_date: str) -> WeeklyWorklog:
        """解析日期範圍內的所有 session"""
        worklog = WeeklyWorklog(start_date=start_date, end_date=end_date)
        start = datetime.strptime(start_date, '%Y-%m-%d').date()
        end = datetime.strptime(end_date, '%Y-%m-%d').date()

        for project_dir in self.projects_dir.iterdir():
            if not project_dir.is_dir():
                continue

            project_name = self._extract_project_name(project_dir.name)

            for session_file in project_dir.glob("*.jsonl"):
                if session_file.name.startswith("agent-"):
                    continue

                sessions = self._parse_session_file_range(
                    session_file, start, end, project_name
                )
                worklog.sessions.extend(sessions)

        worklog.sessions.sort(key=lambda s: s.start_time)
        return worklog

    def parse_date(self, target_date: str) -> WeeklyWorklog:
        """解析單一日期（向後兼容）"""
        return self.parse_date_range(target_date, target_date)

    def _extract_project_name(self, dir_name: str) -> str:
        """從目錄名稱提取專案名稱"""
        parts = dir_name.split('-')
        for part in reversed(parts):
            if part and part not in ['Users', 'weifanliao', 'PycharmProjects', 'Downloads']:
                return part
        return dir_name

    def _parse_session_file_range(
        self, session_file: Path, start_date, end_date, project_name: str
    ) -> list[WorkSession]:
        """解析 session 文件中指定日期範圍的記錄"""
        # 按日期分組的數據
        by_date: dict[str, dict] = defaultdict(lambda: {
            'timestamps': [],
            'todos': [],
            'messages': [],
            'project_path': ''
        })

        session_id = session_file.stem

        try:
            with open(session_file, 'r') as f:
                for line in f:
                    try:
                        data = json.loads(line)
                    except json.JSONDecodeError:
                        continue

                    if 'timestamp' not in data:
                        continue

                    ts = datetime.fromisoformat(data['timestamp'].replace('Z', '+00:00'))
                    date_key = ts.strftime('%Y-%m-%d')

                    # 檢查是否在範圍內
                    if not (start_date <= ts.date() <= end_date):
                        continue

                    day_data = by_date[date_key]
                    day_data['timestamps'].append(ts)

                    if 'cwd' in data and not day_data['project_path']:
                        day_data['project_path'] = data['cwd']

                    if data.get('type') == 'user' and 'message' in data:
                        msg = data['message']
                        if isinstance(msg, dict) and 'content' in msg:
                            content = msg['content']
                            if isinstance(content, str) and len(content) > 10:
                                summary = content[:100].replace('\n', ' ').strip()
                                if summary and not summary.startswith('⎿'):
                                    day_data['messages'].append(summary)

                    if 'toolUseResult' in data:
                        result = data['toolUseResult']
                        if 'newTodos' in result:
                            for todo in result['newTodos']:
                                if todo.get('status') == 'completed':
                                    day_data['todos'].append(todo.get('content', ''))

        except Exception as e:
            print(f"Error parsing {session_file}: {e}")
            return []

        # 轉換為 WorkSession 列表
        sessions = []
        for date_key, day_data in by_date.items():
            if not day_data['timestamps']:
                continue

            start_time = min(day_data['timestamps'])
            end_time = max(day_data['timestamps'])
            duration = int((end_time - start_time).total_seconds() / 60)

            if duration < 5:  # 至少 5 分鐘
                continue

            sessions.append(WorkSession(
                project_path=day_data['project_path'],
                project_name=project_name,
                session_id=session_id,
                start_time=start_time,
                end_time=end_time,
                duration_minutes=duration,
                date=date_key,
                summary=day_data['messages'][:5],
                todos=list(set(day_data['todos']))[:10]
            ))

        return sessions


class WorklogHelper:
    """主要的 Worklog Helper 類"""

    def __init__(self):
        self.parser = ClaudeSessionParser()
        self.config = Config.load()
        self.mapping = ProjectMapping()
        self.uploader: Optional[WorklogUploader] = None

    def list_dates(self, limit: int = 10) -> list[str]:
        """列出最近有工作記錄的日期"""
        return self.parser.get_available_dates()[:limit]

    def analyze_range(self, start_date: str, end_date: str) -> WeeklyWorklog:
        """分析日期範圍的工作"""
        return self.parser.parse_date_range(start_date, end_date)

    def analyze_date(self, date: str) -> WeeklyWorklog:
        """分析單一日期"""
        return self.parser.parse_date(date)

    def format_weekly_report(self, worklog: WeeklyWorklog) -> str:
        """格式化週間報告"""
        lines = [
            f"📅 期間: {worklog.start_date} ~ {worklog.end_date}",
            f"📆 工作天數: {len(worklog.dates_covered)} 天",
            f"⏱️  總工時: {worklog.total_minutes} 分鐘 ({worklog.total_minutes / 60:.1f} 小時)",
            f"📁 專案數: {len(worklog.get_project_summaries())}",
            "",
            "=" * 60,
            ""
        ]

        for idx, project in enumerate(worklog.get_project_summaries(), 1):
            jira_tag = f" → {project.jira_id}" if project.jira_id else ""

            lines.append(f"[{idx}] 🗂️  {project.project_name}{jira_tag}")
            lines.append(f"    總時長: {project.total_minutes} 分鐘 ({project.total_hours:.1f} 小時)")
            lines.append(f"    路徑: {project.project_path}")
            lines.append("    每日明細:")
            lines.append(project.get_daily_breakdown())

            # 收集所有 todos
            all_todos = []
            for entry in project.daily_entries:
                all_todos.extend(entry.todos)
            all_todos = list(set(all_todos))[:5]

            if all_todos:
                lines.append("    完成項目:")
                for todo in all_todos:
                    lines.append(f"      ✓ {todo}")

            lines.append("")

        return "\n".join(lines)

    def format_upload_preview(self, worklog: WeeklyWorklog) -> str:
        """格式化上傳預覽 - 按專案的每日記錄"""
        lines = [
            "",
            "📤 即將上傳的 Worklog (每日一筆):",
            "=" * 60,
            ""
        ]

        for project in worklog.get_project_summaries():
            if not project.jira_id:
                continue

            lines.append(f"  📁 {project.jira_id} ({project.project_name})")
            for entry in sorted(project.daily_entries, key=lambda e: e.date):
                hours = entry.minutes / 60
                desc = entry.get_description(project.project_name)[:40]
                lines.append(f"      {entry.date}: {hours:.1f}h - {desc}...")
            lines.append("")

        return "\n".join(lines)

    def setup_uploader(self) -> bool:
        """設置上傳器"""
        if not self.config.is_configured():
            return False

        try:
            self.uploader = WorklogUploader(
                jira_url=self.config.jira_url,
                token=self.config.get_token(),
                email=self.config.jira_email or None,
                auth_type=self.config.auth_type,
                tempo_token=self.config.tempo_api_token or None
            )
            return True
        except Exception as e:
            print(f"Failed to setup uploader: {e}")
            return False

    def upload_worklogs(self, worklog: WeeklyWorklog, use_tempo: bool = False) -> list[dict]:
        """上傳 worklogs - 每個專案的每日記錄分開上傳"""
        if not self.uploader:
            raise ValueError("Uploader not configured")

        results = []
        for project in worklog.get_project_summaries():
            if not project.jira_id:
                continue

            for entry in project.daily_entries:
                worklog_entry = WorklogEntry(
                    issue_key=project.jira_id,
                    date=entry.date,
                    time_spent_seconds=entry.minutes * 60,
                    description=entry.get_description(project.project_name)
                )

                try:
                    result = self.uploader.upload_worklog(worklog_entry, use_tempo=use_tempo)
                    results.append({
                        "issue": project.jira_id,
                        "date": entry.date,
                        "status": "success",
                        "result": result
                    })
                    print(f"  ✓ {project.jira_id} ({entry.date}) - 上傳成功")
                except Exception as e:
                    results.append({
                        "issue": project.jira_id,
                        "date": entry.date,
                        "status": "failed",
                        "error": str(e)
                    })
                    print(f"  ✗ {project.jira_id} ({entry.date}) - 上傳失敗: {e}")

        return results


def get_week_range(reference_date: str = None) -> tuple[str, str]:
    """獲取指定日期所在週的範圍 (週一到週日)"""
    if reference_date:
        ref = datetime.strptime(reference_date, '%Y-%m-%d')
    else:
        ref = datetime.now()

    # 找到週一
    monday = ref - timedelta(days=ref.weekday())
    sunday = monday + timedelta(days=6)

    return monday.strftime('%Y-%m-%d'), sunday.strftime('%Y-%m-%d')


def get_last_week_range() -> tuple[str, str]:
    """獲取上週的範圍"""
    today = datetime.now()
    last_week = today - timedelta(days=7)
    return get_week_range(last_week.strftime('%Y-%m-%d'))


def interactive_mode():
    """交互模式"""
    helper = WorklogHelper()

    print("=" * 60)
    print("  Worklog Helper - Claude Code Session → Jira Worklog")
    print("=" * 60)
    print()

    # Phase 1: 選擇時間範圍
    print("🔍 Phase 1: 選擇時間範圍\n")

    this_week = get_week_range()
    last_week = get_last_week_range()

    print("選擇模式:")
    print(f"  1. 本週 ({this_week[0]} ~ {this_week[1]})")
    print(f"  2. 上週 ({last_week[0]} ~ {last_week[1]})")
    print("  3. 自訂範圍")
    print("  4. 單日")

    choice = input("\n選擇 (預設 1): ").strip() or "1"

    if choice == "1":
        start_date, end_date = this_week
    elif choice == "2":
        start_date, end_date = last_week
    elif choice == "3":
        start_date = input("開始日期 (YYYY-MM-DD): ").strip()
        end_date = input("結束日期 (YYYY-MM-DD): ").strip()
    else:
        dates = helper.list_dates(7)
        print("\n最近有記錄的日期:")
        for i, d in enumerate(dates, 1):
            print(f"  {i}. {d}")
        day_choice = input("\n選擇日期 (數字或 YYYY-MM-DD): ").strip() or "1"
        if day_choice.isdigit():
            start_date = end_date = dates[int(day_choice) - 1]
        else:
            start_date = end_date = day_choice

    print(f"\n📊 分析期間: {start_date} ~ {end_date}\n")
    worklog = helper.analyze_range(start_date, end_date)

    if not worklog.sessions:
        print("該期間沒有工作記錄")
        return

    print(helper.format_weekly_report(worklog))

    # Phase 2: 對應 Jira ID
    print("\n🔗 Phase 2: 對應 Jira Issue\n")
    print("請為每個專案指定 Jira Issue ID (例如: PROJ-123)")
    print("直接 Enter 使用上次的 ID，輸入 '-' 跳過，'q' 取消\n")

    projects = worklog.get_project_summaries()
    for idx, project in enumerate(projects, 1):
        suggestion = helper.mapping.get(project.project_name)
        suggestion_hint = f" [{suggestion}]" if suggestion else ""

        prompt = f"[{idx}/{len(projects)}] {project.project_name} ({project.total_hours:.1f}h){suggestion_hint}: "
        jira_id = input(prompt).strip()

        if jira_id.lower() == 'q':
            print("\n已取消")
            return

        if jira_id == '':
            jira_id = suggestion

        if jira_id and jira_id != '-':
            project.jira_id = jira_id.upper()
            helper.mapping.set(project.project_name, project.jira_id)

    # 更新 sessions
    project_jira_map = {p.project_name: p.jira_id for p in projects if p.jira_id}
    for session in worklog.sessions:
        session.jira_id = project_jira_map.get(session.project_name)

    # Phase 3: 確認並上傳
    print("\n📤 Phase 3: 確認並上傳\n")

    assigned = [p for p in projects if p.jira_id]
    if not assigned:
        print("沒有任何專案被指定 Jira ID，已取消")
        return

    print(helper.format_upload_preview(worklog))

    total_entries = sum(len(p.daily_entries) for p in assigned)
    print(f"共 {total_entries} 筆 worklog 待上傳\n")

    confirm = input("確認上傳? (y/N): ").strip().lower()
    if confirm != 'y':
        print("\n已取消上傳")
        save_pending(worklog, projects)
        return

    if not helper.config.is_configured():
        print("\n⚠️  尚未配置 Jira 連接資訊")
        print("請執行: python worklog_helper.py --setup")
        save_pending(worklog, projects)
        return

    if not helper.setup_uploader():
        print("\n⚠️  無法連接到 Jira")
        save_pending(worklog, projects)
        return

    print("\n正在上傳...\n")
    results = helper.upload_worklogs(worklog, use_tempo=bool(helper.config.tempo_api_token))

    success = sum(1 for r in results if r['status'] == 'success')
    failed = sum(1 for r in results if r['status'] == 'failed')
    print(f"\n完成! 成功: {success}, 失敗: {failed}")


def save_pending(worklog: WeeklyWorklog, projects: list[ProjectSummary]):
    """保存待上傳的 worklog"""
    pending_file = Path.home() / ".worklog-helper" / "pending.json"
    pending_file.parent.mkdir(parents=True, exist_ok=True)

    data = {
        "start_date": worklog.start_date,
        "end_date": worklog.end_date,
        "projects": [
            {
                "name": p.project_name,
                "jira_id": p.jira_id,
                "daily_entries": [
                    {
                        "date": e.date,
                        "minutes": e.minutes,
                        "description": e.get_description(p.project_name)
                    }
                    for e in p.daily_entries
                ]
            }
            for p in projects if p.jira_id
        ]
    }

    with open(pending_file, 'w') as f:
        json.dump(data, f, indent=2, ensure_ascii=False)

    print(f"\n💾 已保存待上傳記錄到: {pending_file}")


def setup_config():
    """配置設定"""
    config = Config.load()

    print("=" * 60)
    print("  Worklog Helper - 配置設定")
    print("=" * 60)
    print()

    print(f"Jira URL [{config.jira_url}]: ", end="")
    url = input().strip()
    if url:
        config.jira_url = url

    print(f"Jira Email [{config.jira_email}]: ", end="")
    email = input().strip()
    if email:
        config.jira_email = email

    print("Jira API Token (輸入新值或按 Enter 保留): ", end="")
    token = input().strip()
    if token:
        config.jira_api_token = token

    print("Tempo API Token (可選，按 Enter 跳過): ", end="")
    tempo = input().strip()
    if tempo:
        config.tempo_api_token = tempo

    config.save()
    print("\n✓ 配置已保存")

    print("\n測試連接...")
    try:
        uploader = WorklogUploader(
            config.jira_url,
            config.jira_email,
            config.jira_api_token
        )
        success, msg = uploader.test_connection()
        if success:
            print(f"✓ {msg}")
        else:
            print(f"✗ {msg}")
    except Exception as e:
        print(f"✗ 連接失敗: {e}")


def main():
    """主程序入口"""
    import argparse

    parser = argparse.ArgumentParser(description="Worklog Helper - Claude Code to Jira")
    parser.add_argument("--setup", action="store_true", help="配置 Jira 連接")
    parser.add_argument("--date", type=str, help="指定單一日期 (YYYY-MM-DD)")
    parser.add_argument("--week", action="store_true", help="分析本週")
    parser.add_argument("--last-week", action="store_true", help="分析上週")
    parser.add_argument("--from", dest="from_date", type=str, help="開始日期")
    parser.add_argument("--to", dest="to_date", type=str, help="結束日期")
    parser.add_argument("--list", action="store_true", help="列出可用日期")

    args = parser.parse_args()

    if args.setup:
        setup_config()
    elif args.list:
        helper = WorklogHelper()
        dates = helper.list_dates(14)
        for d in dates:
            print(d)
    else:
        interactive_mode()


if __name__ == "__main__":
    main()
