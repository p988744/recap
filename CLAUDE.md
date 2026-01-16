# CLAUDE.md

This file provides guidance for Claude when working on the Recap codebase.

## Project Overview

Recap is a work tracking and reporting desktop application built with Tauri v2. It automatically collects work records from various sources (Git, Claude Code, GitLab) and helps users manage their work items for daily reporting and performance reviews.

## Architecture

### Frontend-Backend Communication

The application uses **Tauri IPC (Inter-Process Communication)** exclusively. There is no HTTP server.

```
Frontend (React/TypeScript)
     │
     └── invoke('command_name', { params }) ──► Tauri Commands (Rust)
                                                    │
                                                    ▼
                                               SQLite Database
```

### Key Directories

```
web/
├── src/                      # Frontend (React + TypeScript)
│   ├── components/          # UI components (shadcn/ui)
│   ├── pages/              # Page components
│   └── lib/
│       ├── api.ts          # API interface (detects Tauri environment)
│       └── tauri-api.ts    # Tauri Commands wrapper
└── src-tauri/               # Backend (Rust)
    └── src/
        ├── lib.rs          # App entry, command registration
        ├── commands/       # Tauri Commands (IPC handlers)
        ├── services/       # Business logic
        ├── models/         # Data models
        ├── db/            # SQLite database
        └── auth/          # JWT authentication
```

## Development Guidelines

### Adding New Features

1. **Backend (Rust)**:
   - Create a new Tauri command in `src-tauri/src/commands/`
   - Use `#[tauri::command]` attribute
   - Register command in `lib.rs` `invoke_handler`
   - Token-based auth: pass `token: String` as first parameter, verify with `verify_token(&token)`

2. **Frontend (TypeScript)**:
   - Add type definitions and invoke function in `src/lib/tauri-api.ts`
   - Update `src/lib/api.ts` to use new Tauri command

### Tauri Command Pattern

```rust
// src-tauri/src/commands/example.rs
use tauri::State;
use crate::auth::verify_token;
use super::AppState;

#[tauri::command]
pub async fn example_command(
    state: State<'_, AppState>,
    token: String,
    param: SomeType,
) -> Result<ReturnType, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;
    // ... implementation
    Ok(result)
}
```

### Frontend API Pattern

```typescript
// src/lib/tauri-api.ts
export async function exampleCommand(token: string, param: SomeType): Promise<ReturnType> {
  return invoke<ReturnType>('example_command', { token, param })
}

// src/lib/api.ts
exampleCommand: async (param: SomeType) => {
  if (isTauri) {
    return tauriApi.exampleCommand(getRequiredToken(), param)
  }
  // Fallback for non-Tauri environment (if needed)
  throw new Error('This feature requires the desktop app')
}
```

### Code Style

- **Rust**: Follow Rust conventions, use `rustfmt`
- **TypeScript**: Follow existing patterns in the codebase
- **React**: Use functional components with hooks
- **CSS**: Use Tailwind CSS classes

### Database

- SQLite database stored in user's app data directory
- Use `sqlx` for async database operations
- Use parameterized queries to prevent SQL injection

### Authentication

- JWT tokens for user authentication
- Token passed as parameter to all authenticated commands
- Token stored in frontend localStorage

### Error Handling

- Rust: Return `Result<T, String>` from commands
- TypeScript: Handle errors with try/catch around `invoke()` calls

## Testing

```bash
# Frontend build check
cd web && npm run build

# Rust compilation check
cd web/src-tauri && cargo check

# Run Tauri development mode
cd web && cargo tauri dev
```

## Common Tasks

### Adding a new API endpoint

1. Create command in `src-tauri/src/commands/<module>.rs`
2. Register in `src-tauri/src/lib.rs`
3. Add TypeScript types and function in `src/lib/tauri-api.ts`
4. Update `src/lib/api.ts` to use the new command

### Modifying database schema

1. Update models in `src-tauri/src/models/`
2. Update database initialization in `src-tauri/src/db/`
3. Run migrations if needed

### Adding a new page

1. Create page component in `src/pages/`
2. Add route in `src/App.tsx`
3. Update navigation if needed

## Important Notes

- **No HTTP Server**: All communication uses Tauri IPC
- **Token Authentication**: Always pass token to authenticated commands
- **AppState**: Use `State<'_, AppState>` to access shared database connection
- **Async/Await**: All database operations are async
- **Error Messages**: Return user-friendly error messages from commands

## Team Collaboration & Git Worktree Strategy

### Team Roles

| 角色 | 負責範圍 | Worktree 分支 |
|------|----------|---------------|
| **Core 開發者** | `crates/recap-core/` | `refactor/core-v2` |
| **Desktop 開發者** | `src-tauri/` + `src/` | `refactor/desktop-v2` |
| **CLI 開發者** | `crates/recap-cli/` | `refactor/cli-v2` |

> QA 由三人輪流兼任，每個 PR 需要另一位成員 review。

### Git Worktree Setup

**目錄結構：**
```
~/Projects/
├── recap/                    # 主專案 (main branch)
├── recap-worktrees/          # Worktree 專用目錄
│   ├── core-dev/             # Core 開發者
│   ├── desktop-dev/          # Desktop 開發者
│   └── cli-dev/              # CLI 開發者
```

**初始化指令：**
```bash
# 建立 worktree 目錄
mkdir -p ../recap-worktrees

# 建立各角色的 worktree
git worktree add ../recap-worktrees/core-dev -b refactor/core-v2
git worktree add ../recap-worktrees/desktop-dev -b refactor/desktop-v2
git worktree add ../recap-worktrees/cli-dev -b refactor/cli-v2

# 進入 worktree 後執行
cd ../recap-worktrees/desktop-dev
npm install        # 安裝前端依賴
cargo build        # 編譯 Rust
```

**每個 Worktree 的 Claude Code 初始化：**
```bash
# 進入 worktree 後，執行 /init 讓 Claude Code 識別專案
claude
> /init
```

### Branch Strategy

```
main (穩定版，保護分支)
│
└── develop (整合分支) ✅ 已建立
    │
    ├── refactor/core-v2      ← Core 開發者
    │   └── 完成後先合併到 develop
    │
    ├── refactor/desktop-v2   ← Desktop 開發者
    │   └── 需先 rebase develop 取得 core 更新
    │
    └── refactor/cli-v2       ← CLI 開發者
        └── 需先 rebase develop 取得 core 更新
```

**合併順序：**
1. Core → develop（其他分支依賴 core）
2. CLI / Desktop 各自 rebase develop
3. CLI / Desktop → develop
4. develop 穩定測試後 → main

### Branch Progress

| 分支 | 狀態 | Commits | 說明 |
|------|------|---------|------|
| `main` | 🟢 穩定 | - | 保護分支 |
| `develop` | 🟢 已建立 | 0 | 整合分支，等待 Core 合併 |
| `refactor/core-v2` | 🟡 進行中 | 1 | 完成 recap-core 單元測試 |
| `refactor/desktop-v2` | 🟢 活躍 | 3 | CI + Settings/work_items 測試 |
| `refactor/cli-v2` | ⚪ 尚未開始 | 0 | 等待 Core 完成 |

**各分支已完成工作：**

- **Core (`refactor/core-v2`)**
  - `8ac7f6d` test: Add comprehensive unit tests for recap-core modules

- **Desktop (`refactor/desktop-v2`)**
  - `4208f00` ci: Add GitHub Actions CI workflow
  - `e7a6083` test: Add unit tests for Settings-related services
  - `f559bb7` test: Add comprehensive unit tests for work_items.rs

- **CLI (`refactor/cli-v2`)**
  - 尚無進度

### Worktree Best Practices

參考 [Claude Code Worktree 最佳實踐](https://incident.io/blog/shipping-faster-with-claude-code-and-git-worktrees)：

1. **獨立環境** - 每個 worktree 有獨立的 `node_modules` 和 `target/`
2. **定期提交** - 小步提交，方便追蹤和 revert
3. **同步 develop** - 每天開始前 `git fetch && git rebase origin/develop`
4. **避免同分支** - 不要在多個 worktree checkout 同一分支
5. **資源管理** - 完成後用 `git worktree remove` 清理

### Collaboration Rules

1. **修改 `recap-core` 時**
   - 必須通知其他開發者
   - 更新 CHANGELOG.md
   - 確保向下相容或協調升級

2. **跨模組依賴**
   - Desktop/CLI 只「使用」core，不直接修改
   - 需要 core 新功能時，開 issue 給 Core 開發者

3. **PR Review**
   - 每個 PR 需要另一位成員 review
   - Core 的 PR 需要 Desktop 和 CLI 開發者都確認

---

## Code Organization Principles

### Refactoring Prerequisites

**重構前必須確保：**

1. **測試覆蓋率**
   - 被重構模組必須有對應的測試案例
   - 測試須涵蓋所有公開 API 的主要路徑
   - 重構前後測試必須全部通過

2. **重構流程**
   ```
   確認現有測試 → 補齊缺失測試 → 執行重構 → 驗證測試通過
   ```

3. **測試類型**
   | 層級 | Rust | TypeScript |
   |------|------|------------|
   | 單元測試 | `#[cfg(test)]` 模組內 | Vitest |
   | 整合測試 | `tests/` 目錄 | Playwright |

### File Size Guidelines

| 類型 | 建議上限 |
|------|---------|
| Rust 模組 (.rs) | 300 行 |
| React 元件 (.tsx) | 200 行 |
| TypeScript 模組 (.ts) | 300 行 |
| 單一函數/方法 | 50 行 |

### Rust Module Organization

**目錄結構原則：**

```
src-tauri/src/
├── lib.rs              # 入口，只做 mod 宣告和 re-export
├── commands/           # Tauri Commands（按功能拆分）
│   ├── mod.rs          # pub use 子模組
│   ├── auth.rs         # 單一職責：認證
│   ├── work_items/     # 大型模組拆成資料夾
│   │   ├── mod.rs
│   │   ├── queries.rs  # 查詢操作
│   │   ├── mutations.rs# 新增/更新/刪除
│   │   └── types.rs    # 該模組專用型別
│   └── ...
├── services/           # 業務邏輯（可跨 command 共用）
├── models/             # 資料模型
└── db/                 # 資料庫操作
```

**拆分時機：**
- 單一 `.rs` 檔案超過 300 行 → 考慮拆成資料夾
- 模組內有 3+ 個不同職責 → 按職責拆分
- 多個 command 共用邏輯 → 抽到 `services/`

**命名規則：**
- 檔案/模組：`snake_case` (例：`work_items.rs`)
- 結構體/列舉：`PascalCase` (例：`WorkItem`)
- 函數/變數：`snake_case` (例：`get_work_item`)
- 常數：`SCREAMING_SNAKE_CASE` (例：`MAX_PAGE_SIZE`)

### TypeScript/React Organization

**目錄結構原則：**

```
src/
├── types/              # 共用型別定義（單一來源）
│   ├── index.ts        # 統一匯出
│   ├── auth.ts
│   ├── work-items.ts
│   └── ...
├── services/           # API 層（取代原本的 tauri-api.ts + api.ts）
│   ├── index.ts        # 統一匯出 api 物件
│   ├── auth.ts
│   ├── work-items.ts
│   └── ...
├── hooks/              # 共用 Custom Hooks
├── components/         # 可重用 UI 元件
│   ├── ui/             # shadcn/ui 基礎元件
│   └── [ComponentName]/
│       ├── index.tsx   # 元件本體
│       ├── hooks.ts    # 元件專用 hooks（可選）
│       └── types.ts    # 元件專用型別（可選）
├── pages/              # 頁面元件
│   └── [PageName]/     # 大型頁面拆成資料夾
│       ├── index.tsx   # 頁面主體（組合子元件）
│       ├── components/ # 頁面專用子元件
│       └── hooks.ts    # 頁面專用 hooks
└── lib/                # 工具函數
    └── utils.ts
```

**拆分時機：**
- 元件超過 200 行 → 抽取子元件或 custom hook
- 頁面超過 300 行 → 拆成資料夾結構
- 邏輯在 2+ 處重複 → 抽成 custom hook 或 utils
- 型別在 2+ 檔案使用 → 移到 `types/`

**型別管理原則：**
- 所有共用型別定義在 `types/` 目錄，**只定義一次**
- 元件 Props 型別可定義在元件檔案內
- 禁止在多個檔案重複定義相同型別

**元件設計原則：**
- 展示元件 (Presentational)：只負責 UI，不含業務邏輯
- 容器元件 (Container)：負責資料獲取和狀態管理
- 頁面元件：組合容器和展示元件，處理路由

### Migration Examples

**範例 1：大型 Rust 模組拆分**

```
# Before: work_items.rs (2295 行)

# After:
commands/work_items/
├── mod.rs           # pub use + 共用 helper
├── queries.rs       # list, get, stats, timeline
├── mutations.rs     # create, update, delete
├── sync.rs          # batch_sync, aggregate
└── types.rs         # WorkItemFilters, CreateRequest 等
```

**範例 2：大型 React 頁面拆分**

```
# Before: Settings.tsx (1572 行)

# After:
pages/Settings/
├── index.tsx              # 主頁面，組合子元件 (~100 行)
├── components/
│   ├── GeneralSettings.tsx
│   ├── JiraSettings.tsx
│   ├── LlmSettings.tsx
│   ├── GitLabSettings.tsx
│   └── SourceSettings.tsx
└── hooks.ts               # useSettings, useConfigUpdate
```

**範例 3：API 層整合**

```
# Before:
lib/tauri-api.ts (1116 行) + lib/api.ts (1093 行)
# 問題：型別重複定義、職責混淆

# After:
types/
├── index.ts
├── work-items.ts    # WorkItem, WorkItemFilters...
├── auth.ts          # UserResponse, TokenResponse...
└── ...

services/
├── index.ts         # export const api = { auth, workItems, ... }
├── auth.ts          # login, register, getCurrentUser
├── work-items.ts    # list, create, update, delete
└── ...
```

### Migration Checklist

重構單一模組時的檢查清單：

- [ ] 確認現有測試覆蓋該模組
- [ ] 補齊缺失的測試案例
- [ ] 建立新的目錄/檔案結構
- [ ] 逐步移動程式碼，保持測試通過
- [ ] 更新 import/export 路徑
- [ ] 執行完整測試套件
- [ ] 更新相關文件

---

## Desktop Refactoring Plan (v2)

### Current Status

**需要重構的大型檔案：**

| 檔案 | 行數 | 上限 | 優先級 | 狀態 |
|------|------|------|--------|------|
| `work_items.rs` | 2295 | 300 | P0 | 🔴 待處理 |
| `Settings.tsx` | 1562 | 200 | P0 | 🔴 待處理 |
| `WorkItems.tsx` | 1263 | 200 | P1 | 🔴 待處理 |
| `reports.rs` | 942 | 300 | P1 | 🔴 待處理 |
| `claude.rs` | 855 | 300 | P2 | 🔴 待處理 |
| `Reports.tsx` | 841 | 200 | P2 | 🔴 待處理 |
| `auth.rs` | 766 | 300 | P2 | 🔴 待處理 |
| `Dashboard.tsx` | 655 | 200 | P3 | 🔴 待處理 |
| `gitlab.rs` | 572 | 300 | P3 | 🔴 待處理 |
| `sources.rs` | 473 | 300 | P3 | 🔴 待處理 |

### Phase 1: Foundation (Week 1)

**目標：** 建立測試基礎，確保重構安全

| 任務 | 說明 | 驗收標準 |
|------|------|----------|
| 1.1 補齊 Rust 測試 | `work_items.rs` 單元測試 | 覆蓋率 > 70% |
| 1.2 補齊前端測試 | `Settings.tsx` 元件測試 | 主要流程有測試 |
| 1.3 設定 CI | GitHub Actions 跑測試 | PR 自動測試 |

### Phase 2: Rust Commands 重構 (Week 2-3)

**目標：** 拆分最大的 Rust 模組

```
# work_items.rs (2295行) 拆分計劃
commands/work_items/
├── mod.rs              # 入口，re-export 所有 commands
├── types.rs            # WorkItemFilters, GroupedQuery, 等型別 (~100行)
├── queries.rs          # list, get, stats, timeline (~400行)
├── mutations.rs        # create, update, delete (~200行)
├── sync.rs             # batch_sync, aggregate (~300行)
├── grouped.rs          # get_grouped_work_items (~200行)
└── query_builder.rs    # SafeQueryBuilder 模組 (~150行)
```

| 任務 | 說明 | 依賴 |
|------|------|------|
| 2.1 拆分 `work_items.rs` | 按上述結構拆分 | 1.1 完成 |
| 2.2 拆分 `reports.rs` | queries / export / types | 2.1 完成 |
| 2.3 拆分 `claude.rs` | sessions / import / types | 2.1 完成 |

### Phase 3: React Pages 重構 (Week 3-4)

**目標：** 拆分大型頁面元件

```
# Settings.tsx (1562行) 拆分計劃
pages/Settings/
├── index.tsx                 # 主頁面框架 (~150行)
├── components/
│   ├── ProfileSection.tsx    # 個人資料 (~120行)
│   ├── AccountSection.tsx    # 帳號設定 (~80行)
│   ├── IntegrationsSection/
│   │   ├── index.tsx         # 整合服務主框架
│   │   ├── GitRepoCard.tsx   # 本地 Git
│   │   ├── ClaudeCodeCard.tsx# Claude Code
│   │   ├── JiraTempoCard.tsx # Jira/Tempo
│   │   └── GitLabCard.tsx    # GitLab
│   ├── PreferencesSection.tsx# 偏好設定 (~150行)
│   └── AboutSection.tsx      # 關於 (~60行)
└── hooks/
    └── useSettings.ts        # 狀態管理 (~200行)
```

| 任務 | 說明 | 依賴 |
|------|------|------|
| 3.1 拆分 `Settings.tsx` | 按上述結構拆分 | 1.2 完成 |
| 3.2 拆分 `WorkItems.tsx` | List/Project/Task/Timeline 視圖 | 3.1 完成 |
| 3.3 拆分 `Reports.tsx` | ReportList/ReportDetail/ExportModal | 3.1 完成 |

### Phase 4: Polish (Week 5)

| 任務 | 說明 |
|------|------|
| 4.1 拆分剩餘模組 | Dashboard, auth.rs, gitlab.rs, sources.rs |
| 4.2 更新文件 | API docs, 元件文件 |
| 4.3 效能優化 | 檢查 bundle size, 懶載入 |
| 4.4 最終測試 | 全功能回歸測試 |

### Progress Tracking

```
Phase 1: 🟩🟩⬜ 66%  (1.1 ✅ 1.2 ✅ 1.3 ⏳)
Phase 2: ⬜⬜⬜ 0%
Phase 3: ⬜⬜⬜ 0%
Phase 4: ⬜⬜⬜ 0%
Overall:  ~15% complete
```

**Phase 1 細項：**
- [x] 1.1 補齊 Rust 測試 (`work_items.rs` 單元測試)
- [x] 1.2 補齊前端測試 (`Settings.tsx` 元件測試)
- [ ] 1.3 設定 CI (GitHub Actions) - Desktop 分支已完成，待合併

> 更新日期：2025-01-16
