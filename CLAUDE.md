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
