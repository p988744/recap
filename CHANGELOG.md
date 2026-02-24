# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.2.0-beta.2] - 2026-02-24

### Fixed

- **Compaction TOCTOU 競態條件** - `begin_compaction()` 改用 `AtomicBool::compare_exchange` 原子操作，消除兩個 compaction 同時執行的風險
- **LLM API 無超時保護** - `reqwest::Client` 加入 30 秒 timeout，防止 LLM API 無回應時永久阻塞 compaction
- **E2E 測試假陽性** - 移除 `if (isDisplayed)` guard 模式，改為 `waitForDisplayed` + `expect().toBeDisplayed()` 明確斷言

### Added

- **danger_zone.rs 測試覆蓋** - 新增 14 個測試覆蓋 `clear_synced_data` 和 `factory_reset` 破壞性操作，包含用戶隔離和邊界情況
- **GPT-5 路由測試** - 新增 35 個測試覆蓋 GPT-5 Responses API 偵測、請求序列化、回應解析和路由整合

## [2.2.0-beta.1] - 2026-02-24

### Added

- **E2E 測試基礎設施** - 新增 WebdriverIO + tauri-driver E2E 測試框架
  - 4 個測試檔案覆蓋登入、儀表板、導覽、設定關鍵路徑
  - GitHub Actions CI workflow（ubuntu-22.04 + xvfb）
  - 註：tauri-driver 不支援 macOS，E2E 僅在 CI Linux 環境執行
- **同步狀態 UI 強化** - DataSyncStatus 顯示同步經過時間、2 分鐘後出現取消按鈕、5 分鐘卡住警告

### Fixed

- **同步重啟競態條件** - `restart()` 改為輪詢 lifecycle state（最多 10 秒），避免舊同步未結束就啟動新同步
- **LLM 警告可見性** - LLM 呼叫的 warning 訊息現在顯示在 `last_error`，使用者可在 UI 看到
- **Compaction panic 安全** - Compaction 迴圈加入 panic catch，單一專案失敗不會中斷整體壓縮
- **同步卡住恢復** - 偵測並自動恢復卡在 Syncing/Compacting 狀態的同步服務
- **取消同步指令** - 新增 `cancel_sync` command，前端可主動中斷長時間同步

### Improved

- **Compaction 平行化** - 壓縮迴圈改用 `chunks + join_all` 平行處理，提升大量專案的壓縮速度
- **背景同步測試覆蓋** - 新增 20 個 background-sync service 測試

## [2.2.0-alpha.17] - 2026-02-24

### Fixed

- **Compaction panic 安全** - Compaction 迴圈加入 panic catch，單一專案失敗不會中斷整體壓縮
- **同步卡住恢復** - 偵測並自動恢復卡在 Syncing/Compacting 狀態的同步服務
- **取消同步指令** - 新增 `cancel_sync` command，前端可主動中斷長時間同步
- **同步啟動競態條件** - 修正背景同步啟動時的 race condition 和 stuck state recovery

### Improved

- **Compaction 平行化** - 壓縮迴圈改用 `chunks + join_all` 平行處理，提升大量專案的壓縮速度

## [2.2.0-alpha.16] - 2026-02-13

### Improved

- **LLM 錯誤訊息可展開** - 呼叫記錄中的 ERR 狀態可點擊展開查看完整錯誤訊息

## [2.2.0-alpha.15] - 2026-02-13

### Added

- **Git commit 使用者篩選** - 自動偵測 `git config user.email`，只顯示自己的 commit（排除他人 commit）
  - 影響範圍：snapshot 擷取、worklog overview、timeline、commit-centric worklog、CLI dashboard
- **摘要 Prompt 自訂** - 進階設定新增可編輯的 LLM 摘要 Prompt，支援 `{data}`、`{length_hint}`、`{context_section}` 變數
  - 預設 Prompt 直接顯示為 textarea 值，使用者可直接修改
  - 新增預覽功能：以範例資料替換變數，即時檢視最終 Prompt 效果
  - 「恢復預設」按鈕一鍵還原
- **進階設定頁面重構** - 將 LLM 參數（摘要字數、推理強度、Prompt）從「系統設定」移至獨立的「進階設定」分頁，與危險區域合併

### Fixed

- **LLM 測試連線誤判** - GPT-5 Responses API 回傳極短回應（trivial response / no text content）不再誤判為失敗

## [2.2.0-alpha.14] - 2026-02-12

### Added

- **LLM 預設快速切換** - 儲存多組 LLM 設定，一鍵切換不同模型/API
  - 「已儲存的設定」區塊：顯示預設名稱、provider/model、使用中標記
  - 點擊預設即套用（自動更新 provider、model、API key、base URL）
  - 支援新增/刪除預設，預設按最近使用排序
  - 新增 `llm_presets` 資料表與 4 個 Tauri commands

### Fixed

- **LLM 設定表單空白** - 修正點擊已選中的 provider 按鈕會清空模型名稱和 API URL 的問題
- **測試連線使用表單值** - 「測試連線」按鈕現在測試表單當前輸入的值（而非 DB 已存值），可先測試再儲存
- **LLM max_tokens 參數** - 各 LLM 呼叫點加入明確的 max_tokens 參數

## [2.2.0-alpha.13] - 2026-02-11

### Fixed

- **README HTML 渲染** - 專案描述頁面的 README 現在正確渲染 HTML 標籤（新增 `rehype-raw` plugin）

### Removed

- **隱藏 Antigravity 整合** - 前端完全移除所有 Antigravity (Gemini) 相關 UI
  - 移除專案卡片、時間軸、Worklog 等處的 Antigravity 來源標籤
  - 移除 Dashboard 和 WorkItems 的 Antigravity 來源篩選器
  - 移除設定頁面的 Antigravity API 狀態檢查和路徑設定
  - 移除同步設定中的 Antigravity 開關
  - 移除 Onboarding 流程中的 Antigravity 連線測試
  - 後端同步設定保持不變，僅隱藏前端 UI

## [2.2.0-alpha.12] - 2026-02-11

### Added

- **通用 HTTP 匯出功能** - 支援將工作項目透過自訂 HTTP API 匯出到任意外部服務
  - Settings 頁面新增 HTTP Export 設定卡片，可配置多個匯出端點（URL、Auth、Payload Template、LLM Prompt）
  - 支援 `{{title}}`、`{{hours}}`、`{{date}}` 等 placeholder 的 JSON 模板引擎
  - 支援 Bearer / Basic / Custom Header 認證方式
  - 支援批次模式（一次 POST 陣列）與逐筆模式
  - 選用 LLM 自訂 prompt 摘要（`{{llm_summary}}` placeholder）
  - 匯出前可預覽 Payload（Dry Run）
  - 匯出歷史追蹤，避免重複匯出到相同目標
- **統一匯出下拉選單** - ThisWeek、Worklog、WorkItems 頁面的 Tempo 匯出與 HTTP 匯出合併為單一下拉選單

## [2.2.0-alpha.11] - 2026-02-10

### Changed

- **背景同步排程引擎重寫** - 用 `tokio-cron-scheduler` 取代手刻 `tokio::time::interval` loop，解決「下次同步」時間不準確的長期問題
  - Scheduler 獨立管理計時，不受同步操作阻塞影響
  - 新增 overlap prevention：前一次同步/壓縮未完成時自動跳過
  - `get_status()` 呼叫 `refresh_next_times()` 從 scheduler 取得真實下次觸發時間

## [2.1.0-rc.1] - 2026-02-06

### Added

- **Antigravity (Gemini Code) 整合** - 透過 API 即時偵測 Antigravity 應用狀態、背景同步 session 資料
- **Projects 頁面** - 全新 master-detail 佈局，含 README 預覽、專案時間軸、描述管理
- **ThisWeek 頁面** - 本週總覽含 heatmap、Gantt chart、可展開的每日卡片
- **手動專案/工作項目** - 支援新增、編輯、刪除手動工作項目，含專案選擇器
- **Compaction 系統** - 批次壓縮、進度追蹤、背景執行、danger zone UI
- **Onboarding 引導精靈** - 重新設計為初始設定精靈
- **Tempo 匯出** - 日期詳情頁直接匯出至 Tempo
- **SyncSource trait** - 資料來源抽象化，簡化新增整合來源
- **Release CI/CD** - 自動化跨平台建置與發布工作流程

### Changed

- Gantt chart 顯示手動項目，並重新命名 Antigravity badge
- 手動項目格式從 Markdown 改為 JSONL
- 移除 GitLab/Jira 同步開關和回顧頁面
- 專案描述改用 README 內容

### Fixed

- Compaction interval 單位不一致問題
- 背景同步狀態在 app 重啟後遺失
- Gantt chart 佈局溢出與連續時間段合併
- Antigravity 資料顯示與 LLM 摘要
- Git commit 資料流中的時間戳與 git root 解析
- Windows 建置時隱藏 CMD 視窗

## [2.0.0-rc.3] - 2026-01-28

### Changed

- **專案詳情改用 Dialog** - 將 Project Detail 從 Drawer 側面板改為 Dialog 模態視窗，簡化元件結構

### Fixed

- **CI artifact 上傳路徑** - 修正 build-desktop workflow 中 artifact 上傳路徑指向 workspace target 目錄
- **Windows CI 建置** - 移除 MSI 打包目標，僅使用 NSIS 安裝程式

## [2.0.0-rc.2] - 2026-01-28

### Fixed

- **Jira Issue 搜尋** - 修復 JQL 查詢無法搜尋部分 key（如 `PROJ-1`）和專案前綴（如 `PROJ`）的問題
- **ManualItemCard 佈局統一** - 手動項目卡片改為與 ProjectCard 相同的兩欄結構
- **IssueKeyCombobox 鍵盤操作** - 支援方向鍵選擇、Enter 確認、Escape 關閉，含 ARIA 無障礙屬性
- **TypeScript 編譯錯誤** - 修復 Dashboard SyncState 型別和所有測試 mock 資料

## [2.0.0-rc.1] - 2026-01-28

### Added

- **Worklog 頁面** - 全新每日工時總覽，按日期分組顯示工作紀錄
  - 每小時明細展開檢視
  - 手動新增工作項目
  - 專案卡片顯示 Git commits + Claude Code sessions 統計
- **Tempo 工時匯出** - 三種匯出模式
  - 單筆匯出（per-project）
  - 單日批次匯出（Export Day）
  - 整週批次匯出（Export Week）
  - 匯出前 LLM 自動摘要工作描述
  - 預覽（Preview / Dry-run）確認後再正式匯出
  - Jira Issue Key 驗證與自動補全（含 issue cache）
  - JiraBadge 顯示 issue 類型和摘要
- **LLM Token 用量追蹤** - 記錄每次 LLM 呼叫的 token 使用量
- **時區與週起始日設定** - 支援自訂時區和週起始日

### Changed

- **跨平台路徑處理** - 所有路徑操作改用 `Path` API 和 `dirs` crate，支援 Windows
- **API 層重構** - `lib/api.ts` + `lib/tauri-api.ts` 整合為 `services/` 目錄
- **大型模組拆分** - 所有 Rust commands 和 React 頁面已按模組/功能拆分

### Fixed

- LLM 摘要短路條件：單行描述不再跳過 LLM
- 移除 backend 重複摘要（frontend 已摘要，backend 不再重複呼叫）
- SQLite database lock 問題修復
- Worklog UTC 時區顯示修正

### Removed

- `upload_single_worklog` command（未使用的 dead code）

## [2.1.0] - 2026-01-09

### Changed

- **架構重構：移除 HTTP API，改用 Tauri IPC**
  - 前端直接透過 `invoke()` 呼叫 Rust Commands
  - 移除 Axum HTTP 伺服器（減少約 4,700 行程式碼）
  - 移除 Vite proxy 設定
  - 更快的前後端通訊，無需網路層
- **完整的 Tauri Commands 支援**
  - Auth: 登入、註冊、自動登入
  - Config: 設定管理、LLM 設定、Jira 設定
  - Work Items: CRUD、統計、時間軸、分組檢視
  - Claude Code: Session 列表、匯入、摘要、同步
  - Reports: 個人報表、摘要報表、Excel 匯出
  - Sync: 同步狀態、自動同步
  - GitLab: 專案管理、同步、搜尋
  - Tempo: 連線測試、Issue 驗證、Worklog 同步
  - Users: Profile 管理

### Added

- `addGitLabProject` 自動從 GitLab API 取得專案資訊
- `CLAUDE.md` 開發指南文件
- macOS 程式碼簽章和公證（Apple Developer ID）

### Removed

- Python CLI（已移除，專注於 Desktop App）

## [2.0.0] - 2026-01-09

### Added

- **Recap 桌面應用程式** - 全新 Tauri 桌面版本（Rust + React）
  - 跨平台支援：Linux、macOS（Intel & Apple Silicon）、Windows
  - 系統匣常駐
  - 自動更新功能
- **GitHub Actions CI/CD** - 自動化跨平台建置與發布
  - Nightly builds（每日自動建置）
  - Release builds（推送 tag 時觸發）
  - 自動清理舊的 nightly releases

### Changed

- 專案架構調整，桌面應用程式位於 `web/` 目錄
- GitLab CI 簡化為僅處理 Python CLI 部分

## [1.0.2] - 2025-12-31

### Added

- **Git 模式** - 無需 Claude Code 也能使用！從 Git commit 歷史估算工作時間
- **Git 倉庫管理指令**
  - `tempo git-add` - 快速添加多個倉庫
  - `tempo git-remove` - 移除倉庫（支援名稱或路徑）
  - `tempo git-list` - 列出已設定的倉庫及狀態
- **模式切換** - `tempo setup-git --enable/--disable` 快速切換預設模式
- **臨時模式覆蓋** - `--git` / `--no-git` 選項臨時切換模式

### Changed

- 更新 README 為完整使用手冊
- `analyze` 和 `dates` 命令現在會讀取 config 中的 `use_git_mode` 設定

### Fixed

- 移除 Outlook 整合中的硬編碼 Client ID（安全性修復）

## [1.0.1] - 2025-12-31

### Added

- **工時正規化** - 自動將每日工時正規化為標準 8 小時
- **30 分鐘單位** - 工時以 30 分鐘為單位四捨五入，產生更整齊的工時紀錄
- **比例分配** - 多任務時依實際花費時間比例分配標準工時

### Changed

- 上傳預覽現在顯示「原始時間」和「正規化時間」兩欄

## [1.0.0] - 2025-12-31

### Added

- **Claude Code Session 解析** - 自動從 `~/.claude/projects/` 讀取工作 session
- **Jira 整合** - 支援 Jira Server (PAT) 和 Jira Cloud (API Token) 認證
- **Tempo API 支援** - 可選的 Tempo Timesheets API 整合
- **LLM 智能彙整** - 支援多種 LLM 提供者自動彙整工作描述
  - Ollama (本地)
  - OpenAI
  - Anthropic Claude
  - Google Gemini
  - OpenAI Compatible endpoints
- **互動式 CLI** - 使用 Typer + Rich 的美觀命令列介面
- **時間範圍選擇** - 本週、上週、過去 N 天、自訂日期範圍
- **專案對應記憶** - 自動記住專案與 Jira Issue 的對應關係
- **每日流水帳格式** - 以日期為單位顯示工作紀錄
- **Outlook 整合 (實驗性)** - Microsoft 365 行事曆整合（需管理員授權）

### Commands

- `tempo` - 互動模式
- `tempo analyze` - 分析工作紀錄
- `tempo setup` - 配置 Jira 連接
- `tempo setup-llm` - 配置 LLM
- `tempo dates` - 列出可用日期
- `tempo outlook-login` - Outlook 登入
- `tempo outlook-logout` - Outlook 登出
