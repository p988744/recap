# Tempo Sync

自動同步開發活動到 Jira Tempo Worklog 的 CLI 工具。

支援從 **Claude Code sessions** 或 **Git commits** 收集工作紀錄，透過 LLM 自動彙整描述，一鍵上傳到 Jira Tempo。

## 功能特點

- **雙來源支援** - Claude Code sessions 或 Git commits（無需 Claude Code 也能使用）
- **LLM 智能彙整** - 自動將多個工作項目彙整成簡潔的 worklog 描述
- **工時正規化** - 自動將每日工時調整為 8 小時，以 30 分鐘為單位
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

# 如需 LLM 支援
pip install tempo-sync[openai]      # OpenAI
pip install tempo-sync[anthropic]   # Anthropic Claude
pip install tempo-sync[all-llm]     # 所有 LLM 提供者
```

### 2. 配置 Jira

```bash
tempo setup
```

### 3. 開始使用

**使用 Claude Code sessions：**
```bash
tempo              # 互動模式
tempo analyze -w   # 分析本週
```

**使用 Git commits（無需 Claude Code）：**
```bash
# 添加要追蹤的 Git 倉庫
tempo git-add ~/projects/app1 ~/projects/app2

# 啟用 Git 模式
tempo setup-git --enable

# 開始分析
tempo analyze -w
```

---

## 使用手冊

### 命令總覽

| 命令 | 說明 |
|------|------|
| `tempo` | 互動模式（推薦新手使用） |
| `tempo analyze` | 分析工作紀錄並生成報告 |
| `tempo dates` | 列出有工作紀錄的日期 |
| `tempo setup` | 配置 Jira 連接 |
| `tempo setup-llm` | 配置 LLM 提供者 |
| `tempo setup-git` | 配置 Git 模式 |
| `tempo git-add` | 添加 Git 倉庫 |
| `tempo git-remove` | 移除 Git 倉庫 |
| `tempo git-list` | 列出已設定的 Git 倉庫 |
| `tempo outlook-login` | 登入 Outlook（實驗性） |
| `tempo outlook-logout` | 登出 Outlook |

---

### 互動模式

```bash
tempo
```

最簡單的使用方式。啟動後會：

1. 顯示目前配置狀態（Jira、LLM、來源模式）
2. 讓你選擇時間範圍
3. 顯示工作紀錄流水帳
4. 引導你對應 Jira Issue
5. 確認後上傳

**互動式對應技巧：**
- 直接按 Enter → 使用上次的 Issue ID
- 輸入新 ID → 更新對應並記住
- 輸入 `-` → 跳過此項目
- 輸入 `q` → 取消整個流程

---

### 分析命令 (tempo analyze)

```bash
# 基本用法
tempo analyze [選項]

# 時間範圍選項（擇一）
tempo analyze --week          # 本週（週一到週日）
tempo analyze --last-week     # 上週
tempo analyze --days 7        # 過去 7 天
tempo analyze --date 2025-01-15              # 指定單日
tempo analyze --from 2025-01-01 --to 2025-01-07  # 自訂範圍

# 其他選項
tempo analyze -w --upload     # 分析後直接進入上傳流程
tempo analyze -w --git        # 強制使用 Git 模式
tempo analyze -w --no-git     # 強制使用 Claude Code 模式
tempo analyze --git --repo ~/project  # 指定特定倉庫（不使用已存設定）
```

**選項說明：**

| 選項 | 簡寫 | 說明 |
|------|------|------|
| `--week` | `-w` | 分析本週 |
| `--last-week` | `-l` | 分析上週 |
| `--days N` | `-n N` | 分析過去 N 天 |
| `--date DATE` | `-d DATE` | 指定日期 (YYYY-MM-DD) |
| `--from DATE` | | 開始日期 |
| `--to DATE` | | 結束日期 |
| `--upload` | `-u` | 分析後直接進入上傳流程 |
| `--git` | `-g` | 使用 Git 模式 |
| `--no-git` | | 使用 Claude Code 模式 |
| `--repo PATH` | `-r PATH` | 指定 Git 倉庫（可多次使用） |

---

### Git 模式

Git 模式讓沒有 Claude Code 的用戶也能使用此工具，從 Git commit 歷史估算工作時間。

**設定流程：**

```bash
# 1. 添加要追蹤的倉庫（可一次多個）
tempo git-add ~/projects/frontend ~/projects/backend ~/projects/api

# 2. 查看已設定的倉庫
tempo git-list

# 3. 啟用 Git 模式為預設
tempo setup-git --enable

# 4. 之後直接使用（不需要額外參數）
tempo analyze --week
tempo dates
```

**管理倉庫：**

```bash
# 添加倉庫
tempo git-add <路徑...>

# 移除倉庫（可用名稱或完整路徑）
tempo git-remove frontend
tempo git-remove ~/projects/frontend

# 移除所有倉庫
tempo git-remove --all

# 列出倉庫狀態
tempo git-list
```

**切換模式：**

```bash
# 啟用 Git 模式為預設
tempo setup-git --enable

# 停用 Git 模式（改用 Claude Code）
tempo setup-git --disable

# 臨時切換（不改變預設）
tempo analyze -w --git      # 強制 Git 模式
tempo analyze -w --no-git   # 強制 Claude Code 模式
```

**工時計算邏輯：**
- 單一 commit → 30 分鐘
- 多個 commit → 時間跨度 + 30 分鐘緩衝
- commit message 作為工作描述

---

### 工時正規化

系統會自動將每日工時正規化為 8 小時，以 30 分鐘為單位四捨五入。

**範例：**
```
原始時間：專案A 2h, 專案B 3h, 專案C 1h（共 6h）
正規化後：專案A 2.5h, 專案B 4h, 專案C 1.5h（共 8h）
```

上傳預覽會顯示「原始時間」和「上傳時間」兩欄，讓你確認正規化結果。

**配置選項（在 config.json）：**
```json
{
  "daily_work_hours": 8.0,    // 每日標準工時
  "normalize_hours": true      // 是否啟用正規化
}
```

---

### 配置 Jira (tempo setup)

```bash
tempo setup
```

**認證方式：**

| 類型 | 適用於 | 需要資訊 |
|------|--------|----------|
| PAT | Jira Server | Personal Access Token |
| Basic | Jira Cloud | Email + API Token |

**取得 Token：**

- **Jira Server PAT**: Jira → 個人設定 → Personal Access Tokens
- **Jira Cloud API Token**: https://id.atlassian.com/manage-profile/security/api-tokens
- **Tempo API Token**（選填）: Tempo → Settings → API Integration

---

### 配置 LLM (tempo setup-llm)

LLM 用於自動彙整工作描述。未配置時會顯示原始描述。

```bash
tempo setup-llm
```

**支援的提供者：**

| 提供者 | 說明 | 需要 API Key |
|--------|------|--------------|
| Ollama | 本地運行 | 否 |
| OpenAI | GPT-4、GPT-3.5 | 是 |
| Anthropic | Claude | 是 |
| Gemini | Google Gemini | 是 |
| OpenAI Compatible | 自訂端點 | 視情況 |

**Ollama 設定（免費本地方案）：**
```bash
# 安裝 Ollama
brew install ollama  # macOS

# 下載模型
ollama pull llama3.2

# 設定 tempo-sync
tempo setup-llm
# 選擇 ollama，模型填入 llama3.2
```

---

## 配置檔案

所有配置儲存在 `~/.tempo-sync/`：

```
~/.tempo-sync/
├── config.json              # 主要配置
├── project_mapping.json     # 專案與 Issue 的對應
└── outlook_token_cache.json # Outlook 認證（如有）
```

### config.json 完整結構

```json
{
  "jira_url": "https://jira.example.com",
  "jira_pat": "",
  "jira_email": "",
  "jira_api_token": "",
  "auth_type": "pat",
  "tempo_api_token": "",

  "llm_provider": "ollama",
  "llm_model": "",
  "llm_api_key": "",
  "llm_base_url": "",

  "daily_work_hours": 8.0,
  "normalize_hours": true,

  "use_git_mode": false,
  "git_repos": [],

  "outlook_enabled": false,
  "outlook_client_id": "",
  "outlook_tenant_id": ""
}
```

---

## 常見問題

### Q: 找不到 Claude Code session？

確認資料目錄存在：
```bash
ls ~/.claude/projects/
```

如果沒有 Claude Code，請使用 Git 模式：
```bash
tempo git-add ~/your-project
tempo setup-git --enable
```

### Q: 如何修改專案與 Issue 的對應？

方法一：下次上傳時輸入新的 Issue ID（會自動更新）

方法二：直接編輯配置檔
```bash
nano ~/.tempo-sync/project_mapping.json
```

### Q: 工時正規化後數字不對？

正規化會將每日總工時調整為 8 小時，並以 30 分鐘為單位。如果你不需要此功能：

```bash
# 編輯 config.json
nano ~/.tempo-sync/config.json
# 將 "normalize_hours" 改為 false
```

### Q: 可以先預覽不上傳嗎？

可以。在上傳確認時選擇「否」即可。系統會顯示完整的預覽表格，包含：
- Jira Issue
- 日期
- 原始時數 / 上傳時數
- 工作描述

### Q: Outlook 顯示「需要管理員核准」？

Azure AD 權限需要 IT 管理員授權。請聯繫管理員：
1. 前往 Microsoft Entra → 企業應用程式
2. 找到你的應用程式
3. 授與管理員同意

---

## Outlook 整合（實驗性）

> ⚠️ 需要 Azure AD 管理員授權

### 設定步驟

1. **註冊 Azure AD 應用程式**
   - 前往 [Microsoft Entra 管理中心](https://entra.microsoft.com)
   - 應用程式 → 應用程式註冊 → 新增註冊
   - 支援的帳戶類型：「僅此組織目錄中的帳戶」

2. **設定 API 權限**
   - API 權限 → 新增權限 → Microsoft Graph → 委派的權限
   - 勾選 `Calendars.Read`
   - 點擊「代表組織授與管理員同意」

3. **啟用公用用戶端流程**
   - 驗證 → 進階設定 → 「允許公用用戶端流程」設為「是」

4. **執行登入**
   ```bash
   tempo outlook-login
   ```

---

## 開發

```bash
# Clone
git clone https://gitting.eland.com.tw/rd2/tempo-sync.git
cd tempo-sync

# 建立環境
uv venv && source .venv/bin/activate

# 安裝開發依賴
uv pip install -e ".[dev,all-llm,outlook]"

# 執行測試
pytest

# 程式碼檢查
ruff check .
```

---

## 版本歷史

詳見 [CHANGELOG.md](CHANGELOG.md)

## License

MIT
