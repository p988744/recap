# Recap

> **自動回顧你的工作，讓你專注於創造價值**

智慧、自動化取得實際工作的紀錄（Git、Claude Code、GitLab 等），自動彙整工作紀錄供日常報告、績效評核，減輕人工文書作業負擔，專注於工作本身。

## 功能特點

- **多來源自動收集** - 從 Git commits、Claude Code sessions、GitLab 自動追蹤工作
- **Worklog 每日/每週總覽** - 按日期分組檢視工作紀錄，支援每小時明細展開
- **LLM 智能彙整** - 自動將工作描述摘要為簡潔的 Tempo worklog 描述
- **工時正規化** - 自動將每日工時調整為標準工時
- **Jira Tempo 整合** - 單筆、單日、整週匯出工時到 Jira Tempo，含預覽與 Issue 驗證
- **多種檢視模式** - 時間軸、專案分組、列表檢視
- **Excel 報表匯出** - 匯出工作報表
- **跨平台支援** - macOS (Apple Silicon / Intel)、Windows、Linux

## 安裝

### Desktop App（推薦）

從 [Releases](https://github.com/p988744/recap/releases) 下載對應平台的安裝檔：

| 平台 | 檔案 |
|------|------|
| macOS (Apple Silicon) | `Recap_x.x.x_aarch64.dmg` |
| macOS (Intel) | `Recap_x.x.x_x64.dmg` |
| Windows | `Recap_x.x.x_x64-setup.exe` 或 `.msi` |
| Linux | `recap_x.x.x_amd64.deb` 或 `.AppImage` |

#### macOS 安裝說明

下載 DMG 後，將 Recap 拖入 Applications 資料夾。首次開啟若出現安全警告，請右鍵點擊 App → 選擇「打開」。

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
cd recap/web

# 安裝前端依賴
npm install

# 開發模式
cargo tauri dev

# 建置 Release（產出 DMG / EXE / AppImage）
cargo tauri build
```

產出檔案位於 `web/target/release/bundle/`

## 使用方式

### 1. 首次設定

啟動 App 後，進入 **Settings** 頁面配置：

- **Git Repos** - 選擇要追蹤的本地 Git 倉庫
- **Claude Code** - 選擇要追蹤的 Claude Code 專案
- **GitLab** - 設定 GitLab URL 和 Access Token
- **Jira / Tempo** - 設定 Jira URL、PAT 和 Tempo Token
- **AI (LLM)** - 設定 LLM Provider（用於工作描述摘要）

### 2. 同步工作紀錄

在 **Dashboard** 頁面：
- 點擊「Sync All」自動同步所有來源
- 或個別同步 Claude Code / GitLab

### 3. Worklog（每日工時）

在 **Worklog** 頁面：
- 按日期檢視每日工作紀錄（Git commits + Claude Code sessions）
- 展開查看每小時明細
- 手動新增工作項目
- 單筆匯出、單日批次匯出、整週匯出到 Tempo

### 4. 匯出到 Tempo

在 Worklog 頁面：
- **單筆匯出** - 點擊專案卡片上的「Export」按鈕
- **單日批次匯出** - 點擊日期標題旁的「Export Day」按鈕
- **整週匯出** - 點擊頁面頂部的「Export Week」按鈕
- 每次匯出前 LLM 會自動摘要工作描述
- 支援預覽（Preview）確認後再正式匯出

### 5. 管理工作項目

在 **Work Items** 頁面：
- **Timeline** - 時間軸檢視，顯示每日工作時段
- **Grouped** - 按專案和 Jira Issue 分組
- **List** - 傳統列表檢視

### 6. 匯出報表

在 **Reports** 頁面：
- 選擇日期範圍
- 點擊「Export Excel」下載報表

---

## 技術架構

```
recap/
├── web/                        # Desktop App
│   ├── src/                   # React 前端 (TypeScript)
│   │   ├── components/ui/    # shadcn/ui 基礎元件
│   │   ├── pages/            # 頁面（每頁拆為 components/ + hooks/）
│   │   │   ├── Dashboard/
│   │   │   ├── Worklog/      # 工時總覽（含 Tempo 匯出）
│   │   │   ├── WorkItems/
│   │   │   ├── Reports/
│   │   │   └── Settings/
│   │   ├── services/          # Tauri API 封裝（按模組拆分）
│   │   └── types/             # 共用型別定義
│   ├── src-tauri/             # Rust 後端
│   │   └── src/
│   │       ├── commands/      # Tauri IPC Commands（按模組拆分）
│   │       ├── services/      # 業務邏輯
│   │       ├── models/        # 資料模型
│   │       └── db/            # SQLite 資料庫
│   └── crates/
│       ├── recap-core/        # 共用核心邏輯
│       └── recap-cli/         # CLI 工具
```

### 技術棧

- **Frontend**: React 18 + TypeScript + Tailwind CSS + shadcn/ui
- **Backend**: Rust + Tauri v2 IPC + SQLite (sqlx)
- **Desktop**: Tauri v2（跨平台：macOS / Windows / Linux）
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

---

## 開發

### 環境需求

- Node.js 20+
- Rust 1.77+
- Tauri CLI (`cargo install tauri-cli`)

### 開發模式

```bash
cd web
npm install
cargo tauri dev
```

### 建置

```bash
cd web
cargo tauri build
```

### 測試

```bash
cd web

# 前端測試
npm test

# Rust 測試
cargo test --workspace

# TypeScript 型別檢查
npx tsc --noEmit
```

---

## License

MIT
