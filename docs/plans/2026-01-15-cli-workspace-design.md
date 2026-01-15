# Recap CLI & Workspace Architecture Design

**Date:** 2026-01-15
**Status:** Approved
**Author:** Claude Code + User collaborative brainstorming

## Overview

Build a CLI version of the Recap app that shares 100% of core business logic with the Tauri app. This enables:
- Testing via Claude Code in non-GUI environments
- Production usage in headless servers (cron jobs, CI/CD)
- TDD development workflow

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Output format | Human-friendly default, `--json` flag | Best of both worlds |
| System config | Environment variables | `RECAP_DB_PATH` can override DB location |
| User data | Always in DB | Consistent with Tauri, ready for future team features |
| Auth flow | Simplified, no login required | Local app, keep `user_id` for future |
| Command style | `noun verb` (e.g., `recap work list`) | Modern, discoverable |
| Architecture | Workspace with multiple crates | Clean separation, independent compilation |
| Migration | Gradual, keep Tauri running | Safe, verifiable at each step |

## Project Structure (Final State)

```
web/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── recap-core/              # Shared business logic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── services/        # WorkItemService, SyncService, etc.
│   │       ├── models/          # WorkItem, User, GitRepo, etc.
│   │       ├── db/              # Database connection & queries
│   │       ├── auth/            # JWT & password hashing
│   │       └── error.rs         # Unified error type
│   │
│   ├── recap-cli/               # CLI binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/        # CLI command implementations
│   │       └── output/          # Table/JSON formatters
│   │
│   └── recap-tauri/             # Tauri binary
│       ├── Cargo.toml
│       ├── tauri.conf.json
│       ├── capabilities/
│       ├── icons/
│       └── src/
│           ├── lib.rs
│           └── commands/        # Thin wrappers calling recap-core
│
├── src/                         # Frontend (React) - unchanged
└── package.json
```

## CLI Command Design

```bash
recap <noun> <verb> [options] [--json]

# Work Items
recap work list [--date <DATE>] [--source <SOURCE>]
recap work add --title "..." --hours 2.5
recap work update <ID> --hours 3.0
recap work delete <ID>

# Sync
recap sync run [--source <SOURCE>]
recap sync status

# Sources
recap source list
recap source add git <PATH>
recap source add gitlab --url <URL> --token <TOKEN>
recap source remove git <PATH>

# Reports
recap report daily [--date <DATE>]
recap report weekly [--week <WEEK>]
recap report export --format excel --output file.xlsx

# Config
recap config show
recap config set <KEY> <VALUE>

# Global Options
--json          # JSON output for programmatic use
--quiet         # Suppress progress messages
--db <PATH>     # Override DB path (or use RECAP_DB_PATH env var)
```

## recap-core API Design

```rust
// crates/recap-core/src/lib.rs
pub mod db;
pub mod models;
pub mod services;
pub mod auth;
pub mod error;

pub use db::Database;
pub use error::{Error, Result};
```

### Service Pattern

```rust
// services/work_items.rs
pub struct WorkItemService<'a> {
    db: &'a Database,
}

impl<'a> WorkItemService<'a> {
    pub fn new(db: &'a Database) -> Self;
    pub async fn list(&self, filter: WorkItemFilter) -> Result<Vec<WorkItem>>;
    pub async fn create(&self, input: CreateWorkItem) -> Result<WorkItem>;
    pub async fn update(&self, id: &str, input: UpdateWorkItem) -> Result<WorkItem>;
    pub async fn delete(&self, id: &str) -> Result<()>;
}
```

### Usage from CLI and Tauri

```rust
// CLI usage
let db = Database::open(db_path).await?;
let service = WorkItemService::new(&db);
let items = service.list(filter).await?;

// Tauri usage (thin wrapper)
#[tauri::command]
async fn list_work_items(
    state: State<'_, AppState>,
    filter: WorkItemFilter,
) -> Result<Vec<WorkItem>, String> {
    let service = WorkItemService::new(&state.db);
    service.list(filter).await.map_err(|e| e.to_string())
}
```

## Migration Plan

### Phase 1: Create Workspace Skeleton
- Create `crates/` directory
- Create empty `recap-core` crate
- Modify root `Cargo.toml` for workspace
- Add `src-tauri` as workspace member
- **Verify:** Tauri app compiles and runs

### Phase 2: Extract Shared Modules to recap-core
- Move `models/` → `recap-core/src/models/`
- Move `db/` → `recap-core/src/db/`
- Move `auth/` → `recap-core/src/auth/`
- Move `services/` → `recap-core/src/services/`
- Update `src-tauri` to `use recap_core::{...}`
- **Verify:** Tauri app functionality unchanged

### Phase 3: Build recap-cli
- Create `crates/recap-cli/` crate
- Implement CLI command framework (clap)
- Implement commands one by one, test with Claude Code
- **Verify:** CLI functionality matches Tauri

### Phase 4: Unify Tauri Location
- Move `src-tauri/` → `crates/recap-tauri/`
- Update `tauri.conf.json` paths
- Update npm scripts
- **Verify:** `cargo tauri dev` works

## Acceptance Criteria

| Phase | Verification |
|-------|-------------|
| 1 | `cargo tauri dev` starts normally |
| 2 | `cargo tauri dev` + manual test of core features |
| 3 | Claude Code runs `recap work list`, `recap sync run` successfully |
| 4 | Both `cargo tauri dev` and `cargo run -p recap-cli` work |

## Development Approach

- **TDD:** Write tests first, then implementation
- **CLI First:** Complete CLI features before Tauri integration
- **Incremental:** Each change should keep the app runnable

## References

- [Command Line Interface Guidelines](https://clig.dev/)
- [Clap Best Practices](https://hemaks.org/posts/building-production-ready-cli-tools-in-rust-with-clap-from-zero-to-hero/)
- [Modern Rust CLI Tools](https://dev.to/dev_tips/15-rust-cli-tools-that-will-make-you-abandon-bash-scripts-forever-4mgi)
