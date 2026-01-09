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

| 平台 | 架構 | 檔案 |
|------|------|------|
| macOS | Apple Silicon | `Recap_x.x.x_aarch64.dmg` |
| macOS | Intel | `Recap_x.x.x_x64.dmg` |
| Windows | x64 | `Recap_x.x.x_x64-setup.exe` 或 `.msi` |
| Linux | x64 | `recap_x.x.x_amd64.deb` 或 `.AppImage` |

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
├── web/                    # Desktop App
│   ├── src/               # React 前端 (TypeScript)
│   │   ├── components/    # UI 元件
│   │   ├── pages/        # 頁面
│   │   └── lib/          # API 客戶端
│   └── src-tauri/        # Rust 後端
│       ├── src/
│       │   ├── api/      # REST API 路由
│       │   ├── services/ # 業務邏輯
│       │   ├── models/   # 資料模型
│       │   └── db/       # SQLite 資料庫
│       └── Cargo.toml
└── src/recap/             # Python CLI (legacy)
```

### 技術棧

- **Frontend**: React + TypeScript + Tailwind CSS + shadcn/ui
- **Backend**: Rust + Axum + SQLite
- **Desktop**: Tauri v2
- **Build**: Vite + Cargo

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

---

## API 端點

Desktop App 在本地啟動 HTTP 伺服器（預設 port 8000）：

| 端點 | 說明 |
|------|------|
| `POST /api/auth/login` | 登入 |
| `GET /api/work-items` | 取得工作項目 |
| `GET /api/work-items/timeline` | 時間軸檢視 |
| `GET /api/work-items/grouped` | 分組檢視 |
| `POST /api/claude/sync` | 同步 Claude Code |
| `POST /api/gitlab/sync` | 同步 GitLab |
| `POST /api/tempo/sync` | 同步到 Tempo |
| `GET /api/reports/export/excel` | 匯出 Excel |

---

## License

MIT
