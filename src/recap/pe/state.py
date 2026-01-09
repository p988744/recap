"""PE Helper State - Reflex 狀態管理"""

import os
import reflex as rx
from typing import List
from dotenv import load_dotenv

# 整合共用配置
from ..config import Config as SharedConfig

from .services.excel_analyzer import analyze_worklog
from .services.llm_service import generate_all_drafts, get_llm_config, generate_item_summary, generate_category_summary, auto_categorize_tasks
from .services.excel_exporter import export_to_excel
from .services.db_service import (
    save_session, load_session, save_work_items, load_work_items,
    save_categories, load_categories, generate_session_key, generate_session_token, list_sessions,
    get_titles, add_title, get_departments, save_departments, set_option,
    get_jira_options, save_jira_options, delete_session
)
from .services.tempo_service import fetch_worklogs, transform_to_analysis_result, test_connection, get_user_info_from_pat

load_dotenv()


class AppState(rx.State):
    """應用程式狀態"""

    # ===== 步驟控制 =====
    # 1: 輸入基本資料, 2: 上傳檔案, 3: 資料預覽, 4: 生成報告
    current_step: int = 1
    # Step 3 的 Tab: "categories" 或 "tasks"
    step3_active_tab: str = "categories"

    # ===== 檔案上傳 =====
    uploading: bool = False
    file_name: str = ""

    # ===== 分析結果 =====
    summary_text: str = ""
    project_text: str = ""
    issues_text: str = ""
    pe_text: str = ""
    analysis_data: dict = {}

    # ===== 可編輯工作項目 =====
    # 每個項目: {id, project, issue_key, issue_name, hours, worklogs, summary, tags, included, custom_note, category_id}
    work_items: list[dict] = []
    available_tags: list[str] = []  # 所有可用標籤
    new_tag_input: str = ""  # 新增標籤輸入框
    generating_summary_for: int = -1  # 正在生成摘要的項目 ID
    selected_item_id: int = -1  # 選中編輯的項目 ID
    session_key: str = ""  # 當前 session key（用於持久化）
    recent_sessions: list[dict] = []  # 最近的 sessions

    # ===== Token 機制 =====
    token_input: str = ""  # 用戶輸入的 token
    show_token_modal: bool = False  # 顯示 token 對話框
    saved_token: str = rx.LocalStorage(name="pe_helper_token")  # 存在 localStorage 的 token
    token_history: str = rx.LocalStorage(name="pe_helper_token_history")  # Token 歷史（JSON 格式）
    token_history_list: list[dict] = []  # 解析後的 token 歷史列表

    # ===== Session 管理 =====
    show_session_modal: bool = False  # 顯示 session 選擇對話框
    session_modal_tab: str = "recent"  # "recent" 或 "new"

    # ===== 自訂分類 =====
    # 每個分類: {id, name, color, description}
    categories: list[dict] = []
    new_category_name: str = ""
    editing_category_id: int = -1
    expanded_category_id: int = -1  # 展開編輯的工作項目 ID
    edit_category_name: str = ""  # 編輯中的工作項目名稱
    edit_category_description: str = ""  # 編輯中的工作項目描述

    # ===== 員工資料 =====
    emp_name: str = ""
    emp_dept: str = ""
    emp_title: str = ""
    emp_start_date: str = ""
    emp_manager: str = ""
    emp_period: str = "2025/1/1~2025/12/31"
    emp_sick_leave: float = 0.0
    emp_personal_leave: float = 0.0
    emp_absent: float = 0.0

    # ===== LLM 設定 =====
    api_url: str = ""
    api_key: str = ""
    model: str = ""
    position_type: str = "技術職"
    custom_style: str = ""

    # ===== Jira/Tempo 設定 =====
    jira_url: str = ""
    jira_username: str = ""
    jira_token: str = ""  # PAT (Personal Access Token)
    jira_user_key: str = ""  # Jira User Key (e.g., JIRAUSER10236)
    jira_display_name: str = ""  # 使用者顯示名稱
    jira_date_from: str = ""
    jira_date_to: str = ""
    jira_connection_status: str = ""  # 連線測試結果
    jira_loading: bool = False  # 是否正在從 Jira 載入
    show_jira_settings: bool = False  # 顯示 Jira 設定對話框
    current_data_source: str = ""  # 當前 session 的資料來源 ("xlsx" 或 "jira")

    # ===== AI 草稿 =====
    work_draft: str = ""
    skill_draft: str = ""
    ethics_draft: str = ""

    # ===== UI 狀態 =====
    analysis_tab: str = "summary"
    is_generating: bool = False

    # ===== 拖拉狀態 =====
    dragging_task_id: int = -1  # 正在拖拉的 task ID
    dragging_worklog_id: int = -1  # 正在拖拉的 worklog ID（如有）
    drag_source_task_id: int = -1  # worklog 來源的 task ID
    is_exporting: bool = False
    download_url: str = ""
    error_message: str = ""
    success_message: str = ""
    filter_category_id: int = -999  # 篩選的工作項目 ID（-999 = 全部, -1 = 未分類）

    # ===== 全域進度狀態 =====
    is_busy: bool = False  # 是否正在執行耗時任務
    busy_title: str = ""  # 任務標題
    busy_message: str = ""  # 當前進度訊息
    busy_progress: int = 0  # 進度百分比 (0-100)
    busy_total: int = 0  # 總項目數
    busy_current: int = 0  # 當前項目

    # ===== 下拉選單選項 =====
    title_options: List[str] = []  # 職稱選項

    # ===== 部門資料 =====
    # 每個部門: {id, name, division, division_manager, dept_manager, functions}
    departments: List[dict] = []
    editing_dept_id: int = -1  # 正在編輯的部門 ID

    # ===== 設定頁面 =====
    show_settings: bool = False
    show_basic_settings: bool = False  # 基本設定面板展開狀態（用於後續步驟）
    new_title: str = ""
    # 部門編輯欄位
    edit_dept_name: str = ""
    edit_dept_division: str = ""
    edit_dept_division_manager: str = ""
    edit_dept_manager: str = ""
    edit_dept_functions: str = ""

    # ===== 員工資料（處級主管由部門帶入）=====
    emp_division_manager: str = ""  # 處級主管（從部門自動帶入）

    def _init_options(self):
        """初始化下拉選單選項（從 SQLite 讀取）"""
        self.title_options = get_titles()
        self.departments = get_departments()

    def _init_llm_config(self):
        """初始化 LLM 設定（優先使用共用配置）"""
        # 優先從共用配置載入（tempo setup-llm 設定的）
        shared_config = SharedConfig.load()
        if shared_config.has_llm_config():
            llm_config = shared_config.get_llm_config()
            self.api_url = llm_config.openai_base_url or "https://api.openai.com/v1"
            self.api_key = llm_config.api_key or ""
            self.model = llm_config.model or "gpt-4o-mini"
        else:
            # 回退到 PE 專屬資料庫
            config = get_llm_config()
            self.api_url = config.get("api_url", "https://api.openai.com/v1")
            self.api_key = config.get("api_key", "")
            self.model = config.get("model", "gpt-4o-mini")

    def _init_jira_config(self):
        """初始化 Jira/Tempo 設定（優先使用共用配置）"""
        from datetime import datetime

        # 優先從共用配置載入（tempo setup 設定的）
        shared_config = SharedConfig.load()
        if shared_config.is_configured():
            self.jira_url = shared_config.jira_url
            self.jira_token = shared_config.get_token()
            # 嘗試從 Jira API 取得使用者名稱
            if not self.jira_username:
                self.jira_username = shared_config.jira_email or ""
        else:
            # 回退到 PE 專屬資料庫
            config = get_jira_options()
            self.jira_url = config.get("url", "")
            if not self.jira_username:
                self.jira_username = ""
            if not self.jira_token:
                self.jira_token = ""

        # 預設日期範圍為當年度
        year = datetime.now().year
        if not self.jira_date_from:
            self.jira_date_from = f"{year}-01-01"
        if not self.jira_date_to:
            self.jira_date_to = f"{year}-12-31"

    def _start_busy(self, title: str, message: str = "", total: int = 0):
        """開始耗時任務"""
        self.is_busy = True
        self.busy_title = title
        self.busy_message = message
        self.busy_progress = 0
        self.busy_total = total
        self.busy_current = 0
        self.error_message = ""
        self.success_message = ""

    def _update_busy(self, message: str = "", current: int = None):
        """更新進度"""
        if message:
            self.busy_message = message
        if current is not None:
            self.busy_current = current
            if self.busy_total > 0:
                self.busy_progress = int((current / self.busy_total) * 100)

    def _end_busy(self, success_message: str = "", error_message: str = ""):
        """結束耗時任務"""
        self.is_busy = False
        self.busy_title = ""
        self.busy_message = ""
        self.busy_progress = 0
        if success_message:
            self.success_message = success_message
        if error_message:
            self.error_message = error_message

    @rx.event
    def on_load(self):
        """頁面載入時初始化"""
        self._init_options()
        self._init_llm_config()
        self._init_jira_config()
        self._load_recent_sessions()
        # 檢查 localStorage 中的 saved_token 並自動載入
        self._try_auto_load_token()

    def _load_token_history(self):
        """從 localStorage 載入 Token 歷史"""
        import json
        try:
            if self.token_history:
                self.token_history_list = json.loads(self.token_history)
            else:
                self.token_history_list = []
        except (json.JSONDecodeError, TypeError):
            self.token_history_list = []

    def _save_token_to_history(self, token: str, emp_name: str, file_name: str):
        """將 Token 儲存到歷史記錄"""
        import json
        from datetime import datetime

        # 載入現有歷史
        self._load_token_history()

        # 移除相同 token 的舊記錄
        self.token_history_list = [h for h in self.token_history_list if h.get("token") != token]

        # 新增到最前面
        self.token_history_list.insert(0, {
            "token": token,
            "emp_name": emp_name or "未命名",
            "file_name": file_name or "",
            "updated_at": datetime.now().strftime("%Y-%m-%d %H:%M"),
        })

        # 只保留最近 10 筆
        self.token_history_list = self.token_history_list[:10]

        # 儲存到 localStorage
        self.token_history = json.dumps(self.token_history_list)

    def _try_auto_load_token(self):
        """嘗試從 localStorage 自動載入 session"""
        # 先載入 token 歷史
        self._load_token_history()

        if self.saved_token and not self.session_key:
            try:
                session = load_session(self.saved_token)
                if session:
                    self.session_key = self.saved_token
                    self.emp_name = session.get("emp_name", "")
                    self.emp_dept = session.get("emp_dept", "")
                    self.emp_title = session.get("emp_title", "")
                    self.emp_start_date = session.get("emp_start_date", "")
                    self.emp_manager = session.get("emp_manager", "")
                    self.emp_period = session.get("emp_period", "")
                    self.file_name = session.get("file_name", "")
                    self.work_draft = session.get("work_draft", "")
                    self.skill_draft = session.get("skill_draft", "")
                    self.ethics_draft = session.get("ethics_draft", "")

                    if session.get("analysis_data"):
                        self.analysis_data = session["analysis_data"]

                    items = load_work_items(self.saved_token)
                    if items:
                        self.work_items = items
                        self.available_tags = list(set(item.get("project", "") for item in items))

                    cats = load_categories(self.saved_token)
                    if cats:
                        self.categories = cats

                    if self.analysis_data:
                        self.current_step = 3
                        self.success_message = f"✅ 已自動載入上次的工作進度"
            except Exception as e:
                print(f"Auto-load token failed: {e}")

    def _load_recent_sessions(self):
        """載入最近的 sessions"""
        try:
            self.recent_sessions = list_sessions(limit=5)
        except Exception as e:
            print(f"Failed to load recent sessions: {e}")

    def _auto_save(self):
        """自動儲存當前狀態"""
        # 使用唯一 token 作為 session key（區分 XLSX 和 JIRA 來源）
        if not self.session_key and self.file_name:
            prefix = "JIRA_" if self.current_data_source == "jira" else ""
            self.session_key = generate_session_token(prefix)

        if self.session_key:
            # 同步到 localStorage
            self.saved_token = self.session_key

            # 更新 token 歷史
            self._save_token_to_history(self.session_key, self.emp_name, self.file_name)

            try:
                save_session(
                    session_key=self.session_key,
                    emp_name=self.emp_name,
                    emp_dept=self.emp_dept,
                    emp_title=self.emp_title,
                    emp_start_date=self.emp_start_date,
                    emp_manager=self.emp_manager,
                    emp_period=self.emp_period,
                    file_name=self.file_name,
                    analysis_data=self.analysis_data,
                    work_draft=self.work_draft,
                    skill_draft=self.skill_draft,
                    ethics_draft=self.ethics_draft,
                    # Jira session 相關資料
                    data_source=self.current_data_source or "xlsx",
                    jira_pat=self.jira_token if self.current_data_source == "jira" else "",
                    jira_username=self.jira_username if self.current_data_source == "jira" else "",
                    jira_user_key=self.jira_user_key if self.current_data_source == "jira" else ""
                )
                if self.work_items:
                    save_work_items(self.session_key, self.work_items)
                if self.categories:
                    save_categories(self.session_key, self.categories)
            except Exception as e:
                print(f"Auto-save failed: {e}")

    @rx.event
    def load_saved_session(self, session_key: str):
        """載入已儲存的 session"""
        try:
            session = load_session(session_key)
            if session:
                self.session_key = session_key
                self.emp_name = session.get("emp_name", "")
                self.emp_dept = session.get("emp_dept", "")
                self.emp_title = session.get("emp_title", "")
                self.emp_start_date = session.get("emp_start_date", "")
                self.emp_manager = session.get("emp_manager", "")
                self.emp_period = session.get("emp_period", "")
                self.file_name = session.get("file_name", "")
                self.analysis_data = session.get("analysis_data", {})
                self.work_draft = session.get("work_draft", "")
                self.skill_draft = session.get("skill_draft", "")
                self.ethics_draft = session.get("ethics_draft", "")

                # 載入 Jira session 相關資料
                self.current_data_source = session.get("data_source", "xlsx")
                if self.current_data_source == "jira":
                    self.jira_token = session.get("jira_pat", "")
                    self.jira_username = session.get("jira_username", "")
                    self.jira_user_key = session.get("jira_user_key", "")

                # 載入工作項目
                items = load_work_items(session_key)
                if items:
                    self.work_items = items
                    self.available_tags = list(set(item.get("project", "") for item in items))

                # 載入分類（工作項目）
                cats = load_categories(session_key)
                if cats:
                    self.categories = cats

                self.success_message = f"✅ 已載入 session: {self.emp_name}"
                if self.analysis_data:
                    self.current_step = 3  # 跳到資料整理步驟
        except Exception as e:
            self.error_message = f"載入失敗：{str(e)}"

    # ===== Token 機制 =====
    @rx.event
    def set_token_input(self, value: str):
        """設定 token 輸入值"""
        self.token_input = value.upper().strip()

    @rx.event
    def load_by_token(self):
        """透過 token 載入 session"""
        if not self.token_input:
            self.error_message = "請輸入 Token"
            return

        token = self.token_input.upper().strip()
        try:
            session = load_session(token)
            if session:
                self.session_key = token
                self.emp_name = session.get("emp_name", "")
                self.emp_dept = session.get("emp_dept", "")
                self.emp_title = session.get("emp_title", "")
                self.emp_start_date = session.get("emp_start_date", "")
                self.emp_manager = session.get("emp_manager", "")
                self.emp_period = session.get("emp_period", "")
                self.file_name = session.get("file_name", "")
                self.work_draft = session.get("work_draft", "")
                self.skill_draft = session.get("skill_draft", "")
                self.ethics_draft = session.get("ethics_draft", "")

                if session.get("analysis_data"):
                    self.analysis_data = session["analysis_data"]

                # 載入工作項目
                items = load_work_items(token)
                if items:
                    self.work_items = items
                    self.available_tags = list(set(item.get("project", "") for item in items))

                # 載入分類
                cats = load_categories(token)
                if cats:
                    self.categories = cats

                self.token_input = ""
                self.success_message = f"✅ 已載入工作進度"
                if self.analysis_data:
                    self.current_step = 3
            else:
                self.error_message = f"找不到此 Token 的資料：{token}"
        except Exception as e:
            self.error_message = f"載入失敗：{str(e)}"

    @rx.event
    def toggle_token_modal(self):
        """切換 token 對話框顯示"""
        self.show_token_modal = not self.show_token_modal

    @rx.event
    def close_token_modal(self):
        """關閉 token 對話框"""
        self.show_token_modal = False

    # ===== Session 管理 =====
    @rx.event
    def open_session_modal(self):
        """開啟 session 管理對話框"""
        self._load_token_history()
        self.show_session_modal = True

    @rx.event
    def close_session_modal(self):
        """關閉 session 管理對話框"""
        self.show_session_modal = False

    @rx.event
    def set_session_modal_tab(self, tab: str):
        """切換 session modal 的 tab"""
        self.session_modal_tab = tab

    @rx.event
    def clear_session(self):
        """清除當前 session，從頭開始"""
        # 重置所有狀態
        self.current_step = 1
        self.file_name = ""
        self.summary_text = ""
        self.project_text = ""
        self.issues_text = ""
        self.pe_text = ""
        self.analysis_data = {}
        self.work_items = []
        self.available_tags = []
        self.new_tag_input = ""
        self.generating_summary_for = -1
        self.selected_item_id = -1
        self.session_key = ""
        self.categories = []
        self.new_category_name = ""
        self.editing_category_id = -1
        self.expanded_category_id = -1
        self.edit_category_name = ""
        self.edit_category_description = ""

        # 重置員工資料
        self.emp_name = ""
        self.emp_dept = ""
        self.emp_title = ""
        self.emp_start_date = ""
        self.emp_manager = ""
        self.emp_period = "2025/1/1~2025/12/31"
        self.emp_sick_leave = 0.0
        self.emp_personal_leave = 0.0
        self.emp_absent = 0.0
        self.emp_division_manager = ""

        # 重置 AI 草稿
        self.work_draft = ""
        self.skill_draft = ""
        self.ethics_draft = ""

        # 重置 UI 狀態
        self.download_url = ""
        self.error_message = ""
        self.success_message = "✅ 已清除工作進度，可以從頭開始"
        self.filter_category_id = -999

        # 清除 localStorage 中的 token
        self.saved_token = ""

        # 關閉 modal
        self.show_session_modal = False

    @rx.event
    def switch_session(self, session_key: str):
        """切換到指定的 session"""
        self.close_session_modal()
        self.load_saved_session(session_key)

    @rx.event
    def handle_delete_session(self, session_key: str):
        """刪除指定的 session"""
        try:
            if delete_session(session_key):
                # 重新載入 session 列表
                self._load_recent_sessions()
                self._load_token_history()

                # 如果刪除的是當前 session，清除狀態
                if session_key == self.session_key:
                    self.clear_session()
                else:
                    self.success_message = f"✅ 已刪除 session: {session_key}"
            else:
                self.error_message = f"找不到 session: {session_key}"
        except Exception as e:
            self.error_message = f"刪除失敗：{str(e)}"

    @rx.event
    def load_by_token_and_close(self):
        """透過 token 載入 session 並關閉 modal"""
        if not self.token_input:
            self.error_message = "請輸入 Token"
            return

        token = self.token_input.upper().strip()
        try:
            from .services.db_service import load_session, load_work_items, load_categories
            session = load_session(token)
            if session:
                self.session_key = token
                self.emp_name = session.get("emp_name", "")
                self.emp_dept = session.get("emp_dept", "")
                self.emp_title = session.get("emp_title", "")
                self.emp_start_date = session.get("emp_start_date", "")
                self.emp_manager = session.get("emp_manager", "")
                self.emp_period = session.get("emp_period", "")
                self.file_name = session.get("file_name", "")
                self.work_draft = session.get("work_draft", "")
                self.skill_draft = session.get("skill_draft", "")
                self.ethics_draft = session.get("ethics_draft", "")

                if session.get("analysis_data"):
                    self.analysis_data = session["analysis_data"]

                # 載入工作項目
                items = load_work_items(token)
                if items:
                    self.work_items = items
                    self.available_tags = list(set(item.get("project", "") for item in items))

                # 載入分類
                cats = load_categories(token)
                if cats:
                    self.categories = cats

                # 更新 localStorage
                self.saved_token = token

                # 更新 token 歷史
                self._save_token_to_history(token, self.emp_name, self.file_name)

                self.token_input = ""
                self.success_message = f"✅ 已載入工作進度"
                self.show_session_modal = False
                if self.analysis_data:
                    self.current_step = 3
            else:
                self.error_message = f"找不到此 Token 的資料：{token}"
        except Exception as e:
            self.error_message = f"載入失敗：{str(e)}"

    # ===== 步驟導航 =====
    @rx.event
    def go_to_step(self, step: int):
        """跳轉到指定步驟"""
        can_go = False
        if step == 1:
            can_go = True
        elif step == 2:
            can_go = self.has_basic_info
        elif step == 3:
            can_go = self.has_basic_info and self.has_analysis
        elif step == 4:
            can_go = self.has_basic_info and self.has_analysis

        if can_go:
            self.current_step = step
            self.error_message = ""
            self.success_message = ""

    @rx.event
    def next_step(self):
        """下一步"""
        if self.current_step >= 4:
            return

        # 驗證每個步驟
        if self.current_step == 1 and not self.has_basic_info:
            self.error_message = "請填寫員工姓名"
            return
        if self.current_step == 2 and not self.has_analysis:
            self.error_message = "請先上傳並分析檔案"
            return

        self.current_step += 1
        self.error_message = ""

    @rx.event
    def prev_step(self):
        """上一步"""
        if self.current_step > 1:
            self.current_step -= 1
            self.error_message = ""

    @rx.event
    def set_step3_tab(self, tab: str):
        """切換 Step 3 的 Tab"""
        self.step3_active_tab = tab

    # ===== 檔案處理 =====
    @rx.event
    async def handle_upload(self, files: list[rx.UploadFile]):
        """處理檔案上傳"""
        self.uploading = True
        self.error_message = ""
        self.success_message = ""

        try:
            for file in files:
                self.file_name = file.filename
                upload_data = await file.read()
                outfile = rx.get_upload_dir() / file.filename

                with outfile.open("wb") as f:
                    f.write(upload_data)

                result = analyze_worklog(str(outfile))

                self.summary_text = result["summary_text"]
                self.project_text = result["project_text"]
                self.issues_text = result["issues_text"]
                self.pe_text = result["pe_text"]
                self.analysis_data = result["data"]

                if result["data"].get("user_name"):
                    self.emp_name = result["data"]["user_name"]

                # 建立可編輯工作項目列表
                self._build_work_items(result["data"])

                # 標記資料來源為 Excel，並生成新的 session token（無前綴）
                self.current_data_source = "xlsx"
                self.session_key = generate_session_token("")  # Excel 上傳不加前綴

                # 自動儲存
                self._auto_save()

                self.success_message = f"✅ 分析完成！共 {len(result['data'].get('project_issues', {}))} 個專案"

                # 顯示 Token 對話框並自動跳轉到下一步
                self.show_token_modal = True
                self.current_step = 3  # 直接跳到資料整理步驟

        except Exception as e:
            self.error_message = f"上傳錯誤：{str(e)}"
        finally:
            self.uploading = False

    def _build_work_items(self, data: dict):
        """從分析資料建立可編輯 Task 列表"""
        items = []
        project_issues = data.get("project_issues", {})
        issue_dates = data.get("issue_dates", {})  # 每個 issue 的日期範圍

        item_id = 0
        worklog_id = 0
        for project, issues in project_issues.items():
            for issue in issues:
                raw_worklogs = issue.get("descriptions", [])
                # 將 worklog 轉換為 dict 格式，支援獨立分類
                worklogs = []
                for text in raw_worklogs:
                    worklogs.append({
                        "id": worklog_id,
                        "text": text,
                        "work_item_id": -1,  # -1 表示跟隨 Task 的分類
                    })
                    worklog_id += 1

                # 取得此 issue 的日期範圍
                issue_key = issue.get("key", "")
                date_range = issue_dates.get(issue_key, {"start": "", "end": ""})

                items.append({
                    "id": item_id,
                    "project": project,
                    "issue_key": issue_key,
                    "issue_name": issue.get("name", ""),
                    "hours": issue.get("hours", 0),
                    "worklogs": worklogs,  # Worklog 物件列表
                    "summary": "",  # AI 生成的摘要（初始為空）
                    "tags": [project],  # 預設用專案名稱當標籤
                    "included": True,
                    "custom_note": "",
                    "category_id": -1,  # 未分類（工作項目 ID）
                    "date_start": date_range.get("start", ""),  # 日期範圍開始
                    "date_end": date_range.get("end", ""),  # 日期範圍結束
                })
                item_id += 1

        # 按工時排序
        items.sort(key=lambda x: x["hours"], reverse=True)
        self.work_items = items

        # 建立可用標籤（從專案名稱）
        self.available_tags = list(set(project_issues.keys()))

    # ===== 工作項目操作 =====
    @rx.event
    def select_item(self, item_id: int):
        """選擇要編輯的項目"""
        self.selected_item_id = item_id

    @rx.event
    def toggle_item_included(self, item_id: int):
        """切換項目是否納入報告"""
        for item in self.work_items:
            if item["id"] == item_id:
                item["included"] = not item["included"]
                break
        self.work_items = self.work_items.copy()

    @rx.event
    def update_item_note(self, item_id: int, note: str):
        """更新項目備註"""
        for item in self.work_items:
            if item["id"] == item_id:
                item["custom_note"] = note
                break
        self.work_items = self.work_items.copy()

    @rx.event
    def add_tag_to_item(self, item_id: int, tag: str):
        """為項目新增標籤"""
        if not tag:
            return
        for item in self.work_items:
            if item["id"] == item_id:
                if tag not in item["tags"]:
                    item["tags"].append(tag)
                break
        # 同時加入可用標籤
        if tag not in self.available_tags:
            self.available_tags.append(tag)
        self.work_items = self.work_items.copy()

    @rx.event
    def remove_tag_from_item(self, item_id: int, tag: str):
        """移除項目的標籤"""
        for item in self.work_items:
            if item["id"] == item_id:
                if tag in item["tags"]:
                    item["tags"].remove(tag)
                break
        self.work_items = self.work_items.copy()

    @rx.event
    def set_new_tag_input(self, value: str):
        """設定新標籤輸入"""
        self.new_tag_input = value

    @rx.event
    def add_new_tag(self):
        """新增全域標籤"""
        if self.new_tag_input and self.new_tag_input not in self.available_tags:
            self.available_tags.append(self.new_tag_input)
        self.new_tag_input = ""

    @rx.event
    def batch_add_tag(self, tag: str, item_ids: list[int]):
        """批次為多個項目加標籤"""
        for item in self.work_items:
            if item["id"] in item_ids and tag not in item["tags"]:
                item["tags"].append(tag)
        self.work_items = self.work_items.copy()

    # ===== 分類操作 =====
    @rx.event
    def set_new_category_name(self, value: str):
        """設定新分類名稱"""
        self.new_category_name = value

    @rx.event
    def add_category(self):
        """新增分類"""
        if not self.new_category_name.strip():
            return

        # 生成新 ID
        new_id = max([c["id"] for c in self.categories], default=-1) + 1

        # 預設顏色列表
        colors = ["primary", "secondary", "accent", "info", "success", "warning", "error"]
        color = colors[new_id % len(colors)]

        self.categories.append({
            "id": new_id,
            "name": self.new_category_name.strip(),
            "color": color,
            "description": "",
        })
        self.new_category_name = ""
        self.categories = self.categories.copy()

    @rx.event
    def delete_category(self, category_id: int):
        """刪除分類"""
        # 將該分類下的項目設為未分類
        for item in self.work_items:
            if item.get("category_id") == category_id:
                item["category_id"] = -1
        self.work_items = self.work_items.copy()

        # 刪除分類
        self.categories = [c for c in self.categories if c["id"] != category_id]

    @rx.event
    def update_category_name(self, category_id: int, name: str):
        """更新分類名稱"""
        for cat in self.categories:
            if cat["id"] == category_id:
                cat["name"] = name
                break
        self.categories = self.categories.copy()

    # ===== 工作項目編輯 Modal =====
    @rx.event
    def open_category_modal(self, category_id: int):
        """開啟工作項目編輯 Modal"""
        self.expanded_category_id = category_id
        for cat in self.categories:
            if cat.get("id") == category_id:
                self.edit_category_name = cat.get("name", "")
                self.edit_category_description = cat.get("description", "")
                break

    @rx.event
    def close_category_modal(self):
        """關閉工作項目編輯 Modal"""
        self.expanded_category_id = -1
        self.edit_category_name = ""
        self.edit_category_description = ""

    @rx.event
    def set_edit_category_name(self, value: str):
        """設定編輯中的工作項目名稱"""
        self.edit_category_name = value

    @rx.event
    def set_edit_category_description(self, value: str):
        """設定編輯中的工作項目描述"""
        self.edit_category_description = value

    @rx.event
    def save_category_details(self):
        """儲存工作項目詳細資料"""
        if self.expanded_category_id < 0:
            return
        for cat in self.categories:
            if cat.get("id") == self.expanded_category_id:
                cat["name"] = self.edit_category_name.strip()
                cat["description"] = self.edit_category_description.strip()
                break
        self.categories = self.categories.copy()
        self._auto_save()
        self.success_message = "✅ 工作項目已更新"
        self.close_category_modal()

    @rx.event
    def remove_task_from_category(self, task_id: int):
        """從目前工作項目中移除 Task（設為未分類）"""
        for item in self.work_items:
            if item["id"] == task_id:
                item["category_id"] = -1
                break
        self.work_items = self.work_items.copy()
        self._auto_save()

    @rx.event
    def remove_worklog_from_category(self, task_id: int, worklog_id: int):
        """從目前工作項目中移除 Worklog（設為跟隨 Task）"""
        for item in self.work_items:
            if item["id"] == task_id:
                for wl in item.get("worklogs", []):
                    if isinstance(wl, dict) and wl.get("id") == worklog_id:
                        wl["work_item_id"] = -1  # 跟隨 Task
                        break
                break
        self.work_items = self.work_items.copy()
        self._auto_save()

    @rx.event
    def assign_item_to_category(self, item_id: int, category_id: int):
        """將工作項目指派到分類"""
        for item in self.work_items:
            if item["id"] == item_id:
                item["category_id"] = category_id
                break
        self.work_items = self.work_items.copy()

    @rx.event
    def assign_item_to_category_str(self, item_id: int, category_id_str: str):
        """將工作項目指派到分類（接受字串參數）"""
        try:
            category_id = int(category_id_str)
        except (ValueError, TypeError):
            category_id = -1
        for item in self.work_items:
            if item["id"] == item_id:
                item["category_id"] = category_id
                break
        self.work_items = self.work_items.copy()

    @rx.event
    def batch_assign_to_category(self, item_ids: list[int], category_id: int):
        """批次將多個項目指派到分類"""
        for item in self.work_items:
            if item["id"] in item_ids:
                item["category_id"] = category_id
        self.work_items = self.work_items.copy()

    @rx.event
    def auto_create_categories_from_projects(self):
        """根據現有專案自動建立工作項目（簡易版）"""
        existing_names = {c["name"] for c in self.categories}
        projects = set(item.get("project", "") for item in self.work_items if item.get("project"))

        colors = ["primary", "secondary", "accent", "info", "success", "warning", "error"]
        new_id = max([c["id"] for c in self.categories], default=-1) + 1

        for project in sorted(projects):
            if project not in existing_names:
                self.categories.append({
                    "id": new_id,
                    "name": project,
                    "color": colors[new_id % len(colors)],
                    "description": f"從專案 {project} 自動建立",
                })
                new_id += 1

        # 自動將 Task 指派到對應工作項目
        for item in self.work_items:
            project = item.get("project", "")
            for cat in self.categories:
                if cat["name"] == project:
                    item["category_id"] = cat["id"]
                    break

        self.categories = self.categories.copy()
        self.work_items = self.work_items.copy()
        self.success_message = f"✅ 已建立 {len(projects)} 個工作項目"

    @rx.event(background=True)
    async def ai_auto_categorize(self):
        """使用 AI 根據現有分類自動將 Worklogs 分配到適當的工作項目（細粒度到 Worklog 層級）"""
        async with self:
            if not self.categories:
                self.error_message = "❌ 請先新增工作項目分類"
                return

            if not self.work_items:
                self.error_message = "❌ 請先上傳工時資料"
                return

            # 檢查 API 設定
            llm_config = get_llm_config()
            api_url = self.api_url or llm_config.get("api_url", "")
            api_key = self.api_key or llm_config.get("api_key", "")
            model = self.model or llm_config.get("model", "")

            if not api_key:
                self.error_message = "❌ 請先設定 API Key"
                return

            # 開啟進度指示
            self.is_busy = True
            self.busy_title = "AI 自動分類中"
            self.busy_message = "正在分析 Worklogs 並分配到工作項目..."
            self.busy_progress = 0

        try:
            from .services.llm_service import auto_categorize_worklogs

            # 呼叫 AI 分類（Worklog 層級）
            result = await auto_categorize_worklogs(
                categories=self.categories,
                work_items=self.work_items,
                api_url=api_url,
                api_key=api_key,
                model=model,
            )

            async with self:
                if not result:
                    self.error_message = "❌ AI 分類失敗，請檢查設定後重試"
                    self.is_busy = False
                    return

                # 套用分類結果到每個 Worklog
                categorized_count = 0
                total_worklogs = 0

                for item in self.work_items:
                    task_id = item.get("id", 0)
                    worklogs = item.get("worklogs", [])

                    for idx, wl in enumerate(worklogs):
                        if isinstance(wl, dict):
                            total_worklogs += 1
                            key = f"{task_id}:{idx}"
                            if key in result:
                                new_cat_id = result[key]
                                wl["work_item_id"] = new_cat_id
                                if new_cat_id >= 0:
                                    categorized_count += 1

                    # 更新 item 的 worklogs
                    item["worklogs"] = worklogs

                    # 同時更新 Task 的 category_id（使用最多 worklogs 所屬的分類）
                    cat_counts = {}
                    for wl in worklogs:
                        if isinstance(wl, dict):
                            cat_id = wl.get("work_item_id", -1)
                            if cat_id >= 0:
                                cat_counts[cat_id] = cat_counts.get(cat_id, 0) + 1
                    if cat_counts:
                        # 找出最多 worklogs 的分類
                        dominant_cat = max(cat_counts.keys(), key=lambda k: cat_counts[k])
                        item["category_id"] = dominant_cat
                    else:
                        item["category_id"] = -1

                self.work_items = self.work_items.copy()
                self._auto_save()

                self.is_busy = False
                self.success_message = f"✅ AI 已將 {categorized_count}/{total_worklogs} 個 Worklogs 分配到工作項目"

        except Exception as e:
            async with self:
                self.is_busy = False
                self.error_message = f"❌ AI 分類錯誤：{str(e)}"

    # ===== 拖拉功能 =====
    @rx.event
    def start_drag_task(self, task_id: int):
        """開始拖拉 Task"""
        self.dragging_task_id = task_id
        self.dragging_worklog_id = -1
        self.drag_source_task_id = -1

    @rx.event
    def start_drag_worklog(self, task_id: int, worklog_id: int):
        """開始拖拉 Worklog"""
        self.dragging_task_id = -1
        self.dragging_worklog_id = worklog_id
        self.drag_source_task_id = task_id

    @rx.event
    def end_drag(self):
        """結束拖拉"""
        self.dragging_task_id = -1
        self.dragging_worklog_id = -1
        self.drag_source_task_id = -1

    @rx.event
    def drop_on_category(self, category_id: int):
        """將拖拉中的項目放到分類"""
        if self.dragging_task_id >= 0:
            # 拖拉的是 Task
            for item in self.work_items:
                if item["id"] == self.dragging_task_id:
                    item["category_id"] = category_id
                    break
            self.work_items = self.work_items.copy()
            self._auto_save()
        elif self.dragging_worklog_id >= 0 and self.drag_source_task_id >= 0:
            # 拖拉的是 Worklog - 更新該 worklog 的分類
            for item in self.work_items:
                if item["id"] == self.drag_source_task_id:
                    for wl in item.get("worklogs", []):
                        if isinstance(wl, dict) and wl.get("id") == self.dragging_worklog_id:
                            wl["category_id"] = category_id
                            break
                    break
            self.work_items = self.work_items.copy()
            self._auto_save()

        # 清除拖拉狀態
        self.dragging_task_id = -1
        self.dragging_worklog_id = -1
        self.drag_source_task_id = -1

    # ===== AI 建議工作項目 =====
    suggesting_work_items: bool = False

    @rx.event
    async def ai_suggest_work_items(self):
        """使用 AI 分析 Tasks 並建議工作項目分類"""
        if self.is_busy:
            return

        if not self.work_items:
            self.error_message = "請先上傳並分析檔案"
            return

        if not self.api_key:
            self.error_message = "請先設定 API Key"
            return

        self.suggesting_work_items = True
        self._start_busy("AI 智慧分類", f"正在分析 {len(self.work_items)} 個 Tasks...")
        yield  # 讓 UI 更新顯示進度對話框

        try:
            from .services.llm_service import suggest_work_items_from_tasks

            suggestions = await suggest_work_items_from_tasks(
                work_items=self.work_items,
                api_url=self.api_url,
                api_key=self.api_key,
                model=self.model,
            )

            if not suggestions:
                self._end_busy(error_message="AI 無法產生建議，請稍後再試")
                return

            self._update_busy("正在建立工作項目...")

            # 清空現有分類
            self.categories = []

            colors = ["primary", "secondary", "accent", "info", "success", "warning", "error"]

            for i, suggestion in enumerate(suggestions):
                cat_id = i
                self.categories.append({
                    "id": cat_id,
                    "name": suggestion.get("name", f"工作項目 {i+1}"),
                    "color": colors[i % len(colors)],
                    "description": suggestion.get("description", ""),
                })

                # 將建議的 Tasks 指派到此工作項目
                task_ids = suggestion.get("task_ids", [])
                for item in self.work_items:
                    if item["id"] in task_ids:
                        item["category_id"] = cat_id

            self.categories = self.categories.copy()
            self.work_items = self.work_items.copy()
            self._end_busy(success_message=f"✅ AI 已建議 {len(suggestions)} 個工作項目並自動分類")
            self._auto_save()

        except Exception as e:
            self._end_busy(error_message=f"AI 建議錯誤：{str(e)}")
        finally:
            self.suggesting_work_items = False

    @rx.event
    def set_filter_category(self, category_id_str: str):
        """設定篩選的工作項目"""
        try:
            self.filter_category_id = int(category_id_str)
        except (ValueError, TypeError):
            self.filter_category_id = -999  # 全部

    # ===== Worklog 操作 =====
    def _get_next_worklog_id(self) -> int:
        """取得下一個 worklog ID"""
        max_id = -1
        for item in self.work_items:
            for wl in item.get("worklogs", []):
                if isinstance(wl, dict) and wl.get("id", -1) > max_id:
                    max_id = wl.get("id", -1)
        return max_id + 1

    @rx.event
    def add_worklog(self, item_id: int):
        """新增空白 worklog"""
        for item in self.work_items:
            if item["id"] == item_id:
                new_wl = {
                    "id": self._get_next_worklog_id(),
                    "text": "",
                    "work_item_id": -1,  # 跟隨 Task 的分類
                }
                item["worklogs"].append(new_wl)
                break
        self.work_items = self.work_items.copy()

    @rx.event
    def remove_worklog(self, item_id: int, worklog_id: int):
        """移除指定 worklog"""
        for item in self.work_items:
            if item["id"] == item_id:
                item["worklogs"] = [wl for wl in item["worklogs"]
                                    if not (isinstance(wl, dict) and wl.get("id") == worklog_id)]
                break
        self.work_items = self.work_items.copy()

    @rx.event
    def update_worklog_text(self, item_id: int, worklog_id: int, text: str):
        """更新指定 worklog 內容"""
        for item in self.work_items:
            if item["id"] == item_id:
                for wl in item["worklogs"]:
                    if isinstance(wl, dict) and wl.get("id") == worklog_id:
                        wl["text"] = text
                        break
                break
        self.work_items = self.work_items.copy()

    @rx.event
    def assign_worklog_to_work_item(self, item_id: int, worklog_id: int, work_item_id_str: str):
        """將 worklog 獨立指派到工作項目"""
        try:
            work_item_id = int(work_item_id_str)
        except (ValueError, TypeError):
            work_item_id = -1

        for item in self.work_items:
            if item["id"] == item_id:
                for wl in item["worklogs"]:
                    if isinstance(wl, dict) and wl.get("id") == worklog_id:
                        wl["work_item_id"] = work_item_id
                        break
                break
        self.work_items = self.work_items.copy()

    @rx.event
    def update_item_summary(self, item_id: int, text: str):
        """手動更新項目摘要"""
        for item in self.work_items:
            if item["id"] == item_id:
                item["summary"] = text
                break
        self.work_items = self.work_items.copy()

    def _extract_worklog_texts(self, worklogs: list) -> list[str]:
        """從 worklog 列表提取文字內容"""
        texts = []
        for wl in worklogs:
            if isinstance(wl, dict):
                text = wl.get("text", "")
            else:
                text = str(wl)
            if text:
                texts.append(text)
        return texts

    @rx.event
    async def regenerate_item_summary(self, item_id: int):
        """重新生成單一 Task 的摘要（非同步）"""
        if not self.api_key:
            self.error_message = "請先設定 API Key"
            return

        self.generating_summary_for = item_id
        self.error_message = ""

        try:
            for item in self.work_items:
                if item["id"] == item_id:
                    worklogs = item.get("worklogs", [])
                    worklog_texts = self._extract_worklog_texts(worklogs)
                    if not worklog_texts:
                        item["summary"] = ""
                        break

                    # 使用 await 呼叫非同步函數
                    summary = await generate_item_summary(
                        worklogs=worklog_texts,
                        issue_name=item.get("issue_name", ""),
                        hours=item.get("hours", 0),
                        api_url=self.api_url,
                        api_key=self.api_key,
                        model=self.model
                    )
                    item["summary"] = summary
                    break
            self.work_items = self.work_items.copy()
        except Exception as e:
            self.error_message = f"生成摘要錯誤：{str(e)}"
        finally:
            self.generating_summary_for = -1

    @rx.event
    async def generate_all_summaries(self):
        """為所有工作項目（分類）生成摘要（非同步）"""
        if self.is_busy:
            return

        if not self.api_key:
            self.error_message = "請先設定 API Key"
            return

        # 找出有分配 Task 的分類
        categories_with_tasks = []
        for cat in self.categories:
            cat_tasks = [item for item in self.work_items
                        if item.get("included", True) and item.get("category_id") == cat.get("id")]
            if cat_tasks:
                categories_with_tasks.append((cat, cat_tasks))

        if not categories_with_tasks:
            self.error_message = "沒有已分類的工作項目需要生成摘要"
            return

        self._start_busy("生成摘要", f"準備處理 {len(categories_with_tasks)} 個工作項目...", len(categories_with_tasks))
        yield  # 讓 UI 更新顯示進度對話框

        for idx, (cat, cat_tasks) in enumerate(categories_with_tasks):
            self.generating_summary_for = cat["id"]
            self._update_busy(f"處理中：{cat.get('name', '')} ({idx + 1}/{len(categories_with_tasks)})", idx + 1)
            yield  # 讓 UI 更新顯示當前進度
            try:
                # 使用 await 呼叫非同步函數
                summary = await generate_category_summary(
                    category_name=cat.get("name", ""),
                    category_description=cat.get("description", ""),
                    tasks=cat_tasks,
                    api_url=self.api_url,
                    api_key=self.api_key,
                    model=self.model
                )
                cat["summary"] = summary
            except Exception as e:
                self.generating_summary_for = -1
                self._end_busy(error_message=f"生成摘要錯誤：{str(e)}")
                return

        self.categories = self.categories.copy()
        self.generating_summary_for = -1
        self._end_busy(success_message="✅ 所有工作項目摘要生成完成！")
        self._auto_save()  # 自動儲存

    @rx.event
    async def regenerate_category_summary(self, category_id: int):
        """重新生成單一工作項目（分類）的摘要（非同步）"""
        if self.is_busy:
            return

        if not self.api_key:
            self.error_message = "請先設定 API Key"
            return

        # 找到分類
        cat = next((c for c in self.categories if c.get("id") == category_id), None)
        if not cat:
            return

        # 找到分類下的 tasks
        cat_tasks = [item for item in self.work_items
                    if item.get("included", True) and item.get("category_id") == category_id]

        if not cat_tasks:
            self.error_message = "此工作項目沒有已納入的 Task"
            return

        self.generating_summary_for = category_id
        try:
            summary = await generate_category_summary(
                category_name=cat.get("name", ""),
                category_description=cat.get("description", ""),
                tasks=cat_tasks,
                api_url=self.api_url,
                api_key=self.api_key,
                model=self.model
            )
            cat["summary"] = summary
            self.categories = self.categories.copy()
            self.success_message = f"✅ 已更新「{cat.get('name', '')}」的摘要"
            self._auto_save()
        except Exception as e:
            self.error_message = f"生成摘要錯誤：{str(e)}"
        finally:
            self.generating_summary_for = -1

    @rx.event
    def update_category_summary(self, category_id: int, text: str):
        """手動更新工作項目（分類）摘要"""
        for cat in self.categories:
            if cat.get("id") == category_id:
                cat["summary"] = text
                break
        self.categories = self.categories.copy()
        self._auto_save()

    # ===== AI 草稿 =====
    @rx.event
    async def generate_drafts(self):
        """產生 AI 草稿 - 使用編輯後的工作項目"""
        if self.is_busy:
            return

        if not self.work_items:
            self.error_message = "請先上傳並分析檔案"
            return

        # 檢查是否有納入的項目
        included_items = [item for item in self.work_items if item.get("included", True)]
        if not included_items:
            self.error_message = "請至少選擇一個工作項目"
            return

        if not self.api_key:
            self.error_message = "請先設定 API Key"
            return

        self.is_generating = True
        self._start_busy("產生績效草稿", "正在分析工作項目並產生草稿...")

        try:
            work, skill, ethics = generate_all_drafts(
                self.api_url,
                self.api_key,
                self.model,
                self.position_type,
                self.custom_style,
                self.work_items,  # 使用編輯後的工作項目
                self.categories   # 傳遞分類
            )
            self.work_draft = work
            self.skill_draft = skill
            self.ethics_draft = ethics
            self._end_busy(success_message="✅ AI 草稿產生完成！")
            self._auto_save()  # 自動儲存
        except Exception as e:
            self._end_busy(error_message=f"生成草稿錯誤：{str(e)}")
        finally:
            self.is_generating = False

    # ===== 匯出 =====
    @rx.event
    def export_excel(self):
        """匯出 Excel"""
        import shutil
        import os

        if not self.analysis_data:
            self.error_message = "請先上傳並分析檔案"
            return

        self.is_exporting = True
        self.error_message = ""

        try:
            file_path = export_to_excel(
                emp_name=self.emp_name,
                emp_dept=self.emp_dept,
                emp_title=self.emp_title,
                emp_start_date=self.emp_start_date,
                emp_manager=self.emp_manager,
                emp_period=self.emp_period,
                emp_sick_leave=self.emp_sick_leave,
                emp_personal_leave=self.emp_personal_leave,
                emp_absent=self.emp_absent,
                work_draft=self.work_draft,
                skill_draft=self.skill_draft,
                ethics_draft=self.ethics_draft,
                analysis_data=self.analysis_data
            )

            if file_path:
                # 複製檔案到 .web/public 目錄供下載 (Next.js 的公開目錄)
                public_dir = os.path.join(os.getcwd(), ".web", "public", "exports")
                os.makedirs(public_dir, exist_ok=True)

                # 取得檔名
                filename = os.path.basename(file_path)
                dest_path = os.path.join(public_dir, filename)

                # 複製檔案
                shutil.copy(file_path, dest_path)

                # 設定可供下載的 URL (.web/public 目錄由 Next.js 提供服務)
                self.download_url = f"/exports/{filename}"
                self.success_message = "✅ Excel 匯出成功！"
            else:
                self.error_message = "匯出失敗"
        except Exception as e:
            self.error_message = f"匯出錯誤：{str(e)}"
        finally:
            self.is_exporting = False

    # ===== 重置 =====
    @rx.event
    def reset_all(self):
        """重置所有資料"""
        self.current_step = 1
        self.file_name = ""
        self.summary_text = ""
        self.project_text = ""
        self.issues_text = ""
        self.pe_text = ""
        self.analysis_data = {}
        self.emp_name = ""
        self.emp_dept = ""
        self.emp_title = ""
        self.emp_start_date = ""
        self.emp_manager = ""
        self.emp_period = "2025/1/1~2025/12/31"
        self.emp_sick_leave = 0.0
        self.emp_personal_leave = 0.0
        self.emp_absent = 0.0
        self.work_draft = ""
        self.skill_draft = ""
        self.ethics_draft = ""
        self.download_url = ""
        self.error_message = ""
        self.success_message = ""
        self.work_items = []
        self.available_tags = []
        self.new_tag_input = ""

    # ===== Tab 切換 =====
    @rx.event
    def set_analysis_tab(self, tab: str):
        self.analysis_tab = tab

    # ===== Setters =====
    @rx.event
    def set_emp_name(self, value: str):
        self.emp_name = value

    @rx.event
    def set_emp_dept(self, value: str):
        self.emp_dept = value

    @rx.event
    def set_emp_title(self, value: str):
        self.emp_title = value

    @rx.event
    def set_emp_start_date(self, value: str):
        self.emp_start_date = value

    @rx.event
    def set_emp_manager(self, value: str):
        self.emp_manager = value

    @rx.event
    def set_emp_period(self, value: str):
        self.emp_period = value

    @rx.event
    def set_emp_sick_leave(self, value: str):
        try:
            self.emp_sick_leave = float(value) if value else 0.0
        except ValueError:
            pass

    @rx.event
    def set_emp_personal_leave(self, value: str):
        try:
            self.emp_personal_leave = float(value) if value else 0.0
        except ValueError:
            pass

    @rx.event
    def set_emp_absent(self, value: str):
        try:
            self.emp_absent = float(value) if value else 0.0
        except ValueError:
            pass

    @rx.event
    def set_api_url(self, value: str):
        self.api_url = value

    @rx.event
    def set_api_key(self, value: str):
        self.api_key = value

    @rx.event
    def set_model(self, value: str):
        self.model = value

    @rx.event
    def set_position_type(self, value: str):
        self.position_type = value

    @rx.event
    def set_custom_style(self, value: str):
        self.custom_style = value

    @rx.event
    def set_work_draft(self, value: str):
        self.work_draft = value

    @rx.event
    def set_skill_draft(self, value: str):
        self.skill_draft = value

    @rx.event
    def set_ethics_draft(self, value: str):
        self.ethics_draft = value

    @rx.event
    def set_emp_division_manager(self, value: str):
        self.emp_division_manager = value

    # ===== 設定頁面操作 =====
    @rx.event
    def toggle_settings(self):
        """切換設定頁面顯示"""
        self.show_settings = not self.show_settings
        if not self.show_settings:
            # 關閉時清空編輯狀態
            self._clear_dept_edit_form()

    @rx.event
    def toggle_basic_settings(self):
        """切換基本設定面板展開狀態"""
        self.show_basic_settings = not self.show_basic_settings

    def _save_titles(self):
        """儲存職稱到 SQLite"""
        set_option("titles", self.title_options)

    def _save_departments(self):
        """儲存部門到 SQLite"""
        save_departments(self.departments)

    # ===== 職稱管理 =====
    @rx.event
    def set_new_title(self, value: str):
        self.new_title = value

    @rx.event
    def add_new_title(self):
        """新增職稱選項"""
        if self.new_title.strip() and self.new_title.strip() not in self.title_options:
            self.title_options.append(self.new_title.strip())
            self.title_options = self.title_options.copy()
            self._save_titles()
        self.new_title = ""

    @rx.event
    def remove_title(self, title: str):
        """移除職稱選項"""
        if title in self.title_options:
            self.title_options.remove(title)
            self.title_options = self.title_options.copy()
            self._save_titles()

    # ===== 部門編輯表單 =====
    @rx.event
    def set_edit_dept_name(self, value: str):
        self.edit_dept_name = value

    @rx.event
    def set_edit_dept_division(self, value: str):
        self.edit_dept_division = value

    @rx.event
    def set_edit_dept_division_manager(self, value: str):
        self.edit_dept_division_manager = value

    @rx.event
    def set_edit_dept_manager(self, value: str):
        self.edit_dept_manager = value

    @rx.event
    def set_edit_dept_functions(self, value: str):
        self.edit_dept_functions = value

    def _clear_dept_edit_form(self):
        """清空部門編輯表單"""
        self.editing_dept_id = -1
        self.edit_dept_name = ""
        self.edit_dept_division = ""
        self.edit_dept_division_manager = ""
        self.edit_dept_manager = ""
        self.edit_dept_functions = ""

    @rx.event
    def start_new_dept(self):
        """開始新增部門"""
        self._clear_dept_edit_form()
        self.editing_dept_id = -2  # -2 表示新增模式

    @rx.event
    def edit_dept(self, dept_id: int):
        """開始編輯部門"""
        self._ensure_dept_format()
        for dept in self.departments:
            if isinstance(dept, dict) and dept.get("id") == dept_id:
                self.editing_dept_id = dept_id
                self.edit_dept_name = dept.get("name", "")
                self.edit_dept_division = dept.get("division", "")
                self.edit_dept_division_manager = dept.get("division_manager", "")
                self.edit_dept_manager = dept.get("dept_manager", "")
                self.edit_dept_functions = dept.get("functions", "")
                break

    @rx.event
    def cancel_dept_edit(self):
        """取消編輯"""
        self._clear_dept_edit_form()

    def _ensure_dept_format(self):
        """確保部門資料為新格式"""
        if not self.departments:
            return
        # 檢查是否為舊格式（字串列表）
        if self.departments and isinstance(self.departments[0], str):
            self.departments = [
                {"id": i, "name": name, "division": "", "division_manager": "", "dept_manager": "", "functions": ""}
                for i, name in enumerate(self.departments)
            ]

    @rx.event
    def save_dept(self):
        """儲存部門（新增或更新）"""
        if not self.edit_dept_name.strip():
            self.error_message = "請輸入部門名稱"
            return

        # 確保格式正確
        self._ensure_dept_format()

        dept_data = {
            "name": self.edit_dept_name.strip(),
            "division": self.edit_dept_division.strip(),
            "division_manager": self.edit_dept_division_manager.strip(),
            "dept_manager": self.edit_dept_manager.strip(),
            "functions": self.edit_dept_functions.strip(),
        }

        if self.editing_dept_id == -2:
            # 新增模式
            max_id = -1
            for d in self.departments:
                if isinstance(d, dict) and d.get("id", -1) > max_id:
                    max_id = d.get("id", -1)
            dept_data["id"] = max_id + 1
            self.departments.append(dept_data)
            self.success_message = f"✅ 已新增部門：{dept_data['name']}"
        else:
            # 編輯模式
            for i, dept in enumerate(self.departments):
                if isinstance(dept, dict) and dept.get("id") == self.editing_dept_id:
                    dept_data["id"] = self.editing_dept_id
                    self.departments[i] = dept_data
                    self.success_message = f"✅ 已更新部門：{dept_data['name']}"
                    break

        self.departments = self.departments.copy()
        self._save_departments()
        self._clear_dept_edit_form()

    @rx.event
    def delete_dept(self, dept_id: int):
        """刪除部門"""
        self._ensure_dept_format()
        self.departments = [d for d in self.departments if isinstance(d, dict) and d.get("id") != dept_id]
        self._save_departments()
        if self.editing_dept_id == dept_id:
            self._clear_dept_edit_form()

    # ===== 員工部門選擇 =====
    @rx.event
    def select_emp_dept(self, dept_name: str):
        """選擇員工部門，自動帶入主管資訊"""
        self.emp_dept = dept_name
        self._ensure_dept_format()
        # 查找部門資料並帶入主管
        for dept in self.departments:
            if isinstance(dept, dict) and dept.get("name") == dept_name:
                self.emp_manager = dept.get("dept_manager", "")
                self.emp_division_manager = dept.get("division_manager", "")
                break

    # ===== Jira/Tempo 操作 =====
    @rx.event
    def set_jira_url(self, value: str):
        """設定 Jira URL"""
        self.jira_url = value.strip()

    @rx.event
    def set_jira_username(self, value: str):
        """設定 Jira 使用者名稱"""
        self.jira_username = value.strip()

    @rx.event
    def set_jira_token(self, value: str):
        """設定 Jira Token"""
        self.jira_token = value

    @rx.event
    def set_jira_date_from(self, value: str):
        """設定 Jira 查詢開始日期"""
        self.jira_date_from = value

    @rx.event
    def set_jira_date_to(self, value: str):
        """設定 Jira 查詢結束日期"""
        self.jira_date_to = value

    @rx.event
    def toggle_jira_settings(self):
        """切換 Jira 設定對話框"""
        self.show_jira_settings = not self.show_jira_settings
        if self.show_jira_settings:
            self.jira_connection_status = ""

    @rx.event
    def close_jira_settings(self):
        """關閉 Jira 設定對話框"""
        self.show_jira_settings = False

    @rx.event
    def save_jira_settings(self):
        """儲存 Jira 設定"""
        save_jira_options({
            "url": self.jira_url,
            "username": self.jira_username,
            "token": self.jira_token,
            "date_from": self.jira_date_from,
            "date_to": self.jira_date_to,
        })
        self.success_message = "✅ Jira 設定已儲存"

    @rx.event
    def test_jira_connection(self):
        """測試 Jira 連線（使用 PAT 自動取得使用者資訊）"""
        if not self.jira_url or not self.jira_token:
            self.jira_connection_status = "❌ 請填寫 Jira URL 和 PAT"
            return

        self.jira_connection_status = "⏳ 測試中..."

        # 先用 PAT 取得使用者資訊
        user_info = get_user_info_from_pat(self.jira_url, self.jira_token)

        if not user_info.get("success"):
            self.jira_connection_status = f"❌ {user_info.get('error', 'PAT 驗證失敗')}"
            return

        # 更新使用者資訊
        self.jira_username = user_info.get("username", "")
        self.jira_user_key = user_info.get("user_key", "")
        self.jira_display_name = user_info.get("display_name", "")

        # 測試 Tempo API
        result = test_connection(
            jira_url=self.jira_url,
            username=self.jira_username,
            token=self.jira_token
        )

        if result.get("success"):
            self.jira_connection_status = f"✅ 連線成功！使用者：{self.jira_display_name}"
        else:
            self.jira_connection_status = f"❌ {result.get('message', '連線失敗')}"

    @rx.event
    async def fetch_from_jira(self):
        """從 Jira Tempo 抓取 worklog 資料（使用 PAT 自動取得使用者資訊）"""
        if not self.jira_url:
            self.error_message = "請先在「設定」中配置 Jira URL"
            self.show_jira_settings = True
            return

        if not self.jira_token:
            self.error_message = "請輸入您的 Personal Access Token (PAT)"
            return

        if not self.jira_date_from or not self.jira_date_to:
            self.error_message = "請設定查詢日期範圍"
            return

        self.jira_loading = True
        self.error_message = ""
        self.success_message = ""
        self._start_busy("從 Jira 載入資料", "正在驗證 PAT...")

        try:
            # 使用 PAT 自動取得使用者資訊
            user_info = get_user_info_from_pat(self.jira_url, self.jira_token)

            if not user_info.get("success"):
                self._end_busy(error_message=user_info.get("error", "PAT 驗證失敗"))
                self.jira_loading = False
                return

            # 儲存使用者資訊
            self.jira_username = user_info.get("username", "")
            self.jira_user_key = user_info.get("user_key", "")
            self.jira_display_name = user_info.get("display_name", "")

            self._update_busy(f"已驗證使用者：{self.jira_display_name}")

            # 抓取 worklogs
            self._update_busy("正在抓取 worklog 資料...")
            result = fetch_worklogs(
                jira_url=self.jira_url,
                username=self.jira_username,  # 使用從 PAT 取得的 username
                token=self.jira_token,
                date_from=self.jira_date_from,
                date_to=self.jira_date_to
            )

            if not result.get("success"):
                self._end_busy(error_message=result.get("error", "抓取失敗"))
                self.jira_loading = False
                return

            worklogs = result.get("worklogs", [])
            user_name = result.get("user_name", "") or self.jira_display_name

            if not worklogs:
                self._end_busy(error_message="找不到任何 worklog 資料")
                self.jira_loading = False
                return

            # 轉換為分析結果格式
            self._update_busy("正在分析資料...")
            analysis_result = transform_to_analysis_result(worklogs, user_name)

            # 標記資料來源為 Jira，並強制生成新的 JIRA_ session token
            self.current_data_source = "jira"
            self.session_key = generate_session_token("JIRA_")  # 強制生成 JIRA_ 前綴的 token

            # 更新狀態（與 handle_upload 相同的處理）
            self.file_name = f"Jira_{self.jira_username}_{self.jira_date_from}_{self.jira_date_to}"
            self.summary_text = analysis_result["summary_text"]
            self.project_text = analysis_result["project_text"]
            self.issues_text = analysis_result["issues_text"]
            self.pe_text = analysis_result["pe_text"]
            self.analysis_data = analysis_result["data"]

            if user_name:
                self.emp_name = user_name

            # 設定考核期間
            date_range = analysis_result["data"].get("date_range", {})
            if date_range.get("start") and date_range.get("end"):
                self.emp_period = f"{date_range['start']}~{date_range['end']}"

            # 建立可編輯工作項目列表
            self._build_work_items(analysis_result["data"])

            # 自動儲存（會包含 Jira PAT 和使用者資訊）
            self._auto_save()

            self._end_busy(success_message=f"✅ 已載入 {len(worklogs)} 筆 worklog 資料")

            # 顯示 Token 對話框並跳轉到下一步
            self.show_token_modal = True
            self.current_step = 3

        except Exception as e:
            self._end_busy(error_message=f"載入錯誤：{str(e)}")
        finally:
            self.jira_loading = False

    # ===== Computed Properties =====
    @rx.var
    def dept_names(self) -> List[str]:
        """部門名稱列表"""
        result = []
        for d in self.departments:
            if isinstance(d, dict) and d.get("name"):
                result.append(d.get("name", ""))
        return result

    @rx.var
    def is_editing_dept(self) -> bool:
        """是否正在編輯部門"""
        return self.editing_dept_id != -1

    @rx.var
    def is_new_dept_mode(self) -> bool:
        """是否為新增部門模式"""
        return self.editing_dept_id == -2

    @rx.var
    def has_jira_config(self) -> bool:
        """是否已設定 Jira URL（PAT 在 Step 2 輸入）"""
        return bool(self.jira_url)

    @rx.var
    def categories_with_count(self) -> List[dict]:
        """工作項目列表，含已指派的 Task 數量和詳細內容"""
        result = []
        for cat in self.categories:
            cat_id = cat.get("id")

            # 找出直接歸類到此分類的 Tasks
            assigned_tasks = []
            for item in self.work_items:
                if item.get("category_id") == cat_id and item.get("included", True):
                    assigned_tasks.append({
                        "id": item.get("id"),
                        "issue_key": item.get("issue_key", ""),
                        "issue_name": item.get("issue_name", ""),
                        "hours": item.get("hours", 0),
                    })

            # 找出獨立歸類到此分類的 Worklogs（不跟隨 Task）
            orphan_worklogs = []
            for item in self.work_items:
                # Task 不在此分類，但其中有 worklog 在此分類
                if item.get("category_id") != cat_id:
                    for wl in item.get("worklogs", []):
                        if isinstance(wl, dict) and wl.get("work_item_id") == cat_id:
                            orphan_worklogs.append({
                                "id": wl.get("id"),
                                "text": wl.get("text", ""),
                                "parent_task_key": item.get("issue_key", ""),
                                "parent_task_name": item.get("issue_name", ""),
                            })

            result.append({
                **cat,
                "count": len(assigned_tasks),
                "tasks": assigned_tasks,
                "orphan_worklogs": orphan_worklogs,
                "orphan_count": len(orphan_worklogs),
            })
        return result

    @rx.var
    def expanded_category_data(self) -> dict:
        """展開中的工作項目詳細資料（用於 Modal）"""
        if self.expanded_category_id < 0:
            return {}

        cat_id = self.expanded_category_id

        # 找到分類
        cat = next((c for c in self.categories if c.get("id") == cat_id), None)
        if not cat:
            return {}

        # 找出直接歸類到此分類的 Tasks（含 worklogs）
        assigned_tasks = []
        for item in self.work_items:
            if item.get("category_id") == cat_id and item.get("included", True):
                # 取得該 Task 的 worklogs（排除已獨立歸類到其他分類的）
                task_worklogs = []
                for wl in item.get("worklogs", []):
                    if isinstance(wl, dict):
                        wl_cat_id = wl.get("work_item_id", -1)
                        # worklog 跟隨 task (-1) 或歸類到同一分類
                        if wl_cat_id == -1 or wl_cat_id == cat_id:
                            task_worklogs.append({
                                "id": wl.get("id"),
                                "text": wl.get("text", ""),
                                "hours": wl.get("hours", 0),
                            })
                assigned_tasks.append({
                    "id": item.get("id"),
                    "issue_key": item.get("issue_key", ""),
                    "issue_name": item.get("issue_name", ""),
                    "hours": item.get("hours", 0),
                    "worklogs": task_worklogs,
                    "worklog_count": len(task_worklogs),
                })

        # 找出獨立歸類到此分類的 Worklogs
        orphan_worklogs = []
        for item in self.work_items:
            if item.get("category_id") != cat_id:
                for wl in item.get("worklogs", []):
                    if isinstance(wl, dict) and wl.get("work_item_id") == cat_id:
                        orphan_worklogs.append({
                            "id": wl.get("id"),
                            "text": wl.get("text", ""),
                            "parent_task_id": item.get("id"),
                            "parent_task_key": item.get("issue_key", ""),
                            "parent_task_name": item.get("issue_name", ""),
                        })

        return {
            **cat,
            "tasks": assigned_tasks,
            "orphan_worklogs": orphan_worklogs,
            "task_count": len(assigned_tasks),
            "orphan_count": len(orphan_worklogs),
        }

    @rx.var
    def show_category_modal(self) -> bool:
        """是否顯示工作項目編輯 Modal"""
        return self.expanded_category_id >= 0

    @rx.var
    def categories_with_summary_count(self) -> int:
        """已有摘要的工作項目數量"""
        return sum(1 for cat in self.categories if cat.get("summary", "").strip())

    @rx.var
    def total_categories_count(self) -> int:
        """總工作項目數量"""
        return len(self.categories)

    @rx.var
    def all_summaries_generated(self) -> bool:
        """是否所有工作項目都已生成摘要"""
        if not self.categories:
            return False
        return all(cat.get("summary", "").strip() for cat in self.categories)

    @rx.var
    def unassigned_count(self) -> int:
        """未指派工作項目的 Task 數量"""
        return sum(1 for item in self.work_items if item.get("category_id", -1) == -1)

    @rx.var
    def filtered_work_items(self) -> List[dict]:
        """篩選後的工作項目列表"""
        if self.filter_category_id == -999:
            # 全部顯示
            return self.work_items
        else:
            # 依分類篩選（-1 = 未分類）
            return [item for item in self.work_items if item.get("category_id", -1) == self.filter_category_id]

    @rx.var
    def has_basic_info(self) -> bool:
        """是否有基本資料"""
        return bool(self.emp_name)

    @rx.var
    def has_analysis(self) -> bool:
        """是否有分析結果"""
        return bool(self.analysis_data)

    @rx.var
    def has_drafts(self) -> bool:
        """是否有 AI 草稿"""
        return bool(self.work_draft)

    @rx.var
    def show_custom_style(self) -> bool:
        """是否顯示自訂風格輸入框"""
        return self.position_type == "自訂"

    @rx.var
    def step1_complete(self) -> bool:
        """步驟 1 是否完成 - 基本資料"""
        return bool(self.emp_name)

    @rx.var
    def step2_complete(self) -> bool:
        """步驟 2 是否完成 - 上傳檔案"""
        return bool(self.analysis_data)

    @rx.var
    def step3_complete(self) -> bool:
        """步驟 3 是否完成 - 資料預覽"""
        return bool(self.analysis_data)

    @rx.var
    def step4_complete(self) -> bool:
        """步驟 4 是否完成 - 生成報告"""
        return bool(self.download_url)

    @rx.var
    def total_hours_display(self) -> str:
        """總工時顯示"""
        hours = self.analysis_data.get("total_hours", 0)
        return f"{hours:.1f}" if hours else "0"

    @rx.var
    def project_count(self) -> int:
        """專案數量"""
        return len(self.analysis_data.get("project_issues", {}))

    @rx.var
    def issue_count(self) -> int:
        """工作項目數量"""
        return len(self.analysis_data.get("issue_summary", []))

    @rx.var
    def included_items_count(self) -> int:
        """納入報告的項目數量"""
        return len([item for item in self.work_items if item.get("included", True)])

    @rx.var
    def total_items_count(self) -> int:
        """總項目數量"""
        return len(self.work_items)

    @rx.var
    def included_hours(self) -> float:
        """納入報告的總工時"""
        return sum(item.get("hours", 0) for item in self.work_items if item.get("included", True))

    @rx.var
    def items_by_tag(self) -> dict:
        """按標籤分組的項目"""
        result = {}
        for item in self.work_items:
            for tag in item.get("tags", []):
                if tag not in result:
                    result[tag] = []
                result[tag].append(item)
        return result

    @rx.var
    def selected_item(self) -> dict:
        """取得選中的項目"""
        for item in self.work_items:
            if item["id"] == self.selected_item_id:
                return item
        return {}

    @rx.var
    def selected_item_worklogs(self) -> list[dict]:
        """取得選中 Task 的 worklogs"""
        item = self.selected_item
        worklogs = item.get("worklogs", []) if item else []
        # 確保返回 dict 列表格式
        result = []
        for wl in worklogs:
            if isinstance(wl, dict):
                result.append(wl)
            else:
                # 舊格式相容：轉換為新格式
                result.append({"id": -1, "text": str(wl), "work_item_id": -1})
        return result

    @rx.var
    def selected_item_summary(self) -> str:
        """取得選中項目的摘要"""
        item = self.selected_item
        return item.get("summary", "") if item else ""

    @rx.var
    def has_selected_item(self) -> bool:
        """是否有選中項目"""
        return self.selected_item_id >= 0

