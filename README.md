# Tempo Sync

同步開發活動到 Jira Tempo worklog。

## 安裝

```bash
# 使用 uv (推薦)
uv tool install tempo-sync

# 或使用 pip
pip install tempo-sync
```

## 使用方式

```bash
# 互動模式（預設）
tempo

# 分析本週
tempo analyze --week

# 分析上週
tempo analyze --last-week

# 分析後直接上傳
tempo analyze --upload

# 配置 Jira 連接
tempo setup

# 列出可用日期
tempo dates
```

## 資料來源

目前支援：
- **Claude Code sessions**: 自動解析 `~/.claude/` 目錄下的 session 數據
- **Outlook 行事曆**: 整合 Microsoft 365 會議和請假資訊（需管理員授權）

未來規劃：
- Git commits
- GitHub/GitLab 活動

## 工作流程

1. **盤點**: 自動解析各來源的工作紀錄
2. **對應**: 互動式指定每個項目對應的 Jira Issue ID
3. **上傳**: 確認後自動上傳到 Jira/Tempo

## 功能特點

- 支援時間範圍選擇（本週、上週、自訂範圍、過去 N 天）
- 記住專案與 Jira Issue 的對應關係
- LLM 自動彙整工作描述
- 美觀的 CLI 介面（使用 Rich）
- 支援 Jira REST API 和 Tempo API

## 配置

首次使用需要配置 Jira 連接：

```bash
tempo setup
```

需要提供：
- Jira URL (例如 `https://your-jira.com`)
- Jira PAT 或 Email + API Token
- Tempo API Token (可選)

配置儲存在 `~/.tempo-sync/config.json`。

### LLM 配置（可選）

```bash
tempo setup-llm
```

支援 Anthropic、OpenAI、Gemini、Ollama 或 OpenAI 相容端點。

### Outlook 整合（可選）

```bash
tempo outlook-login
```

需要 Azure AD 應用程式註冊及管理員授權。

## License

MIT
