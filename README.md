# Recap

> **自動回顧你的工作，讓你專注於創造價值**

智慧、自動化取得實際工作的紀錄（Git、Claude Code、GitLab 等），自動彙整工作紀錄供日常報告、績效評核，減輕人工文書作業負擔，專注於工作本身。

## 功能特點

- **多來源自動收集** - 從 Git commits、Claude Code sessions、GitLab 自動追蹤工作
- **LLM 智能彙整** - 自動將多個工作項目彙整成簡潔的描述
- **工時正規化** - 自動將每日工時調整為標準工時
- **Jira Tempo 整合** - 一鍵上傳工時到 Jira Tempo
- **多種檢視模式** - 時間軸、專案分組、列表檢視
- **Excel 報表匯出** - 匯出工作報表

## 安裝

### Desktop App（推薦）

從 [GitHub Releases](https://github.com/p988744/recap/releases) 下載對應平台的安裝檔：

| 平台 | 檔案 |
|------|------|
| macOS (Apple Silicon) | `Recap_x.x.x_aarch64.dmg` |
| Windows | `Recap_x.x.x_x64-setup.exe` 或 `.msi` |
| Linux | `recap_x.x.x_amd64.deb` 或 `.AppImage` |

#### macOS 安裝說明

App 已經過 Apple 簽章和公證，下載後可直接開啟使用。

#### Windows 安裝說明

首次執行可能會出現 Windows SmartScreen 警告：

1. 點擊「**更多資訊**」
2. 點擊「**仍要執行**」

#### Linux 安裝說明

**Debian/Ubuntu (.deb)**
```bash
sudo dpkg -i recap_*_amd64.deb
```

**AppImage**
```bash
chmod +x Recap_*.AppImage
./Recap_*.AppImage
```

### 從原始碼建置

```bash
# Clone
git clone https://github.com/p988744/recap.git
cd recap

# 安裝前端依賴
cd web && npm install && cd ..

# 建置 Desktop App
cd web && ~/.cargo/bin/cargo tauri build
```

## 使用方式

### 1. 首次設定

啟動 App 後，進入 **Settings** 頁面配置：

- **Claude Code** - 選擇要追蹤的 Claude Code 專案
- **GitLab** - 設定 GitLab URL 和 Access Token
- **Tempo** - 設定 Jira URL、PAT 和 Tempo Token

### 2. 同步工作紀錄

在 **Dashboard** 頁面：
- 點擊「Sync All」自動同步所有來源
- 或個別同步 Claude Code / GitLab

### 3. 管理工作項目

在 **Work Items** 頁面：
- **Timeline** - 時間軸檢視，顯示每日工作時段
- **Grouped** - 按專案和 Jira Issue 分組
- **List** - 傳統列表檢視

### 4. 匯出報表

在 **Reports** 頁面：
- 選擇日期範圍
- 點擊「Export Excel」下載報表

### 5. 同步到 Tempo

在 Work Items 頁面：
- 選擇要同步的項目
- 確認 Jira Issue Key 已設定
- 點擊「Sync to Tempo」

---

## 技術架構

```
recap/
└── web/                    # Desktop App
    ├── src/               # React 前端 (TypeScript)
    │   ├── components/    # UI 元件
    │   ├── pages/        # 頁面
    │   └── lib/          # Tauri API 客戶端
    └── src-tauri/        # Rust 後端
        ├── src/
        │   ├── commands/  # Tauri IPC Commands
        │   ├── services/ # 業務邏輯
        │   ├── models/   # 資料模型
        │   └── db/       # SQLite 資料庫
        └── Cargo.toml
```

### 技術棧

- **Frontend**: React + TypeScript + Tailwind CSS + shadcn/ui
- **Backend**: Rust + Tauri IPC + SQLite
- **Desktop**: Tauri v2
- **Build**: Vite + Cargo

### 架構說明

應用程式使用 Tauri v2 的 IPC (Inter-Process Communication) 機制，前端直接透過 `invoke()` 呼叫 Rust 後端的 Commands，無需 HTTP 伺服器。

```
Frontend (React)
     │
     └── invoke() ──► Tauri Commands (#[tauri::command])
                           │
                           ▼
                      SQLite Database
```

### Tauri Commands

| 模組 | Commands |
|------|----------|
| **Auth** | `get_app_status`, `register_user`, `login`, `auto_login`, `get_current_user` |
| **Config** | `get_config`, `update_config`, `update_llm_config`, `update_jira_config` |
| **Work Items** | `list_work_items`, `create_work_item`, `get_work_item`, `update_work_item`, `delete_work_item`, `get_stats_summary`, `get_grouped_work_items`, `get_timeline_data`, `batch_sync_tempo`, `aggregate_work_items` |
| **Claude** | `list_claude_sessions`, `import_claude_sessions`, `summarize_claude_session`, `sync_claude_projects` |
| **Reports** | `get_personal_report`, `get_summary_report`, `get_category_report`, `get_source_report`, `export_excel_report` |
| **Sync** | `get_sync_status`, `auto_sync`, `list_available_projects` |
| **GitLab** | `get_gitlab_status`, `configure_gitlab`, `remove_gitlab_config`, `list_gitlab_projects`, `add_gitlab_project`, `remove_gitlab_project`, `sync_gitlab`, `search_gitlab_projects` |
| **Tempo** | `test_tempo_connection`, `validate_jira_issue`, `sync_worklogs_to_tempo`, `upload_single_worklog`, `get_tempo_worklogs` |
| **Users** | `get_profile`, `update_profile` |

---

## 開發

### 環境需求

- Node.js 18+
- Rust 1.70+
- Cargo + Tauri CLI

### 開發模式

```bash
cd web

# 安裝依賴
npm install

# 啟動開發伺服器
~/.cargo/bin/cargo tauri dev
```

### 建置

```bash
cd web
~/.cargo/bin/cargo tauri build
```

產出檔案位於 `web/src-tauri/target/release/bundle/`

### 專案結構

```
web/
├── src/                      # 前端原始碼
│   ├── components/          # React UI 元件
│   ├── pages/              # 頁面元件
│   └── lib/
│       ├── api.ts          # API 介面（自動偵測 Tauri 環境）
│       └── tauri-api.ts    # Tauri Commands 封裝
├── src-tauri/               # Rust 後端
│   ├── src/
│   │   ├── lib.rs          # 應用程式進入點
│   │   ├── commands/       # Tauri Commands
│   │   │   ├── mod.rs      # AppState 定義
│   │   │   ├── auth.rs     # 認證
│   │   │   ├── config.rs   # 設定
│   │   │   ├── work_items.rs
│   │   │   ├── claude.rs
│   │   │   ├── reports.rs
│   │   │   ├── sync.rs
│   │   │   ├── gitlab.rs
│   │   │   ├── tempo.rs
│   │   │   └── users.rs
│   │   ├── services/       # 業務邏輯
│   │   ├── models/         # 資料模型
│   │   ├── db/            # 資料庫
│   │   └── auth/          # JWT 認證
│   └── Cargo.toml
├── package.json
└── vite.config.ts
```

---

## License

MIT
