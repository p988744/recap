# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
