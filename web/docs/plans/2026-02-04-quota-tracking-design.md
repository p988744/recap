# Quota Tracking System Design

> v2.2.0 主要功能：追蹤 Claude Code 和 Antigravity 的 quota 用量

## 目標

讓用戶了解 AI 工具的使用程度，判斷訂閱方案是否足夠。

## 設計決策

| 項目 | 決策 |
|------|------|
| 資料策略 | 混合模式：即時查詢 + 定期快照 |
| 快照密度 | 可調整，最小 5 分鐘 |
| 觸發機制 | 背景定時器 |
| Claude 認證 | OAuth API（使用 CLI token） |
| UI | Tray 文字 + Dashboard 卡片 + 獨立頁面 |
| 警告 | 系統通知 + Tray 文字顏色 |
| 門檻 | 用戶可調，預設 80%/95% |

## 架構總覽

```
┌─────────────────────────────────────────────────────────────────┐
│                        Quota Tracking System                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐       │
│  │ Claude       │    │ Antigravity  │    │ (Future)     │       │
│  │ Provider     │    │ Provider     │    │ Provider     │       │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘       │
│         │                   │                   │                │
│         └───────────────────┼───────────────────┘                │
│                             ▼                                    │
│                   ┌──────────────────┐                          │
│                   │  QuotaProvider   │  ← Trait (抽象介面)       │
│                   │  trait           │                          │
│                   └────────┬─────────┘                          │
│                            │                                     │
│         ┌──────────────────┼──────────────────┐                 │
│         ▼                  ▼                  ▼                 │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐           │
│  │ QuotaStore  │   │ QuotaAlert  │   │ QuotaTimer  │           │
│  │ (SQLite)    │   │ (通知/Tray) │   │ (定時觸發)  │           │
│  └─────────────┘   └─────────────┘   └─────────────┘           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## 資料模型

### 資料庫 Schema

```sql
-- 快照記錄表
CREATE TABLE quota_snapshots (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    provider TEXT NOT NULL,          -- 'claude' | 'antigravity'
    model TEXT,                       -- 'sonnet' | 'opus' | 'gemini-pro' | null (總量)
    window_type TEXT NOT NULL,        -- 'five_hour' | 'seven_day' | 'monthly'
    used_percent REAL NOT NULL,       -- 0.0 ~ 100.0
    resets_at TEXT,                   -- ISO8601 timestamp
    extra_credits_used REAL,          -- Claude extra usage (美元)
    extra_credits_limit REAL,         -- Claude monthly limit
    raw_response TEXT,                -- 原始 API 回應 (debug 用)
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE INDEX idx_quota_provider_time ON quota_snapshots(user_id, provider, created_at);
```

### 用戶設定（擴充現有 settings）

- `quota_interval_minutes`: 5 | 10 | 15 | 30 | 60 (預設 15)
- `quota_warning_threshold`: 0-100 (預設 80)
- `quota_critical_threshold`: 0-100 (預設 95)
- `quota_notifications_enabled`: boolean (預設 true)

### Rust 資料結構

```rust
pub struct QuotaSnapshot {
    pub provider: QuotaProvider,      // Claude | Antigravity
    pub model: Option<String>,
    pub window_type: QuotaWindowType, // FiveHour | SevenDay | Monthly
    pub used_percent: f64,
    pub resets_at: Option<DateTime<Utc>>,
    pub extra_credits: Option<ExtraCredits>,
    pub fetched_at: DateTime<Utc>,
}

pub struct ExtraCredits {
    pub used: f64,
    pub limit: f64,
    pub currency: String,  // "USD"
}
```

## Rust Trait 定義

```rust
// crates/recap-core/src/services/quota/mod.rs

use async_trait::async_trait;

#[async_trait]
pub trait QuotaProvider: Send + Sync {
    /// Provider 識別名稱
    fn provider_id(&self) -> &'static str;

    /// 取得當前 quota 用量
    async fn fetch_quota(&self) -> Result<Vec<QuotaSnapshot>, QuotaError>;

    /// 檢查 Provider 是否可用（已安裝/已登入）
    async fn is_available(&self) -> bool;

    /// 取得帳戶資訊（email、方案名稱）
    async fn get_account_info(&self) -> Result<Option<AccountInfo>, QuotaError>;
}

pub struct AccountInfo {
    pub email: Option<String>,
    pub plan_name: Option<String>,
    pub organization: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum QuotaError {
    #[error("Provider not installed")]
    NotInstalled,
    #[error("Authentication required")]
    Unauthorized,
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
}
```

## Claude OAuth 實作

```rust
// crates/recap-core/src/services/quota/claude.rs

pub struct ClaudeQuotaProvider {
    oauth_token_path: PathBuf,  // ~/.claude/credentials.json
}

impl ClaudeQuotaProvider {
    /// 從 Claude CLI 的認證檔讀取 OAuth token
    fn load_oauth_token(&self) -> Result<String, QuotaError>;

    /// 呼叫 Anthropic OAuth Usage API
    /// GET https://api.anthropic.com/api/oauth/usage
    /// Headers:
    ///   Authorization: Bearer {token}
    ///   anthropic-beta: oauth-2025-04-20
    async fn call_usage_api(&self, token: &str) -> Result<OAuthUsageResponse, QuotaError>;
}

#[derive(Deserialize)]
struct OAuthUsageResponse {
    five_hour: Option<UsageWindow>,      // 5 小時滑動窗口
    seven_day: Option<UsageWindow>,      // 7 天總量
    seven_day_opus: Option<UsageWindow>, // 7 天 Opus 專用
    seven_day_sonnet: Option<UsageWindow>,
    extra_usage: Option<ExtraUsage>,     // 額外付費用量
}

#[derive(Deserialize)]
struct UsageWindow {
    utilization: f64,    // 0.0 ~ 1.0
    resets_at: String,   // ISO8601
}
```

**Token 位置：** `~/.claude/credentials.json`

## 背景定時器與警告系統

```rust
// crates/recap-core/src/services/quota/timer.rs

pub struct QuotaTimer {
    interval: Duration,
    providers: Vec<Box<dyn QuotaProvider>>,
    store: QuotaStore,
    alert: QuotaAlert,
}

impl QuotaTimer {
    pub fn start(&self, app_handle: AppHandle) {
        tauri::async_runtime::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                self.tick().await;
            }
        });
    }

    async fn tick(&self) {
        for provider in &self.providers {
            if let Ok(snapshots) = provider.fetch_quota().await {
                self.store.save_snapshots(&snapshots).await;
                self.alert.check_and_notify(&snapshots).await;
            }
        }
    }
}
```

### Tray 文字顯示

```rust
impl QuotaAlert {
    pub fn update_tray_title(&self, snapshots: &[QuotaSnapshot]) {
        let claude = snapshots.iter()
            .find(|s| s.provider == Provider::Claude && s.window_type == FiveHour);
        let antigravity = snapshots.iter()
            .find(|s| s.provider == Provider::Antigravity);

        let title = match (claude, antigravity) {
            (Some(c), Some(a)) => format!("C:{}% A:{}%", c.used_percent, a.used_percent),
            (Some(c), None) => format!("{}%", c.used_percent),
            (None, Some(a)) => format!("{}%", a.used_percent),
            _ => "—".to_string(),
        };

        tray.set_title(Some(&title));
    }
}
```

**顏色規則：**
- Normal（< 80%）：預設顏色
- Warning（80-95%）：黃色
- Critical（> 95%）：紅色

## 前端 UI

### Dashboard 卡片

```
┌─────────────────────────────────────────────┐
│  📊 Quota Usage                    [⟳ 重新整理] │
├─────────────────────────────────────────────┤
│                                             │
│  Claude Code              Antigravity       │
│  ┌───────────────┐       ┌───────────────┐ │
│  │ 5hr   ████░░ 45%│      │ Pro  ███░░░ 40%│ │
│  │ 7day  ██░░░░ 23%│      │Flash █░░░░░ 15%│ │
│  │ Opus  █░░░░░ 12%│      └───────────────┘ │
│  └───────────────┘                          │
│                                             │
│  Resets in 2h 15m          Resets in 4h 30m │
│                                             │
└─────────────────────────────────────────────┘
```

### 獨立頁面（Quota）

- 歷史趨勢圖表
- Provider/Window 篩選器
- Settings（間隔、門檻）

## 檔案結構

```
crates/recap-core/src/services/quota/
├── mod.rs              # 模組入口
├── provider.rs         # QuotaProvider trait
├── claude.rs           # Claude OAuth 實作
├── antigravity.rs      # Antigravity 實作 (Phase 2)
├── store.rs            # SQLite 儲存
├── alert.rs            # 警告與通知
└── timer.rs            # 背景定時器

web/src-tauri/src/commands/
└── quota.rs            # Tauri Commands

web/src/
├── pages/Quota/        # 獨立頁面
│   ├── index.tsx
│   ├── components/
│   │   ├── QuotaChart.tsx
│   │   └── QuotaSettings.tsx
│   └── hooks.ts
├── pages/Dashboard/components/
│   └── QuotaCard.tsx   # Dashboard 卡片
└── services/quota.ts   # API 封裝
```

## 實作順序

### Phase 1: Claude（本階段）

| 順序 | 任務 | 說明 |
|------|------|------|
| 1 | 資料庫 schema | 新增 `quota_snapshots` 表 |
| 2 | Trait 定義 | `QuotaProvider` trait |
| 3 | Claude 實作 | OAuth token 讀取 + API 呼叫 |
| 4 | Store 實作 | 儲存/查詢快照 |
| 5 | Timer 實作 | 背景定時器 |
| 6 | Alert 實作 | 通知 + Tray 文字 |
| 7 | Tauri Commands | 前端 API |
| 8 | Dashboard 卡片 | QuotaCard 元件 |
| 9 | 獨立頁面 | Quota 頁面 + 圖表 |
| 10 | Settings 整合 | 間隔/門檻設定 |

### Phase 2: Antigravity

使用相同的 `QuotaProvider` trait，實作 `AntigravityQuotaProvider`。

## Debug Logging 機制

由於 Tauri 不易 debug，所有模組必須加入完整的 console log。

### Rust 端 Logging

```rust
// 使用 log crate
use log::{debug, info, warn, error};

impl ClaudeQuotaProvider {
    async fn fetch_quota(&self) -> Result<Vec<QuotaSnapshot>, QuotaError> {
        info!("[quota:claude] Starting quota fetch");

        let token = match self.load_oauth_token() {
            Ok(t) => {
                debug!("[quota:claude] OAuth token loaded successfully");
                t
            }
            Err(e) => {
                error!("[quota:claude] Failed to load OAuth token: {:?}", e);
                return Err(e);
            }
        };

        debug!("[quota:claude] Calling API: {}", Self::USAGE_API_URL);
        let response = self.call_usage_api(&token).await?;

        info!("[quota:claude] Quota fetched: 5hr={:.1}%, 7day={:.1}%",
            response.five_hour.map(|w| w.utilization * 100.0).unwrap_or(0.0),
            response.seven_day.map(|w| w.utilization * 100.0).unwrap_or(0.0)
        );

        Ok(snapshots)
    }
}
```

### 前端 Logging

```typescript
// src/services/quota.ts
const LOG_PREFIX = '[quota]';

export async function fetchQuota(): Promise<QuotaSnapshot[]> {
  console.log(`${LOG_PREFIX} Fetching quota...`);

  try {
    const result = await invoke<QuotaSnapshot[]>('get_current_quota', {
      token: getRequiredToken(),
    });
    console.log(`${LOG_PREFIX} Quota fetched:`, result);
    return result;
  } catch (error) {
    console.error(`${LOG_PREFIX} Failed to fetch quota:`, error);
    throw error;
  }
}
```

### Log 分類前綴

| 模組 | 前綴 | 說明 |
|------|------|------|
| Claude Provider | `[quota:claude]` | OAuth、API 呼叫 |
| Antigravity Provider | `[quota:antigravity]` | Language Server API |
| Timer | `[quota:timer]` | 定時器觸發 |
| Store | `[quota:store]` | 資料庫讀寫 |
| Alert | `[quota:alert]` | 通知、Tray 更新 |
| Frontend | `[quota]` | React 元件、API 呼叫 |

### 開發時查看 Log

```bash
# Tauri dev 模式會在 terminal 顯示 Rust log
cd web && RUST_LOG=debug cargo tauri dev

# 前端 log 在 Chrome DevTools Console 查看
```

## 參考資料

- [CodexBar](https://github.com/steipete/CodexBar) - macOS quota 監控工具
- Anthropic OAuth Usage API: `GET https://api.anthropic.com/api/oauth/usage`
- Antigravity API: `POST /exa.language_server_pb.LanguageServerService/GetUserStatus`
