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
