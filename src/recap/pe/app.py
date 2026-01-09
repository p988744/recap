"""PE Helper - Tempo Worklog 分析器 (Reflex + Tailwind + daisyUI)"""

import reflex as rx
from .state import AppState


# ===== daisyUI 元件封裝 =====

def steps_indicator() -> rx.Component:
    """步驟指示器 - 4 步驟，可點擊跳轉"""
    return rx.el.ul(
        # Step 1: 基本資料 - 永遠可點擊
        rx.el.li(
            rx.el.span("基本資料", class_name="text-xs md:text-sm"),
            class_name=rx.cond(
                AppState.current_step >= 1,
                "step step-primary cursor-pointer hover:opacity-70 transition-opacity",
                "step cursor-pointer hover:opacity-70 transition-opacity",
            ),
            on_click=lambda: AppState.go_to_step(1),
        ),
        # Step 2: 上傳檔案 - 有基本資料才可點擊
        rx.el.li(
            rx.el.span("上傳檔案", class_name="text-xs md:text-sm"),
            class_name=rx.cond(
                AppState.current_step >= 2,
                "step step-primary cursor-pointer hover:opacity-70 transition-opacity",
                rx.cond(
                    AppState.has_basic_info,
                    "step cursor-pointer hover:opacity-70 transition-opacity",
                    "step cursor-not-allowed opacity-50",
                ),
            ),
            on_click=lambda: AppState.go_to_step(2),
        ),
        # Step 3: 資料整理 - 有分析資料才可點擊
        rx.el.li(
            rx.el.span("資料整理", class_name="text-xs md:text-sm"),
            class_name=rx.cond(
                AppState.current_step >= 3,
                "step step-primary cursor-pointer hover:opacity-70 transition-opacity",
                rx.cond(
                    AppState.has_analysis,
                    "step cursor-pointer hover:opacity-70 transition-opacity",
                    "step cursor-not-allowed opacity-50",
                ),
            ),
            on_click=lambda: AppState.go_to_step(3),
        ),
        # Step 4: 生成報告 - 有分析資料才可點擊
        rx.el.li(
            rx.el.span("生成報告", class_name="text-xs md:text-sm"),
            class_name=rx.cond(
                AppState.current_step >= 4,
                "step step-primary cursor-pointer hover:opacity-70 transition-opacity",
                rx.cond(
                    AppState.has_analysis,
                    "step cursor-pointer hover:opacity-70 transition-opacity",
                    "step cursor-not-allowed opacity-50",
                ),
            ),
            on_click=lambda: AppState.go_to_step(4),
        ),
        class_name="steps steps-horizontal w-full",
    )


def alert_message() -> rx.Component:
    """訊息提示"""
    return rx.fragment(
        rx.cond(
            AppState.error_message != "",
            rx.el.div(
                rx.el.span(AppState.error_message),
                class_name="alert alert-error",
            ),
        ),
        rx.cond(
            AppState.success_message != "",
            rx.el.div(
                rx.el.span(AppState.success_message),
                class_name="alert alert-success",
            ),
        ),
    )


def loading_overlay() -> rx.Component:
    """全域載入遮罩 - 耗時任務時顯示"""
    return rx.cond(
        AppState.is_busy,
        rx.el.div(
            # 半透明背景遮罩
            rx.el.div(
                class_name="fixed inset-0 bg-black/50 z-50",
            ),
            # 載入對話框
            rx.el.div(
                rx.el.div(
                    # 標題
                    rx.el.h3(
                        AppState.busy_title,
                        class_name="font-bold text-lg mb-4",
                    ),
                    # 進度文字
                    rx.el.div(
                        rx.el.span(class_name="loading loading-spinner loading-md text-primary"),
                        rx.el.span(AppState.busy_message, class_name="ml-3"),
                        class_name="flex items-center mb-4",
                    ),
                    # 進度條（有總數時顯示）
                    rx.cond(
                        AppState.busy_total > 0,
                        rx.el.div(
                            rx.el.progress(
                                value=AppState.busy_progress,
                                max=100,
                                class_name="progress progress-primary w-full",
                            ),
                            rx.el.div(
                                rx.el.span(
                                    AppState.busy_current.to_string(),
                                    class_name="font-mono",
                                ),
                                rx.el.span(" / ", class_name="text-base-content/50"),
                                rx.el.span(
                                    AppState.busy_total.to_string(),
                                    class_name="font-mono",
                                ),
                                rx.el.span(
                                    " (",
                                    class_name="text-base-content/50 ml-2",
                                ),
                                rx.el.span(
                                    AppState.busy_progress.to_string(),
                                    class_name="font-bold text-primary",
                                ),
                                rx.el.span(
                                    "%)",
                                    class_name="text-base-content/50",
                                ),
                                class_name="text-sm text-center mt-2",
                            ),
                            class_name="w-full",
                        ),
                    ),
                    # 提示
                    rx.el.p(
                        "請稍候，正在處理中...",
                        class_name="text-xs text-base-content/50 mt-4 text-center",
                    ),
                    class_name="card-body",
                ),
                class_name="card bg-base-100 border border-base-300 w-80 fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50",
            ),
        ),
    )


def card(title: str, children: list[rx.Component], class_name: str = "") -> rx.Component:
    """卡片元件"""
    return rx.el.div(
        rx.el.div(
            rx.el.h2(title, class_name="card-title text-lg"),
            *children,
            class_name="card-body",
        ),
        class_name=f"card bg-base-100 border border-base-300 {class_name}",
    )


def form_control(label: str, input_element: rx.Component) -> rx.Component:
    """表單控制項"""
    return rx.el.div(
        rx.el.label(
            rx.el.span(label, class_name="label-text font-medium"),
            class_name="label",
        ),
        input_element,
        class_name="form-control w-full",
    )


def text_input(placeholder: str, value, on_change, input_type: str = "text") -> rx.Component:
    """文字輸入框 - 使用 debounce 避免 IME 選字問題"""
    return rx.input(
        type=input_type,
        placeholder=placeholder,
        value=value,
        on_change=on_change,
        debounce_timeout=300,  # 300ms debounce 解決中文 IME 問題
        class_name="input input-bordered w-full",
    )


def nav_buttons(show_prev: bool = True, show_next: bool = True, next_disabled=False) -> rx.Component:
    """導航按鈕 - 固定在底部"""
    return rx.el.div(
        rx.cond(
            show_prev,
            rx.el.button(
                rx.icon("arrow_left", size=16),
                " 上一步",
                on_click=AppState.prev_step,
                class_name="btn btn-outline",
            ),
            rx.el.div(),
        ),
        rx.cond(
            show_next,
            rx.el.button(
                "下一步 ",
                rx.icon("arrow_right", size=16),
                on_click=AppState.next_step,
                disabled=next_disabled,
                class_name="btn btn-primary",
            ),
            rx.el.div(),
        ),
        class_name="flex justify-between sticky bottom-0 bg-base-200/95 backdrop-blur-sm py-4 -mx-4 px-4 mt-4 border-t border-base-300",
    )


# ===== 設定 Modal =====

def option_list_section(
    title: str,
    options: list,
    new_value,
    set_new_value,
    add_func,
    remove_func,
    placeholder: str = "輸入新選項...",
) -> rx.Component:
    """選項管理區塊"""
    return rx.el.div(
        rx.el.h4(title, class_name="font-medium text-sm mb-2"),
        # 新增輸入
        rx.el.div(
            rx.input(
                placeholder=placeholder,
                value=new_value,
                on_change=set_new_value,
                debounce_timeout=300,
                class_name="input input-bordered input-sm flex-1",
            ),
            rx.el.button(
                rx.icon("plus", size=14),
                on_click=add_func,
                class_name="btn btn-primary btn-sm",
            ),
            class_name="flex gap-2 mb-2",
        ),
        # 選項列表
        rx.el.div(
            rx.foreach(
                options,
                lambda opt: rx.el.div(
                    rx.el.span(opt, class_name="text-sm"),
                    rx.el.button(
                        rx.icon("x", size=12),
                        on_click=lambda: remove_func(opt),
                        class_name="btn btn-ghost btn-xs text-error",
                    ),
                    class_name="flex items-center justify-between px-2 py-1 bg-base-200 rounded",
                ),
            ),
            class_name="flex flex-col gap-1 max-h-32 overflow-y-auto",
        ),
        class_name="mb-4",
    )


def dept_edit_form() -> rx.Component:
    """部門編輯表單"""
    return rx.el.div(
        rx.el.div(
            rx.el.h4(
                rx.cond(
                    AppState.is_new_dept_mode,
                    "新增部門",
                    "編輯部門",
                ),
                class_name="font-bold text-base mb-4",
            ),
            rx.el.div(
                # 部門名稱
                rx.el.div(
                    rx.el.label("部門名稱 *", class_name="label-text text-sm font-medium"),
                    rx.input(
                        placeholder="例如：資訊部",
                        value=AppState.edit_dept_name,
                        on_change=AppState.set_edit_dept_name,
                        debounce_timeout=300,
                        class_name="input input-bordered input-sm w-full",
                    ),
                    class_name="form-control",
                ),
                # 所屬處室
                rx.el.div(
                    rx.el.label("所屬處室", class_name="label-text text-sm font-medium"),
                    rx.input(
                        placeholder="例如：技術處",
                        value=AppState.edit_dept_division,
                        on_change=AppState.set_edit_dept_division,
                        debounce_timeout=300,
                        class_name="input input-bordered input-sm w-full",
                    ),
                    class_name="form-control",
                ),
                class_name="grid grid-cols-2 gap-4 mb-4",
            ),
            rx.el.div(
                # 處級主管
                rx.el.div(
                    rx.el.label("處級主管", class_name="label-text text-sm font-medium"),
                    rx.input(
                        placeholder="例如：王處長",
                        value=AppState.edit_dept_division_manager,
                        on_change=AppState.set_edit_dept_division_manager,
                        debounce_timeout=300,
                        class_name="input input-bordered input-sm w-full",
                    ),
                    class_name="form-control",
                ),
                # 部門主管
                rx.el.div(
                    rx.el.label("部門主管", class_name="label-text text-sm font-medium"),
                    rx.input(
                        placeholder="例如：陳經理",
                        value=AppState.edit_dept_manager,
                        on_change=AppState.set_edit_dept_manager,
                        debounce_timeout=300,
                        class_name="input input-bordered input-sm w-full",
                    ),
                    class_name="form-control",
                ),
                class_name="grid grid-cols-2 gap-4 mb-4",
            ),
            # 部門職能
            rx.el.div(
                rx.el.label("部門職能與執掌", class_name="label-text text-sm font-medium"),
                rx.text_area(
                    placeholder="描述部門的主要職能與執掌...",
                    value=AppState.edit_dept_functions,
                    on_change=AppState.set_edit_dept_functions,
                    debounce_timeout=300,
                    rows="3",
                    class_name="textarea textarea-bordered w-full text-sm",
                ),
                class_name="form-control mb-4",
            ),
            # 按鈕
            rx.el.div(
                rx.el.button(
                    "取消",
                    on_click=AppState.cancel_dept_edit,
                    class_name="btn btn-ghost btn-sm",
                ),
                rx.el.button(
                    rx.icon("save", size=14),
                    " 儲存",
                    on_click=AppState.save_dept,
                    class_name="btn btn-primary btn-sm",
                ),
                class_name="flex justify-end gap-2",
            ),
            class_name="p-4 bg-base-200 rounded-lg",
        ),
    )


def dept_list_item(dept: dict) -> rx.Component:
    """部門列表項目"""
    return rx.el.div(
        rx.el.div(
            rx.el.div(
                rx.el.span(dept["name"], class_name="font-bold text-sm"),
                rx.cond(
                    dept["division"] != "",
                    rx.el.span(
                        rx.el.span(" ("),
                        rx.el.span(dept["division"]),
                        rx.el.span(")"),
                        class_name="text-xs text-base-content/60",
                    ),
                ),
                class_name="flex items-center gap-1",
            ),
            rx.el.div(
                rx.cond(
                    dept["dept_manager"] != "",
                    rx.el.span(
                        rx.el.span("主管：", class_name="text-base-content/40"),
                        rx.el.span(dept["dept_manager"]),
                        class_name="text-xs text-base-content/60",
                    ),
                ),
                rx.cond(
                    dept["division_manager"] != "",
                    rx.el.span(
                        rx.el.span("處長：", class_name="text-base-content/40"),
                        rx.el.span(dept["division_manager"]),
                        class_name="text-xs text-base-content/60",
                    ),
                ),
                class_name="flex gap-3",
            ),
            class_name="flex-1",
        ),
        rx.el.div(
            rx.el.button(
                rx.icon("pencil", size=12),
                on_click=lambda: AppState.edit_dept(dept["id"]),
                class_name="btn btn-ghost btn-xs",
            ),
            rx.el.button(
                rx.icon("trash_2", size=12),
                on_click=lambda: AppState.delete_dept(dept["id"]),
                class_name="btn btn-ghost btn-xs text-error",
            ),
            class_name="flex gap-1",
        ),
        class_name="flex items-center justify-between p-3 bg-base-100 rounded-lg border hover:border-primary transition-all",
    )


def settings_modal() -> rx.Component:
    """設定 Modal - 管理部門、職稱等選項"""
    return rx.cond(
        AppState.show_settings,
        rx.el.div(
            # Overlay
            rx.el.div(
                on_click=AppState.toggle_settings,
                class_name="fixed inset-0 bg-black/50 z-40",
            ),
            # Modal
            rx.el.div(
                rx.el.div(
                    # Header
                    rx.el.div(
                        rx.el.h2("系統設定", class_name="text-xl font-bold"),
                        rx.el.button(
                            rx.icon("x", size=20),
                            on_click=AppState.toggle_settings,
                            class_name="btn btn-ghost btn-sm btn-circle",
                        ),
                        class_name="flex justify-between items-center mb-4",
                    ),

                    # 部門管理
                    rx.el.div(
                        rx.el.div(
                            rx.el.h3(
                                rx.icon("building_2", size=16, class_name="inline mr-2"),
                                "部門管理",
                                class_name="font-bold text-lg",
                            ),
                            rx.cond(
                                ~AppState.is_editing_dept,
                                rx.el.button(
                                    rx.icon("plus", size=14),
                                    " 新增部門",
                                    on_click=AppState.start_new_dept,
                                    class_name="btn btn-primary btn-sm",
                                ),
                            ),
                            class_name="flex justify-between items-center mb-4 border-b pb-2",
                        ),
                        rx.el.p(
                            "每個部門包含：所屬處室、處級主管、部門主管、部門職能與執掌",
                            class_name="text-xs text-base-content/60 mb-4",
                        ),
                        # 編輯表單
                        rx.cond(
                            AppState.is_editing_dept,
                            dept_edit_form(),
                        ),
                        # 部門列表
                        rx.cond(
                            ~AppState.is_editing_dept,
                            rx.el.div(
                                rx.cond(
                                    AppState.departments.length() > 0,
                                    rx.el.div(
                                        rx.foreach(
                                            AppState.departments,
                                            dept_list_item,
                                        ),
                                        class_name="flex flex-col gap-2 max-h-48 overflow-y-auto",
                                    ),
                                    rx.el.div(
                                        rx.el.p("尚未設定任何部門", class_name="text-base-content/50 text-sm"),
                                        rx.el.p("點擊「新增部門」開始建立", class_name="text-base-content/40 text-xs"),
                                        class_name="text-center py-6 bg-base-200 rounded-lg",
                                    ),
                                ),
                            ),
                        ),
                        class_name="mb-6",
                    ),

                    # 職稱管理
                    rx.el.div(
                        rx.el.h3(
                            rx.icon("badge", size=16, class_name="inline mr-2"),
                            "職稱管理",
                            class_name="font-bold text-lg mb-4 border-b pb-2",
                        ),
                        option_list_section(
                            "職稱選項",
                            AppState.title_options,
                            AppState.new_title,
                            AppState.set_new_title,
                            AppState.add_new_title,
                            AppState.remove_title,
                            "輸入職稱...",
                        ),
                        class_name="mb-6",
                    ),

                    # LLM 設定
                    rx.el.div(
                        rx.el.h3(
                            rx.icon("bot", size=16, class_name="inline mr-2"),
                            "AI 連線設定",
                            class_name="font-bold text-lg mb-4 border-b pb-2",
                        ),
                        rx.el.div(
                            form_control("API URL", text_input("https://api.openai.com/v1", AppState.api_url, AppState.set_api_url)),
                            form_control("API Key", text_input("sk-...", AppState.api_key, AppState.set_api_key, "password")),
                            form_control("Model", text_input("gpt-4o-mini", AppState.model, AppState.set_model)),
                            class_name="grid grid-cols-1 md:grid-cols-3 gap-4",
                        ),
                        class_name="mb-6",
                    ),

                    # Jira/Tempo 設定（只有 URL，PAT 在 Step 2 輸入）
                    rx.el.div(
                        rx.el.h3(
                            rx.icon("cloud", size=16, class_name="inline mr-2"),
                            "Jira/Tempo 連線設定",
                            class_name="font-bold text-lg mb-4 border-b pb-2",
                        ),
                        rx.el.p(
                            "設定公司 Jira Server URL（PAT 請在「上傳檔案」步驟輸入）",
                            class_name="text-xs text-base-content/60 mb-4",
                        ),
                        rx.el.div(
                            form_control("Jira URL", text_input("https://jira.example.com", AppState.jira_url, AppState.set_jira_url)),
                            class_name="mb-4",
                        ),
                        rx.el.div(
                            rx.el.button(
                                rx.icon("save", size=14),
                                " 儲存設定",
                                on_click=AppState.save_jira_settings,
                                class_name="btn btn-primary btn-sm",
                            ),
                            class_name="flex items-center gap-2",
                        ),
                    ),

                    # Footer
                    rx.el.div(
                        rx.el.button(
                            "關閉",
                            on_click=AppState.toggle_settings,
                            class_name="btn btn-primary",
                        ),
                        class_name="flex justify-end mt-6 pt-4 border-t",
                    ),

                    class_name="card-body max-h-[80vh] overflow-y-auto",
                ),
                class_name="card bg-base-100 border border-base-300 w-full max-w-4xl mx-4 fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50",
            ),
        ),
    )


def token_modal() -> rx.Component:
    """Token 顯示對話框 - 顯示用戶的 session token"""
    return rx.cond(
        AppState.show_token_modal,
        rx.el.div(
            # Overlay
            rx.el.div(
                on_click=AppState.close_token_modal,
                class_name="fixed inset-0 bg-black/50 z-40",
            ),
            # Modal
            rx.el.div(
                rx.el.div(
                    # Header
                    rx.el.div(
                        rx.icon("key", size=24, class_name="text-primary"),
                        rx.el.h2("您的專屬 Token", class_name="text-xl font-bold"),
                        class_name="flex items-center gap-3 mb-4",
                    ),

                    # Token 顯示
                    rx.el.div(
                        rx.el.div(
                            rx.el.span(
                                AppState.session_key,
                                class_name="font-mono text-2xl tracking-[0.3em] font-bold text-primary",
                            ),
                            class_name="bg-base-200 px-6 py-4 rounded-lg text-center border-2 border-dashed border-primary/30",
                        ),
                        class_name="mb-4",
                    ),

                    # 說明
                    rx.el.div(
                        rx.el.p(
                            rx.icon("info", size=14, class_name="inline mr-1"),
                            "請記下此 Token，下次可用它繼續編輯您的工作。",
                            class_name="text-sm text-base-content/70",
                        ),
                        rx.el.p(
                            "Token 會自動存入瀏覽器，同一瀏覽器下次會自動載入。",
                            class_name="text-xs text-base-content/50 mt-1",
                        ),
                        class_name="mb-6",
                    ),

                    # 按鈕
                    rx.el.div(
                        rx.el.button(
                            rx.icon("check", size=16),
                            " 我知道了",
                            on_click=AppState.close_token_modal,
                            class_name="btn btn-primary",
                        ),
                        class_name="flex justify-center",
                    ),

                    class_name="card-body",
                ),
                class_name="card bg-base-100 border border-base-300 w-full max-w-md mx-4 fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50",
            ),
        ),
    )


def session_modal() -> rx.Component:
    """Session 管理 Modal - 離開/切換工作進度"""
    return rx.cond(
        AppState.show_session_modal,
        rx.el.div(
            # Overlay
            rx.el.div(
                on_click=AppState.close_session_modal,
                class_name="fixed inset-0 bg-black/50 z-40",
            ),
            # Modal
            rx.el.div(
                rx.el.div(
                    # Header
                    rx.el.div(
                        rx.el.div(
                            rx.icon("folder_open", size=24, class_name="text-primary"),
                            rx.el.h2("工作進度管理", class_name="text-xl font-bold"),
                            class_name="flex items-center gap-3",
                        ),
                        rx.el.button(
                            rx.icon("x", size=20),
                            on_click=AppState.close_session_modal,
                            class_name="btn btn-ghost btn-sm btn-circle",
                        ),
                        class_name="flex justify-between items-center mb-4",
                    ),

                    # 當前 Session 資訊
                    rx.cond(
                        AppState.session_key != "",
                        rx.el.div(
                            rx.el.div(
                                rx.icon("key", size=14, class_name="text-success"),
                                rx.el.span("目前工作進度", class_name="text-sm font-medium"),
                                class_name="flex items-center gap-2 mb-2",
                            ),
                            rx.el.div(
                                rx.el.div(
                                    rx.el.span("Token: ", class_name="text-xs text-base-content/60"),
                                    rx.el.span(
                                        AppState.session_key,
                                        class_name="font-mono text-sm font-bold text-primary tracking-wider",
                                    ),
                                    class_name="flex items-center gap-1",
                                ),
                                rx.el.div(
                                    rx.el.span("員工: ", class_name="text-xs text-base-content/60"),
                                    rx.el.span(AppState.emp_name, class_name="text-sm font-medium"),
                                    class_name="flex items-center gap-1",
                                ),
                                rx.el.div(
                                    rx.el.span("檔案: ", class_name="text-xs text-base-content/60"),
                                    rx.el.span(AppState.file_name, class_name="text-sm truncate max-w-[200px]"),
                                    class_name="flex items-center gap-1",
                                ),
                                class_name="grid grid-cols-3 gap-4",
                            ),
                            class_name="p-4 bg-success/10 border border-success/30 rounded-lg mb-4",
                        ),
                    ),

                    # Token 歷史記錄
                    rx.cond(
                        AppState.token_history_list.length() > 0,
                        rx.el.div(
                            rx.el.div(
                                rx.icon("history", size=14, class_name="text-base-content/60"),
                                rx.el.span("歷史記錄", class_name="text-sm font-medium"),
                                class_name="flex items-center gap-2 mb-2",
                            ),
                            rx.el.div(
                                rx.foreach(
                                    AppState.token_history_list,
                                    lambda h: rx.el.div(
                                        rx.el.div(
                                            rx.el.div(
                                                rx.el.span(
                                                    h["token"],
                                                    class_name="font-mono text-xs font-bold text-primary tracking-wide",
                                                ),
                                                rx.cond(
                                                    h["token"] == AppState.session_key,
                                                    rx.el.span("目前", class_name="badge badge-success badge-xs ml-2"),
                                                ),
                                                class_name="flex items-center",
                                            ),
                                            rx.el.div(
                                                rx.el.span(h["emp_name"], class_name="text-sm font-medium"),
                                                rx.cond(
                                                    h["file_name"] != "",
                                                    rx.el.span(
                                                        rx.el.span(" • ", class_name="text-base-content/30"),
                                                        rx.el.span(h["file_name"], class_name="text-xs text-base-content/60 truncate max-w-[150px]"),
                                                    ),
                                                ),
                                                class_name="flex items-center",
                                            ),
                                            rx.el.span(h["updated_at"], class_name="text-xs text-base-content/40"),
                                            class_name="flex-1",
                                        ),
                                        rx.el.div(
                                            # 切換按鈕
                                            rx.el.button(
                                                rx.cond(
                                                    h["token"] == AppState.session_key,
                                                    rx.icon("check", size=14),
                                                    rx.icon("arrow_right", size=14),
                                                ),
                                                on_click=lambda: AppState.switch_session(h["token"]),
                                                disabled=h["token"] == AppState.session_key,
                                                class_name=rx.cond(
                                                    h["token"] == AppState.session_key,
                                                    "btn btn-disabled btn-sm btn-circle",
                                                    "btn btn-ghost btn-sm btn-circle",
                                                ),
                                            ),
                                            # 刪除按鈕
                                            rx.el.button(
                                                rx.icon("trash_2", size=14),
                                                on_click=lambda: AppState.handle_delete_session(h["token"]),
                                                class_name="btn btn-ghost btn-sm btn-circle text-error/60 hover:text-error hover:bg-error/10",
                                            ),
                                            class_name="flex items-center gap-1",
                                        ),
                                        class_name="flex items-center justify-between p-2 hover:bg-base-300 rounded transition-all",
                                    ),
                                ),
                                class_name="flex flex-col max-h-40 overflow-y-auto bg-base-200 rounded-lg",
                            ),
                            class_name="mb-4",
                        ),
                    ),

                    # 載入其他工作進度（透過 Token）
                    rx.el.div(
                        rx.el.div(
                            rx.icon("key", size=14, class_name="text-base-content/60"),
                            rx.el.span("輸入 Token 載入", class_name="text-sm font-medium"),
                            class_name="flex items-center gap-2 mb-2",
                        ),
                        rx.el.div(
                            rx.el.input(
                                placeholder="輸入 Token（例如：ABC12XYZ）",
                                value=AppState.token_input,
                                on_change=AppState.set_token_input,
                                class_name="input input-bordered input-sm flex-1 font-mono uppercase tracking-widest",
                            ),
                            rx.el.button(
                                rx.icon("arrow_right", size=14),
                                on_click=AppState.load_by_token_and_close,
                                class_name="btn btn-primary btn-sm btn-circle",
                            ),
                            class_name="flex gap-2",
                        ),
                        class_name="p-4 bg-base-200 rounded-lg mb-4",
                    ),

                    # 開始新的工作
                    rx.el.div(
                        rx.el.div(
                            rx.icon("plus", size=14, class_name="text-base-content/60"),
                            rx.el.span("開始新的工作", class_name="text-sm font-medium"),
                            class_name="flex items-center gap-2 mb-2",
                        ),
                        rx.cond(
                            AppState.session_key != "",
                            rx.el.div(
                                rx.el.div(
                                    rx.icon("triangle_alert", size=16, class_name="text-warning"),
                                    rx.el.p("目前的工作進度會保留，請記住 Token 以便日後載入", class_name="text-sm text-base-content/60"),
                                    class_name="flex items-center gap-2 mb-3",
                                ),
                                rx.el.button(
                                    rx.icon("plus", size=14),
                                    " 清除並開始新工作",
                                    on_click=AppState.clear_session,
                                    class_name="btn btn-outline btn-warning btn-sm w-full",
                                ),
                                class_name="p-4 bg-warning/10 border border-warning/30 rounded-lg",
                            ),
                            rx.el.div(
                                rx.el.p("目前沒有進行中的工作", class_name="text-sm text-base-content/50"),
                                class_name="p-4 bg-base-200 rounded-lg text-center",
                            ),
                        ),
                    ),

                    # Footer
                    rx.el.div(
                        rx.el.button(
                            "關閉",
                            on_click=AppState.close_session_modal,
                            class_name="btn btn-ghost",
                        ),
                        class_name="flex justify-end mt-6 pt-4 border-t",
                    ),

                    class_name="card-body max-h-[80vh] overflow-y-auto",
                ),
                class_name="card bg-base-100 border border-base-300 w-full max-w-lg mx-4 fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50",
            ),
        ),
    )


def category_edit_modal() -> rx.Component:
    """工作項目編輯 Modal"""
    return rx.cond(
        AppState.show_category_modal,
        rx.el.div(
            # Overlay
            rx.el.div(
                on_click=AppState.close_category_modal,
                class_name="fixed inset-0 bg-black/50 z-40",
            ),
            # Modal
            rx.el.div(
                rx.el.div(
                    # Header
                    rx.el.div(
                        rx.el.div(
                            rx.icon("folder", size=24, class_name="text-primary"),
                            rx.el.h2("編輯工作項目", class_name="text-xl font-bold"),
                            class_name="flex items-center gap-3",
                        ),
                        rx.el.button(
                            rx.icon("x", size=20),
                            on_click=AppState.close_category_modal,
                            class_name="btn btn-ghost btn-sm btn-circle",
                        ),
                        class_name="flex justify-between items-center mb-4",
                    ),

                    # 基本資料編輯
                    rx.el.div(
                        rx.el.div(
                            rx.el.label("工作項目名稱", class_name="label-text font-medium"),
                            rx.input(
                                value=AppState.edit_category_name,
                                on_change=AppState.set_edit_category_name,
                                debounce_timeout=300,
                                class_name="input input-bordered w-full",
                                placeholder="輸入工作項目名稱...",
                            ),
                            class_name="form-control",
                        ),
                        rx.el.div(
                            rx.el.label("工作項目描述", class_name="label-text font-medium"),
                            rx.text_area(
                                value=AppState.edit_category_description,
                                on_change=AppState.set_edit_category_description,
                                debounce_timeout=300,
                                class_name="textarea textarea-bordered w-full",
                                placeholder="描述這個工作項目的內容...",
                                rows="2",
                            ),
                            class_name="form-control mt-3",
                        ),
                        class_name="mb-4",
                    ),

                    # 摘要區塊
                    rx.el.div(
                        rx.el.div(
                            rx.el.label("AI 摘要", class_name="label-text font-medium"),
                            rx.el.button(
                                rx.cond(
                                    AppState.generating_summary_for == AppState.expanded_category_id,
                                    rx.el.span(class_name="loading loading-spinner loading-xs"),
                                    rx.icon("sparkles", size=14),
                                ),
                                " 重新生成",
                                on_click=lambda: AppState.regenerate_category_summary(AppState.expanded_category_id),
                                disabled=AppState.generating_summary_for >= 0,
                                class_name="btn btn-ghost btn-xs",
                            ),
                            class_name="flex justify-between items-center",
                        ),
                        rx.text_area(
                            value=AppState.expanded_category_data["summary"].to(str),
                            on_change=lambda v: AppState.update_category_summary(AppState.expanded_category_id, v),
                            debounce_timeout=500,
                            class_name="textarea textarea-bordered w-full text-sm",
                            placeholder="AI 生成的摘要會顯示在這裡...",
                            rows="3",
                        ),
                        class_name="form-control mb-4",
                    ),

                    # 已歸類的 Tasks
                    rx.el.div(
                        rx.el.div(
                            rx.el.span("已歸類的 Tasks", class_name="label-text font-medium"),
                            rx.el.span(
                                AppState.expanded_category_data["task_count"].to(int).to_string() + " 個",
                                class_name="badge badge-primary badge-sm ml-2",
                            ),
                            class_name="flex items-center mb-2",
                        ),
                        rx.cond(
                            AppState.expanded_category_data["task_count"].to(int) > 0,
                            rx.el.div(
                                rx.foreach(
                                    AppState.expanded_category_data["tasks"].to(list[dict]),
                                    lambda task: rx.el.div(
                                        # Task 標題列
                                        rx.el.div(
                                            rx.el.div(
                                                rx.el.span(task["issue_key"], class_name="font-mono text-xs text-primary font-bold"),
                                                rx.el.span(task["issue_name"], class_name="text-sm text-base-content/70 truncate flex-1 ml-2"),
                                                rx.el.span(task["hours"].to_string() + "h", class_name="text-xs text-base-content/50 ml-2"),
                                                class_name="flex items-center flex-1",
                                            ),
                                            rx.el.button(
                                                rx.icon("x", size=12),
                                                on_click=lambda: AppState.remove_task_from_category(task["id"].to(int)),
                                                class_name="btn btn-ghost btn-xs text-error",
                                                title="移除（設為未分類）",
                                            ),
                                            class_name="flex items-center justify-between",
                                        ),
                                        # Worklogs 列表
                                        rx.cond(
                                            task["worklog_count"].to(int) > 0,
                                            rx.el.div(
                                                rx.foreach(
                                                    task["worklogs"].to(list[dict]),
                                                    lambda wl: rx.el.div(
                                                        rx.el.span("↳", class_name="text-base-content/30 mr-1"),
                                                        rx.el.span(wl["text"], class_name="text-xs text-base-content/60 truncate flex-1"),
                                                        rx.cond(
                                                            wl["hours"].to(float) > 0,
                                                            rx.el.span(wl["hours"].to_string() + "h", class_name="text-xs text-base-content/40 ml-1"),
                                                        ),
                                                        class_name="flex items-center pl-4 py-0.5",
                                                    ),
                                                ),
                                                class_name="flex flex-col border-l-2 border-base-300 ml-2 mt-1",
                                            ),
                                        ),
                                        class_name="px-3 py-2 bg-base-200 rounded hover:bg-base-300",
                                    ),
                                ),
                                class_name="flex flex-col gap-2 max-h-60 overflow-y-auto",
                            ),
                            rx.el.p("尚未有 Task 歸類到此工作項目", class_name="text-sm text-base-content/50 italic"),
                        ),
                        class_name="mb-4",
                    ),

                    # 獨立歸類的 Worklogs
                    rx.cond(
                        AppState.expanded_category_data["orphan_count"].to(int) > 0,
                        rx.el.div(
                            rx.el.div(
                                rx.icon("corner_down_right", size=14, class_name="text-warning"),
                                rx.el.span("獨立歸類的 Worklogs", class_name="label-text font-medium ml-1"),
                                rx.el.span(
                                    AppState.expanded_category_data["orphan_count"].to(int).to_string() + " 個",
                                    class_name="badge badge-warning badge-sm ml-2",
                                ),
                                class_name="flex items-center mb-2",
                            ),
                            rx.el.div(
                                rx.foreach(
                                    AppState.expanded_category_data["orphan_worklogs"].to(list[dict]),
                                    lambda wl: rx.el.div(
                                        rx.el.div(
                                            rx.el.span("↳ ", class_name="text-base-content/30"),
                                            rx.el.span(wl["parent_task_key"], class_name="font-mono text-xs text-base-content/50"),
                                            rx.el.span(wl["text"], class_name="text-sm text-base-content/60 truncate flex-1 ml-2"),
                                            class_name="flex items-center flex-1",
                                        ),
                                        rx.el.button(
                                            rx.icon("x", size=12),
                                            on_click=lambda: AppState.remove_worklog_from_category(
                                                wl["parent_task_id"].to(int),
                                                wl["id"].to(int),
                                            ),
                                            class_name="btn btn-ghost btn-xs text-error",
                                            title="移除（跟隨 Task）",
                                        ),
                                        class_name="flex items-center justify-between px-3 py-2 bg-warning/10 rounded border-l-2 border-warning/30",
                                    ),
                                ),
                                class_name="flex flex-col gap-1 max-h-32 overflow-y-auto",
                            ),
                            class_name="mb-4",
                        ),
                    ),

                    # 按鈕列
                    rx.el.div(
                        rx.el.button(
                            "取消",
                            on_click=AppState.close_category_modal,
                            class_name="btn btn-ghost",
                        ),
                        rx.el.button(
                            rx.icon("save", size=16),
                            " 儲存",
                            on_click=AppState.save_category_details,
                            class_name="btn btn-primary",
                        ),
                        class_name="flex justify-end gap-2 pt-4 border-t",
                    ),

                    class_name="card-body max-h-[80vh] overflow-y-auto",
                ),
                class_name="card bg-base-100 border border-base-300 w-full max-w-2xl mx-4 fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50",
            ),
        ),
    )


# ===== 可收合基本設定面板（用於後續步驟）=====

def collapsible_basic_settings() -> rx.Component:
    """可收合的基本設定面板 - 在步驟 2/3/4 顯示"""
    return rx.el.div(
        # 標題列（可點擊展開/收合）
        rx.el.div(
            rx.el.div(
                rx.icon("user", size=16, class_name="text-primary"),
                rx.el.span("基本設定", class_name="font-medium"),
                rx.el.span(
                    AppState.emp_name,
                    class_name="text-sm text-base-content/60 ml-2",
                ),
                class_name="flex items-center gap-2",
            ),
            rx.icon(
                rx.cond(AppState.show_basic_settings, "chevron-up", "chevron-down"),
                size=16,
                class_name="text-base-content/60",
            ),
            on_click=AppState.toggle_basic_settings,
            class_name="flex items-center justify-between p-3 cursor-pointer hover:bg-base-200 rounded-lg",
        ),
        # 展開的內容
        rx.cond(
            AppState.show_basic_settings,
            rx.el.div(
                # 員工資料
                rx.el.div(
                    rx.el.h4("員工資料", class_name="font-medium text-sm mb-2"),
                    rx.el.div(
                        form_control("姓名", text_input("", AppState.emp_name, AppState.set_emp_name)),
                        form_control("部門", dept_select_dropdown()),
                        form_control("職稱", text_input("", AppState.emp_title, AppState.set_emp_title)),
                        class_name="grid grid-cols-1 md:grid-cols-3 gap-3",
                    ),
                    rx.el.div(
                        form_control("到職日期", rx.input(
                            type="date",
                            value=AppState.emp_start_date,
                            on_change=AppState.set_emp_start_date,
                            class_name="input input-bordered input-sm w-full",
                        )),
                        form_control("部門主管", text_input("", AppState.emp_manager, AppState.set_emp_manager)),
                        form_control("考核期間", text_input("", AppState.emp_period, AppState.set_emp_period)),
                        class_name="grid grid-cols-1 md:grid-cols-3 gap-3 mt-2",
                    ),
                    class_name="mb-4",
                ),
                # 假勤資料
                rx.el.div(
                    rx.el.h4("假勤資料", class_name="font-medium text-sm mb-2"),
                    rx.el.div(
                        form_control("病假(h)", text_input("0", AppState.emp_sick_leave.to_string(), AppState.set_emp_sick_leave, "number")),
                        form_control("事假(h)", text_input("0", AppState.emp_personal_leave.to_string(), AppState.set_emp_personal_leave, "number")),
                        form_control("曠職(h)", text_input("0", AppState.emp_absent.to_string(), AppState.set_emp_absent, "number")),
                        class_name="grid grid-cols-3 gap-3",
                    ),
                    class_name="mb-4",
                ),
                # AI 設定
                rx.el.div(
                    rx.el.h4("AI 報告設定", class_name="font-medium text-sm mb-2"),
                    rx.el.div(
                        rx.el.div(
                            rx.el.span("風格：", class_name="text-sm mr-2"),
                            rx.el.label(
                                rx.el.input(type="radio", name="pos_type", value="技術職", checked=AppState.position_type == "技術職", on_change=lambda: AppState.set_position_type("技術職"), class_name="radio radio-primary radio-xs"),
                                rx.el.span("技術職", class_name="ml-1 text-sm"),
                                class_name="flex items-center cursor-pointer",
                            ),
                            rx.el.label(
                                rx.el.input(type="radio", name="pos_type", value="管理職", checked=AppState.position_type == "管理職", on_change=lambda: AppState.set_position_type("管理職"), class_name="radio radio-primary radio-xs"),
                                rx.el.span("管理職", class_name="ml-1 text-sm"),
                                class_name="flex items-center cursor-pointer",
                            ),
                            class_name="flex items-center gap-4",
                        ),
                        rx.el.button(
                            rx.icon("settings", size=12),
                            " AI 連線",
                            on_click=AppState.toggle_settings,
                            class_name="btn btn-ghost btn-xs",
                        ),
                        class_name="flex items-center justify-between",
                    ),
                ),
                class_name="p-3 pt-0",
            ),
        ),
        class_name="bg-base-100 border border-base-300 rounded-lg mb-4",
    )


# ===== Step 1: 輸入基本資料 =====

def token_input_panel() -> rx.Component:
    """Token 輸入面板 - 繼續先前的工作"""
    return rx.el.div(
        rx.el.div(
            rx.icon("key", size=16, class_name="text-base-content/60"),
            rx.el.span("繼續先前的工作", class_name="text-sm font-medium text-base-content/60"),
            class_name="flex items-center gap-2 mb-3",
        ),
        rx.el.div(
            rx.el.input(
                placeholder="輸入 Token（例如：ABC12XYZ）",
                value=AppState.token_input,
                on_change=AppState.set_token_input,
                class_name="input input-bordered input-sm flex-1 font-mono uppercase tracking-widest",
            ),
            rx.el.button(
                rx.icon("arrow-right", size=16),
                on_click=AppState.load_by_token,
                class_name="btn btn-primary btn-sm",
                title="載入",
            ),
            class_name="flex gap-2",
        ),
        rx.el.p(
            "如果您之前有使用過此工具，可以輸入 Token 來繼續編輯。",
            class_name="text-xs text-base-content/50 mt-2",
        ),
        class_name="p-4 bg-base-200 rounded-lg mb-4 border border-base-300",
    )


def select_or_input(
    options: list,
    value,
    on_change,
    placeholder: str = "選擇或輸入...",
) -> rx.Component:
    """下拉選單（含自訂輸入選項）"""
    return rx.cond(
        options.length() > 0,
        rx.el.div(
            rx.el.select(
                rx.el.option("-- 選擇 --", value=""),
                rx.foreach(
                    options,
                    lambda opt: rx.el.option(opt, value=opt),
                ),
                rx.el.option("自訂...", value="__custom__"),
                value=rx.cond(
                    options.contains(value),
                    value,
                    rx.cond(value != "", "__custom__", ""),
                ),
                on_change=on_change,
                class_name="select select-bordered w-full",
            ),
            # 當選擇「自訂」時顯示輸入框
            rx.cond(
                ~options.contains(value) & (value != ""),
                rx.input(
                    placeholder=placeholder,
                    value=value,
                    on_change=on_change,
                    debounce_timeout=300,
                    class_name="input input-bordered w-full mt-2",
                ),
            ),
            class_name="w-full",
        ),
        # 沒有選項時直接顯示輸入框
        text_input(placeholder, value, on_change),
    )


def dept_select_dropdown() -> rx.Component:
    """部門選擇下拉選單 - 選擇後自動帶入主管"""
    return rx.el.div(
        rx.cond(
            AppState.departments.length() > 0,
            rx.el.div(
                rx.el.select(
                    rx.el.option("-- 選擇部門 --", value=""),
                    rx.foreach(
                        AppState.departments,
                        lambda dept: rx.el.option(
                            dept["name"],
                            value=dept["name"],
                        ),
                    ),
                    value=AppState.emp_dept,
                    on_change=AppState.select_emp_dept,
                    class_name="select select-bordered w-full",
                ),
                # 顯示已選部門的詳細資訊
                rx.cond(
                    AppState.emp_dept != "",
                    rx.el.div(
                        rx.icon("info", size=12, class_name="text-info"),
                        rx.el.span("主管：", class_name="text-xs text-base-content/60"),
                        rx.el.span(AppState.emp_manager, class_name="text-xs font-medium"),
                        rx.el.span("，處長：", class_name="text-xs text-base-content/60"),
                        rx.el.span(AppState.emp_division_manager, class_name="text-xs font-medium"),
                        class_name="flex items-center gap-1 mt-1",
                    ),
                ),
            ),
            # 沒有部門時顯示提示
            rx.el.div(
                rx.el.p("尚未設定部門", class_name="text-sm text-base-content/50"),
                rx.el.button(
                    rx.icon("settings", size=12),
                    " 前往設定",
                    on_click=AppState.toggle_settings,
                    class_name="btn btn-ghost btn-xs",
                ),
                class_name="flex items-center gap-2 p-2 bg-base-200 rounded-lg",
            ),
        ),
    )


def step1_basic_info() -> rx.Component:
    """步驟一：輸入基本資料"""
    return rx.el.div(
        # Token 輸入面板
        token_input_panel(),

        # 員工基本資料
        card("員工資料", [
            rx.el.div(
                form_control("員工姓名 *", text_input("王小明", AppState.emp_name, AppState.set_emp_name)),
                form_control("部門", dept_select_dropdown()),
                form_control("職稱", select_or_input(
                    AppState.title_options,
                    AppState.emp_title,
                    AppState.set_emp_title,
                    "輸入職稱...",
                )),
                class_name="grid grid-cols-1 md:grid-cols-3 gap-4",
            ),
            rx.el.div(
                form_control("到職日期", rx.input(
                    type="date",
                    value=AppState.emp_start_date,
                    on_change=AppState.set_emp_start_date,
                    class_name="input input-bordered w-full",
                )),
                form_control("部門主管", text_input("（由部門自動帶入）", AppState.emp_manager, AppState.set_emp_manager)),
                form_control("處級主管", text_input("（由部門自動帶入）", AppState.emp_division_manager, AppState.set_emp_division_manager)),
                class_name="grid grid-cols-1 md:grid-cols-3 gap-4 mt-4",
            ),
            rx.el.div(
                form_control("考核期間", text_input("2025/1/1~2025/12/31", AppState.emp_period, AppState.set_emp_period)),
                class_name="grid grid-cols-1 md:grid-cols-3 gap-4 mt-4",
            ),
        ]),

        # 假勤資料
        card("假勤資料", [
            rx.el.div(
                form_control("病假累計時數", text_input("0", AppState.emp_sick_leave.to_string(), AppState.set_emp_sick_leave, "number")),
                form_control("事假累計時數", text_input("0", AppState.emp_personal_leave.to_string(), AppState.set_emp_personal_leave, "number")),
                form_control("曠職累計時數", text_input("0", AppState.emp_absent.to_string(), AppState.set_emp_absent, "number")),
                class_name="grid grid-cols-1 md:grid-cols-3 gap-4",
            ),
        ]),

        # AI 設定（簡化版）
        card("AI 報告設定", [
            rx.el.div(
                rx.el.div(
                    rx.el.span("報告風格：", class_name="font-medium mr-4"),
                    rx.el.div(
                        rx.el.label(
                            rx.el.input(type="radio", name="position_type", value="技術職", checked=AppState.position_type == "技術職", on_change=lambda: AppState.set_position_type("技術職"), class_name="radio radio-primary radio-sm"),
                            rx.el.span("技術職", class_name="ml-2"),
                            class_name="flex items-center cursor-pointer",
                        ),
                        rx.el.label(
                            rx.el.input(type="radio", name="position_type", value="管理職", checked=AppState.position_type == "管理職", on_change=lambda: AppState.set_position_type("管理職"), class_name="radio radio-primary radio-sm"),
                            rx.el.span("管理職", class_name="ml-2"),
                            class_name="flex items-center cursor-pointer",
                        ),
                        rx.el.label(
                            rx.el.input(type="radio", name="position_type", value="自訂", checked=AppState.position_type == "自訂", on_change=lambda: AppState.set_position_type("自訂"), class_name="radio radio-primary radio-sm"),
                            rx.el.span("自訂", class_name="ml-2"),
                            class_name="flex items-center cursor-pointer",
                        ),
                        class_name="flex gap-6",
                    ),
                    class_name="flex items-center",
                ),
                rx.el.button(
                    rx.icon("settings", size=14),
                    " AI 連線設定",
                    on_click=AppState.toggle_settings,
                    class_name="btn btn-outline btn-sm",
                ),
                class_name="flex items-center justify-between",
            ),
            rx.cond(
                AppState.show_custom_style,
                rx.input(
                    placeholder="例如：著重客戶溝通、專案管理、跨部門協調...",
                    value=AppState.custom_style,
                    on_change=AppState.set_custom_style,
                    debounce_timeout=300,
                    class_name="input input-bordered w-full mt-4",
                ),
            ),
            rx.cond(
                AppState.api_key != "",
                rx.el.div(
                    rx.icon("circle_check", size=14, class_name="text-success"),
                    rx.el.span(f"已設定 API Key，模型：{AppState.model}", class_name="text-sm text-success"),
                    class_name="flex items-center gap-2 mt-2",
                ),
                rx.el.div(
                    rx.icon("circle_alert", size=14, class_name="text-warning"),
                    rx.el.span("尚未設定 API Key，請點擊「AI 連線設定」", class_name="text-sm text-warning"),
                    class_name="flex items-center gap-2 mt-2",
                ),
            ),
        ]),

        nav_buttons(show_prev=False, next_disabled=~AppState.has_basic_info),
        class_name="flex flex-col gap-4",
    )


# ===== Step 2: 上傳檔案 =====

def step2_upload() -> rx.Component:
    """步驟二：上傳檔案"""
    return rx.el.div(
        # 可收合基本設定
        collapsible_basic_settings(),

        # 兩種方式並排顯示（Jira 為主，Excel 為備選）
        rx.el.div(
            # 方式一：從 Jira 載入（主要）
            rx.el.div(
                card("從 Jira 載入（建議）", [
                    rx.el.div(
                        rx.icon("cloud_download", size=40, class_name="text-primary"),
                        rx.el.p("直接從 Tempo API 取得資料", class_name="text-sm text-base-content/70"),
                        class_name="flex flex-col items-center gap-1 py-2",
                    ),
                    # PAT 輸入區域
                    rx.el.div(
                        rx.el.label("Personal Access Token (PAT)", class_name="label label-text text-xs font-medium"),
                        rx.el.input(
                            type="password",
                            placeholder="輸入您的 Jira PAT...",
                            value=AppState.jira_token,
                            on_change=AppState.set_jira_token,
                            class_name="input input-bordered input-sm w-full font-mono",
                        ),
                        # PAT 取得說明（可收合）
                        rx.el.details(
                            rx.el.summary(
                                rx.icon("circle_help", size=12, class_name="inline mr-1"),
                                "如何取得 PAT？",
                                class_name="text-xs text-info cursor-pointer hover:underline",
                            ),
                            rx.el.ol(
                                rx.el.li("登入 Jira → 點擊右上角頭像 → 「個人設定」"),
                                rx.el.li("選擇「Personal Access Tokens」"),
                                rx.el.li("點擊「Create token」建立新 Token"),
                                rx.el.li("複製 Token（只會顯示一次）"),
                                class_name="list-decimal list-inside text-xs space-y-1 mt-2 p-2 bg-info/10 rounded",
                            ),
                            class_name="mt-1",
                        ),
                        class_name="form-control mb-3",
                    ),
                    # 顯示已驗證的使用者資訊
                    rx.cond(
                        AppState.jira_display_name != "",
                        rx.el.div(
                            rx.icon("user-check", size=14, class_name="text-success"),
                            rx.el.span(f"使用者：{AppState.jira_display_name}", class_name="text-sm font-medium"),
                            class_name="flex items-center gap-2 mb-3 p-2 bg-success/10 rounded-lg justify-center",
                        ),
                    ),
                    # 日期範圍選擇
                    rx.el.div(
                        rx.el.div(
                            rx.el.label("開始日期", class_name="label label-text text-xs"),
                            rx.el.input(
                                type="date",
                                value=AppState.jira_date_from,
                                on_change=AppState.set_jira_date_from,
                                class_name="input input-bordered input-sm w-full",
                            ),
                            class_name="form-control flex-1",
                        ),
                        rx.el.div(
                            rx.el.label("結束日期", class_name="label label-text text-xs"),
                            rx.el.input(
                                type="date",
                                value=AppState.jira_date_to,
                                on_change=AppState.set_jira_date_to,
                                class_name="input input-bordered input-sm w-full",
                            ),
                            class_name="form-control flex-1",
                        ),
                        class_name="flex gap-4 mb-4",
                    ),
                    rx.el.button(
                        rx.cond(
                            AppState.jira_loading,
                            rx.el.span(
                                rx.el.span(class_name="loading loading-spinner loading-sm"),
                                " 載入中...",
                                class_name="flex items-center gap-2",
                            ),
                            rx.el.span(
                                rx.icon("download", size=16),
                                " 從 Jira 載入",
                                class_name="flex items-center gap-2",
                            ),
                        ),
                        on_click=AppState.fetch_from_jira,
                        disabled=AppState.jira_loading,
                        class_name="btn btn-primary w-full",
                    ),
                    # 提示：需要設定 Jira URL
                    rx.cond(
                        AppState.jira_url == "",
                        rx.el.div(
                            rx.icon("info", size=14, class_name="text-warning"),
                            rx.el.span("請先在「設定」中配置 Jira URL", class_name="text-xs"),
                            rx.el.button(
                                "前往設定",
                                on_click=AppState.toggle_settings,
                                class_name="btn btn-ghost btn-xs",
                            ),
                            class_name="flex items-center gap-2 mt-2 p-2 bg-warning/10 rounded-lg",
                        ),
                    ),
                ]),
                class_name="flex-1",
            ),

            # 分隔線
            rx.el.div(
                rx.el.div(class_name="h-full w-px bg-base-300"),
                rx.el.span("或", class_name="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-base-100 px-2 text-base-content/50"),
                class_name="relative hidden md:flex items-center justify-center px-4",
            ),

            # 方式二：上傳 Excel（備選）
            rx.el.div(
                card("上傳 Tempo 報表（備選）", [
                    rx.el.div(
                        rx.icon("cloud_upload", size=40, class_name="text-base-content/40"),
                        rx.el.p("若無法連線 Jira，可手動上傳 Excel", class_name="text-sm text-base-content/60"),
                        class_name="flex flex-col items-center gap-1 py-2",
                    ),
                    rx.upload(
                        rx.el.div(
                            rx.el.p("拖放或點擊選擇檔案", class_name="text-sm text-base-content/60"),
                            rx.el.p("支援 .xls, .xlsx", class_name="text-xs text-base-content/40"),
                            class_name="flex flex-col items-center gap-1 py-4",
                        ),
                        id="upload_excel",
                        accept={
                            "application/vnd.ms-excel": [".xls"],
                            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet": [".xlsx"],
                        },
                        max_files=1,
                        class_name="border-2 border-dashed border-base-300 rounded-lg hover:border-secondary hover:bg-base-200 transition-all cursor-pointer w-full",
                    ),
                    rx.el.div(
                        rx.foreach(
                            rx.selected_files("upload_excel"),
                            lambda file: rx.el.span(file, class_name="badge badge-success"),
                        ),
                        class_name="flex gap-2 flex-wrap mt-2",
                    ),
                    rx.el.button(
                        rx.cond(
                            AppState.uploading,
                            rx.el.span(
                                rx.el.span(class_name="loading loading-spinner loading-sm"),
                                " 分析中...",
                                class_name="flex items-center gap-2",
                            ),
                            rx.el.span(
                                rx.icon("search", size=16),
                                " 開始分析",
                                class_name="flex items-center gap-2",
                            ),
                        ),
                        on_click=AppState.handle_upload(rx.upload_files(upload_id="upload_excel")),
                        disabled=AppState.uploading,
                        class_name="btn btn-secondary btn-outline w-full",
                    ),
                ]),
                class_name="flex-1",
            ),

            class_name="flex flex-col md:flex-row gap-4",
        ),

        # 分析結果顯示（放在兩個方式下方）
        rx.cond(
            AppState.has_analysis,
            rx.el.div(
                rx.icon("circle_check", size=20, class_name="text-success"),
                rx.el.span("已分析：", class_name="font-medium"),
                rx.el.span(AppState.file_name, class_name="text-base-content/70"),
                class_name="flex items-center gap-2 p-3 bg-success/10 rounded-lg",
            ),
        ),

        nav_buttons(next_disabled=~AppState.has_analysis),
        class_name="flex flex-col gap-4",
    )


# ===== Step 3: 資料整理 =====

def task_row(item: dict) -> rx.Component:
    """單一 Task 表格列 - compact 設計"""
    is_selected = AppState.selected_item_id == item["id"]

    return rx.el.tr(
        # 勾選
        rx.el.td(
            rx.el.input(
                type="checkbox",
                checked=item["included"],
                on_change=lambda _: AppState.toggle_item_included(item["id"]),
                class_name="checkbox checkbox-success checkbox-sm",
            ),
            class_name="w-8",
        ),
        # Issue Key
        rx.el.td(
            rx.el.span(item["issue_key"], class_name="font-mono text-primary text-xs font-bold"),
            class_name="w-24",
        ),
        # Issue Name
        rx.el.td(
            rx.el.div(
                rx.el.span(item["issue_name"], class_name="font-medium text-sm"),
                rx.el.p("點擊編輯查看 Worklog", class_name="text-xs text-base-content/40 mt-1"),
                class_name="max-w-md",
            ),
        ),
        # 工作項目（下拉選單）
        rx.el.td(
            rx.el.select(
                rx.el.option("未分類", value="-1"),
                rx.foreach(
                    AppState.categories,
                    lambda cat: rx.el.option(cat["name"], value=cat["id"].to_string()),
                ),
                value=item["category_id"].to_string(),
                on_change=lambda e: AppState.assign_item_to_category_str(item["id"], e),
                class_name="select select-xs select-bordered w-full max-w-[100px]",
            ),
            class_name="w-28",
        ),
        # 時數
        rx.el.td(
            rx.el.span(item["hours"].to_string() + "h", class_name="text-sm font-medium"),
            class_name="w-16 text-right",
        ),
        # 操作
        rx.el.td(
            rx.el.button(
                rx.icon("pencil", size=12),
                on_click=lambda: AppState.select_item(item["id"]),
                class_name=rx.cond(
                    is_selected,
                    "btn btn-primary btn-xs",
                    "btn btn-ghost btn-xs",
                ),
                title="編輯細節",
            ),
            class_name="w-12",
        ),
        class_name=rx.cond(
            item["included"],
            rx.cond(
                is_selected,
                "cursor-pointer bg-primary/20 border-l-4 border-primary font-semibold",
                "hover:bg-base-200 cursor-pointer",
            ),
            "opacity-40 cursor-pointer",
        ),
        on_click=lambda: AppState.select_item(item["id"]),
    )


def worklog_item(wl: dict) -> rx.Component:
    """單一 Worklog 項目 - 支援獨立分類"""
    return rx.el.div(
        # Worklog 文字內容
        rx.el.div(
            rx.input(
                value=wl["text"],
                on_change=lambda v: AppState.update_worklog_text(AppState.selected_item_id, wl["id"], v),
                debounce_timeout=300,
                class_name="input input-bordered input-sm flex-1 text-sm",
                placeholder="輸入工作紀錄內容...",
            ),
            class_name="flex-1",
        ),
        # 工作項目分類選擇
        rx.el.div(
            rx.el.select(
                rx.el.option("跟隨 Task", value="-1"),
                rx.foreach(
                    AppState.categories,
                    lambda cat: rx.el.option(cat["name"], value=cat["id"].to_string()),
                ),
                value=wl["work_item_id"].to_string(),
                on_change=lambda e: AppState.assign_worklog_to_work_item(
                    AppState.selected_item_id, wl["id"], e
                ),
                class_name="select select-xs select-bordered w-24",
                title="指派到工作項目（或跟隨 Task）",
            ),
        ),
        # 刪除按鈕
        rx.el.button(
            rx.icon("trash_2", size=12),
            on_click=lambda: AppState.remove_worklog(AppState.selected_item_id, wl["id"]),
            class_name="btn btn-ghost btn-sm text-error",
        ),
        class_name="flex gap-2 items-center",
    )


def worklog_editor() -> rx.Component:
    """Worklog 編輯器 - 選中 Task 的詳細編輯區"""
    return rx.cond(
        AppState.has_selected_item,
        rx.el.div(
            rx.el.div(
                # 標題列
                rx.el.h3(
                    rx.el.span(AppState.selected_item["issue_key"], class_name="text-primary font-mono"),
                    " ",
                    AppState.selected_item["issue_name"],
                    class_name="font-bold text-lg",
                ),

                # Worklogs 列表
                rx.el.div(
                    rx.el.div(
                        rx.el.div(
                            rx.el.span("Worklogs", class_name="label-text font-medium"),
                            rx.el.span("可獨立指派到不同工作項目", class_name="text-xs text-base-content/50 ml-2"),
                        ),
                        rx.el.button(
                            rx.icon("plus", size=12),
                            " 新增",
                            on_click=lambda: AppState.add_worklog(AppState.selected_item_id),
                            class_name="btn btn-outline btn-xs",
                        ),
                        class_name="label flex justify-between",
                    ),
                    rx.el.div(
                        rx.foreach(
                            AppState.selected_item_worklogs,
                            worklog_item,
                        ),
                        class_name="flex flex-col gap-2 max-h-72 overflow-y-auto",
                    ),
                    class_name="form-control mt-4",
                ),

                # 備註
                rx.el.div(
                    rx.el.label(
                        rx.el.span("補充備註", class_name="label-text font-medium"),
                        class_name="label",
                    ),
                    rx.input(
                        value=AppState.selected_item["custom_note"],
                        on_change=lambda v: AppState.update_item_note(AppState.selected_item_id, v),
                        placeholder="額外補充說明...",
                        debounce_timeout=300,
                        class_name="input input-bordered w-full input-sm",
                    ),
                    class_name="form-control mt-4",
                ),

                class_name="card-body",
            ),
            class_name="card bg-base-100 border-2 border-primary",
        ),
    )


def category_filter_option(cat: dict) -> rx.Component:
    """分類篩選選項"""
    return rx.el.option(cat["name"], value=cat["id"].to_string())


def category_filter_dropdown() -> rx.Component:
    """分類篩選下拉選單"""
    return rx.el.div(
        rx.icon("filter", size=14, class_name="text-base-content/50"),
        rx.el.select(
            rx.el.option("全部", value="-999"),
            rx.foreach(
                AppState.categories_with_count,
                category_filter_option,
            ),
            rx.cond(
                AppState.unassigned_count > 0,
                rx.el.option("未分類", value="-1"),
            ),
            value=AppState.filter_category_id.to_string(),
            on_change=AppState.set_filter_category,
            class_name="select select-ghost select-sm min-w-28",
        ),
        class_name="flex items-center gap-1",
    )


def step3_organize() -> rx.Component:
    """步驟三：資料整理 - 使用 Tabs 介面"""
    return rx.el.div(
        # 可收合基本設定
        collapsible_basic_settings(),

        # 操作提示
        rx.cond(
            ~AppState.all_summaries_generated,
            rx.el.div(
                rx.icon("info", size=16, class_name="text-info flex-shrink-0"),
                rx.el.div(
                    rx.el.span("操作說明：", class_name="font-semibold"),
                    rx.el.span("請先使用「AI 智慧分類」自動分類工作項目，或手動調整分類。完成後點擊「全部生成摘要」產生各分類的工作內容摘要。"),
                    class_name="text-sm",
                ),
                class_name="alert alert-info py-2 flex items-start gap-2",
            ),
        ),

        # 統計摘要
        rx.el.div(
            rx.el.div(
                rx.el.div("納入項目", class_name="stat-title"),
                rx.el.div(
                    AppState.included_items_count.to_string() + " / " + AppState.total_items_count.to_string(),
                    class_name="stat-value text-primary text-2xl",
                ),
                class_name="stat",
            ),
            rx.el.div(
                rx.el.div("納入工時", class_name="stat-title"),
                rx.el.div(AppState.included_hours.to_string() + " h", class_name="stat-value text-secondary text-2xl"),
                class_name="stat",
            ),
            rx.el.div(
                rx.el.div("工作項目摘要", class_name="stat-title"),
                rx.el.div(
                    AppState.categories_with_summary_count.to_string() + " / " + AppState.total_categories_count.to_string(),
                    class_name="stat-value text-accent text-2xl",
                ),
                class_name="stat",
            ),
            class_name="stats stats-horizontal bg-base-100 border border-base-300 w-full",
        ),

        # ===== Tabs 介面 =====
        rx.el.div(
            # Tab 標籤
            rx.el.div(
                rx.el.button(
                    rx.icon("folder_kanban", size=16),
                    " 工作項目",
                    rx.el.span(
                        AppState.total_categories_count.to_string(),
                        class_name="badge badge-sm ml-1",
                    ),
                    on_click=lambda: AppState.set_step3_tab("categories"),
                    class_name=rx.cond(
                        AppState.step3_active_tab == "categories",
                        "tab tab-active",
                        "tab",
                    ),
                ),
                rx.el.button(
                    rx.icon("list", size=16),
                    " 任務列表",
                    rx.el.span(
                        AppState.total_items_count.to_string(),
                        class_name="badge badge-sm ml-1",
                    ),
                    on_click=lambda: AppState.set_step3_tab("tasks"),
                    class_name=rx.cond(
                        AppState.step3_active_tab == "tasks",
                        "tab tab-active",
                        "tab",
                    ),
                ),
                class_name="tabs tabs-bordered mb-4",
            ),

            # Tab 內容
            rx.cond(
                AppState.step3_active_tab == "categories",
                # ===== Tab 1: 工作項目 =====
                rx.el.div(
                    # 工具列
                    rx.el.div(
                        rx.el.div(
                            rx.el.button(
                                rx.cond(
                                    AppState.suggesting_work_items,
                                    rx.el.span(
                                        rx.el.span(class_name="loading loading-spinner loading-xs"),
                                        " AI 分析中...",
                                        class_name="flex items-center gap-1",
                                    ),
                                    rx.el.span(
                                        rx.icon("sparkles", size=14),
                                        " AI 智慧分類",
                                        class_name="flex items-center gap-1",
                                    ),
                                ),
                                on_click=AppState.ai_suggest_work_items,
                                disabled=AppState.suggesting_work_items,
                                class_name="btn btn-secondary btn-sm",
                                title="讓 AI 分析所有 Task 並建議工作項目分類",
                            ),
                            rx.el.button(
                                rx.icon("folder_plus", size=14),
                                " 從專案建立",
                                on_click=AppState.auto_create_categories_from_projects,
                                class_name="btn btn-outline btn-sm",
                                title="依專案名稱快速建立工作項目",
                            ),
                            rx.el.button(
                                rx.cond(
                                    AppState.is_busy,
                                    rx.el.span(
                                        rx.el.span(class_name="loading loading-spinner loading-xs"),
                                        " 分類中...",
                                        class_name="flex items-center gap-1",
                                    ),
                                    rx.el.span(
                                        rx.icon("wand_sparkles", size=14),
                                        " AI 重新分類",
                                        class_name="flex items-center gap-1",
                                    ),
                                ),
                                on_click=AppState.ai_auto_categorize,
                                disabled=rx.cond(
                                    AppState.is_busy,
                                    True,
                                    AppState.total_categories_count == 0,
                                ),
                                class_name="btn btn-accent btn-sm",
                                title="根據現有工作項目的名稱與描述，讓 AI 重新分類所有 Tasks",
                            ),
                            # 分隔線
                            rx.el.div(class_name="divider divider-horizontal mx-1"),
                            # 全部生成摘要按鈕
                            rx.el.button(
                                rx.cond(
                                    AppState.generating_summary_for >= 0,
                                    rx.el.span(
                                        rx.el.span(class_name="loading loading-spinner loading-xs"),
                                        " 生成中...",
                                        class_name="flex items-center gap-1",
                                    ),
                                    rx.el.span(
                                        rx.icon("file_text", size=14),
                                        " 全部生成摘要",
                                        class_name="flex items-center gap-1",
                                    ),
                                ),
                                on_click=AppState.generate_all_summaries,
                                disabled=rx.cond(
                                    AppState.generating_summary_for >= 0,
                                    True,
                                    AppState.total_categories_count == 0,
                                ),
                                class_name="btn btn-primary btn-sm",
                                title="為所有工作項目生成摘要",
                            ),
                            class_name="flex gap-2 items-center",
                        ),
                        class_name="flex items-center justify-between mb-3",
                    ),
                    # 新增工作項目
                    rx.el.div(
                        rx.input(
                            placeholder="新增工作項目名稱...",
                            value=AppState.new_category_name,
                            on_change=AppState.set_new_category_name,
                            debounce_timeout=300,
                            class_name="input input-bordered input-sm flex-1",
                        ),
                        rx.el.button(
                            rx.icon("plus", size=14),
                            on_click=AppState.add_category,
                            class_name="btn btn-primary btn-sm",
                        ),
                        class_name="flex gap-2 mb-4",
                    ),
                    # 工作項目卡片
                    rx.el.div(
                        rx.foreach(
                            AppState.categories_with_count,
                            lambda cat: rx.el.div(
                                # 標題列
                                rx.el.div(
                                    rx.el.div(
                                        rx.el.span(cat["name"], class_name=f"badge badge-{cat['color']} badge-sm"),
                                        rx.el.span(
                                            cat["count"].to_string() + " Tasks",
                                            class_name="text-xs text-base-content/50 ml-2",
                                        ),
                                        class_name="flex items-center cursor-pointer hover:opacity-70",
                                        on_click=lambda: AppState.open_category_modal(cat["id"].to(int)),
                                    ),
                                    rx.el.div(
                                        rx.el.button(
                                            rx.icon("pencil", size=12),
                                            on_click=lambda: AppState.open_category_modal(cat["id"].to(int)),
                                            class_name="btn btn-ghost btn-xs",
                                            title="編輯工作項目",
                                        ),
                                        rx.el.button(
                                            rx.cond(
                                                AppState.generating_summary_for == cat["id"],
                                                rx.el.span(class_name="loading loading-spinner loading-xs"),
                                                rx.icon("sparkles", size=12),
                                            ),
                                            on_click=lambda: AppState.regenerate_category_summary(cat["id"]),
                                            disabled=AppState.generating_summary_for >= 0,
                                            class_name="btn btn-ghost btn-xs",
                                            title="重新生成摘要",
                                        ),
                                        rx.el.button(
                                            rx.icon("x", size=10),
                                            on_click=lambda: AppState.delete_category(cat["id"]),
                                            class_name="btn btn-ghost btn-xs btn-circle text-error",
                                            title="刪除工作項目",
                                        ),
                                        class_name="flex items-center",
                                    ),
                                    class_name="flex items-center justify-between",
                                ),
                                # Tasks 列表
                                rx.cond(
                                    cat["count"].to(int) > 0,
                                    rx.el.div(
                                        rx.foreach(
                                            cat["tasks"].to(list[dict]),
                                            lambda task: rx.el.div(
                                                rx.el.span(task["issue_key"], class_name="font-mono text-xs text-primary font-bold"),
                                                rx.el.span(task["issue_name"], class_name="text-xs text-base-content/70 truncate flex-1 ml-1"),
                                                rx.el.span(task["hours"].to_string() + "h", class_name="text-xs text-base-content/50"),
                                                class_name="flex items-center gap-1 px-2 py-1 bg-base-100 rounded text-xs",
                                            ),
                                        ),
                                        class_name="flex flex-col gap-1 mt-2 max-h-24 overflow-y-auto",
                                    ),
                                ),
                                # 獨立 Worklogs
                                rx.cond(
                                    cat["orphan_count"].to(int) > 0,
                                    rx.el.div(
                                        rx.el.div(
                                            rx.icon("corner_down_right", size=10, class_name="text-warning"),
                                            rx.el.span("獨立 Worklogs", class_name="text-xs text-warning font-medium"),
                                            class_name="flex items-center gap-1 mb-1",
                                        ),
                                        rx.foreach(
                                            cat["orphan_worklogs"].to(list[dict]),
                                            lambda wl: rx.el.div(
                                                rx.el.div(
                                                    rx.el.span("↳ ", class_name="text-base-content/30"),
                                                    rx.el.span(wl["parent_task_key"], class_name="font-mono text-xs text-base-content/50"),
                                                    class_name="flex items-center",
                                                ),
                                                rx.el.p(wl["text"], class_name="text-xs text-base-content/60 truncate pl-3"),
                                                class_name="px-2 py-1 bg-warning/10 rounded border-l-2 border-warning/30",
                                            ),
                                        ),
                                        class_name="flex flex-col gap-1 mt-2 max-h-20 overflow-y-auto",
                                    ),
                                ),
                                # 摘要
                                rx.cond(
                                    cat["summary"] != "",
                                    rx.el.textarea(
                                        value=cat["summary"],
                                        on_change=lambda v: AppState.update_category_summary(cat["id"], v),
                                        debounce_timeout=500,
                                        class_name="textarea textarea-bordered textarea-xs w-full mt-2 text-xs leading-relaxed",
                                        rows=2,
                                    ),
                                    rx.el.p(
                                        "點擊 ✨ 生成摘要",
                                        class_name="text-xs text-base-content/40 mt-1 italic",
                                    ),
                                ),
                                class_name="p-3 bg-base-200 rounded-lg flex-1 min-w-[280px]",
                            ),
                        ),
                        class_name="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3",
                    ),
                    class_name="min-h-[400px]",
                ),
                # ===== Tab 2: 任務列表 (固定兩欄排版) =====
                rx.el.div(
                    # 兩欄排版
                    rx.el.div(
                        # 左欄：篩選 + 表格
                        rx.el.div(
                            # 篩選列
                            rx.el.div(
                                category_filter_dropdown(),
                                class_name="flex justify-start mb-3",
                            ),
                            # 表格
                            rx.el.div(
                                rx.el.table(
                                    rx.el.thead(
                                        rx.el.tr(
                                            rx.el.th("", class_name="w-8"),
                                            rx.el.th("Key", class_name="w-20"),
                                            rx.el.th("Task 名稱"),
                                            rx.el.th("工作項目", class_name="w-24"),
                                            rx.el.th("時數", class_name="w-14 text-right"),
                                            class_name="text-xs",
                                        ),
                                    ),
                                    rx.el.tbody(
                                        rx.foreach(
                                            AppState.filtered_work_items,
                                            task_row,
                                        ),
                                    ),
                                    class_name="table table-xs table-pin-rows",
                                ),
                                class_name="overflow-x-auto max-h-[calc(100vh-420px)] min-h-[350px] overflow-y-auto",
                            ),
                            class_name="flex-1 min-w-0",
                        ),
                        # 右欄：Worklog 編輯器 (固定顯示)
                        rx.el.div(
                            rx.cond(
                                AppState.has_selected_item,
                                worklog_editor(),
                                # 未選擇時顯示提示
                                rx.el.div(
                                    rx.icon("mouse_pointer_click", size=48, class_name="text-base-content/20 mb-4"),
                                    rx.el.p("點擊左側任務查看詳情", class_name="text-base-content/40 text-sm"),
                                    rx.el.p("可編輯 Worklog 並調整分類", class_name="text-base-content/30 text-xs mt-1"),
                                    class_name="flex flex-col items-center justify-center h-full",
                                ),
                            ),
                            class_name="w-[420px] flex-shrink-0 bg-base-200 rounded-lg p-4 min-h-[400px]",
                        ),
                        class_name="flex gap-4",
                    ),
                    class_name="min-h-[400px]",
                ),
            ),
            class_name="p-4 bg-base-100 rounded-lg border border-base-300",
        ),

        nav_buttons(),
        class_name="flex flex-col gap-4",
    )


# ===== Step 4: 生成報告 =====

def step4_generate() -> rx.Component:
    """步驟四：生成報告內容預覽"""
    return rx.el.div(
        # 可收合基本設定
        collapsible_basic_settings(),

        # AI 草稿產生
        card("AI 草稿產生", [
            rx.el.button(
                rx.cond(
                    AppState.is_generating,
                    rx.el.span(
                        rx.el.span(class_name="loading loading-spinner loading-sm"),
                        " AI 產生中...",
                        class_name="flex items-center gap-2",
                    ),
                    rx.el.span(
                        rx.icon("sparkles", size=16),
                        " 產生績效考核草稿",
                        class_name="flex items-center gap-2",
                    ),
                ),
                on_click=AppState.generate_drafts,
                disabled=AppState.is_generating,
                class_name="btn btn-secondary w-full",
            ),
            rx.cond(
                AppState.has_drafts,
                rx.el.div(
                    rx.icon("circle_check", size=16, class_name="text-success"),
                    rx.el.span("草稿已產生，可在下方編輯", class_name="text-sm"),
                    class_name="flex items-center gap-2 mt-2",
                ),
            ),
        ]),

        # 報告內容預覽 - 模擬輸出格式
        rx.cond(
            AppState.has_drafts,
            rx.el.div(
                # 員工資訊摘要
                card("報告預覽", [
                    rx.el.div(
                        rx.el.div(
                            rx.el.span("員工姓名：", class_name="font-medium"),
                            rx.el.span(AppState.emp_name),
                            class_name="flex gap-2",
                        ),
                        rx.el.div(
                            rx.el.span("部門：", class_name="font-medium"),
                            rx.el.span(AppState.emp_dept),
                            class_name="flex gap-2",
                        ),
                        rx.el.div(
                            rx.el.span("職稱：", class_name="font-medium"),
                            rx.el.span(AppState.emp_title),
                            class_name="flex gap-2",
                        ),
                        rx.el.div(
                            rx.el.span("考核期間：", class_name="font-medium"),
                            rx.el.span(AppState.emp_period),
                            class_name="flex gap-2",
                        ),
                        class_name="grid grid-cols-2 md:grid-cols-4 gap-4 p-4 bg-base-200 rounded-lg text-sm",
                    ),
                ]),

                # 工作成果
                rx.el.div(
                    rx.el.div(
                        rx.el.h3("一、工作成果", class_name="font-bold text-lg border-b-2 border-primary pb-2"),
                        rx.text_area(
                            value=AppState.work_draft,
                            on_change=AppState.set_work_draft,
                            rows="8",
                            debounce_timeout=300,
                            class_name="textarea textarea-bordered w-full mt-4",
                        ),
                        class_name="card-body",
                    ),
                    class_name="card bg-base-100 border border-base-300",
                ),

                # 技能發展
                rx.el.div(
                    rx.el.div(
                        rx.el.h3("二、技能發展", class_name="font-bold text-lg border-b-2 border-secondary pb-2"),
                        rx.text_area(
                            value=AppState.skill_draft,
                            on_change=AppState.set_skill_draft,
                            rows="5",
                            debounce_timeout=300,
                            class_name="textarea textarea-bordered w-full mt-4",
                        ),
                        class_name="card-body",
                    ),
                    class_name="card bg-base-100 border border-base-300",
                ),

                # 職場專業素養
                rx.el.div(
                    rx.el.div(
                        rx.el.h3("三、職場專業素養", class_name="font-bold text-lg border-b-2 border-accent pb-2"),
                        rx.text_area(
                            value=AppState.ethics_draft,
                            on_change=AppState.set_ethics_draft,
                            rows="5",
                            debounce_timeout=300,
                            class_name="textarea textarea-bordered w-full mt-4",
                        ),
                        class_name="card-body",
                    ),
                    class_name="card bg-base-100 border border-base-300",
                ),
                class_name="flex flex-col gap-4",
            ),
        ),

        # 匯出
        card("匯出報告", [
            rx.el.button(
                rx.cond(
                    AppState.is_exporting,
                    rx.el.span(
                        rx.el.span(class_name="loading loading-spinner loading-sm"),
                        " 匯出中...",
                        class_name="flex items-center gap-2",
                    ),
                    rx.el.span(
                        rx.icon("download", size=16),
                        " 匯出 Excel 報告",
                        class_name="flex items-center gap-2",
                    ),
                ),
                on_click=AppState.export_excel,
                disabled=AppState.is_exporting,
                class_name="btn btn-success w-full",
            ),
            rx.cond(
                AppState.download_url != "",
                rx.link(
                    rx.el.button(
                        rx.icon("file_spreadsheet", size=16),
                        " 下載檔案",
                        class_name="btn btn-info w-full mt-2",
                    ),
                    href=AppState.download_url,
                    is_external=True,
                ),
            ),
        ]),

        # 導航
        rx.el.div(
            rx.el.button(
                rx.icon("arrow_left", size=16),
                " 上一步",
                on_click=AppState.prev_step,
                class_name="btn btn-outline",
            ),
            rx.el.button(
                rx.icon("rotate_ccw", size=16),
                " 重新開始",
                on_click=AppState.reset_all,
                class_name="btn btn-error btn-outline",
            ),
            class_name="flex justify-between pt-6",
        ),
        class_name="flex flex-col gap-4",
    )


# ===== 主頁面 =====

def index() -> rx.Component:
    """主頁面"""
    return rx.el.div(
        # 全域載入遮罩
        loading_overlay(),

        # Settings Modal
        settings_modal(),

        # Token Modal
        token_modal(),

        # Session Modal
        session_modal(),

        # Category Edit Modal
        category_edit_modal(),

        rx.el.div(
            # Header
            rx.el.div(
                rx.el.div(
                    rx.el.h1("Tempo Worklog 分析器", class_name="text-2xl md:text-3xl font-bold"),
                    rx.el.p("快速將 Tempo 工時報表轉換為績效考核格式", class_name="text-base-content/60 text-sm md:text-base"),
                    class_name="text-center flex-1",
                ),
                rx.el.div(
                    # 離開/切換 Session 按鈕
                    rx.el.button(
                        rx.icon("folder_open", size=18),
                        on_click=AppState.open_session_modal,
                        class_name="btn btn-ghost btn-circle",
                        title="工作進度管理",
                    ),
                    # 設定按鈕
                    rx.el.button(
                        rx.icon("settings", size=20),
                        on_click=AppState.toggle_settings,
                        class_name="btn btn-ghost btn-circle",
                        title="系統設定",
                    ),
                    class_name="absolute right-4 top-4 flex gap-1",
                ),
                class_name="relative py-6",
            ),

            # Step Indicator
            steps_indicator(),

            # Message Bar
            rx.el.div(
                alert_message(),
                class_name="py-2",
            ),

            # Main Content
            rx.match(
                AppState.current_step,
                (1, step1_basic_info()),
                (2, step2_upload()),
                (3, step3_organize()),
                (4, step4_generate()),
                step1_basic_info(),
            ),

            class_name="max-w-7xl mx-auto px-4 py-4",
        ),
        class_name="min-h-screen bg-base-200",
        data_theme="gentle-paw",
        on_mount=AppState.on_load,
    )


# ===== App =====
app = rx.App(
    stylesheets=[
        "/styles.css",
        "https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap",
    ],
)
app.add_page(index, title="Tempo Worklog 分析器")
