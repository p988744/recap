"""Excel 分析服務 - 處理 Tempo worklog Excel 檔案"""

import logging
import pandas as pd
from collections import defaultdict
from typing import TypedDict

logger = logging.getLogger(__name__)


class AnalysisResult(TypedDict):
    user_name: str
    total_hours: float
    project_hours: float
    meeting_hours: float
    leave_hours: float
    admin_hours: float
    project_issues: dict
    issue_descriptions: dict
    issue_summary: list
    project_summary: list
    date_range: dict  # 整體日期範圍 {"start": "2025-01-01", "end": "2025-12-31"}
    issue_dates: dict  # 每個 issue 的日期範圍 {issue_key: {"start": ..., "end": ...}}


class AnalysisOutput(TypedDict):
    summary_text: str
    project_text: str
    issues_text: str
    pe_text: str
    data: AnalysisResult


def analyze_worklog(file_path: str) -> AnalysisOutput:
    """
    分析 Tempo worklog Excel 檔案

    Args:
        file_path: Excel 檔案路徑

    Returns:
        AnalysisOutput 包含四個 Markdown 文字和分析資料
    """
    try:
        logger.info(f"Reading Excel file: {file_path}")
        df = pd.read_excel(file_path)

        # 偵測欄位名稱（支援不同格式的 Tempo 匯出）
        hours_col = 'Hours' if 'Hours' in df.columns else 'Time Spent (h)'
        project_col = 'Project Key'
        issue_key_col = 'Issue Key'
        issue_summary_col = 'Issue summary'
        work_desc_col = 'Work Description'

        if hours_col not in df.columns:
            logger.error("Missing hours column in Excel file")
            return {
                "summary_text": "錯誤：找不到工時欄位 (Hours 或 Time Spent (h))",
                "project_text": "",
                "issues_text": "",
                "pe_text": "",
                "data": {}
            }

        # 取得使用者名稱
        user_name = ""
        if 'Full name' in df.columns and len(df) > 0:
            name = str(df['Full name'].iloc[0])
            if '(' in name:
                user_name = name.split('(')[1].replace(')', '')
            else:
                user_name = name

        # ===== 日期處理 =====
        date_col = 'Work date' if 'Work date' in df.columns else None
        date_range = {"start": "", "end": ""}
        issue_dates = {}

        if date_col and date_col in df.columns:
            # 轉換日期欄位
            df[date_col] = pd.to_datetime(df[date_col], errors='coerce')

            # 計算整體日期範圍
            valid_dates = df[date_col].dropna()
            if len(valid_dates) > 0:
                min_date = valid_dates.min()
                max_date = valid_dates.max()
                date_range = {
                    "start": min_date.strftime('%Y/%m/%d'),
                    "end": max_date.strftime('%Y/%m/%d')
                }

            # 計算每個 issue 的日期範圍
            for issue_key in df[issue_key_col].unique():
                issue_df = df[df[issue_key_col] == issue_key]
                issue_valid_dates = issue_df[date_col].dropna()
                if len(issue_valid_dates) > 0:
                    issue_dates[issue_key] = {
                        "start": issue_valid_dates.min().strftime('%Y/%m/%d'),
                        "end": issue_valid_dates.max().strftime('%Y/%m/%d')
                    }

        # ===== 總覽統計 =====
        total_hours = df[hours_col].sum()

        # 排除 NSP 的專案工時
        work_df = df[df[project_col] != 'NSP']
        project_hours = work_df[hours_col].sum()

        # NSP 分類統計
        nsp_df = df[df[project_col] == 'NSP']
        meeting_hours = nsp_df[nsp_df[issue_summary_col].str.contains('會議|Meeting', case=False, na=False)][hours_col].sum()
        leave_hours = nsp_df[nsp_df[issue_summary_col].str.contains('休假', na=False)][hours_col].sum()
        admin_hours = nsp_df[nsp_df[issue_summary_col].str.contains('行政|雜項', na=False)][hours_col].sum()

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

        # ===== 專案時數分布 =====
        project_summary = df.groupby(project_col)[hours_col].sum().sort_values(ascending=False)

        project_text = "## 📁 專案時數分布\n\n| 專案 | 時數 | 佔比 |\n|------|------|------|\n"
        for project, hours in project_summary.items():
            if project != 'NSP':
                pct = (hours / project_hours * 100) if project_hours > 0 else 0
                project_text += f"| {project} | {hours:.1f} h | {pct:.1f}% |\n"

        # ===== 主要工作項目（包含 NSP 會議等）=====
        # 使用全部資料來產生工作項目，而不是只用 work_df
        issue_summary = df.groupby([issue_key_col, issue_summary_col])[hours_col].sum().sort_values(ascending=False)

        # 收集每個 issue 的工作描述（使用全部資料）
        issue_descriptions = defaultdict(list)  # 改用 list 保持順序
        for _, row in df.iterrows():
            key = row[issue_key_col]
            desc = row.get(work_desc_col, '')
            if pd.notna(desc) and desc and str(desc) not in issue_descriptions[key]:
                issue_descriptions[key].append(str(desc))  # 不限制數量

        issues_text = f"## 🎯 全部工作項目 (共 {len(issue_summary)} 項)\n\n"
        for idx, ((issue_key, issue_name), hours) in enumerate(issue_summary.items(), 1):
            pct = (hours / project_hours * 100) if project_hours > 0 else 0
            issues_text += f"### {idx}. {issue_key}: {issue_name}\n"
            issues_text += f"- **時數**: {hours:.1f} h ({pct:.1f}%)\n"

            # 加入工作描述
            descs = list(issue_descriptions.get(issue_key, []))[:5]
            if descs:
                issues_text += "- **工作內容**:\n"
                for desc in descs:
                    issues_text += f"  - {desc}\n"
            issues_text += "\n"

        # ===== 績效考核建議格式 =====
        pe_text = f"""## 📝 績效考核表建議格式

以下是根據工時資料整理的工作成果建議，可直接複製到績效考核表：

---

"""
        # 按專案分組整理主要工作（包含 NSP）
        project_issues = defaultdict(list)
        for (issue_key, issue_name), hours in issue_summary.items():
            filtered_df = df[df[issue_key_col] == issue_key]  # 使用全部 df 而非 work_df
            if filtered_df.empty:
                logger.warning(f"No project found for issue: {issue_key}")
                continue
            project = filtered_df[project_col].iloc[0]
            project_issues[project].append({
                'key': issue_key,
                'name': issue_name,
                'hours': hours,
                'descriptions': issue_descriptions.get(issue_key, [])  # 不限制數量
            })

        item_num = 1
        for project, issues in sorted(project_issues.items(), key=lambda x: -sum(i['hours'] for i in x[1])):
            total_proj_hours = sum(i['hours'] for i in issues)
            weight = total_proj_hours / project_hours if project_hours > 0 else 0

            pe_text += f"### 項次 {item_num}: {project} 相關工作\n"
            pe_text += f"- **權重建議**: {weight:.0%}\n"
            pe_text += f"- **總時數**: {total_proj_hours:.1f} h\n"
            pe_text += f"- **具體成果說明**:\n"

            for issue in issues:
                pe_text += f"  - {issue['key']} {issue['name']} ({issue['hours']:.1f}h)\n"
                for desc in issue['descriptions'][:2]:
                    pe_text += f"    - {desc}\n"

            pe_text += "\n"
            item_num += 1

        # 組裝分析資料
        result_data: AnalysisResult = {
            'user_name': user_name,
            'total_hours': float(total_hours),
            'project_hours': float(project_hours),
            'meeting_hours': float(meeting_hours),
            'leave_hours': float(leave_hours),
            'admin_hours': float(admin_hours),
            'project_issues': dict(project_issues),
            'issue_descriptions': {k: list(v) for k, v in issue_descriptions.items()},
            'issue_summary': [(k, float(v)) for k, v in issue_summary.items()],
            'project_summary': [(k, float(v)) for k, v in project_summary.items() if k != 'NSP'],
            'date_range': date_range,
            'issue_dates': issue_dates
        }

        logger.info(f"Analysis complete: {len(project_issues)} projects, {len(issue_summary)} issues")

        return {
            "summary_text": summary_text,
            "project_text": project_text,
            "issues_text": issues_text,
            "pe_text": pe_text,
            "data": result_data
        }

    except Exception as e:
        logger.exception(f"Error analyzing worklog: {e}")
        return {
            "summary_text": f"錯誤：{str(e)}",
            "project_text": "",
            "issues_text": "",
            "pe_text": "",
            "data": {}
        }
