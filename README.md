<p align="center">
  <img src="web/src-tauri/icons/128x128@2x.png" width="120" alt="Recap" />
</p>

<h1 align="center">Recap</h1>

<p align="center">
  <strong>Stop writing reports. Start shipping code.</strong>
</p>

<p align="center">
  <a href="https://github.com/p988744/recap/releases/latest"><img src="https://img.shields.io/github/v/release/p988744/recap?include_prereleases&style=flat-square&color=B09872" alt="Release" /></a>
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-333?style=flat-square" alt="Platform" />
  <img src="https://img.shields.io/badge/built%20with-Tauri%20v2-24C8D8?style=flat-square" alt="Tauri v2" />
</p>

<p align="center">
  Recap silently tracks your actual work from Claude Code sessions, then turns it into clean daily reports, Tempo worklogs, or custom API payloads. One click, zero busywork.
</p>

---

## Why Recap?

You spend hours coding. Then you spend more time *explaining* what you coded. Recap fixes that.

It watches your dev tools in the background, compacts raw activity into meaningful summaries with LLM, and lets you export everywhere — Jira Tempo, internal dashboards, or any HTTP endpoint you configure.

**The result:** Your standup notes, timesheets, and performance reviews write themselves.

## Features

### Auto-Tracking
- **Claude Code** — Session tracking with tool usage breakdown, Git commits, and project-level activity

### Smart Summaries
- **LLM-powered compaction** — Raw activity becomes concise work descriptions
- **Hour normalization** — Auto-adjust daily hours to your standard workday
- **Project-level summaries** — Understand what happened at a glance

### Views That Make Sense
- **This Week** — Heatmap + Gantt chart for your weekly rhythm
- **Worklog** — Day-by-day breakdown with hourly detail
- **Projects** — Timeline, README preview, Git diff viewer
- **Work Items** — List, project group, task group, and timeline modes

### Export Anywhere
- **Jira Tempo** — Single item, daily batch, or full week export with issue validation
- **HTTP Export** — Push to any API with customizable JSON templates, auth, and LLM prompts
- **Excel** — Download formatted reports
- **Export History** — Track what's been sent to avoid duplicates

### Background Sync
- Scheduled auto-sync via `tokio-cron-scheduler`
- System tray for quick access and manual trigger
- In-app updater for seamless version upgrades

## Quick Start

### Download

Grab the latest release from [**Releases**](https://github.com/p988744/recap/releases/latest):

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `Recap_x.x.x_aarch64.dmg` |
| macOS (Intel) | `Recap_x.x.x_x64.dmg` |
| Windows | `Recap_x.x.x_x64-setup.exe` |
| Linux | `recap_x.x.x_amd64.deb` / `.AppImage` |

### Setup (2 minutes)

1. **Add sources** — Point Recap at your Claude Code projects
2. **Configure export** — Connect Jira Tempo, set up HTTP endpoints, or both
3. **Sync** — Hit sync once, then let background scheduling handle the rest

That's it. Open Recap at the end of the day and your work log is ready.

## HTTP Export

The newest addition — export work items to *any* external service via configurable HTTP endpoints.

```json
{
  "date": "{{date}}",
  "project": "{{title}}",
  "hours": {{hours}},
  "summary": "{{llm_summary}}"
}
```

- Template engine with `{{placeholder}}` syntax for all work item fields
- Optional LLM prompt to generate custom summaries per endpoint
- Bearer, Basic, or custom header authentication
- Batch mode (array POST) or per-item mode
- Dry-run preview before sending
- Export history tracking to prevent duplicate submissions

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Frontend | React 18 + TypeScript + Tailwind CSS + shadcn/ui |
| Backend | Rust + Tauri v2 IPC + SQLite (sqlx) |
| Desktop | Tauri v2 (macOS / Windows / Linux) |
| Build | Vite + Cargo |

All communication uses Tauri IPC — no HTTP server, no ports to manage.

## Development

```bash
git clone https://github.com/p988744/recap.git
cd recap/web

npm install        # Frontend dependencies
cargo tauri dev    # Dev mode with hot reload
```

```bash
# Tests
npm test                          # Frontend (Vitest)
cargo test -p recap-core -p recap # Backend (excluding CLI)
npx tsc --noEmit                  # Type check
```

## License

MIT
