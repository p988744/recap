# Worklog Helper

從 Claude Code session 自動生成 Jira Tempo worklog。

## 安裝

```bash
# 使用 uv (推薦)
uv tool install worklog-helper

# 或使用 pip
pip install worklog-helper
```

## 使用方式

```bash
# 互動模式（預設）
worklog

# 分析本週
worklog analyze --week

# 分析上週
worklog analyze --last-week

# 分析後直接上傳
worklog analyze --upload

# 配置 Jira 連接
worklog setup

# 列出可用日期
worklog dates
```

## 工作流程

1. **盤點**: 自動解析 `~/.claude/` 目錄下的 session 數據
2. **對應**: 互動式指定每個專案對應的 Jira Issue ID
3. **上傳**: 確認後自動上傳到 Jira/Tempo

## 功能特點

- 支援時間範圍選擇（本週、上週、自訂範圍、單日）
- 記住專案與 Jira Issue 的對應關係
- 美觀的 CLI 介面（使用 Rich）
- 支援 Jira REST API 和 Tempo API

## 配置

首次使用需要配置 Jira 連接：

```bash
worklog setup
```

需要提供：
- Jira URL (例如 `https://your-jira.com`)
- Jira Email
- Jira API Token
- Tempo API Token (可選)

配置儲存在 `~/.worklog-helper/config.json`。

## License

MIT
