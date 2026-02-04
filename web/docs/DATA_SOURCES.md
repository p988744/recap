# Data Sources Architecture

本文件說明 Recap 如何從不同來源取得工作資料，以及資料流程的差異。

## 資料來源概覽

| 來源 | 識別方式 | Session ID 格式 | 資料取得方式 |
|------|----------|-----------------|--------------|
| Claude Code | `source = 'claude_code'` | UUID (如 `fe4dd10f-...`) | 本地 JSONL 檔案 |
| Antigravity (Gemini Code) | `source = 'antigravity'` | UUID (如 `fe4dd10f-...`)* | HTTP API |

> *注意：Antigravity API 早期使用 `agent-*` 格式（如 `agent-a8e7a53`），現已改為 UUID 格式。

---

## 資料表關係

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           資料流程圖                                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Claude Code                          Antigravity                       │
│  (本地 JSONL)                         (HTTP API)                        │
│      │                                    │                             │
│      ▼                                    ▼                             │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                    snapshot_raw_data                              │   │
│  │  - 儲存每小時的原始資料（user_messages, tool_calls, files 等）     │   │
│  │  - 主鍵：id (UUID)                                                │   │
│  │  - 索引：session_id + hour_bucket                                 │   │
│  └───────────────────────────┬──────────────────────────────────────┘   │
│                              │                                          │
│                              ▼ (LLM Compaction)                         │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                    work_summaries                                 │   │
│  │  - LLM 生成的摘要（hourly → daily → weekly → monthly）            │   │
│  │  - 包含 summary, key_activities, git_commits_summary              │   │
│  │  - source_snapshot_ids 連結回原始快照                              │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  Antigravity 額外會建立：                                               │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                    work_items                                     │   │
│  │  - 每個 session 對應一個 work_item                                 │   │
│  │  - 包含 title, description, hours, date 等                        │   │
│  │  - description 包含原始 API 摘要（非 LLM 生成）                    │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Claude Code 資料流程

### 1. 資料來源
- **位置**：`~/.claude/projects/{project-hash}/sessions/*.jsonl`
- **格式**：JSON Lines，每行一個訊息事件
- **內容**：user messages, assistant responses, tool calls, timestamps

### 2. 同步流程

```rust
// 位置：crates/recap-core/src/services/sync.rs
pub async fn sync_claude_sessions(pool: &SqlitePool, user_id: &str) -> Result<SyncResult>
```

1. **發現專案**：掃描 `~/.claude/projects/` 目錄
2. **解析 JSONL**：將每個 session 檔案解析為訊息流
3. **儲存快照**：調用 `save_hourly_snapshots()` 存入 `snapshot_raw_data`
4. **LLM 壓縮**：背景服務調用 `compact_hourly_to_daily()` 生成 `work_summaries`

### 3. 資料儲存

```sql
-- snapshot_raw_data
INSERT INTO snapshot_raw_data (
    id,              -- UUID，快照主鍵
    user_id,         -- 使用者 ID
    session_id,      -- Claude session UUID（來自 JSONL 檔名）
    project_path,    -- 專案路徑
    hour_bucket,     -- 小時桶（如 "2026-01-28T17:00:00"）
    user_messages,   -- JSON array of user messages
    assistant_messages,  -- JSON array of assistant summaries
    tool_calls,      -- JSON array of tool call records
    files_modified,  -- JSON array of file paths
    git_commits,     -- JSON array of commit snapshots
    message_count,   -- 訊息數量
    raw_size_bytes   -- 原始資料大小
)
```

---

## Antigravity 資料流程

### 1. 資料來源
- **API 端點**：`https://localhost:{port}/exa.language_server_pb.LanguageServerService/`
- **API 列表**：
  - `GetAllCascadeTrajectories` - 取得所有 session 列表
  - `GetCascadeTrajectorySteps` - 取得 session 詳細步驟

### 2. 同步流程

```rust
// 位置：crates/recap-core/src/services/sources/antigravity.rs
impl SyncSource for AntigravitySource {
    async fn sync_sessions(&self, pool: &SqlitePool, user_id: &str) -> Result<SourceSyncResult>
}
```

1. **發現連線**：從 `ps aux` 找到 Antigravity 進程，提取 port 和 CSRF token
2. **取得 session 列表**：調用 `GetAllCascadeTrajectories` API
3. **建立 work_items**：每個 session 對應一個 work_item（Phase 1）
4. **取得詳細步驟**：調用 `GetCascadeTrajectorySteps` API
5. **儲存快照**：調用 `save_hourly_snapshots()` 存入 `snapshot_raw_data`（Phase 2）
6. **LLM 壓縮**：背景服務處理後生成 `work_summaries`

### 3. 資料儲存

```sql
-- work_items（Antigravity 特有）
INSERT INTO work_items (
    id,              -- UUID
    user_id,
    source,          -- 'antigravity'
    source_id,       -- Antigravity session ID
    title,           -- "[project_name] {summary}"
    description,     -- 原始 API 摘要（包含 📋 Summary:, 🌿 Branch: 等欄位）
    hours,           -- 從 timestamps 計算的工時
    date,            -- 工作日期
    project_path,
    session_id,      -- Antigravity session ID
    start_time,      -- session 開始時間
    end_time         -- session 結束時間
)

-- snapshot_raw_data（與 Claude Code 共用）
INSERT INTO snapshot_raw_data (
    session_id,      -- Antigravity session ID（現為 UUID 格式）
    -- ... 其他欄位同 Claude Code
)
```

---

## 查詢資料時的來源判斷

### 小時明細查詢 (`get_hourly_breakdown`)

```rust
// 位置：src-tauri/src/commands/snapshots.rs

// 1. 優先查詢 work_summaries（LLM 生成的摘要）
let summaries = query("SELECT * FROM work_summaries WHERE project_path = ? AND scale = 'hourly' ...");

// 2. 如果沒有 summaries，fallback 到 snapshot_raw_data
let snapshots = query("SELECT * FROM snapshot_raw_data WHERE project_path = ? ...");

// 3. 查詢 Antigravity work_items 以判斷來源
let antigravity_items = query("SELECT * FROM work_items WHERE source = 'antigravity' AND date = ? ...");

// 4. 如果某小時的 summary 來自 Antigravity session，標記 source = 'antigravity'
// 5. 只有在沒有 LLM 摘要時，才使用 work_items.description 作為 fallback
```

### 來源識別邏輯

| 情況 | source 值 | 摘要來源 |
|------|-----------|----------|
| 有 work_summaries 且來自 Claude Code snapshot | `claude_code` | LLM 生成 |
| 有 work_summaries 且來自 Antigravity snapshot | `antigravity` | LLM 生成 |
| 沒有 work_summaries，使用 snapshot_raw_data | `claude_code` | 原始 user_messages |
| 沒有 snapshot，使用 Antigravity work_item | `antigravity` | API description |

---

## Session ID 格式歷史

### 舊格式（2026-01-09 之前）
- **Claude Code**：UUID（如 `19a4ae5a-c6c6-41fd-9154-72378e94eb63`）
- **Antigravity**：`agent-*` 格式（如 `agent-a8e7a53`）

### 新格式（2026-01-09 之後）
- **Claude Code**：UUID（不變）
- **Antigravity**：UUID（如 `fe4dd10f-ac4f-4684-9cf6-b750fc5b33fc`）

> ⚠️ 不要依賴 session ID 格式判斷來源！請使用 `work_items.source` 欄位。

---

## 常見問題

### Q: 為什麼 Antigravity 的摘要顯示原始輸入而非 LLM 生成？

**原因**：
1. LLM 壓縮尚未執行（需要等待背景服務處理）
2. snapshot_raw_data 沒有正確儲存（檢查 API 時間戳是否有效）
3. 查詢時優先使用了 work_item.description 而非 work_summaries

**檢查步驟**：
```sql
-- 檢查是否有快照
SELECT * FROM snapshot_raw_data WHERE project_path LIKE '%your-project%' ORDER BY hour_bucket DESC LIMIT 5;

-- 檢查是否有 LLM 摘要
SELECT * FROM work_summaries WHERE project_path LIKE '%your-project%' ORDER BY period_start DESC LIMIT 5;

-- 檢查 work_items 來源
SELECT source, session_id, title FROM work_items WHERE project_path LIKE '%your-project%' ORDER BY date DESC LIMIT 5;
```

### Q: 為什麼兩個來源的資料會顯示在同一個時段？

**原因**：同一個專案可能同時使用 Claude Code 和 Antigravity，各自產生不同的 session。

**預期行為**：
- 每個來源的資料應該分別顯示
- 如果同一小時有多個來源，會合併到同一個時段並顯示來源標籤

### Q: Antigravity 連線失敗怎麼辦？

**檢查步驟**：
1. 確認 Antigravity 應用程式正在執行
2. 檢查 `ps aux | grep language_server` 是否有進程
3. 確認 port 和 CSRF token 正確

```bash
# 取得連線資訊
ps aux | grep language_server_macos | head -1

# 測試 API
curl -sk -X POST "https://localhost:{port}/exa.language_server_pb.LanguageServerService/GetAllCascadeTrajectories" \
  -H "Content-Type: application/json" \
  -H "Connect-Protocol-Version: 1" \
  -H "X-Codeium-Csrf-Token: {token}" \
  -d '{}'
```

---

## 相關檔案

| 功能 | 檔案位置 |
|------|----------|
| Claude Code 同步 | `crates/recap-core/src/services/sync.rs` |
| Antigravity 同步 | `crates/recap-core/src/services/sources/antigravity.rs` |
| 快照儲存 | `crates/recap-core/src/services/snapshot.rs` |
| LLM 壓縮 | `crates/recap-core/src/services/compaction.rs` |
| 小時明細查詢 | `src-tauri/src/commands/snapshots.rs` |
| 工作日誌查詢 | `src-tauri/src/commands/snapshots.rs` (`get_worklog_overview`) |
