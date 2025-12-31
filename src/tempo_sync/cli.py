#!/usr/bin/env python3
"""
Tempo Sync CLI - 現代化的命令列介面

使用 Typer + Rich 提供美觀的互動體驗
"""

from datetime import datetime, timedelta
from typing import Optional

import typer
from rich.console import Console
from rich.panel import Panel
from rich.prompt import Confirm, Prompt
from rich.table import Table
from rich.progress import Progress, SpinnerColumn, TextColumn

from .session_parser import WorklogHelper, WeeklyWorklog, ProjectSummary
from .config import Config
from .tempo_api import WorklogUploader

app = typer.Typer(
    name="tempo",
    help="同步開發活動到 Jira Tempo worklog",
    no_args_is_help=False,
)
console = Console()


def get_week_range(reference_date: Optional[str] = None) -> tuple[str, str]:
    """獲取指定日期所在週的範圍 (週一到週日)"""
    if reference_date:
        ref = datetime.strptime(reference_date, '%Y-%m-%d')
    else:
        ref = datetime.now()
    monday = ref - timedelta(days=ref.weekday())
    sunday = monday + timedelta(days=6)
    return monday.strftime('%Y-%m-%d'), sunday.strftime('%Y-%m-%d')


def get_last_week_range() -> tuple[str, str]:
    """獲取上週的範圍"""
    today = datetime.now()
    last_week = today - timedelta(days=7)
    return get_week_range(last_week.strftime('%Y-%m-%d'))


def get_outlook_events(helper, start_date: str, end_date: str) -> dict[str, list]:
    """取得 Outlook 行事曆事件"""
    config = helper.config
    if not config.outlook_enabled:
        return {}

    try:
        from .outlook_helper import OutlookClient
        client = OutlookClient(client_id=config.outlook_client_id, tenant_id=config.outlook_tenant_id)
        if not client.is_authenticated():
            return {}

        return client.get_events_by_date(start_date, end_date)
    except ImportError:
        return {}
    except Exception as e:
        console.print(f"[yellow]⚠ 無法取得 Outlook 行事曆: {e}[/yellow]")
        return {}


def normalize_daily_hours(entries: list[dict], daily_hours: float = 8.0) -> list[dict]:
    """
    將每日工時正規化為標準工時。

    例如：一天有 3 個任務分別花了 2h, 3h, 1h（共 6h），
    正規化到 8h 後變成：2.67h, 4h, 1.33h

    Args:
        entries: 要上傳的 entries 列表
        daily_hours: 每日標準工時（預設 8 小時）

    Returns:
        正規化後的 entries 列表（新增 normalized_minutes 欄位）
    """
    daily_minutes = daily_hours * 60

    # 按日期分組
    by_date: dict[str, list[dict]] = {}
    for e in entries:
        if e.get('entry'):
            date = e['entry'].date
            original_minutes = e['entry'].minutes
        else:
            date = e['date']
            original_minutes = e['minutes']

        e['original_minutes'] = original_minutes

        if date not in by_date:
            by_date[date] = []
        by_date[date].append(e)

    # 對每天進行正規化
    for date, day_entries in by_date.items():
        # 計算當天總分鐘數
        total_minutes = sum(e['original_minutes'] for e in day_entries)

        if total_minutes == 0:
            continue

        # 按比例分配標準工時
        for e in day_entries:
            ratio = e['original_minutes'] / total_minutes
            e['normalized_minutes'] = int(ratio * daily_minutes)

    return entries


def summarize_worklog_entries(worklog: WeeklyWorklog, helper, outlook_events: dict = None) -> dict[str, str]:
    """使用 LLM 彙整所有工作項目，返回 {(date, project_name): description}"""
    from .llm_helper import summarize_work

    config = helper.config
    if not config.has_llm_config():
        return {}

    llm_config = config.get_llm_config()
    summaries = {}

    # 收集所有項目
    all_entries = []
    for project in worklog.get_project_summaries():
        for entry in project.daily_entries:
            all_entries.append((project.project_name, entry))

    if not all_entries:
        return {}

    console.print(f"[dim]使用 {llm_config.provider} 彙整 {len(all_entries)} 個項目...[/dim]")

    for project_name, entry in all_entries:
        key = f"{entry.date}|{project_name}"
        result = summarize_work(
            project_name=project_name,
            todos=entry.todos,
            summaries=entry.summaries,
            config=llm_config
        )
        summaries[key] = result.description

    console.print(f"[green]✓ 彙整完成[/green]\n")
    return summaries


def display_worklog_table(worklog: WeeklyWorklog, summaries: dict[str, str] = None, outlook_events: dict[str, list] = None):
    """使用 Rich 顯示每日工作流水帳"""
    console.print(f"[bold]📊 工作記錄 ({worklog.start_date} ~ {worklog.end_date})[/bold]\n")

    # 收集每日資料
    daily_data: dict[str, list[tuple]] = {}  # date -> [(project_name, entry), ...]
    for project in worklog.get_project_summaries():
        for entry in project.daily_entries:
            if entry.date not in daily_data:
                daily_data[entry.date] = []
            daily_data[entry.date].append(("code", project.project_name, entry))

    # 加入 Outlook 事件
    if outlook_events:
        for date, events in outlook_events.items():
            if date not in daily_data:
                daily_data[date] = []
            for event in events:
                daily_data[date].append(("outlook", event.subject, event))

    # 按日期排序顯示
    for date in sorted(daily_data.keys(), reverse=True):
        entries = daily_data[date]

        # 計算當天總時數
        day_total = 0
        for item in entries:
            if item[0] == "code":
                day_total += item[2].minutes / 60
            else:  # outlook
                day_total += item[2].duration_minutes / 60

        # 日期標題
        console.print(f"[bold cyan]{date}[/bold cyan] [dim]({day_total:.1f}h)[/dim]")

        for item in entries:
            if item[0] == "code":
                _, project_name, entry = item
                hours = entry.minutes / 60
                console.print(f"  [magenta]{hours:.1f}h[/magenta] [white]{project_name}[/white]")

                # 顯示 LLM 彙整的描述或原始項目
                key = f"{entry.date}|{project_name}"
                if summaries and key in summaries:
                    console.print(f"       [yellow]→[/yellow] {summaries[key]}")
                elif entry.todos:
                    for todo in entry.todos[:3]:
                        console.print(f"       [green]✓[/green] [dim]{todo}[/dim]")
                    if len(entry.todos) > 3:
                        console.print(f"       [dim]...還有 {len(entry.todos) - 3} 項[/dim]")
                elif entry.summaries:
                    console.print(f"       [dim]{entry.summaries[0][:50]}...[/dim]")
            else:
                # Outlook 事件
                _, subject, event = item
                hours = event.duration_minutes / 60
                if event.is_leave:
                    console.print(f"  [magenta]{hours:.1f}h[/magenta] [red]📅 {subject}[/red] [dim](請假)[/dim]")
                elif event.is_meeting:
                    console.print(f"  [magenta]{hours:.1f}h[/magenta] [blue]📅 {subject}[/blue] [dim](會議)[/dim]")
                else:
                    console.print(f"  [magenta]{hours:.1f}h[/magenta] [blue]📅 {subject}[/blue]")

        console.print()

    # 總計
    total_hours = worklog.total_minutes / 60
    outlook_hours = 0
    if outlook_events:
        for events in outlook_events.values():
            outlook_hours += sum(e.duration_minutes for e in events) / 60

    console.print(f"[bold]總計:[/bold] {total_hours + outlook_hours:.1f} 小時 | "
                  f"{len(worklog.get_project_summaries())} 個專案 | "
                  f"{len(daily_data)} 天")
    if outlook_hours > 0:
        console.print(f"       [dim](Code: {total_hours:.1f}h + Outlook: {outlook_hours:.1f}h)[/dim]")


@app.command()
def analyze(
    week: bool = typer.Option(False, "--week", "-w", help="分析本週"),
    last_week: bool = typer.Option(False, "--last-week", "-l", help="分析上週"),
    days: Optional[int] = typer.Option(None, "--days", "-n", help="分析過去 N 天"),
    date: Optional[str] = typer.Option(None, "--date", "-d", help="指定日期 (YYYY-MM-DD)"),
    from_date: Optional[str] = typer.Option(None, "--from", help="開始日期"),
    to_date: Optional[str] = typer.Option(None, "--to", help="結束日期"),
    upload: bool = typer.Option(False, "--upload", "-u", help="分析後直接進入上傳流程"),
):
    """
    分析 Claude Code session 並生成工作報告

    預設進入互動模式，可以選擇時間範圍
    """
    helper = WorklogHelper()

    # 決定時間範圍
    if week:
        start, end = get_week_range()
    elif last_week:
        start, end = get_last_week_range()
    elif days:
        today = datetime.now().strftime('%Y-%m-%d')
        start = (datetime.now() - timedelta(days=days-1)).strftime('%Y-%m-%d')
        end = today
    elif date:
        start = end = date
    elif from_date and to_date:
        start, end = from_date, to_date
    else:
        # 互動模式
        start, end = interactive_date_selection(helper)

    if not start:
        return

    # 分析
    with Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        console=console,
    ) as progress:
        progress.add_task("正在分析 Claude Code sessions...", total=None)
        worklog = helper.analyze_range(start, end)

    if not worklog.sessions:
        console.print("[yellow]該期間沒有工作記錄[/yellow]")
        return

    # LLM 彙整
    summaries = summarize_worklog_entries(worklog, helper)

    display_worklog_table(worklog, summaries)

    if upload:
        console.print()
        interactive_upload(helper, worklog, summaries)


def interactive_date_selection(helper: WorklogHelper, show_header: bool = True) -> tuple[str, str]:
    """互動式日期選擇"""
    if show_header:
        console.print(Panel.fit(
            "[bold]Tempo Sync[/bold]\n"
            "同步開發活動到 Jira Tempo",
            title="🕐",
        ))

    this_week = get_week_range()
    last_week = get_last_week_range()
    two_weeks_ago = get_week_range((datetime.now() - timedelta(days=14)).strftime('%Y-%m-%d'))

    today = datetime.now().strftime('%Y-%m-%d')
    past_3 = (datetime.now() - timedelta(days=2)).strftime('%Y-%m-%d')
    past_7 = (datetime.now() - timedelta(days=6)).strftime('%Y-%m-%d')
    past_14 = (datetime.now() - timedelta(days=13)).strftime('%Y-%m-%d')

    console.print("\n選擇時間範圍:")
    console.print(f"  [cyan]1.[/cyan] 過去 7 天 ({past_7} ~ {today})")
    console.print(f"  [cyan]2.[/cyan] 過去 3 天 ({past_3} ~ {today})")
    console.print(f"  [cyan]3.[/cyan] 過去 14 天 ({past_14} ~ {today})")
    console.print(f"  [cyan]4.[/cyan] 過去 N 天")
    console.print(f"  [cyan]5.[/cyan] 本週 ({this_week[0]} ~ {this_week[1]})")
    console.print(f"  [cyan]6.[/cyan] 上週 ({last_week[0]} ~ {last_week[1]})")
    console.print(f"  [cyan]7.[/cyan] 兩週前 ({two_weeks_ago[0]} ~ {two_weeks_ago[1]})")
    console.print(f"  [cyan]8.[/cyan] 自訂範圍")
    console.print(f"  [cyan]9.[/cyan] 單日")
    console.print(f"  [cyan]q.[/cyan] 離開")

    choice = Prompt.ask("\n選擇", default="1")

    if choice.lower() == 'q':
        return "", ""
    elif choice == "1":
        return past_7, today
    elif choice == "2":
        return past_3, today
    elif choice == "3":
        return past_14, today
    elif choice == "4":
        n = Prompt.ask("天數", default="7")
        try:
            days = int(n)
            start = (datetime.now() - timedelta(days=days-1)).strftime('%Y-%m-%d')
            return start, today
        except ValueError:
            console.print("[red]無效的天數[/red]")
            return "", ""
    elif choice == "5":
        return this_week
    elif choice == "6":
        return last_week
    elif choice == "7":
        return two_weeks_ago
    elif choice == "8":
        start = Prompt.ask("開始日期 (YYYY-MM-DD)")
        end = Prompt.ask("結束日期 (YYYY-MM-DD)")
        return start, end
    else:
        dates = helper.list_dates(14)
        if not dates:
            console.print("[yellow]找不到任何 session 數據[/yellow]")
            return "", ""

        console.print("\n最近有記錄的日期:")
        for i, d in enumerate(dates, 1):
            console.print(f"  [cyan]{i}.[/cyan] {d}")

        day_choice = Prompt.ask("選擇日期", default="1")
        if day_choice.isdigit() and 1 <= int(day_choice) <= len(dates):
            selected = dates[int(day_choice) - 1]
            return selected, selected
        return day_choice, day_choice


def get_outlook_status(config) -> tuple[bool, str]:
    """取得 Outlook 連接狀態"""
    if not config.outlook_enabled:
        return False, "[dim]未啟用[/dim] [dim](tempo outlook-login)[/dim]"

    try:
        from .outlook_helper import OutlookClient
        client = OutlookClient(client_id=config.outlook_client_id, tenant_id=config.outlook_tenant_id)
        user = client.get_current_user()
        if user:
            return True, f"[green]✓[/green] {user}"
        else:
            return False, "[yellow]需重新登入[/yellow] [dim](tempo outlook-login)[/dim]"
    except ImportError:
        return False, "[dim]未安裝[/dim] [dim](pip install tempo-sync[outlook])[/dim]"
    except Exception:
        return False, "[yellow]需重新登入[/yellow]"


def display_config_status(helper: WorklogHelper):
    """顯示配置狀態"""
    config = helper.config

    # Jira 狀態
    if config.is_configured():
        auth_info = "PAT" if config.auth_type == "pat" else config.jira_email
        jira_status = f"[green]✓[/green] {config.jira_url} [dim]({auth_info})[/dim]"
    else:
        jira_status = "[red]✗ 未配置[/red] [dim](tempo setup)[/dim]"

    # LLM 狀態
    if config.has_llm_config():
        llm_status = f"[green]✓[/green] {config.llm_provider} ({config.llm_model or '預設'})"
    else:
        llm_status = "[dim]未配置[/dim] [dim](tempo setup-llm)[/dim]"

    # Outlook 狀態
    _, outlook_status = get_outlook_status(config)

    console.print(f"  Jira:    {jira_status}")
    console.print(f"  LLM:     {llm_status}")
    console.print(f"  Outlook: {outlook_status}")


def interactive_main_loop(helper: WorklogHelper):
    """主要互動循環 - 分析、調整、上傳"""
    console.print(Panel.fit(
        "[bold]Tempo Sync[/bold]\n"
        "同步開發活動到 Jira Tempo",
        title="🕐",
    ))
    display_config_status(helper)

    while True:
        # 選擇時間範圍
        start, end = interactive_date_selection(helper, show_header=False)
        if not start:
            return

        # 分析
        with Progress(
            SpinnerColumn(),
            TextColumn("[progress.description]{task.description}"),
            console=console,
        ) as progress:
            progress.add_task("正在分析 Claude Code sessions...", total=None)
            worklog = helper.analyze_range(start, end)

        # 取得 Outlook 事件
        outlook_events = get_outlook_events(helper, start, end)

        if not worklog.sessions and not outlook_events:
            console.print("[yellow]該期間沒有工作記錄[/yellow]")
            continue

        # LLM 彙整
        console.print()
        summaries = summarize_worklog_entries(worklog, helper)

        # 顯示結果
        display_worklog_table(worklog, summaries, outlook_events)

        # 詢問下一步
        console.print("\n[bold]下一步:[/bold]")
        console.print("  [cyan]1.[/cyan] 繼續 → 對應 Jira Issue 並上傳")
        console.print("  [cyan]2.[/cyan] 重新選擇時間範圍")
        console.print("  [cyan]q.[/cyan] 離開")

        next_action = Prompt.ask("\n選擇", default="1")

        if next_action.lower() == 'q':
            console.print("[dim]已離開[/dim]")
            return
        elif next_action == "2":
            console.print()
            continue
        else:
            # 進入上傳流程，傳遞已彙整的描述和 Outlook 事件
            console.print()
            interactive_upload(helper, worklog, summaries, outlook_events)
            return


def select_llm_provider(helper: WorklogHelper = None):
    """互動式選擇 LLM 提供者，優先使用已儲存的配置"""
    import os
    from .llm_helper import LLMConfig, test_llm_connection

    # 優先使用已儲存的配置
    saved_config = helper.config if helper else Config.load()
    if saved_config.has_llm_config():
        llm_config = saved_config.get_llm_config()
        console.print(f"[dim]使用已儲存的 LLM 配置: {llm_config.provider} ({llm_config.get_model()})[/dim]")

        # 測試連接
        success, msg = test_llm_connection(llm_config)
        if success:
            console.print(f"[green]✓ {msg}[/green]")
            return llm_config
        else:
            console.print(f"[yellow]⚠ {msg}[/yellow]")
            if not Confirm.ask("已儲存的配置連接失敗，要重新選擇嗎?", default=True):
                return None

    # 偵測可用的提供者
    available = []

    # Ollama (本地，通常可用)
    available.append(("1", "Ollama (本地)", "ollama", "", ""))

    # OpenAI Compatible
    if os.environ.get("OPENAI_BASE_URL"):
        available.append(("2", f"OpenAI Compatible ({os.environ.get('OPENAI_BASE_URL')})", "openai-compatible", "", os.environ.get("OPENAI_BASE_URL")))
    else:
        available.append(("2", "OpenAI Compatible (需設定 OPENAI_BASE_URL)", "openai-compatible", "", ""))

    # Cloud providers
    if os.environ.get("ANTHROPIC_API_KEY"):
        available.append(("3", "Anthropic Claude ✓", "anthropic", os.environ.get("ANTHROPIC_API_KEY"), ""))
    else:
        available.append(("3", "Anthropic Claude (需設定 ANTHROPIC_API_KEY)", "anthropic", "", ""))

    if os.environ.get("OPENAI_API_KEY"):
        available.append(("4", "OpenAI GPT ✓", "openai", os.environ.get("OPENAI_API_KEY"), ""))
    else:
        available.append(("4", "OpenAI GPT (需設定 OPENAI_API_KEY)", "openai", "", ""))

    if os.environ.get("GOOGLE_API_KEY") or os.environ.get("GEMINI_API_KEY"):
        available.append(("5", "Google Gemini ✓", "gemini", os.environ.get("GOOGLE_API_KEY") or os.environ.get("GEMINI_API_KEY"), ""))
    else:
        available.append(("5", "Google Gemini (需設定 GOOGLE_API_KEY)", "gemini", "", ""))

    console.print("\n選擇 LLM 提供者:")
    for num, name, _, _, _ in available:
        console.print(f"  [cyan]{num}.[/cyan] {name}")
    console.print("  [cyan]q.[/cyan] 取消")

    choice = Prompt.ask("\n選擇", default="1")

    if choice.lower() == 'q':
        return None

    # 找到選擇的提供者
    selected = None
    for num, name, provider, api_key, base_url in available:
        if num == choice:
            selected = (provider, api_key, base_url)
            break

    if not selected:
        return None

    provider, api_key, base_url = selected

    # 建立配置
    config = LLMConfig(
        provider=provider,
        api_key=api_key,
        model=os.environ.get("LLM_MODEL", ""),
        ollama_host=os.environ.get("OLLAMA_HOST", "http://localhost:11434"),
        openai_base_url=base_url
    )

    # 測試連接
    console.print(f"[dim]測試 {provider} 連接...[/dim]")
    success, msg = test_llm_connection(config)

    if success:
        console.print(f"[green]✓ {msg}[/green]")
        return config
    else:
        console.print(f"[red]✗ {msg}[/red]")
        if not Confirm.ask("連接失敗，仍要繼續?", default=False):
            return None
        return config


def display_upload_preview_entries(entries: list[dict], show_normalized: bool = False):
    """顯示上傳預覽 (entries 格式)"""
    table = Table(title="📤 即將上傳的 Worklog")

    table.add_column("Jira Issue", style="cyan")
    table.add_column("來源", style="dim")
    table.add_column("日期", style="green")
    if show_normalized:
        table.add_column("原始", style="dim", justify="right")
        table.add_column("上傳", style="magenta", justify="right")
    else:
        table.add_column("時數", style="magenta", justify="right")
    table.add_column("描述", style="dim", max_width=40)

    total_original = 0
    total_normalized = 0

    for e in entries:
        if e.get('entry'):
            # Code entry
            original_minutes = e.get('original_minutes', e['entry'].minutes)
            upload_minutes = e.get('normalized_minutes', e['entry'].minutes)
            date = e['entry'].date
            source = e['project'].project_name
        else:
            # Outlook event
            original_minutes = e.get('original_minutes', e['minutes'])
            upload_minutes = e.get('normalized_minutes', e['minutes'])
            date = e['date']
            source = "📅 Outlook"

        total_original += original_minutes
        total_normalized += upload_minutes

        desc = (e['description'][:40] + "...") if len(e['description']) > 40 else e['description']

        if show_normalized:
            table.add_row(
                e['jira_id'],
                source,
                date,
                f"{original_minutes/60:.1f}h",
                f"{upload_minutes/60:.1f}h",
                desc
            )
        else:
            table.add_row(
                e['jira_id'],
                source,
                date,
                f"{upload_minutes/60:.1f}h",
                desc
            )

    console.print(table)

    if show_normalized:
        console.print(f"\n[bold]共 {len(entries)} 筆 worklog[/bold] "
                      f"[dim](原始 {total_original/60:.1f}h → 上傳 {total_normalized/60:.1f}h)[/dim]")
    else:
        console.print(f"\n[bold]共 {len(entries)} 筆 worklog 待上傳[/bold]")


def upload_entries(helper: WorklogHelper, entries: list[dict]):
    """上傳 entries 格式的 worklog"""
    from .tempo_api import WorklogEntry

    # 檢查配置
    if not helper.config.is_configured():
        console.print("\n[red]⚠️ 尚未配置 Jira 連接資訊[/red]")
        console.print("請執行: [cyan]tempo setup[/cyan]")
        return

    if not helper.setup_uploader():
        console.print("\n[red]⚠️ 無法連接到 Jira[/red]")
        return

    console.print("\n正在上傳...\n")

    success = 0
    failed = 0

    for e in entries:
        if e.get('entry'):
            # Code entry
            date = e['entry'].date
            original_minutes = e['entry'].minutes
        else:
            # Outlook event
            date = e['date']
            original_minutes = e['minutes']

        # 使用正規化後的時間（如果有的話）
        minutes = e.get('normalized_minutes', original_minutes)

        worklog_entry = WorklogEntry(
            issue_key=e['jira_id'],
            date=date,
            time_spent_seconds=minutes * 60,
            description=e['description']
        )

        try:
            helper.uploader.upload_worklog(worklog_entry, use_tempo=bool(helper.config.tempo_api_token))
            console.print(f"  [green]✓[/green] {e['jira_id']} ({date})")
            success += 1
        except Exception as ex:
            console.print(f"  [red]✗[/red] {e['jira_id']} ({date}) - {ex}")
            failed += 1

    if failed == 0:
        console.print(f"\n[green]✓ 完成! 成功上傳 {success} 筆[/green]")
    else:
        console.print(f"\n[yellow]完成! 成功: {success}, 失敗: {failed}[/yellow]")


def interactive_upload(helper: WorklogHelper, worklog: WeeklyWorklog, summaries: dict[str, str] = None, outlook_events: dict[str, list] = None):
    """互動式上傳流程 - 每日每個項目為單位對應 Issue"""
    interactive_upload_by_day(helper, worklog, summaries, outlook_events)


def interactive_upload_by_day(helper: WorklogHelper, worklog: WeeklyWorklog, summaries: dict[str, str] = None, outlook_events: dict[str, list] = None):
    """按日期逐一對應 Issue"""
    console.print("[bold]🔗 對應 Jira Issue[/bold]")
    console.print("[dim]直接 Enter 使用建議 ID，輸入 '-' 跳過該項目，'q' 取消[/dim]")

    # 建立所有日期項目的列表
    entries_to_upload = []
    projects = worklog.get_project_summaries()

    # 按日期分組 (type, name, entry/event)
    daily_data: dict[str, list[tuple]] = {}
    for project in projects:
        for entry in project.daily_entries:
            if entry.date not in daily_data:
                daily_data[entry.date] = []
            daily_data[entry.date].append(("code", project, entry))

    # 加入 Outlook 事件
    if outlook_events:
        for date, events in outlook_events.items():
            if date not in daily_data:
                daily_data[date] = []
            for event in events:
                daily_data[date].append(("outlook", None, event))

    # 計算總數
    total = sum(len(entries) for entries in daily_data.values())
    idx = 0

    # 按日期排序（由近到遠）
    for date in sorted(daily_data.keys(), reverse=True):
        entries = daily_data[date]

        # 計算當天總時數
        day_total = 0
        for item in entries:
            if item[0] == "code":
                day_total += item[2].minutes / 60
            else:
                day_total += item[2].duration_minutes / 60

        # 日期標題
        console.print(f"\n[bold cyan]── {date} [/bold cyan][dim]({day_total:.1f}h)[/dim]")

        for item in entries:
            idx += 1

            if item[0] == "code":
                _, project, entry = item
                hours = entry.minutes / 60

                # 取得建議
                suggestion = helper.mapping.get(project.project_name)

                # 顯示項目
                console.print(f"  [{idx}/{total}] [magenta]{hours:.1f}h[/magenta] [white]{project.project_name}[/white]")

                # 顯示 LLM 彙整的描述
                key = f"{entry.date}|{project.project_name}"
                if summaries and key in summaries:
                    console.print(f"         [yellow]→[/yellow] {summaries[key]}")
                elif entry.todos:
                    for todo in entry.todos[:2]:
                        console.print(f"         [green]✓[/green] [dim]{todo}[/dim]")
                elif entry.summaries:
                    console.print(f"         [dim]{entry.summaries[0][:50]}...[/dim]")

                suggestion_hint = f" [{suggestion}]" if suggestion else ""
                jira_id = Prompt.ask(f"         Issue{suggestion_hint}", default=suggestion or "")

                if jira_id.lower() == 'q':
                    console.print("[yellow]已取消[/yellow]")
                    return

                if jira_id and jira_id != '-':
                    jira_id = jira_id.upper()
                    # 使用 LLM 彙整的描述，若無則用原始描述
                    description = summaries.get(key) if summaries else None
                    if not description:
                        description = entry.get_description(project.project_name)

                    entries_to_upload.append({
                        'project': project,
                        'entry': entry,
                        'jira_id': jira_id,
                        'description': description
                    })
                    # 記住這個專案的最後使用 Issue
                    helper.mapping.set(project.project_name, jira_id)

            else:
                # Outlook 事件
                _, _, event = item
                hours = event.duration_minutes / 60

                # 顯示事件類型
                if event.is_leave:
                    console.print(f"  [{idx}/{total}] [magenta]{hours:.1f}h[/magenta] [red]📅 {event.subject}[/red] [dim](請假)[/dim]")
                elif event.is_meeting:
                    console.print(f"  [{idx}/{total}] [magenta]{hours:.1f}h[/magenta] [blue]📅 {event.subject}[/blue] [dim](會議)[/dim]")
                else:
                    console.print(f"  [{idx}/{total}] [magenta]{hours:.1f}h[/magenta] [blue]📅 {event.subject}[/blue]")

                # Outlook 事件的建議 (使用 subject 作為 key)
                suggestion = helper.mapping.get(f"outlook:{event.subject}")
                suggestion_hint = f" [{suggestion}]" if suggestion else ""
                jira_id = Prompt.ask(f"         Issue{suggestion_hint}", default=suggestion or "")

                if jira_id.lower() == 'q':
                    console.print("[yellow]已取消[/yellow]")
                    return

                if jira_id and jira_id != '-':
                    jira_id = jira_id.upper()
                    entries_to_upload.append({
                        'project': None,
                        'entry': None,
                        'event': event,
                        'jira_id': jira_id,
                        'description': event.get_description(),
                        'date': date,
                        'minutes': event.duration_minutes
                    })
                    # 記住這個事件的 Issue
                    helper.mapping.set(f"outlook:{event.subject}", jira_id)

    if not entries_to_upload:
        console.print("[yellow]沒有任何項目被指定 Jira ID，已取消[/yellow]")
        return

    # 正規化工時
    if helper.config.normalize_hours:
        entries_to_upload = normalize_daily_hours(
            entries_to_upload,
            helper.config.daily_work_hours
        )
        console.print(f"\n[dim]📊 工時已正規化為每日 {helper.config.daily_work_hours:.0f} 小時[/dim]")

    # 預覽
    display_upload_preview_entries(entries_to_upload, show_normalized=helper.config.normalize_hours)

    if not Confirm.ask("\n確認上傳?", default=False):
        console.print("[yellow]已取消上傳[/yellow]")
        return

    # 上傳
    upload_entries(helper, entries_to_upload)


@app.command()
def setup():
    """配置 Jira 連接資訊"""
    console.print(Panel.fit(
        "[bold]Jira 連接配置[/bold]",
        title="⚙️",
    ))

    config = Config.load()

    config.jira_url = Prompt.ask("Jira URL", default=config.jira_url)

    # 選擇認證方式
    console.print("\n認證方式:")
    console.print("  [cyan]1.[/cyan] PAT (Personal Access Token) - Jira Server")
    console.print("  [cyan]2.[/cyan] Basic Auth (Email + API Token) - Jira Cloud")

    auth_choice = Prompt.ask("選擇", default="1")

    if auth_choice == "2":
        config.auth_type = "basic"
        config.jira_email = Prompt.ask("Jira Email", default=config.jira_email)
        new_token = Prompt.ask("Jira API Token", password=True, default="")
        if new_token:
            config.jira_api_token = new_token
    else:
        config.auth_type = "pat"
        new_pat = Prompt.ask("Jira PAT", password=True, default="")
        if new_pat:
            config.jira_pat = new_pat

    tempo_token = Prompt.ask("Tempo API Token (可選，直接 Enter 跳過)", password=True, default="")
    if tempo_token:
        config.tempo_api_token = tempo_token

    config.save()
    console.print("\n[green]✓ 配置已保存[/green]")

    # 測試連接
    console.print("\n測試連接...")
    try:
        uploader = WorklogUploader(
            jira_url=config.jira_url,
            token=config.get_token(),
            email=config.jira_email or None,
            auth_type=config.auth_type
        )
        success, msg = uploader.test_connection()
        if success:
            console.print(f"[green]✓ {msg}[/green]")
        else:
            console.print(f"[red]✗ {msg}[/red]")
    except Exception as e:
        console.print(f"[red]✗ 連接失敗: {e}[/red]")


@app.command("setup-llm")
def setup_llm():
    """配置 LLM 設定"""
    from .llm_helper import LLMConfig, test_llm_connection

    console.print(Panel.fit(
        "[bold]LLM 配置[/bold]\n"
        "用於彙整工作描述",
        title="🤖",
    ))

    config = Config.load()

    console.print("\n選擇 LLM 提供者:")
    console.print("  [cyan]1.[/cyan] Ollama (本地)")
    console.print("  [cyan]2.[/cyan] OpenAI Compatible (vLLM, LMStudio 等)")
    console.print("  [cyan]3.[/cyan] Anthropic Claude")
    console.print("  [cyan]4.[/cyan] OpenAI GPT")
    console.print("  [cyan]5.[/cyan] Google Gemini")

    provider_map = {
        "1": "ollama",
        "2": "openai-compatible",
        "3": "anthropic",
        "4": "openai",
        "5": "gemini",
    }

    choice = Prompt.ask("\n選擇", default="2")
    provider = provider_map.get(choice, "ollama")
    config.llm_provider = provider

    if provider == "openai-compatible":
        config.llm_base_url = Prompt.ask("API Base URL", default=config.llm_base_url or "http://localhost:8000")
        new_key = Prompt.ask("API Key (可選)", default="", password=True)
        if new_key:
            config.llm_api_key = new_key
    elif provider in ("anthropic", "openai", "gemini"):
        new_key = Prompt.ask("API Key", password=True, default="")
        if new_key:
            config.llm_api_key = new_key

    config.llm_model = Prompt.ask("模型名稱 (Enter 使用預設)", default=config.llm_model or "")

    config.save()
    console.print("\n[green]✓ LLM 配置已保存[/green]")

    # 測試連接
    console.print("\n測試 LLM 連接...")
    llm_config = config.get_llm_config()
    success, msg = test_llm_connection(llm_config)

    if success:
        console.print(f"[green]✓ {msg}[/green]")
    else:
        console.print(f"[red]✗ {msg}[/red]")


@app.command("outlook-login")
def outlook_login():
    """登入 Microsoft 365 Outlook"""
    try:
        from .outlook_helper import OutlookClient
    except ImportError:
        console.print("[red]請先安裝 outlook 依賴:[/red]")
        console.print("  pip install tempo-sync[outlook]")
        return

    console.print(Panel.fit(
        "[bold]Microsoft 365 登入[/bold]\n"
        "連結 Outlook 行事曆",
        title="📅",
    ))

    config = Config.load()

    # 檢查或設定 client_id 和 tenant_id
    if not config.outlook_client_id or not config.outlook_tenant_id:
        console.print("\n[yellow]需要 Azure AD 應用程式資訊[/yellow]")
        console.print("[dim]請至 Microsoft Entra 管理中心取得以下資訊[/dim]")
        console.print("[dim]https://entra.microsoft.com → 應用程式 → 應用程式註冊 → 概觀[/dim]\n")

        if not config.outlook_client_id:
            client_id = Prompt.ask("應用程式 (用戶端) 識別碼")
            if not client_id:
                console.print("[red]已取消[/red]")
                return
            config.outlook_client_id = client_id

        if not config.outlook_tenant_id:
            tenant_id = Prompt.ask("目錄 (租用戶) 識別碼")
            if not tenant_id:
                console.print("[red]已取消[/red]")
                return
            config.outlook_tenant_id = tenant_id

        config.save()

    client = OutlookClient(client_id=config.outlook_client_id, tenant_id=config.outlook_tenant_id)

    # 檢查是否已登入
    if client.is_authenticated():
        user = client.get_current_user()
        console.print(f"[green]已登入: {user}[/green]")
        if not Confirm.ask("要重新登入嗎?", default=False):
            return
        client.logout()

    # 裝置碼流程
    console.print("\n正在啟動認證流程...")
    try:
        flow, message = client.get_device_flow_prompt()

        # 顯示認證指示
        console.print(f"\n[bold yellow]{message}[/bold yellow]\n")
        console.print("[dim]完成瀏覽器認證後按 Enter 繼續...[/dim]")
        input()

        # 完成認證
        success, result = client.complete_device_flow(flow)

        if success:
            console.print(f"\n[green]✓ 登入成功: {result}[/green]")

            # 更新配置
            config.outlook_enabled = True
            config.save()
            console.print("[green]✓ 已啟用 Outlook 整合[/green]")
        else:
            console.print(f"\n[red]✗ 登入失敗: {result}[/red]")

    except Exception as e:
        console.print(f"[red]✗ 認證失敗: {e}[/red]")


@app.command("outlook-logout")
def outlook_logout():
    """登出 Microsoft 365 Outlook"""
    try:
        from .outlook_helper import OutlookClient
    except ImportError:
        console.print("[yellow]outlook 依賴未安裝[/yellow]")
        return

    config = Config.load()
    client = OutlookClient(client_id=config.outlook_client_id, tenant_id=config.outlook_tenant_id)
    client.logout()

    config.outlook_enabled = False
    config.save()

    console.print("[green]✓ 已登出並停用 Outlook 整合[/green]")


@app.command()
def dates():
    """列出最近有工作記錄的日期"""
    helper = WorklogHelper()
    available_dates = helper.list_dates(14)

    if not available_dates:
        console.print("[yellow]找不到任何 session 數據[/yellow]")
        return

    table = Table(title="📅 可用日期")
    table.add_column("#", style="dim")
    table.add_column("日期", style="cyan")

    for i, d in enumerate(available_dates, 1):
        table.add_row(str(i), d)

    console.print(table)


@app.callback(invoke_without_command=True)
def main(ctx: typer.Context):
    """
    Tempo Sync - 同步開發活動到 Jira Tempo worklog

    使用方式:
      tempo              # 互動模式
      tempo analyze -w   # 分析本週
      tempo analyze -u   # 分析後上傳
      tempo setup        # 配置 Jira
      tempo dates        # 列出可用日期
    """
    if ctx.invoked_subcommand is None:
        # 預設行為：進入互動循環
        helper = WorklogHelper()
        interactive_main_loop(helper)


if __name__ == "__main__":
    app()
