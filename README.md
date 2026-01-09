# Recap

> **自動回顧你的工作，讓你專注於創造價值**

智慧、自動化取得實際工作的紀錄（Git、Claude Code、Antigravity 等），自動彙整工作紀錄供日常報告、績效評核，減輕人工文書作業負擔，專注於工作本身。

## 功能特點

- **多來源自動收集** - 從 Git commits、Claude Code sessions 自動追蹤工作
- **LLM 智能彙整** - 自動將多個工作項目彙整成簡潔的描述
- **工時正規化** - 自動將每日工時調整為標準工時
- **Jira Tempo 整合** - 一鍵上傳工時到 Jira Tempo
- **團隊報表** - 主管可從 Tempo 團隊取得成員工時，匯出 Excel 報表
- **績效考核** - AI 輔助生成績效考核草稿

## 快速開始

### 1. 安裝

```bash
# 使用 uv（推薦）
uv tool install recap

# 或使用 pip
pip install recap

# 如需 LLM 支援
pip install recap[openai]      # OpenAI
pip install recap[anthropic]   # Anthropic Claude
pip install recap[all-llm]     # 所有 LLM 提供者

# 如需績效考核功能
pip install recap[pe]

# 如需匯出 Excel
pip install recap[excel]
```

### 2. 配置

```bash
recap config jira   # 設定 Jira 連接
recap config llm    # 設定 LLM
```

### 3. 開始使用

```bash
recap              # 互動模式
recap week         # 分析本週工作
recap sync         # 同步到 Tempo
```

---

## 命令總覽

| 命令 | 說明 |
|------|------|
| `recap` | 互動模式（推薦新手使用）|
| `recap week` | 分析本週工作 |
| `recap sync` | 同步到 Jira Tempo |
| `recap team` | 團隊報表 |
| `recap pe` | 績效考核 Web UI |
| `recap config` | 配置管理 |

### 資料來源管理

```bash
# Git 模式
recap sources add git ~/projects/my-app
recap sources list
recap sources remove my-app

# 使用 Git 模式分析
recap week --git
```

### 團隊報表（主管功能）

```bash
# 從 Tempo 團隊新增
recap team add --from-tempo

# 產生報表
recap team report RD2 --week
recap team report RD2 --month -o report.xlsx
```

### 績效考核

```bash
# 啟動 Web UI
recap pe

# 指定埠號
recap pe --port 8080
```

---

## 配置檔案

所有配置儲存在 `~/.recap/`：

```
~/.recap/
├── config.json              # 主要配置
├── project_mapping.json     # 專案與 Issue 的對應
├── teams.json               # 團隊配置
└── pe/                      # 績效考核資料
    └── pe_helper.db
```

---

## 開發

```bash
# Clone
git clone https://github.com/anthropics/recap.git
cd recap

# 建立環境
uv venv && source .venv/bin/activate

# 安裝開發依賴
uv pip install -e ".[dev,all-llm,pe,excel]"

# 執行測試
pytest

# 程式碼檢查
ruff check .
```

---

## License

MIT
