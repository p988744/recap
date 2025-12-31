# Tempo Sync

自動同步開發活動到 Jira Tempo Worklog 的 CLI 工具。

從 Claude Code sessions、Outlook 行事曆等多種來源收集工作紀錄，透過 LLM 自動彙整描述，一鍵上傳到 Jira Tempo。

## 功能特點

- **多來源整合** - 目前支援 Claude Code sessions，未來將支援 Git commits、GitHub/GitLab 活動
- **LLM 智能彙整** - 自動將多個工作項目彙整成簡潔的 worklog 描述
- **互動式對應** - 直覺的 CLI 介面，輕鬆將工作項目對應到 Jira Issue
- **記憶對應關係** - 自動記住專案與 Issue 的對應，下次使用免重新設定
- **彈性時間範圍** - 支援本週、上週、過去 N 天、自訂日期範圍
- **雙 API 支援** - 同時支援 Jira REST API 和 Tempo Timesheets API

## 快速開始

### 1. 安裝

```bash
# 使用 uv（推薦）
uv tool install tempo-sync

# 或使用 pip
pip install tempo-sync

# 如需 LLM 支援，安裝對應的額外依賴
pip install tempo-sync[openai]      # OpenAI
pip install tempo-sync[anthropic]   # Anthropic Claude
pip install tempo-sync[all-llm]     # 所有 LLM 提供者
```

### 2. 配置 Jira

```bash
tempo setup
```

依照提示輸入：
- **Jira URL**: 你的 Jira 伺服器位址（例如 `https://jira.example.com`）
- **認證方式**: 選擇 PAT（Personal Access Token）或 Email + API Token
- **Tempo API Token**（選填）: 如果使用 Tempo Timesheets

### 3. 配置 LLM（選填但推薦）

```bash
tempo setup-llm
```

選擇 LLM 提供者：
- **Ollama** - 本地運行，免費，無需 API Key
- **OpenAI** - GPT-4、GPT-3.5
- **Anthropic** - Claude
- **Gemini** - Google Gemini
- **OpenAI Compatible** - 任何相容 OpenAI API 的端點

### 4. 開始使用

```bash
# 互動模式（推薦）
tempo

# 或直接分析本週
tempo analyze --week
```

## 使用方式

### 互動模式

```bash
tempo
```

啟動後會顯示配置狀態，然後進入互動式流程：

1. **選擇時間範圍** - 本週、上週、過去 N 天、自訂範圍
2. **檢視工作紀錄** - 以日期為單位的流水帳格式
3. **上傳確認** - 逐日對應 Jira Issue 並上傳

### 命令列模式

```bash
# 分析本週工作
tempo analyze --week
tempo analyze -w

# 分析上週工作
tempo analyze --last-week
tempo analyze -l

# 分析過去 7 天
tempo analyze --days 7
tempo analyze -d 7

# 分析指定日期範圍
tempo analyze --start 2025-01-01 --end 2025-01-07
tempo analyze -s 2025-01-01 -e 2025-01-07

# 分析後直接進入上傳流程
tempo analyze --week --upload
tempo analyze -w -u
```

### 其他命令

```bash
# 列出有工作紀錄的日期
tempo dates

# 重新配置 Jira
tempo setup

# 重新配置 LLM
tempo setup-llm

# Outlook 整合（需 Azure AD 設定）
tempo outlook-login
tempo outlook-logout
```

## 資料來源

### Claude Code Sessions（已支援）

自動解析 `~/.claude/projects/` 目錄下的 session 資料，包含：
- 專案名稱
- 工作時間
- 對話內容摘要

### Outlook 行事曆（已支援，需設定）

整合 Microsoft 365 行事曆，自動加入：
- 會議時間
- 請假記錄

需要 Azure AD 應用程式註冊，詳見下方 [Outlook 整合設定](#outlook-整合設定)。

### Git Commits（規劃中）

未來將支援從 Git 提交記錄產生 worklog。

### GitHub/GitLab 活動（規劃中）

未來將支援從 GitHub/GitLab 的 PR、Issue、Code Review 等活動產生 worklog。

## 配置說明

所有配置儲存在 `~/.tempo-sync/` 目錄：

```
~/.tempo-sync/
├── config.json           # 主要配置檔
├── project_mapping.json  # 專案與 Jira Issue 的對應
└── outlook_token_cache.json  # Outlook 認證快取
```

### config.json 結構

```json
{
  "jira_url": "https://jira.example.com",
  "jira_pat": "your-personal-access-token",
  "auth_type": "pat",
  "tempo_api_token": "",
  "llm_provider": "openai-compatible",
  "llm_model": "gpt-4o-mini",
  "llm_api_key": "your-api-key",
  "llm_base_url": "https://api.openai.com",
  "outlook_enabled": false,
  "outlook_client_id": "",
  "outlook_tenant_id": ""
}
```

### 認證方式

#### Jira Server（PAT）

1. 前往 Jira → 個人設定 → Personal Access Tokens
2. 建立新的 Token
3. 執行 `tempo setup` 並選擇 PAT 認證

#### Jira Cloud（Email + API Token）

1. 前往 https://id.atlassian.com/manage-profile/security/api-tokens
2. 建立新的 API Token
3. 執行 `tempo setup` 並選擇 Email + API Token 認證

#### Tempo API Token（選填）

如果你的組織使用 Tempo Timesheets：

1. 前往 Tempo → Settings → API Integration
2. 建立新的 Token
3. 在 `tempo setup` 時輸入

## Outlook 整合設定

### 1. 註冊 Azure AD 應用程式

1. 前往 [Microsoft Entra 管理中心](https://entra.microsoft.com)
2. 應用程式 → 應用程式註冊 → 新增註冊
3. 名稱：`Tempo Sync`（或自訂名稱）
4. 支援的帳戶類型：選擇「僅此組織目錄中的帳戶」
5. 重新導向 URI：留空（使用裝置碼流程）

### 2. 設定 API 權限

在應用程式頁面：
1. API 權限 → 新增權限
2. Microsoft Graph → 委派的權限
3. 勾選 `Calendars.Read`
4. 點擊「代表 [組織] 授與管理員同意」（需要管理員）

### 3. 啟用公用用戶端流程

1. 驗證 → 進階設定
2. 「允許公用用戶端流程」設為「是」

### 4. 取得識別碼

記下以下資訊：
- **應用程式 (用戶端) 識別碼**
- **目錄 (租用戶) 識別碼**

### 5. 執行登入

```bash
tempo outlook-login
```

首次執行會要求輸入 Client ID 和 Tenant ID。

## LLM 提供者設定

### Ollama（本地）

```bash
# 安裝 Ollama
brew install ollama  # macOS
# 或參考 https://ollama.ai

# 下載模型
ollama pull llama3.2

# 設定 tempo-sync
tempo setup-llm
# 選擇 ollama，模型填入 llama3.2
```

### OpenAI

```bash
tempo setup-llm
# 選擇 openai
# 輸入 API Key
# 模型建議：gpt-4o-mini（性價比最高）
```

### OpenAI Compatible（自訂端點）

適用於：
- Azure OpenAI
- 本地部署的 LLM API
- 其他相容 OpenAI API 的服務

```bash
tempo setup-llm
# 選擇 openai-compatible
# 輸入 Base URL（例如 https://your-endpoint.com）
# 輸入 API Key
# 輸入模型名稱
```

## 常見問題

### Q: 為什麼找不到我的 Claude Code session？

確認 Claude Code 的資料目錄存在：
```bash
ls ~/.claude/projects/
```

### Q: LLM 彙整功能可以關閉嗎？

目前 LLM 彙整是自動執行的。如果未配置 LLM，會顯示原始的工作描述。

### Q: 如何修改已儲存的專案對應？

編輯 `~/.tempo-sync/project_mapping.json`，或在上傳時重新輸入 Issue ID。

### Q: Outlook 登入顯示「需要管理員核准」？

這是 Azure AD 的權限控制。請聯繫 IT 管理員：
1. 前往 Microsoft Entra → 企業應用程式
2. 找到你註冊的應用程式
3. 授與管理員同意

## 開發

```bash
# Clone 專案
git clone https://gitting.eland.com.tw/rd2/tempo-sync.git
cd tempo-sync

# 建立虛擬環境
uv venv
source .venv/bin/activate

# 安裝開發依賴
uv pip install -e ".[dev,all-llm,outlook]"

# 執行測試
pytest

# 程式碼檢查
ruff check .
```

## License

MIT
