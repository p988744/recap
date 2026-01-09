"""SQLite 資料持久化服務"""

import sqlite3
import json
import os
import logging
from datetime import datetime
from pathlib import Path

logger = logging.getLogger(__name__)

# 資料庫路徑 - 使用 ~/.recap/pe/ 目錄
PE_DATA_DIR = Path.home() / ".recap" / "pe"
DB_PATH = PE_DATA_DIR / "pe_helper.db"


def get_db_connection() -> sqlite3.Connection:
    """取得資料庫連線"""
    # 確保資料目錄存在
    DB_PATH.parent.mkdir(parents=True, exist_ok=True)

    conn = sqlite3.connect(str(DB_PATH))
    conn.row_factory = sqlite3.Row
    return conn


def init_db():
    """初始化資料庫表格"""
    conn = get_db_connection()
    cursor = conn.cursor()

    # 使用者 Session 表
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_key TEXT UNIQUE NOT NULL,
            emp_name TEXT,
            emp_dept TEXT,
            emp_title TEXT,
            emp_start_date TEXT,
            emp_manager TEXT,
            emp_period TEXT,
            file_name TEXT,
            analysis_data TEXT,
            work_draft TEXT,
            skill_draft TEXT,
            ethics_draft TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    """)

    # 工作項目表（含摘要快取）
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS work_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_key TEXT NOT NULL,
            item_id INTEGER NOT NULL,
            project TEXT,
            issue_key TEXT,
            issue_name TEXT,
            hours REAL,
            worklogs TEXT,
            summary TEXT,
            custom_note TEXT,
            included INTEGER DEFAULT 1,
            category_id INTEGER DEFAULT -1,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(session_key, item_id)
        )
    """)

    # 分類（工作項目）表
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_key TEXT NOT NULL,
            category_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            color TEXT DEFAULT 'primary',
            description TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(session_key, category_id)
        )
    """)

    # 摘要快取表（根據 worklogs hash 快取）
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS summary_cache (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            worklogs_hash TEXT UNIQUE NOT NULL,
            issue_name TEXT,
            summary TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    """)

    # 全域設定表（取代 options.json）
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS global_options (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            key TEXT UNIQUE NOT NULL,
            value TEXT NOT NULL,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    """)

    conn.commit()
    conn.close()
    logger.info(f"Database initialized at {DB_PATH}")

    # 執行資料庫遷移
    _run_migrations()


def _run_migrations():
    """執行資料庫遷移 - 添加缺少的欄位"""
    conn = get_db_connection()
    cursor = conn.cursor()

    # 檢查 work_items 表是否有 category_id 欄位
    cursor.execute("PRAGMA table_info(work_items)")
    columns = [col[1] for col in cursor.fetchall()]

    if "category_id" not in columns:
        logger.info("Adding category_id column to work_items table...")
        cursor.execute("ALTER TABLE work_items ADD COLUMN category_id INTEGER DEFAULT -1")
        conn.commit()
        logger.info("Migration completed: category_id column added")

    # 檢查 categories 表是否有 summary 欄位
    cursor.execute("PRAGMA table_info(categories)")
    cat_columns = [col[1] for col in cursor.fetchall()]

    if "summary" not in cat_columns:
        logger.info("Adding summary column to categories table...")
        cursor.execute("ALTER TABLE categories ADD COLUMN summary TEXT DEFAULT ''")
        conn.commit()
        logger.info("Migration completed: summary column added to categories")

    # 檢查 sessions 表是否有 Jira 相關欄位
    cursor.execute("PRAGMA table_info(sessions)")
    session_columns = [col[1] for col in cursor.fetchall()]

    # 添加 data_source 欄位（區分 XLSX 或 JIRA）
    if "data_source" not in session_columns:
        logger.info("Adding data_source column to sessions table...")
        cursor.execute("ALTER TABLE sessions ADD COLUMN data_source TEXT DEFAULT 'xlsx'")
        conn.commit()
        logger.info("Migration completed: data_source column added")

    # 添加 jira_pat 欄位
    if "jira_pat" not in session_columns:
        logger.info("Adding jira_pat column to sessions table...")
        cursor.execute("ALTER TABLE sessions ADD COLUMN jira_pat TEXT DEFAULT ''")
        conn.commit()
        logger.info("Migration completed: jira_pat column added")

    # 添加 jira_username 欄位
    if "jira_username" not in session_columns:
        logger.info("Adding jira_username column to sessions table...")
        cursor.execute("ALTER TABLE sessions ADD COLUMN jira_username TEXT DEFAULT ''")
        conn.commit()
        logger.info("Migration completed: jira_username column added")

    # 添加 jira_user_key 欄位
    if "jira_user_key" not in session_columns:
        logger.info("Adding jira_user_key column to sessions table...")
        cursor.execute("ALTER TABLE sessions ADD COLUMN jira_user_key TEXT DEFAULT ''")
        conn.commit()
        logger.info("Migration completed: jira_user_key column added")

    conn.close()


def generate_session_key(emp_name: str, file_name: str) -> str:
    """生成 session key（舊版，保留相容性）"""
    return f"{emp_name}_{file_name}".replace(" ", "_").lower()


def generate_session_token(prefix: str = "") -> str:
    """生成唯一的 session token

    Args:
        prefix: 可選前綴，用於區分來源（如 "XLSX_" 或 "JIRA_"）

    Returns:
        8 個字符的 token，如果有前綴則為 "PREFIX_XXXXXXXX" 格式
    """
    import secrets
    import string
    # 使用大寫字母和數字，排除容易混淆的字符（0, O, I, 1）
    alphabet = string.ascii_uppercase.replace('O', '').replace('I', '') + string.digits.replace('0', '').replace('1', '')
    token = ''.join(secrets.choice(alphabet) for _ in range(8))
    return f"{prefix}{token}" if prefix else token


def save_session(
    session_key: str,
    emp_name: str = "",
    emp_dept: str = "",
    emp_title: str = "",
    emp_start_date: str = "",
    emp_manager: str = "",
    emp_period: str = "",
    file_name: str = "",
    analysis_data: dict = None,
    work_draft: str = "",
    skill_draft: str = "",
    ethics_draft: str = "",
    data_source: str = "xlsx",
    jira_pat: str = "",
    jira_username: str = "",
    jira_user_key: str = ""
):
    """儲存或更新 session

    Args:
        data_source: 資料來源 ("xlsx" 或 "jira")
        jira_pat: Jira Personal Access Token（僅 jira 來源）
        jira_username: Jira 使用者名稱（僅 jira 來源）
        jira_user_key: Jira 使用者 Key（僅 jira 來源）
    """
    conn = get_db_connection()
    cursor = conn.cursor()

    cursor.execute("""
        INSERT INTO sessions (
            session_key, emp_name, emp_dept, emp_title, emp_start_date,
            emp_manager, emp_period, file_name, analysis_data,
            work_draft, skill_draft, ethics_draft,
            data_source, jira_pat, jira_username, jira_user_key,
            updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(session_key) DO UPDATE SET
            emp_name = excluded.emp_name,
            emp_dept = excluded.emp_dept,
            emp_title = excluded.emp_title,
            emp_start_date = excluded.emp_start_date,
            emp_manager = excluded.emp_manager,
            emp_period = excluded.emp_period,
            file_name = excluded.file_name,
            analysis_data = excluded.analysis_data,
            work_draft = excluded.work_draft,
            skill_draft = excluded.skill_draft,
            ethics_draft = excluded.ethics_draft,
            data_source = excluded.data_source,
            jira_pat = excluded.jira_pat,
            jira_username = excluded.jira_username,
            jira_user_key = excluded.jira_user_key,
            updated_at = excluded.updated_at
    """, (
        session_key, emp_name, emp_dept, emp_title, emp_start_date,
        emp_manager, emp_period, file_name,
        json.dumps(analysis_data) if analysis_data else None,
        work_draft, skill_draft, ethics_draft,
        data_source, jira_pat, jira_username, jira_user_key,
        datetime.now().isoformat()
    ))

    conn.commit()
    conn.close()


def load_session(session_key: str) -> dict | None:
    """載入 session"""
    conn = get_db_connection()
    cursor = conn.cursor()

    cursor.execute("SELECT * FROM sessions WHERE session_key = ?", (session_key,))
    row = cursor.fetchone()
    conn.close()

    if row:
        result = dict(row)
        if result.get("analysis_data"):
            result["analysis_data"] = json.loads(result["analysis_data"])
        return result
    return None


def delete_session(session_key: str) -> bool:
    """刪除 session 及其相關資料

    Args:
        session_key: 要刪除的 session key

    Returns:
        True 如果成功刪除，False 如果 session 不存在
    """
    conn = get_db_connection()
    cursor = conn.cursor()

    # 檢查 session 是否存在
    cursor.execute("SELECT 1 FROM sessions WHERE session_key = ?", (session_key,))
    if not cursor.fetchone():
        conn.close()
        return False

    # 刪除相關的 work_items
    cursor.execute("DELETE FROM work_items WHERE session_key = ?", (session_key,))

    # 刪除相關的 categories
    cursor.execute("DELETE FROM categories WHERE session_key = ?", (session_key,))

    # 刪除 session 本身
    cursor.execute("DELETE FROM sessions WHERE session_key = ?", (session_key,))

    conn.commit()
    conn.close()
    return True


def save_work_items(session_key: str, work_items: list[dict]):
    """儲存工作項目列表"""
    conn = get_db_connection()
    cursor = conn.cursor()

    # 刪除舊的項目
    cursor.execute("DELETE FROM work_items WHERE session_key = ?", (session_key,))

    # 插入新的項目
    for item in work_items:
        cursor.execute("""
            INSERT INTO work_items (
                session_key, item_id, project, issue_key, issue_name,
                hours, worklogs, summary, custom_note, included, category_id, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """, (
            session_key,
            item.get("id", 0),
            item.get("project", ""),
            item.get("issue_key", ""),
            item.get("issue_name", ""),
            item.get("hours", 0),
            json.dumps(item.get("worklogs", [])),
            item.get("summary", ""),
            item.get("custom_note", ""),
            1 if item.get("included", True) else 0,
            item.get("category_id", -1),
            datetime.now().isoformat()
        ))

    conn.commit()
    conn.close()


def save_categories(session_key: str, categories: list[dict]):
    """儲存分類（工作項目）列表"""
    conn = get_db_connection()
    cursor = conn.cursor()

    # 刪除舊的分類
    cursor.execute("DELETE FROM categories WHERE session_key = ?", (session_key,))

    # 插入新的分類
    for cat in categories:
        cursor.execute("""
            INSERT INTO categories (
                session_key, category_id, name, color, description, summary, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
        """, (
            session_key,
            cat.get("id", 0),
            cat.get("name", ""),
            cat.get("color", "primary"),
            cat.get("description", ""),
            cat.get("summary", ""),
            datetime.now().isoformat()
        ))

    conn.commit()
    conn.close()


def load_categories(session_key: str) -> list[dict]:
    """載入分類（工作項目）列表"""
    conn = get_db_connection()
    cursor = conn.cursor()

    cursor.execute("""
        SELECT * FROM categories
        WHERE session_key = ?
        ORDER BY category_id
    """, (session_key,))

    rows = cursor.fetchall()
    conn.close()

    categories = []
    for row in rows:
        row_dict = dict(row)
        cat = {
            "id": row_dict["category_id"],
            "name": row_dict["name"],
            "color": row_dict["color"],
            "description": row_dict.get("description") or "",
            "summary": row_dict.get("summary") or "",
        }
        categories.append(cat)

    return categories


def load_work_items(session_key: str) -> list[dict]:
    """載入工作項目列表"""
    conn = get_db_connection()
    cursor = conn.cursor()

    cursor.execute("""
        SELECT * FROM work_items
        WHERE session_key = ?
        ORDER BY item_id
    """, (session_key,))

    rows = cursor.fetchall()
    conn.close()

    items = []
    for row in rows:
        item = dict(row)
        item["id"] = item.pop("item_id")
        item["worklogs"] = json.loads(item.get("worklogs", "[]"))
        item["included"] = bool(item.get("included", 1))
        item["tags"] = [item.get("project", "")]
        item["category_id"] = item.get("category_id", -1)
        # 移除資料庫欄位
        for key in ["session_key", "created_at", "updated_at"]:
            item.pop(key, None)
        items.append(item)

    return items


def get_worklogs_hash(worklogs: list[str]) -> str:
    """計算 worklogs 的 hash（用於快取）"""
    import hashlib
    content = json.dumps(sorted([w.strip() for w in worklogs if w.strip()]))
    return hashlib.md5(content.encode()).hexdigest()


def get_cached_summary(worklogs: list[str], issue_name: str = "") -> str | None:
    """從快取取得摘要"""
    worklogs_hash = get_worklogs_hash(worklogs)

    conn = get_db_connection()
    cursor = conn.cursor()

    cursor.execute("""
        SELECT summary FROM summary_cache
        WHERE worklogs_hash = ?
    """, (worklogs_hash,))

    row = cursor.fetchone()
    conn.close()

    return row["summary"] if row else None


def cache_summary(worklogs: list[str], issue_name: str, summary: str):
    """快取摘要"""
    worklogs_hash = get_worklogs_hash(worklogs)

    conn = get_db_connection()
    cursor = conn.cursor()

    cursor.execute("""
        INSERT INTO summary_cache (worklogs_hash, issue_name, summary)
        VALUES (?, ?, ?)
        ON CONFLICT(worklogs_hash) DO UPDATE SET
            summary = excluded.summary
    """, (worklogs_hash, issue_name, summary))

    conn.commit()
    conn.close()


def list_sessions(limit: int = 10) -> list[dict]:
    """列出最近的 sessions"""
    conn = get_db_connection()
    cursor = conn.cursor()

    cursor.execute("""
        SELECT session_key, emp_name, file_name, updated_at, data_source
        FROM sessions
        ORDER BY updated_at DESC
        LIMIT ?
    """, (limit,))

    rows = cursor.fetchall()
    conn.close()

    return [dict(row) for row in rows]


# ===== 全域設定 (取代 options.json) =====

def get_option(key: str, default=None):
    """取得單一設定值"""
    conn = get_db_connection()
    cursor = conn.cursor()

    cursor.execute("SELECT value FROM global_options WHERE key = ?", (key,))
    row = cursor.fetchone()
    conn.close()

    if row:
        try:
            return json.loads(row["value"])
        except json.JSONDecodeError:
            return row["value"]
    return default


def set_option(key: str, value):
    """設定單一設定值"""
    conn = get_db_connection()
    cursor = conn.cursor()

    cursor.execute("""
        INSERT INTO global_options (key, value, updated_at)
        VALUES (?, ?, ?)
        ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at
    """, (key, json.dumps(value), datetime.now().isoformat()))

    conn.commit()
    conn.close()


def get_all_options() -> dict:
    """取得所有設定"""
    conn = get_db_connection()
    cursor = conn.cursor()

    cursor.execute("SELECT key, value FROM global_options")
    rows = cursor.fetchall()
    conn.close()

    result = {}
    for row in rows:
        try:
            result[row["key"]] = json.loads(row["value"])
        except json.JSONDecodeError:
            result[row["key"]] = row["value"]
    return result


def migrate_from_options_json():
    """從 options.json 遷移資料到 SQLite（只執行一次）"""
    options_file = Path(__file__).parent.parent.parent / "data" / "options.json"

    if not options_file.exists():
        return

    # 檢查是否已經遷移過
    if get_option("_migrated_from_json"):
        return

    try:
        with open(options_file, 'r', encoding='utf-8') as f:
            old_options = json.load(f)

        # 遷移 titles
        if "titles" in old_options:
            set_option("titles", old_options["titles"])

        # 遷移 departments（清理格式）
        if "departments" in old_options:
            depts = old_options["departments"]
            # 只保留 dict 格式的資料
            clean_depts = [d for d in depts if isinstance(d, dict)]
            set_option("departments", clean_depts)

        # 遷移 managers
        if "managers" in old_options:
            set_option("managers", old_options["managers"])

        # 遷移 LLM 設定（如果有）
        if "llm" in old_options:
            set_option("llm", old_options["llm"])

        # 標記已遷移
        set_option("_migrated_from_json", True)

        logger.info("Successfully migrated options from JSON to SQLite")

        # 備份並刪除舊檔案
        backup_file = options_file.with_suffix('.json.bak')
        options_file.rename(backup_file)
        logger.info(f"Old options.json backed up to {backup_file}")

    except Exception as e:
        logger.error(f"Failed to migrate options: {e}")


# ===== 便捷函數 =====

def get_titles() -> list[str]:
    """取得職稱列表"""
    return get_option("titles", [])


def add_title(title: str):
    """新增職稱"""
    titles = get_titles()
    if title and title not in titles:
        titles.append(title)
        set_option("titles", titles)


def get_departments() -> list[dict]:
    """取得部門列表"""
    return get_option("departments", [])


def save_departments(departments: list[dict]):
    """儲存部門列表"""
    set_option("departments", departments)


def get_llm_options() -> dict:
    """取得 LLM 設定"""
    return get_option("llm", {})


def get_jira_options() -> dict:
    """取得 Jira/Tempo 連線設定（只有 URL 會持久化）"""
    return get_option("jira", {
        "url": ""
    })


def save_jira_options(options: dict):
    """儲存 Jira/Tempo 連線設定（只儲存 URL，username/token 不持久化）"""
    # 只儲存 URL，不儲存 username 和 token（安全考量）
    safe_options = {
        "url": options.get("url", "")
    }
    set_option("jira", safe_options)


# 初始化資料庫
init_db()

# 嘗試從 options.json 遷移
migrate_from_options_json()
