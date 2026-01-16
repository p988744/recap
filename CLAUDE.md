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

> 使用 `/team-status` 指令查看最新狀態

| 分支 | 狀態 | 說明 |
|------|------|------|
| `main` | 🟢 穩定 | 保護分支 |
| `develop` | 🟢 同步 | 整合分支，由 PM 管理 |
| `refactor/core-v2` | ✅ 已合併 | PR #6 已合併至 develop |
| `refactor/desktop-v2` | 🟢 活躍 | Phase 2-3 重構進行中 |
| `refactor/cli-v2` | 🟡 進行中 | 測試覆蓋提升中 |

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

### 分支隔離原則（避免互相污染）

**嚴禁事項：**

| 禁止行為 | 原因 |
|----------|------|
| 直接修改其他成員的分支 | 會造成歷史混亂、衝突 |
| 在 main/develop 上直接開發 | 應在 feature 分支開發 |
| 跨 worktree 共用 node_modules/target | 會造成編譯錯誤 |
| 未經 rebase 就合併 | 會產生不必要的 merge commit |
| Cherry-pick 其他成員未合併的 commit | 會造成重複 commit |

**正確做法：**

```
┌─────────────────────────────────────────────────────────┐
│  Core Worktree        │  Desktop Worktree   │  CLI     │
│  (core-dev/)          │  (desktop-dev/)     │ (cli-dev)│
├───────────────────────┼─────────────────────┼──────────┤
│  只改 crates/         │  只改 src-tauri/    │ 只改     │
│  recap-core/          │  和 web/src/        │ recap-cli│
├───────────────────────┴─────────────────────┴──────────┤
│              ↓ PR 合併至 develop ↓                      │
├─────────────────────────────────────────────────────────┤
│                    develop 分支                         │
│              (整合點，由 PM 管理)                        │
└─────────────────────────────────────────────────────────┘
```

**各角色職責邊界：**

| 角色 | 可修改 | 禁止修改 |
|------|--------|----------|
| Core 開發者 | `crates/recap-core/` | `src-tauri/`, `web/src/`, `crates/recap-cli/` |
| Desktop 開發者 | `web/src-tauri/`, `web/src/` | `crates/recap-core/`, `crates/recap-cli/` |
| CLI 開發者 | `crates/recap-cli/` | `crates/recap-core/`, `src-tauri/`, `web/src/` |
| PM | `CLAUDE.md`, GitHub Issues, `.claude/commands/` | **所有程式碼** |

### PM 角色限制（重要）

**PM 不能進行任何開發工作**，包括但不限於：
- ❌ 建立 feature 分支
- ❌ 修改任何程式碼（.ts, .tsx, .rs, .css 等）
- ❌ 執行重構任務
- ❌ 撰寫測試程式碼

**PM 可以做的事：**
- ✅ Review PR 並提供意見
- ✅ 合併 PR 至 develop/main
- ✅ 建立和管理 GitHub Issues/Milestones
- ✅ 更新 CLAUDE.md 文件
- ✅ 管理 `.claude/commands/` 指令
- ✅ 使用 `/team-status` 追蹤進度
- ✅ 協調團隊成員工作分配

**當 PM 需要新功能或修改時：**
1. 建立 GitHub Issue 描述需求
2. 指派給對應的開發者
3. 等待開發者提交 PR
4. Review 並合併

**需要跨模組修改時：**
1. 開 Issue 說明需求
2. 由負責該模組的開發者處理
3. 等待其 PR 合併後再 rebase 取得更新

### Claude Code 團隊指令

提供 Claude Code slash commands 簡化協作流程，位於 `.claude/commands/`：

| 指令 | 說明 | 參數 |
|------|------|------|
| `/sync` | 同步 develop 分支 | 無 |
| `/pre-pr` | PR 提交前完整檢查 | 無 |
| `/check-boundary` | 檢查職責邊界 | 無 |
| `/create-pr` | 建立 PR（含檢查） | `[issue-number]` |
| `/team-status` | 顯示團隊開發狀態 | 無 |
| `/update-issues` | 更新 GitHub Issues 進度 | 無 |

**指令詳細說明：**

```
/sync
├── 檢查未提交變更
├── git fetch origin
├── git rebase origin/develop
└── 顯示同步結果

/pre-pr
├── 檢查是否已同步 develop
├── 列出你的 commits
├── 檢查修改的檔案
├── 驗證職責邊界
└── 提供 PR 建議

/check-boundary
├── 判斷當前角色（依分支名稱）
├── 檢查修改的檔案
└── 警告越界修改

/create-pr [issue]
├── 執行 pre-pr 檢查
├── 推送分支
├── 使用標準模板建立 PR
└── 關聯指定的 Issue

/team-status
├── 顯示所有 worktrees
├── 各分支進度和 commits
├── GitHub PRs 狀態
└── 建議下一步行動

/update-issues
├── 收集各分支最新狀態
├── 比對 GitHub Issues
└── 建議需要更新的 Issues
```

### 開發者每日工作流程

```
┌─────────────────────────────────────────────────────────────┐
│                    每日開發流程                              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. 開始工作                                                 │
│     $ claude                    # 啟動 Claude Code          │
│     > /sync                     # 同步 develop              │
│                                                             │
│  2. 進行開發                                                 │
│     > 實作功能...               # 正常開發                   │
│     > /check-boundary           # 隨時檢查是否越界           │
│                                                             │
│  3. 準備提交 PR                                              │
│     > /pre-pr                   # 完整檢查                   │
│     > /create-pr 2              # 建立 PR，關聯 Issue #2     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### PM 工作流程

```
┌─────────────────────────────────────────────────────────────┐
│                    PM 管理流程                               │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. 查看團隊狀態                                             │
│     > /team-status              # 了解整體進度               │
│                                                             │
│  2. Review PR                                               │
│     > 在 GitHub 上 review 並合併 PR                          │
│                                                             │
│  3. 更新進度追蹤                                             │
│     > /update-issues            # 同步 GitHub Issues         │
│                                                             │
│  4. 通知團隊                                                 │
│     > 合併後通知開發者執行 /sync                             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**查看所有可用指令：**
```
> /help
```

### Shell 腳本（備用）

如需在終端機直接執行，也提供 shell 腳本版本，位於 `scripts/team/`：

```bash
./scripts/team/sync-develop.sh      # 同步 develop
./scripts/team/pre-pr-check.sh      # PR 前檢查
./scripts/team/check-boundaries.sh  # 檢查職責邊界
./scripts/team/create-pr.sh         # 建立 PR
./scripts/team/status.sh            # 團隊狀態
```

### PR 提交前檢查清單

**必須檢查項目（提交 PR 前）：**

```bash
# 1. 確認已同步 develop
git fetch origin
git rebase origin/develop

# 2. 確認只有自己的 commits
git log origin/develop..HEAD --oneline
# 應該只看到自己的 commits，不應有其他成員的

# 3. 確認沒有修改到其他模組
git diff origin/develop --stat
# 檢查修改的檔案是否都在自己負責的範圍內

# 4. 測試通過
cargo test        # Rust
npm test          # Frontend

# 5. 編譯通過
cargo build
npm run build
```

**PR 描述模板：**

```markdown
## Summary
- 簡述完成的功能

## Changed Files
- 列出修改的檔案（確認都在職責範圍內）

## Checklist
- [ ] 已 rebase origin/develop
- [ ] 只包含自己的 commits
- [ ] 沒有修改其他模組的程式碼
- [ ] 測試通過
- [ ] 編譯通過

## Related Issue
Refs #<issue-number>
```

### PR 提交與合併流程

**角色分工：**
- **開發者**：完成開發後提交 PR 至 `develop`
- **PM**：負責 review 和協調合併順序

**PR 提交流程：**

```
1. 開發者在自己的 worktree 完成工作
2. 確保測試通過：cargo test / npm test
3. 提交 PR 至 develop 分支
4. 在 PR 描述中說明：
   - 完成了什麼功能/修復
   - 測試覆蓋情況
   - 是否有 breaking changes
5. 通知 PM 進行 review
```

**PR 提交指令：**
```bash
# 在 worktree 目錄下
git push -u origin <branch-name>

# 建立 PR（以 Core 為例）
gh pr create --base develop --title "feat(core): Add unit tests for recap-core" --body "## Summary
- Add comprehensive unit tests for recap-core modules
- Coverage > 70%

## Test Plan
- [x] cargo test --package recap-core"
```

**Review 優先順序：**

| 順序 | 分支 | 原因 |
|------|------|------|
| 1 | `refactor/core-v2` → `develop` | Desktop/CLI 依賴 Core |
| 2 | `refactor/desktop-v2` → `develop` | 需先 rebase develop |
| 3 | `refactor/cli-v2` → `develop` | 需先 rebase develop |
| 4 | `develop` → `main` | 所有功能整合測試通過後 |

**合併後通知：**
- Core 合併後，PM 通知 Desktop/CLI 開發者執行：
  ```bash
  git fetch origin
  git rebase origin/develop
  ```

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

**重構完成狀態：**

| 檔案 | 原行數 | 拆分後 | 狀態 |
|------|--------|--------|------|
| `work_items.rs` | 2295 | 6 個檔案 | ✅ 已完成 |
| `Settings.tsx` | 1562 | 8 個檔案 | ✅ 已完成 |
| `WorkItems.tsx` | 1263 | 17 個檔案 | ✅ 已完成 |
| `reports.rs` | 942 | 5 個檔案 | ✅ 已完成 |
| `Reports.tsx` | 841 | 8 個檔案 | ✅ 已完成 |
| `auth.rs` | 766 | 6 個檔案 | ✅ 已完成 |
| `Dashboard.tsx` | 655 | 10 個檔案 | ✅ 已完成 |
| `gitlab.rs` | 572 | 5 個檔案 | ✅ 已完成 |
| `sources.rs` | 473 | 5 個檔案 | ✅ 已完成 |

**測試覆蓋：**
- Frontend: 98 tests (Vitest)
- Backend: 167 tests (Rust #[test])
- CI: GitHub Actions (rust-ci.yml, frontend-ci.yml)

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

**GitHub Issue Tracker:** [Milestone: Desktop Refactoring v2](https://github.com/p988744/recap/milestone/1)

| Issue | 說明 | 狀態 |
|-------|------|------|
| [#1](https://github.com/p988744/recap/issues/1) | [Phase 1] 建立測試基礎 | 🟡 66% |
| [#2](https://github.com/p988744/recap/issues/2) | [Phase 2] Rust Commands 重構 | ⚪ 待開始 |
| [#3](https://github.com/p988744/recap/issues/3) | [Phase 3] React Pages 重構 | ⚪ 待開始 |
| [#4](https://github.com/p988744/recap/issues/4) | [Phase 4] Polish & 整合測試 | ⚪ 待開始 |
| [#5](https://github.com/p988744/recap/issues/5) | [Core] recap-core 單元測試 | 🟡 進行中 |

```
Phase 1: ✅✅✅ 100% (測試基礎已建立)
Phase 2: ✅✅✅ 100% (Rust 模組已重構)
Phase 3: ✅✅✅ 100% (React 頁面已重構)
Phase 4: ✅✅✅ 100% (收尾工作已完成)
Overall: 100% complete
```

**PR 關聯 Issue 方式：**
```bash
# 在 PR 描述或 commit message 中使用
Closes #1   # 合併後自動關閉 Issue
Refs #2     # 僅關聯，不自動關閉
```
> 更新日期：2026-01-16
