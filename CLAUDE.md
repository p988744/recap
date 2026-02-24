# CLAUDE.md

This file provides guidance for Claude when working on the Recap codebase.

## Project Overview

Recap is a work tracking and reporting desktop application built with Tauri v2. It automatically collects work records from Claude Code sessions and helps users manage their work items for daily reporting and performance reviews.

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
│   ├── components/ui/       # shadcn/ui base components
│   ├── pages/               # Page components (each page has components/ + hooks/)
│   │   ├── Dashboard/
│   │   ├── ThisWeek/        # Weekly overview with heatmap + Gantt
│   │   ├── Worklog/         # Worklog overview with Tempo export
│   │   ├── WorkItems/
│   │   ├── Projects/        # Project management with timeline + Git diff
│   │   ├── Reports/
│   │   └── Settings/
│   ├── services/            # Tauri API wrappers (per-module)
│   │   └── integrations/    # External service integrations (tempo, gitlab, http-export...)
│   └── types/               # Shared type definitions
├── src-tauri/               # Backend (Rust)
│   └── src/
│       ├── lib.rs           # App entry, command registration
│       ├── commands/        # Tauri Commands (per-module directories)
│       │   ├── work_items/  # queries, mutations, sync, grouped, commit_centric
│       │   ├── projects/    # queries, descriptions, timeline, summaries, git_diff
│       │   ├── reports/     # queries, export
│       │   ├── gitlab/      # config, projects, sync
│       │   ├── sources/
│       │   ├── auth/
│       │   ├── http_export.rs
│       │   ├── claude.rs
│       │   ├── tempo.rs
│       │   ├── snapshots.rs
│       │   ├── background_sync.rs
│       │   └── ...
│       ├── services/        # Business logic
│       └── auth/            # JWT authentication
└── crates/
    ├── recap-core/          # Shared core logic (worklog, sessions, snapshots, http_export, llm)
    └── recap-cli/           # CLI tool
```

## Development Guidelines

### Adding New Features

1. **Backend (Rust)**:
   - Create a new Tauri command in `src-tauri/src/commands/`
   - Use `#[tauri::command]` attribute
   - Register command in `lib.rs` `invoke_handler`
   - Token-based auth: pass `token: String` as first parameter, verify with `verify_token(&token)`

2. **Frontend (TypeScript)**:
   - Add type definitions in `src/types/`
   - Add invoke function in `src/services/<module>.ts`
   - Export from `src/services/index.ts`

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
// src/services/integrations/example.ts
import { invokeAuth } from '../client'
import type { ReturnType } from '@/types'

export async function exampleCommand(param: SomeType): Promise<ReturnType> {
  return invokeAuth<ReturnType>('example_command', { param })
}

// src/services/integrations/index.ts — re-export as namespace
export * as example from './example'

// src/services/index.ts — re-export
export { example } from './integrations'
```

### Code Style

- **Rust**: Follow Rust conventions, use `rustfmt`
- **TypeScript**: Follow existing patterns in the codebase
- **React**: Use functional components with hooks
- **CSS**: Use Tailwind CSS classes

### Database

- SQLite database stored in `~/Library/Application Support/com.recap.Recap/recap.db`
- Use `sqlx` for async database operations
- Migrations in `crates/recap-core/src/db/mod.rs` — append new `CREATE TABLE IF NOT EXISTS` at end of `run_migrations()`
- Use parameterized queries to prevent SQL injection

### Authentication

- JWT tokens for user authentication
- Token passed as parameter to all authenticated commands
- Frontend uses `invokeAuth()` helper which auto-injects token

### Error Handling

- Rust: Return `Result<T, String>` from commands
- TypeScript: Handle errors with try/catch around `invoke()` calls

### Cross-Platform Compatibility

- **Path handling**: Always use `std::path::Path` API instead of `split('/')` for file name extraction
- **Home directory**: Use `dirs::home_dir()` instead of `std::env::var("HOME")`
- **Path encoding**: Use `replace(['/', '\\'], "-")` for Claude Code directory name encoding
- **Frontend paths**: Use `split(/[/\\]/)` instead of `split('/')`

## Testing

```bash
# Frontend tests (207 tests)
cd web && npm test -- --run

# Rust tests — exclude recap-cli (integration tests timeout >60s each)
cd web && cargo test -p recap-core -p recap --quiet

# TypeScript type check
cd web && npx tsc --noEmit

# Run Tauri development mode
cd web && cargo tauri dev
```

## Common Tasks

### Adding a new API endpoint

1. Create command in `src-tauri/src/commands/<module>.rs`
2. Register in `src-tauri/src/lib.rs` `invoke_handler`
3. Add TypeScript types in `src/types/<module>.ts`, re-export from `src/types/index.ts`
4. Add invoke function in `src/services/integrations/<module>.ts`
5. Re-export from `src/services/integrations/index.ts` and `src/services/index.ts`

### Modifying database schema

1. Add `CREATE TABLE IF NOT EXISTS` or `ALTER TABLE` in `crates/recap-core/src/db/mod.rs`
2. Update models as needed
3. Tables auto-migrate on app startup

### Adding a new page

1. Create page directory in `src/pages/<PageName>/` with `index.tsx`, `components/`, `hooks/`
2. Add route in `src/App.tsx`
3. Update navigation in `src/components/Layout.tsx`

### Version bumps

Four files must be updated in sync:
- `web/Cargo.toml` (workspace version)
- `web/src-tauri/Cargo.toml` (package version)
- `web/package.json`
- `web/src-tauri/tauri.conf.json`

`Cargo.lock` updates automatically. CHANGELOG is at repo root: `CHANGELOG.md`.

## Important Notes

- **No HTTP Server**: All communication uses Tauri IPC
- **Token Authentication**: Always pass token to authenticated commands
- **AppState**: Use `State<'_, AppState>` to access shared database connection
- **Async/Await**: All database operations are async
- **Error Messages**: Return user-friendly error messages from commands
- **Cross-Platform**: Use `Path` API and `dirs` crate for all path operations

## Code Organization Principles

### File Size Guidelines

| Type | Recommended Max |
|------|----------------|
| Rust module (.rs) | 300 lines |
| React component (.tsx) | 200 lines |
| TypeScript module (.ts) | 300 lines |
| Single function/method | 50 lines |

### Split Triggers

- Single `.rs` file > 300 lines → split into directory module
- Module has 3+ responsibilities → split by responsibility
- Multiple commands share logic → extract to `services/`
- Component > 200 lines → extract sub-components or custom hooks
- Page > 300 lines → split into directory structure
- Logic duplicated in 2+ places → extract to custom hook or utils
- Types used in 2+ files → move to `types/`

### Type Management

- All shared types defined in `types/` directory — **define once only**
- Component Props types can be defined in the component file
- Never duplicate type definitions across files

## Technical Documentation

| Document | Description |
|----------|------------|
| [`web/docs/DATA_SOURCES.md`](web/docs/DATA_SOURCES.md) | Data source architecture: Claude Code data flow, table relationships, Session ID format history |

## Development Notes & Troubleshooting

### Git Commit Data Flow

```
Session Files → snapshot.rs (enrich) → snapshot_raw_data (full JSON)
                                              ↓
Frontend ← snapshots.rs (API) ← work_summaries + snapshot fallback
```

**Key functions:**
- `enrich_buckets_with_git_commits` — adds commits during capture
- `resolve_git_root` — finds the actual .git directory
- `get_hourly_breakdown` — API returns hourly detail

**Common issues:**
1. **Commits not showing** → check `snapshot_raw_data.git_commits` for data
2. **Time format errors** → handle both RFC3339 and NaiveDateTime
3. **Can't find git repo** → use `resolve_git_root()` instead of project_path directly

### Debug Commands

```bash
# Check snapshot git commits
sqlite3 ~/Library/Application\ Support/com.recap.Recap/recap.db \
  "SELECT hour_bucket, git_commits FROM snapshot_raw_data WHERE project_path LIKE '%projectName%'"

# Check work_summaries
sqlite3 ~/Library/Application\ Support/com.recap.Recap/recap.db \
  "SELECT period_start, git_commits_summary FROM work_summaries WHERE scale = 'hourly'"

# Check LLM summaries
sqlite3 ~/Library/Application\ Support/com.recap.Recap/recap.db \
  "SELECT project_path, summary_source, LENGTH(summary) FROM work_summaries WHERE scale = 'daily'"
```

### OpenAI API: GPT-5 Requires Responses API

GPT-5 models use Reasoning Tokens and need the Responses API (`/v1/responses`) instead of Chat Completions (`/v1/chat/completions`). Key differences:

| Item | Chat Completions | Responses API |
|------|-----------------|---------------|
| Endpoint | `/v1/chat/completions` | `/v1/responses` |
| Input | `messages` array | `input` string |
| Output | `choices[0].message.content` | `output` array (reasoning + message) |
| Token param | `max_tokens` | `max_output_tokens` |

Detection: `model.starts_with("gpt-5")` → use Responses API. Fallback: trivial response (< 20 chars) → rule-based summary.

**Related files:** `crates/recap-core/src/services/llm.rs`

### Tauri Version Matching

Tauri npm packages and Rust crates **must match major/minor**, otherwise `tauri build` fails.

```bash
# Check versions
npm ls @tauri-apps/api
grep 'name = "tauri"' -A1 web/Cargo.lock
```

### Release CI

- `release.yml` triggered by `v*` tag push
- Builds: macOS (aarch64 + x86_64), Windows (nsis), Linux (deb + appimage)
- Requires GitHub Secrets: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_TEAM_ID`
