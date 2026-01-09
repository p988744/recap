"""LLM 服務 - 處理 AI 草稿生成（支援非同步）"""

import os
import logging
import asyncio
from openai import OpenAI, AsyncOpenAI
from .db_service import get_llm_options

logger = logging.getLogger(__name__)


def get_llm_config() -> dict:
    """取得 LLM 設定（優先順序：環境變數 > SQLite > 預設值）"""
    llm_config = get_llm_options()
    connection = llm_config.get("connection", {})
    args = llm_config.get("args", {})

    return {
        "api_url": os.getenv("LLM_API_URL") or connection.get("api_url", "https://api.openai.com/v1"),
        "api_key": os.getenv("LLM_API_KEY") or connection.get("api_key", ""),
        "model": os.getenv("LLM_MODEL") or connection.get("model", "gpt-4o-mini"),
        "temperature": args.get("temperature", 0.7),
        "max_tokens": args.get("max_tokens", 2000),
        "styles": llm_config.get("styles", {}),
        "prompts": llm_config.get("prompts", {})
    }


def call_llm(prompt: str, api_url: str, api_key: str, model: str) -> str:
    """呼叫 OpenAI-compatible API（同步版本）"""
    if not api_key:
        return "錯誤：請提供 API Key"

    try:
        llm_config = get_llm_config()
        client = OpenAI(base_url=api_url, api_key=api_key)
        response = client.chat.completions.create(
            model=model,
            messages=[{"role": "user", "content": prompt}],
            temperature=llm_config["temperature"],
            max_tokens=llm_config["max_tokens"]
        )
        return response.choices[0].message.content
    except Exception as e:
        logger.exception(f"LLM call error: {e}")
        return f"LLM 呼叫錯誤：{str(e)}"


async def call_llm_async(prompt: str, api_url: str, api_key: str, model: str) -> str:
    """呼叫 OpenAI-compatible API（非同步版本）- 不阻塞其他請求"""
    if not api_key:
        return "錯誤：請提供 API Key"

    try:
        llm_config = get_llm_config()
        client = AsyncOpenAI(base_url=api_url, api_key=api_key)
        response = await client.chat.completions.create(
            model=model,
            messages=[{"role": "user", "content": prompt}],
            temperature=llm_config["temperature"],
            max_tokens=llm_config["max_tokens"]
        )
        return response.choices[0].message.content
    except Exception as e:
        logger.exception(f"LLM async call error: {e}")
        return f"LLM 呼叫錯誤：{str(e)}"


def get_style_instructions(position_type: str, custom_style: str) -> dict:
    """根據職位類型取得風格指示"""
    llm_config = get_llm_config()
    config_styles = llm_config.get("styles", {})

    default_styles = {
        "技術職": {
            "work_focus": "著重技術實作細節、技術難點突破、程式碼品質、系統效能優化、技術文件撰寫等技術面向的成果",
            "skill_focus": "著重技術能力成長，如程式語言精進、新技術學習、架構設計能力、問題解決能力等",
            "ethics_focus": "從技術人員角度描述，強調技術專業、程式碼品質、技術文件、知識分享等"
        },
        "管理職": {
            "work_focus": "著重團隊管理、專案推動、跨部門協調、資源調度、人才培育、目標達成等管理面向的成果",
            "skill_focus": "著重管理能力成長，如領導統御、團隊建設、策略規劃、決策判斷、衝突處理等",
            "ethics_focus": "從管理者角度描述，強調帶領團隊、激勵成員、建立制度、跨部門溝通等"
        }
    }

    styles = {**default_styles, **config_styles}

    if position_type == "自訂":
        return {
            "work_focus": f"著重以下面向：{custom_style}" if custom_style else "依據工作內容自然呈現",
            "skill_focus": f"著重以下面向的能力成長：{custom_style}" if custom_style else "依據工作內容自然呈現",
            "ethics_focus": f"從以下角度描述：{custom_style}" if custom_style else "依據工作內容自然呈現"
        }

    return styles.get(position_type, styles.get("技術職", default_styles["技術職"]))


async def generate_item_summary(
    worklogs: list[str],
    issue_name: str,
    hours: float,
    api_url: str,
    api_key: str,
    model: str,
    use_cache: bool = True
) -> str:
    """為單一工作項目生成摘要（非同步，支援快取）"""
    if not worklogs:
        return ""

    if not api_key:
        return "錯誤：請提供 API Key"

    # 嘗試從快取取得
    if use_cache:
        try:
            from .db_service import get_cached_summary, cache_summary
            cached = get_cached_summary(worklogs, issue_name)
            if cached:
                logger.info(f"Cache hit for summary: {issue_name}")
                return cached
        except Exception as e:
            logger.warning(f"Cache lookup failed: {e}")

    worklog_text = "\n".join(f"- {log}" for log in worklogs if log.strip())

    prompt = f"""請根據以下工作紀錄，生成一段簡潔的工作摘要。

工作項目：{issue_name}
投入時數：{hours:.1f}h

工作紀錄：
{worklog_text}

要求：
1. 使用繁體中文
2. 摘要控制在 50-100 字
3. 重點說明做了什麼、達成什麼成果
4. 語氣專業簡潔
5. 如果可以量化，盡量量化

請直接輸出摘要，不要加標題或前綴。"""

    summary = await call_llm_async(prompt, api_url, api_key, model)

    # 儲存到快取
    if use_cache and not summary.startswith("錯誤") and not summary.startswith("LLM"):
        try:
            from .db_service import cache_summary
            cache_summary(worklogs, issue_name, summary)
            logger.info(f"Cached summary for: {issue_name}")
        except Exception as e:
            logger.warning(f"Cache save failed: {e}")

    return summary


async def generate_category_summary(
    category_name: str,
    category_description: str,
    tasks: list[dict],
    api_url: str,
    api_key: str,
    model: str,
) -> str:
    """為單一分類（工作項目）生成摘要（非同步）

    Args:
        category_name: 分類名稱
        category_description: 分類描述
        tasks: 該分類下的所有 tasks，每個 task 包含 issue_name, hours, worklogs
        api_url: LLM API URL
        api_key: API Key
        model: 模型名稱

    Returns:
        摘要文字
    """
    if not tasks:
        return ""

    if not api_key:
        return "錯誤：請提供 API Key"

    # 計算總時數
    total_hours = sum(t.get("hours", 0) for t in tasks)

    # 組合所有 tasks 的資訊
    task_details = []
    for task in tasks:
        issue_name = task.get("issue_name", "")
        hours = task.get("hours", 0)
        worklogs = task.get("worklogs", [])

        if isinstance(worklogs, list):
            # worklogs 可能是 list[str] 或 list[dict]
            if worklogs and isinstance(worklogs[0], dict):
                worklog_texts = [w.get("text", "") for w in worklogs if w.get("text")]
            else:
                worklog_texts = [str(w) for w in worklogs if w]
        else:
            worklog_texts = []

        task_info = f"【{issue_name}】({hours:.1f}h)"
        if worklog_texts:
            task_info += "\n" + "\n".join(f"  - {log}" for log in worklog_texts[:5])  # 最多取 5 條
        task_details.append(task_info)

    tasks_text = "\n\n".join(task_details)

    prompt = f"""請根據以下工作項目分類的工作紀錄，生成一段績效考核用的工作摘要。

工作項目名稱：{category_name}
{"工作項目描述：" + category_description if category_description else ""}
總投入時數：{total_hours:.1f}h
包含 Tasks 數量：{len(tasks)} 個

各 Task 詳細紀錄：
{tasks_text}

要求：
1. 使用繁體中文
2. 摘要控制在 100-200 字
3. 整合所有 Tasks 的工作內容，歸納出主要成果
4. 語氣專業簡潔，適合績效考核報告
5. 如果可以量化成果，請盡量量化
6. 重點說明：做了什麼、解決什麼問題、達成什麼成果

請直接輸出摘要，不要加標題或前綴。"""

    summary = await call_llm_async(prompt, api_url, api_key, model)
    return summary


def group_items_by_category(work_items: list[dict], categories: list[dict]) -> dict:
    """將工作項目依分類分組"""
    # 建立 category_id -> name 對照表
    cat_map = {cat["id"]: cat["name"] for cat in categories}
    cat_map[-1] = "未分類"  # 預設未分類

    groups = {}
    for item in work_items:
        if not item.get("included", True):
            continue
        # 使用分類，如果沒有分類則使用專案名稱
        category_id = item.get("category_id", -1)
        if category_id >= 0 and category_id in cat_map:
            group_name = cat_map[category_id]
        else:
            group_name = item.get("project", "其他")

        if group_name not in groups:
            groups[group_name] = []
        groups[group_name].append(item)
    return groups


def group_items_by_tag(work_items: list[dict]) -> dict:
    """將工作項目依標籤分組（向後相容）"""
    return group_items_by_category(work_items, [])


def format_work_items_for_prompt(work_items: list[dict], categories: list[dict] = None) -> str:
    """將工作項目格式化為 prompt 用的文字"""
    groups = group_items_by_category(work_items, categories or [])
    result = ""

    # 建立分類名稱到分類物件的對照表
    cat_by_name = {cat.get("name", ""): cat for cat in (categories or [])}

    for group_name, items in sorted(groups.items(), key=lambda x: -sum(i.get('hours', 0) for i in x[1])):
        total_hours = sum(item.get('hours', 0) for item in items)
        result += f"\n【{group_name}】總時數：{total_hours:.1f}h\n"

        # 優先使用分類的 AI 摘要
        category_summary = cat_by_name.get(group_name, {}).get("summary", "")
        if category_summary:
            result += f"工作摘要：{category_summary}\n"

        for item in items:
            result += f"- {item.get('issue_key', '')} {item.get('issue_name', '')} ({item.get('hours', 0):.1f}h)\n"
            # 顯示 worklogs
            for log in item.get('worklogs', [])[:2]:
                if isinstance(log, dict):
                    log_text = log.get("text", "")
                else:
                    log_text = str(log)
                if log_text.strip():
                    result += f"  • {log_text}\n"
            # 加入使用者備註
            if item.get('custom_note'):
                result += f"  → 備註：{item['custom_note']}\n"

    return result


async def suggest_work_items_from_tasks(
    work_items: list[dict],
    api_url: str,
    api_key: str,
    model: str,
) -> list[dict]:
    """
    使用 LLM 分析 Tasks，智慧建議工作項目分類

    Returns:
        list[dict]: 建議的工作項目列表，每個包含:
            - name: 工作項目名稱
            - description: 工作項目描述
            - task_ids: 建議歸類到此項目的 Task ID 列表
    """
    if not work_items:
        return []

    if not api_key:
        raise ValueError("請先設定 API Key")

    # 準備 Task 資料給 LLM
    tasks_text = ""
    for item in work_items:
        worklogs = item.get("worklogs", [])
        worklog_texts = []
        for wl in worklogs:
            if isinstance(wl, dict):
                text = wl.get("text", "")
            else:
                text = str(wl)
            if text:
                worklog_texts.append(text)

        tasks_text += f"""
Task ID: {item['id']}
Issue Key: {item.get('issue_key', '')}
Issue Name: {item.get('issue_name', '')}
專案: {item.get('project', '')}
時數: {item.get('hours', 0):.1f}h
Worklogs: {'; '.join(worklog_texts[:5])}
---
"""

    prompt = f"""你是一位績效考核專家。請分析以下工作紀錄（Tasks），將它們歸類為 3-6 個有意義的「工作項目」。

工作項目應該：
1. 反映實際的工作性質或專案主題
2. 便於在績效考核報告中呈現
3. 將相關的工作整合在一起

請以 JSON 格式回覆，格式如下：
```json
[
  {{
    "name": "工作項目名稱",
    "description": "詳細描述這個工作項目包含的工作內容、目的與範圍（30-50字）",
    "task_ids": [0, 1, 2]
  }}
]
```

注意：
- description 非常重要，後續 AI 會根據 description 來判斷新的工作紀錄應歸類到哪個工作項目
- description 應清楚說明這個工作項目涵蓋哪些類型的工作

以下是需要分析的 Tasks：
{tasks_text}

請直接輸出 JSON，不要加其他說明。"""

    try:
        response = await call_llm_async(prompt, api_url, api_key, model)

        # 解析 JSON
        import json
        import re

        # 嘗試提取 JSON
        json_match = re.search(r'\[[\s\S]*\]', response)
        if json_match:
            suggestions = json.loads(json_match.group())
            return suggestions
        else:
            logger.warning(f"Could not parse LLM response: {response[:200]}")
            return []

    except json.JSONDecodeError as e:
        logger.exception(f"JSON parse error: {e}")
        return []
    except Exception as e:
        logger.exception(f"Error suggesting work items: {e}")
        raise


async def auto_categorize_tasks(
    categories: list[dict],
    work_items: list[dict],
    api_url: str,
    api_key: str,
    model: str,
) -> dict[int, int]:
    """
    使用 AI 根據已定義的分類自動將 Tasks 分配到適當的工作項目

    Args:
        categories: 已定義的工作項目列表，每個包含 id, name, description
        work_items: 待分類的 Tasks 列表
        api_url: LLM API URL
        api_key: API Key
        model: 模型名稱

    Returns:
        dict[int, int]: Task ID -> Category ID 的對照表
    """
    if not categories or not work_items:
        return {}

    if not api_key:
        raise ValueError("請先設定 API Key")

    # 準備分類資料
    categories_text = ""
    for cat in categories:
        categories_text += f"""
分類 ID: {cat['id']}
名稱: {cat.get('name', '')}
描述: {cat.get('description', '無描述')}
---
"""

    # 準備 Task 資料
    tasks_text = ""
    for item in work_items:
        if not item.get("included", True):
            continue

        worklogs = item.get("worklogs", [])
        worklog_texts = []
        for wl in worklogs:
            if isinstance(wl, dict):
                text = wl.get("text", "")
            else:
                text = str(wl)
            if text:
                worklog_texts.append(text)

        tasks_text += f"""
Task ID: {item['id']}
Issue Key: {item.get('issue_key', '')}
Issue Name: {item.get('issue_name', '')}
專案: {item.get('project', '')}
時數: {item.get('hours', 0):.1f}h
工作紀錄: {'; '.join(worklog_texts[:3])}
---
"""

    prompt = f"""你是一位績效考核專家。請根據以下已定義的「工作項目分類」，將 Tasks 分配到最適合的分類中。

## 已定義的工作項目分類：
{categories_text}

## 待分類的 Tasks：
{tasks_text}

## 分類原則：
1. 根據 Task 的 Issue Name、專案名稱、工作紀錄內容，判斷最適合的分類
2. 每個 Task 只能分配到一個分類
3. 如果 Task 無法明確歸類到任何分類，可以設為 -1（未分類）
4. 優先參考分類的「描述」來判斷是否適合

請以 JSON 格式回覆，格式如下：
```json
{{
  "0": 1,
  "1": 2,
  "2": 1,
  "3": -1
}}
```
其中 key 是 Task ID，value 是分配的 Category ID（-1 表示未分類）。

請直接輸出 JSON，不要加其他說明。"""

    try:
        response = await call_llm_async(prompt, api_url, api_key, model)

        # 解析 JSON
        import json
        import re

        # 嘗試提取 JSON
        json_match = re.search(r'\{[\s\S]*\}', response)
        if json_match:
            result = json.loads(json_match.group())
            # 轉換 key 為 int
            return {int(k): int(v) for k, v in result.items()}
        else:
            logger.warning(f"Could not parse LLM response: {response[:200]}")
            return {}

    except json.JSONDecodeError as e:
        logger.exception(f"JSON parse error: {e}")
        return {}
    except Exception as e:
        logger.exception(f"Error auto categorizing tasks: {e}")
        raise


async def auto_categorize_worklogs(
    categories: list[dict],
    work_items: list[dict],
    api_url: str,
    api_key: str,
    model: str,
) -> dict[str, int]:
    """
    使用 AI 根據已定義的分類自動將 Worklogs 分配到適當的工作項目
    分類時會同時參考 Task 和 Worklog 的內容，避免脈絡遺失

    Args:
        categories: 已定義的工作項目列表，每個包含 id, name, description
        work_items: 包含 worklogs 的 Tasks 列表
        api_url: LLM API URL
        api_key: API Key
        model: 模型名稱

    Returns:
        dict[str, int]: "task_id:worklog_index" -> Category ID 的對照表
    """
    if not categories or not work_items:
        return {}

    if not api_key:
        raise ValueError("請先設定 API Key")

    # 準備分類資料
    categories_text = ""
    for cat in categories:
        categories_text += f"""
分類 ID: {cat['id']}
名稱: {cat.get('name', '')}
描述: {cat.get('description', '無描述')}
---
"""

    # 準備 Worklog 資料（包含 Task 脈絡）
    worklogs_text = ""
    worklog_count = 0
    for item in work_items:
        if not item.get("included", True):
            continue

        task_id = item.get("id", 0)
        issue_key = item.get("issue_key", "")
        issue_name = item.get("issue_name", "")
        project = item.get("project", "")

        worklogs = item.get("worklogs", [])
        for idx, wl in enumerate(worklogs):
            if isinstance(wl, dict):
                text = wl.get("text", "")
                hours = wl.get("hours", 0)
            else:
                text = str(wl)
                hours = 0

            if not text.strip():
                continue

            worklogs_text += f"""
Worklog Key: {task_id}:{idx}
所屬 Task: [{issue_key}] {issue_name}
專案: {project}
工作內容: {text}
時數: {hours:.1f}h
---
"""
            worklog_count += 1

    if worklog_count == 0:
        return {}

    prompt = f"""你是一位績效考核專家。請根據以下已定義的「工作項目分類」，將每個 Worklog（工作紀錄）分配到最適合的分類中。

## 已定義的工作項目分類：
{categories_text}

## 待分類的 Worklogs：
{worklogs_text}

## 分類原則（按優先順序）：
1. **Worklog 工作內容優先**：主要根據 Worklog 的「工作內容」文字來判斷分類，不要被「所屬 Task」名稱誤導
   - 例如：Task 名稱是「會議」，但 Worklog 內容是「[專案] XXX評估」，應歸類到專案相關分類
2. 根據分類的「描述」來判斷 Worklog 是否符合該分類的範疇
3. 同一個 Task 下的不同 Worklog 應該根據各自內容分到不同的分類
4. 如果 Worklog 無法明確歸類到任何分類，設為 -1（未分類）

請以 JSON 格式回覆，格式如下：
```json
{{
  "0:0": 1,
  "0:1": 2,
  "1:0": 1,
  "2:0": -1
}}
```
其中 key 是 "task_id:worklog_index"，value 是分配的 Category ID（-1 表示未分類）。

請直接輸出 JSON，不要加其他說明。"""

    try:
        # 使用更大的 max_tokens 避免截斷
        llm_config = get_llm_config()
        client = AsyncOpenAI(base_url=api_url, api_key=api_key)
        response_obj = await client.chat.completions.create(
            model=model,
            messages=[{"role": "user", "content": prompt}],
            temperature=0.3,  # 降低溫度提高一致性
            max_tokens=8000,  # 增加 token 限制
        )
        response = response_obj.choices[0].message.content

        # 解析 JSON
        import json
        import re

        # 嘗試提取 JSON
        json_match = re.search(r'\{[\s\S]*\}', response)
        if json_match:
            result = json.loads(json_match.group())
            # 確保 value 是 int
            return {str(k): int(v) for k, v in result.items()}
        else:
            logger.warning(f"Could not parse LLM response: {response[:200]}")
            return {}

    except json.JSONDecodeError as e:
        logger.exception(f"JSON parse error: {e}")
        return {}
    except Exception as e:
        logger.exception(f"Error auto categorizing worklogs: {e}")
        raise


def generate_work_results_draft(
    api_url: str,
    api_key: str,
    model: str,
    position_type: str,
    custom_style: str,
    work_items: list[dict],
    categories: list[dict] = None
) -> str:
    """產生工作成果草稿 - 使用編輯後的工作項目"""
    if not work_items:
        return "請先上傳並分析檔案"

    if not api_key:
        return "錯誤：請提供 API Key"

    # 過濾只取納入的項目
    included_items = [item for item in work_items if item.get("included", True)]
    if not included_items:
        return "沒有選擇任何工作項目"

    llm_config = get_llm_config()
    style = get_style_instructions(position_type, custom_style)

    # 依分類分組
    groups = group_items_by_category(included_items, categories or [])
    total_hours = sum(item.get('hours', 0) for item in included_items)

    drafts = []
    item_num = 1
    num_groups = len(groups)
    equal_weight = 1 / num_groups if num_groups > 0 else 0

    # 建立分類名稱到分類物件的對照表
    cat_by_name = {cat.get("name", ""): cat for cat in (categories or [])}

    for group_name, items in sorted(groups.items(), key=lambda x: -sum(i.get('hours', 0) for i in x[1])):
        group_hours = sum(item.get('hours', 0) for item in items)

        # 計算此分類的日期範圍（從所有 work_items 中取最早和最晚日期）
        date_starts = [item.get("date_start", "") for item in items if item.get("date_start")]
        date_ends = [item.get("date_end", "") for item in items if item.get("date_end")]
        group_date_start = min(date_starts) if date_starts else ""
        group_date_end = max(date_ends) if date_ends else ""

        raw_details = f"工作類別：{group_name}\n總時數：{group_hours:.1f}h\n\n"

        # 優先使用分類的 AI 摘要
        category_summary = cat_by_name.get(group_name, {}).get("summary", "")
        if category_summary:
            raw_details += f"工作摘要：{category_summary}\n\n"

        raw_details += "工作項目：\n"
        for item in items:
            raw_details += f"- {item.get('issue_key', '')} {item.get('issue_name', '')} ({item.get('hours', 0):.1f}h)\n"
            # 顯示 worklogs 摘要
            for log in item.get('worklogs', [])[:2]:
                if isinstance(log, dict):
                    log_text = log.get("text", "")
                else:
                    log_text = str(log)
                if log_text.strip():
                    raw_details += f"  • {log_text}\n"
            if item.get('custom_note'):
                raw_details += f"  → 使用者補充：{item['custom_note']}\n"

        prompt = f"""請根據以下工作資料，撰寫績效考核表的「具體成果說明」。

風格要求：{style['work_focus']}

撰寫要求：
1. 使用繁體中文
2. 以條列方式呈現主要工作成果（3-5 條）
3. 每項成果包含：做了什麼、達成什麼效果
4. 如果有「使用者補充」內容，請優先參考並融入說明
5. 語氣專業簡潔，避免冗長
6. 可量化的部分盡量量化（如時數、數量、百分比）
7. 總字數控制在 150-250 字

工作資料：
{raw_details}

請直接輸出成果說明，不要加標題或前綴。"""

        summary = call_llm(prompt, api_url, api_key, model)

        # 格式化期間
        period_str = ""
        if group_date_start and group_date_end:
            period_str = f"\n期間：{group_date_start}~{group_date_end}"
        elif group_date_start:
            period_str = f"\n期間：{group_date_start}~"
        elif group_date_end:
            period_str = f"\n期間：~{group_date_end}"

        drafts.append(f"""【項次 {item_num}】{group_name} 相關工作
權重：{equal_weight:.0%} | 時數：{group_hours:.1f}h{period_str}

{summary}
""")
        item_num += 1

    return "\n---\n".join(drafts)


def generate_skill_development_draft(
    api_url: str,
    api_key: str,
    model: str,
    position_type: str,
    custom_style: str,
    work_items: list[dict]
) -> str:
    """產生技能發展草稿 - 使用編輯後的工作項目"""
    if not work_items:
        return "請先上傳並分析檔案"

    if not api_key:
        return "錯誤：請提供 API Key"

    included_items = [item for item in work_items if item.get("included", True)]
    if not included_items:
        return "沒有選擇任何工作項目"

    llm_config = get_llm_config()
    style = get_style_instructions(position_type, custom_style)

    # 格式化工作內容
    all_work = format_work_items_for_prompt(included_items)

    prompt = f"""請根據以下工作內容，分析並撰寫績效考核表的「技能發展」部分。

風格要求：{style['skill_focus']}

撰寫要求：
1. 使用繁體中文
2. 列出 2-4 項技能項目
3. 每項技能包含：技能名稱、具體說明（如何展現/成長/應用）
4. 如果有「備註」內容，請參考融入說明
5. 語氣專業，強調成長與貢獻
6. 每項說明約 50-100 字

格式：
【技能項目 1】XXX能力
具體說明：透過執行XXX專案，熟悉並掌握...

工作內容：
{all_work}

請直接輸出技能發展內容。"""

    return call_llm(prompt, api_url, api_key, model)


def generate_professional_ethics_draft(
    api_url: str,
    api_key: str,
    model: str,
    position_type: str,
    custom_style: str,
    work_items: list[dict]
) -> str:
    """產生職場專業素養草稿 - 使用編輯後的工作項目"""
    if not work_items:
        return "請先上傳並分析檔案"

    if not api_key:
        return "錯誤：請提供 API Key"

    included_items = [item for item in work_items if item.get("included", True)]

    llm_config = get_llm_config()
    style = get_style_instructions(position_type, custom_style)

    # 計算統計
    total_hours = sum(item.get('hours', 0) for item in included_items)
    groups = group_items_by_tag(included_items)

    work_summary = f"總工時：{total_hours:.1f}h\n參與專案/類別數：{len(groups)}\n納入項目數：{len(included_items)}\n"

    prompt = f"""請根據以下工作概況，撰寫績效考核表的「職場專業素養」部分。

風格要求：{style['ethics_focus']}

需要撰寫以下三個項目的具體說明：
1. 責任感：尊重時程，以認真、負責的態度完成工作
2. 團隊精神：在團隊中盡好本份，彼此支援、相互合作
3. 主動積極：願意從各方面追求對工作目標最有益的結果

撰寫要求：
1. 使用繁體中文
2. 每項約 50-80 字
3. 語氣專業正向，但不過度誇大
4. 結合實際工作情境描述

工作概況：
{work_summary}

格式：
【責任感】
具體說明...

【團隊精神】
具體說明...

【主動積極】
具體說明...

請直接輸出。"""

    return call_llm(prompt, api_url, api_key, model)


def generate_all_drafts(
    api_url: str,
    api_key: str,
    model: str,
    position_type: str,
    custom_style: str,
    work_items: list[dict],
    categories: list[dict] = None
) -> tuple[str, str, str]:
    """一次產生所有草稿 - 使用編輯後的工作項目和分類"""
    logger.info(f"Generating all drafts with {len(work_items)} items, {len(categories or [])} categories")
    work_draft = generate_work_results_draft(
        api_url, api_key, model, position_type, custom_style, work_items, categories
    )
    skill_draft = generate_skill_development_draft(
        api_url, api_key, model, position_type, custom_style, work_items
    )
    ethics_draft = generate_professional_ethics_draft(
        api_url, api_key, model, position_type, custom_style, work_items
    )
    return work_draft, skill_draft, ethics_draft
