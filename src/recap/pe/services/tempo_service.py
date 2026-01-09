"""Tempo API 服務 - 從 Jira Tempo 抓取 worklog 資料"""

import logging
import requests
from collections import defaultdict
from datetime import datetime
from typing import TypedDict

logger = logging.getLogger(__name__)


class TempoWorklog(TypedDict):
    """Tempo API 回傳的 worklog 結構"""
    issue_key: str
    issue_summary: str
    project_key: str
    time_spent_hours: float
    work_date: str
    author_name: str
    comment: str


class FetchResult(TypedDict):
    """抓取結果"""
    success: bool
    worklogs: list[TempoWorklog]
    error: str
    user_name: str


class UserInfo(TypedDict):
    """使用者資訊"""
    success: bool
    username: str
    user_key: str
    display_name: str
    email: str
    error: str


def get_user_info_from_pat(jira_url: str, token: str) -> UserInfo:
    """
    從 PAT 取得使用者資訊

    Args:
        jira_url: Jira Server URL
        token: Personal Access Token

    Returns:
        UserInfo 包含 username, user_key, display_name, email
    """
    jira_url = jira_url.rstrip('/')

    try:
        response = requests.get(
            f"{jira_url}/rest/api/2/myself",
            headers={"Authorization": f"Bearer {token}"},
            timeout=10
        )

        if response.status_code == 401:
            return {
                "success": False,
                "username": "",
                "user_key": "",
                "display_name": "",
                "email": "",
                "error": "Token 認證失敗：請確認 PAT 是否正確"
            }

        if response.status_code == 200:
            data = response.json()
            return {
                "success": True,
                "username": data.get("name", ""),
                "user_key": data.get("key", ""),
                "display_name": data.get("displayName", ""),
                "email": data.get("emailAddress", ""),
                "error": ""
            }

        return {
            "success": False,
            "username": "",
            "user_key": "",
            "display_name": "",
            "email": "",
            "error": f"無法取得使用者資訊：HTTP {response.status_code}"
        }

    except requests.exceptions.ConnectionError:
        return {
            "success": False,
            "username": "",
            "user_key": "",
            "display_name": "",
            "email": "",
            "error": f"無法連線到 {jira_url}"
        }
    except requests.exceptions.Timeout:
        return {
            "success": False,
            "username": "",
            "user_key": "",
            "display_name": "",
            "email": "",
            "error": "連線逾時"
        }
    except Exception as e:
        return {
            "success": False,
            "username": "",
            "user_key": "",
            "display_name": "",
            "email": "",
            "error": f"錯誤：{str(e)}"
        }


def _resolve_user_key(jira_url: str, username: str, token: str) -> tuple[str, str, str]:
    """
    解析使用者 key（Tempo API 需要 user key 而非 username）

    Returns:
        tuple: (user_key, display_name, error)
    """
    try:
        # 如果已經是 user key 格式（JIRAUSER12345），直接返回
        if username.upper().startswith("JIRAUSER"):
            return username.upper(), "", ""

        # 查詢使用者資訊
        response = requests.get(
            f"{jira_url}/rest/api/2/user",
            params={"username": username},
            headers={"Authorization": f"Bearer {token}"},
            timeout=10
        )

        if response.status_code == 200:
            data = response.json()
            user_key = data.get("key", username)
            display_name = data.get("displayName", username)
            return user_key, display_name, ""
        elif response.status_code == 401:
            return "", "", "認證失敗"
        elif response.status_code == 404:
            return "", "", f"找不到使用者：{username}"
        else:
            return "", "", f"查詢使用者失敗：HTTP {response.status_code}"
    except Exception as e:
        logger.warning(f"Failed to resolve user key: {e}")
        # 回退使用 username
        return username, "", ""


def fetch_worklogs(
    jira_url: str,
    username: str,
    token: str,
    date_from: str,
    date_to: str
) -> FetchResult:
    """
    從 Tempo API 抓取 worklogs

    Args:
        jira_url: Jira Server URL (e.g., https://ims.eland.com.tw)
        username: Jira 使用者帳號
        token: Personal Access Token
        date_from: 開始日期 (YYYY-MM-DD)
        date_to: 結束日期 (YYYY-MM-DD)

    Returns:
        FetchResult 包含成功狀態、worklogs 列表或錯誤訊息
    """
    # 清理 URL
    jira_url = jira_url.rstrip('/')

    # 解析使用者 key（Tempo API 需要 user key）
    user_key, display_name, resolve_error = _resolve_user_key(jira_url, username, token)
    if resolve_error:
        return {
            "success": False,
            "worklogs": [],
            "error": resolve_error,
            "user_name": ""
        }

    logger.info(f"Resolved user key: {username} -> {user_key}")

    # Tempo Timesheets Server API 端點 (使用 POST search)
    api_url = f"{jira_url}/rest/tempo-timesheets/4/worklogs/search"

    # POST body for search (使用 user key)
    payload = {
        "from": date_from,
        "to": date_to,
        "worker": [user_key]
    }

    headers = {
        "Authorization": f"Bearer {token}",
        "Accept": "application/json",
        "Content-Type": "application/json"
    }

    logger.info(f"Fetching worklogs from {api_url} for {username} ({date_from} to {date_to})")

    try:
        response = requests.post(
            api_url,
            json=payload,
            headers=headers,
            timeout=30,
            verify=True  # SSL 驗證
        )

        if response.status_code == 401:
            return {
                "success": False,
                "worklogs": [],
                "error": "認證失敗：Token 無效或已過期",
                "user_name": ""
            }

        if response.status_code == 403:
            return {
                "success": False,
                "worklogs": [],
                "error": "權限不足：無法存取該使用者的 worklog",
                "user_name": ""
            }

        if response.status_code == 404:
            return {
                "success": False,
                "worklogs": [],
                "error": "找不到 Tempo API：請確認 Jira URL 正確且已安裝 Tempo Timesheets",
                "user_name": ""
            }

        response.raise_for_status()

        data = response.json()
        worklogs = _parse_tempo_response(data)

        # 取得使用者顯示名稱（優先使用 resolver 取得的名稱）
        user_name = display_name or ""
        if not user_name and worklogs:
            user_name = worklogs[0].get("author_name", username)
        if not user_name:
            user_name = username

        logger.info(f"Successfully fetched {len(worklogs)} worklogs for {user_name}")

        return {
            "success": True,
            "worklogs": worklogs,
            "error": "",
            "user_name": user_name
        }

    except requests.exceptions.SSLError:
        return {
            "success": False,
            "worklogs": [],
            "error": "SSL 憑證錯誤：無法建立安全連線",
            "user_name": ""
        }

    except requests.exceptions.ConnectionError:
        return {
            "success": False,
            "worklogs": [],
            "error": f"無法連線到 {jira_url}：請檢查網路連線和 Jira URL",
            "user_name": ""
        }

    except requests.exceptions.Timeout:
        return {
            "success": False,
            "worklogs": [],
            "error": "連線逾時：Jira 伺服器無回應",
            "user_name": ""
        }

    except requests.exceptions.RequestException as e:
        logger.exception(f"Error fetching worklogs: {e}")
        return {
            "success": False,
            "worklogs": [],
            "error": f"API 請求失敗：{str(e)}",
            "user_name": ""
        }


def _parse_tempo_response(data: list | dict) -> list[TempoWorklog]:
    """
    解析 Tempo API 回應

    Tempo Timesheets API 回傳格式 (list of worklogs):
    [
        {
            "id": 12345,
            "issue": {
                "key": "PROJ-123",
                "summary": "Issue title",
                "projectKey": "PROJ"
            },
            "timeSpentSeconds": 3600,
            "started": "2025-01-15",
            "author": {
                "key": "username",
                "displayName": "User Name (帳號)"
            },
            "comment": "Work description"
        }
    ]
    """
    worklogs = []

    # 處理可能的不同回傳格式
    if isinstance(data, dict):
        # 有些版本的 Tempo 會包在 object 裡
        items = data.get("worklogs", data.get("results", []))
    else:
        items = data

    for item in items:
        try:
            # 解析 issue 資訊
            issue = item.get("issue", {})
            issue_key = issue.get("key", "")
            issue_summary = issue.get("summary", "")
            project_key = issue.get("projectKey", "")

            # 如果沒有 projectKey，從 issue_key 解析
            if not project_key and issue_key:
                project_key = issue_key.split("-")[0] if "-" in issue_key else ""

            # 解析時間（秒轉小時）
            time_spent_seconds = item.get("timeSpentSeconds", 0)
            time_spent_hours = time_spent_seconds / 3600 if time_spent_seconds else 0

            # 解析日期
            work_date = item.get("started", "")
            if work_date and "T" in work_date:
                work_date = work_date.split("T")[0]

            # 解析作者
            author = item.get("author", {})
            author_name = author.get("displayName", author.get("key", ""))

            # 清理作者名稱（移除帳號部分）
            if "(" in author_name:
                # "User Name (username)" -> 取中文名
                author_name = author_name.split("(")[1].replace(")", "").strip()

            # 解析備註
            comment = item.get("comment", "") or ""

            worklogs.append({
                "issue_key": issue_key,
                "issue_summary": issue_summary,
                "project_key": project_key,
                "time_spent_hours": time_spent_hours,
                "work_date": work_date,
                "author_name": author_name,
                "comment": comment
            })

        except Exception as e:
            logger.warning(f"Failed to parse worklog item: {e}")
            continue

    return worklogs


def transform_to_analysis_result(worklogs: list[TempoWorklog], user_name: str = "") -> dict:
    """
    將 Tempo worklogs 轉換為 AnalysisResult 格式
    （與 excel_analyzer.py 的輸出格式相同）

    Args:
        worklogs: Tempo worklogs 列表
        user_name: 使用者名稱

    Returns:
        與 excel_analyzer.analyze_worklog() 相同格式的 dict
    """
    if not worklogs:
        return {
            "summary_text": "沒有找到任何 worklog 資料",
            "project_text": "",
            "issues_text": "",
            "pe_text": "",
            "data": {}
        }

    # 如果沒有提供 user_name，從 worklogs 取得
    if not user_name and worklogs:
        user_name = worklogs[0].get("author_name", "")

    # ===== 統計計算 =====
    total_hours = sum(w["time_spent_hours"] for w in worklogs)

    # 排除 NSP 的專案工時
    work_worklogs = [w for w in worklogs if w["project_key"] != "NSP"]
    project_hours = sum(w["time_spent_hours"] for w in work_worklogs)

    # NSP 分類統計
    nsp_worklogs = [w for w in worklogs if w["project_key"] == "NSP"]
    meeting_hours = sum(
        w["time_spent_hours"] for w in nsp_worklogs
        if "會議" in w["issue_summary"] or "meeting" in w["issue_summary"].lower()
    )
    leave_hours = sum(
        w["time_spent_hours"] for w in nsp_worklogs
        if "休假" in w["issue_summary"]
    )
    admin_hours = sum(
        w["time_spent_hours"] for w in nsp_worklogs
        if "行政" in w["issue_summary"] or "雜項" in w["issue_summary"]
    )

    # ===== 日期範圍 =====
    dates = [w["work_date"] for w in worklogs if w["work_date"]]
    date_range = {"start": "", "end": ""}
    if dates:
        sorted_dates = sorted(dates)
        date_range = {
            "start": _format_date(sorted_dates[0]),
            "end": _format_date(sorted_dates[-1])
        }

    # 每個 issue 的日期範圍
    issue_dates = {}
    issue_date_map = defaultdict(list)
    for w in worklogs:
        if w["work_date"]:
            issue_date_map[w["issue_key"]].append(w["work_date"])

    for issue_key, dates_list in issue_date_map.items():
        sorted_dates = sorted(dates_list)
        issue_dates[issue_key] = {
            "start": _format_date(sorted_dates[0]),
            "end": _format_date(sorted_dates[-1])
        }

    # ===== 專案時數分布 =====
    project_summary_dict = defaultdict(float)
    for w in worklogs:
        project_summary_dict[w["project_key"]] += w["time_spent_hours"]

    project_summary = sorted(
        [(k, v) for k, v in project_summary_dict.items()],
        key=lambda x: -x[1]
    )

    # ===== Issue 統計 =====
    issue_hours = defaultdict(float)
    issue_names = {}
    issue_descriptions = defaultdict(list)

    for w in worklogs:
        key = w["issue_key"]
        issue_hours[key] += w["time_spent_hours"]
        issue_names[key] = w["issue_summary"]

        if w["comment"] and w["comment"] not in issue_descriptions[key]:
            issue_descriptions[key].append(w["comment"])

    issue_summary = sorted(
        [((k, issue_names[k]), v) for k, v in issue_hours.items()],
        key=lambda x: -x[1]
    )

    # ===== Project Issues =====
    project_issues = defaultdict(list)
    for (issue_key, issue_name), hours in issue_summary:
        # 找到這個 issue 的 project
        project = next(
            (w["project_key"] for w in worklogs if w["issue_key"] == issue_key),
            ""
        )
        project_issues[project].append({
            "key": issue_key,
            "name": issue_name,
            "hours": hours,
            "descriptions": issue_descriptions.get(issue_key, [])
        })

    # ===== 產生文字輸出 =====
    summary_text = f"""## 📊 總覽統計

| 項目 | 時數 |
|------|------|
| 👤 使用者 | {user_name} |
| ⏱️ 總工時 | {total_hours:.1f} h |
| 💼 專案工時 (排除 NSP) | {project_hours:.1f} h |
| 🗓️ 會議時數 | {meeting_hours:.1f} h |
| 🏖️ 休假時數 | {leave_hours:.1f} h |
| 📋 行政工作 | {admin_hours:.1f} h |
"""

    project_text = "## 📁 專案時數分布\n\n| 專案 | 時數 | 佔比 |\n|------|------|------|\n"
    for project, hours in project_summary:
        if project != "NSP":
            pct = (hours / project_hours * 100) if project_hours > 0 else 0
            project_text += f"| {project} | {hours:.1f} h | {pct:.1f}% |\n"

    issues_text = f"## 🎯 全部工作項目 (共 {len(issue_summary)} 項)\n\n"
    for idx, ((issue_key, issue_name), hours) in enumerate(issue_summary, 1):
        pct = (hours / project_hours * 100) if project_hours > 0 else 0
        issues_text += f"### {idx}. {issue_key}: {issue_name}\n"
        issues_text += f"- **時數**: {hours:.1f} h ({pct:.1f}%)\n"

        descs = list(issue_descriptions.get(issue_key, []))[:5]
        if descs:
            issues_text += "- **工作內容**:\n"
            for desc in descs:
                issues_text += f"  - {desc}\n"
        issues_text += "\n"

    pe_text = f"""## 📝 績效考核表建議格式

以下是根據工時資料整理的工作成果建議，可直接複製到績效考核表：

---

"""
    item_num = 1
    for project, issues in sorted(project_issues.items(), key=lambda x: -sum(i["hours"] for i in x[1])):
        total_proj_hours = sum(i["hours"] for i in issues)
        weight = total_proj_hours / project_hours if project_hours > 0 else 0

        pe_text += f"### 項次 {item_num}: {project} 相關工作\n"
        pe_text += f"- **權重建議**: {weight:.0%}\n"
        pe_text += f"- **總時數**: {total_proj_hours:.1f} h\n"
        pe_text += f"- **具體成果說明**:\n"

        for issue in issues:
            pe_text += f"  - {issue['key']} {issue['name']} ({issue['hours']:.1f}h)\n"
            for desc in issue["descriptions"][:2]:
                pe_text += f"    - {desc}\n"

        pe_text += "\n"
        item_num += 1

    # ===== 組裝結果 =====
    result_data = {
        "user_name": user_name,
        "total_hours": float(total_hours),
        "project_hours": float(project_hours),
        "meeting_hours": float(meeting_hours),
        "leave_hours": float(leave_hours),
        "admin_hours": float(admin_hours),
        "project_issues": dict(project_issues),
        "issue_descriptions": {k: list(v) for k, v in issue_descriptions.items()},
        "issue_summary": issue_summary,
        "project_summary": [(k, v) for k, v in project_summary if k != "NSP"],
        "date_range": date_range,
        "issue_dates": issue_dates
    }

    return {
        "summary_text": summary_text,
        "project_text": project_text,
        "issues_text": issues_text,
        "pe_text": pe_text,
        "data": result_data
    }


def _format_date(date_str: str) -> str:
    """將日期格式化為 YYYY/MM/DD"""
    if not date_str:
        return ""
    try:
        # 嘗試解析各種格式
        for fmt in ["%Y-%m-%d", "%Y/%m/%d", "%d/%m/%Y"]:
            try:
                dt = datetime.strptime(date_str, fmt)
                return dt.strftime("%Y/%m/%d")
            except ValueError:
                continue
        return date_str
    except Exception:
        return date_str


def test_connection(jira_url: str, username: str, token: str) -> dict:
    """
    測試 Jira/Tempo 連線

    Returns:
        dict with keys: success, message, user_display_name
    """
    jira_url = jira_url.rstrip('/')

    # 先測試 Jira 基本連線
    try:
        response = requests.get(
            f"{jira_url}/rest/api/2/myself",
            headers={"Authorization": f"Bearer {token}"},
            timeout=10
        )

        if response.status_code == 401:
            return {
                "success": False,
                "message": "Token 認證失敗",
                "user_display_name": ""
            }

        if response.status_code == 200:
            user_data = response.json()
            display_name = user_data.get("displayName", username)

            # 測試 Tempo API (使用 POST search endpoint)
            today = datetime.now().strftime("%Y-%m-%d")
            tempo_response = requests.post(
                f"{jira_url}/rest/tempo-timesheets/4/worklogs/search",
                json={
                    "from": today,
                    "to": today,
                    "worker": [username]
                },
                headers={
                    "Authorization": f"Bearer {token}",
                    "Accept": "application/json",
                    "Content-Type": "application/json"
                },
                timeout=10
            )

            if tempo_response.status_code == 404:
                return {
                    "success": False,
                    "message": "Jira 連線成功，但找不到 Tempo API",
                    "user_display_name": display_name
                }

            if tempo_response.status_code in [200, 204]:
                return {
                    "success": True,
                    "message": f"連線成功！使用者：{display_name}",
                    "user_display_name": display_name
                }

            # Tempo API 失敗
            return {
                "success": False,
                "message": f"Tempo API 錯誤：HTTP {tempo_response.status_code}",
                "user_display_name": display_name
            }

        return {
            "success": False,
            "message": f"Jira 連線失敗：HTTP {response.status_code}",
            "user_display_name": ""
        }

    except requests.exceptions.ConnectionError:
        return {
            "success": False,
            "message": f"無法連線到 {jira_url}",
            "user_display_name": ""
        }
    except requests.exceptions.Timeout:
        return {
            "success": False,
            "message": "連線逾時",
            "user_display_name": ""
        }
    except Exception as e:
        return {
            "success": False,
            "message": f"錯誤：{str(e)}",
            "user_display_name": ""
        }
