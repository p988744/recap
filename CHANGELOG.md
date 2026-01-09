# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
