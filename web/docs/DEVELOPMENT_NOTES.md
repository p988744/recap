# Development Notes - Git Commit 顯示問題排查記錄

## 問題摘要

在 ThisWeek 頁面的甘特圖中，git commit 資訊無法正確顯示。

## 排查過程與解決方案

### 問題 1：hour_bucket 時間格式不一致

**現象**：`enrich_buckets_with_git_commits` 無法解析時間範圍

**根因**：
- `hour_bucket` 有兩種格式：
  - RFC3339 帶時區：`2026-01-30T10:00:00+08:00`
  - NaiveDateTime 不帶時區：`2026-01-30T10:00:00`
- 原本只處理 RFC3339，遇到 NaiveDateTime 會 `continue` 跳過

**修復**：
```rust
// snapshot.rs - enrich_buckets_with_git_commits
let (start_str, end_str) = match DateTime::parse_from_rfc3339(start) {
    Ok(dt) => { /* RFC3339 處理 */ }
    Err(_) => {
        // Fallback: 解析為 NaiveDateTime 並轉換為本地時區
        match NaiveDateTime::parse_from_str(start, "%Y-%m-%dT%H:%M:%S") {
            Ok(ndt) => {
                let local_start = Local.from_local_datetime(&ndt).single();
                // ...
            }
            Err(_) => continue,
        }
    }
};
```

---

### 問題 2：project_path 與 git 目錄不一致

**現象**：`get_commits_in_time_range` 找不到 commits

**根因**：
- `project_path` 儲存的是專案目錄（如 `/Users/.../recap/web`）
- 但 `.git` 可能在父目錄（如 `/Users/.../recap`）
- 直接用 project_path 執行 `git log` 會失敗

**修復**：
```rust
// 使用 resolve_git_root 找到真正的 git 目錄
use super::sync::resolve_git_root;
let git_root = resolve_git_root(project_path);
let commits = get_commits_in_time_range(&git_root, &start_str, &end_str);
```

---

### 問題 3：work_summaries 的 commits 缺少 timestamp

**現象**：recap 專案有 commit marker，但 elandGpuManagement 沒有

**根因**：
- `work_summaries.git_commits_summary` 格式為字串：`"hash: message (+adds-dels)"`
- 解析時 timestamp 被設為空字串
- 前端 `parseCommitTime("")` 回傳 `null`，導致 marker 不顯示

**修復**：
```rust
// snapshots.rs - get_hourly_breakdown
// 先從 snapshot_raw_data 建立 hash -> timestamp 對照表
let commit_timestamps: HashMap<String, String> = /* 從 snapshot 查詢 */;

// 解析 summary 時補充 timestamp
let timestamp = commit_timestamps.get(&hash).cloned().unwrap_or_default();
```

---

### 問題 4：work_summaries 的 git_commits_summary 完全為空

**現象**：某些專案的 summary 有資料，但 commits 為空

**根因**：
- Summary 可能在 commit 捕獲前就生成
- 或 LLM 摘要沒有包含所有 commits

**修復**：
```rust
// 當 summary commits 為空時，從 snapshot_raw_data 取得
if commits.is_empty() {
    if let Some(snapshot_commits) = commits_by_hour.get(&hour_start) {
        commits = snapshot_commits.clone();
    }
}
```

---

## 常用除錯流程

### 1. 檢查資料庫內容

```bash
# 檢查 snapshot_raw_data 的 git_commits
sqlite3 ~/Library/Application\ Support/com.recap.Recap/recap.db \
  "SELECT hour_bucket, git_commits FROM snapshot_raw_data
   WHERE project_path LIKE '%projectName%' AND hour_bucket >= '2026-01-30'"

# 檢查 work_summaries 的 git_commits_summary
sqlite3 ~/Library/Application\ Support/com.recap.Recap/recap.db \
  "SELECT period_start, git_commits_summary FROM work_summaries
   WHERE scale = 'hourly' AND project_path LIKE '%projectName%'"

# 檢查有 commits 的記錄數量
sqlite3 ~/Library/Application\ Support/com.recap.Recap/recap.db \
  "SELECT project_path, hour_bucket, json_array_length(git_commits)
   FROM snapshot_raw_data
   WHERE json_array_length(git_commits) > 0"
```

### 2. 重新觸發資料更新

```bash
# 清除特定日期的 summaries（讓系統重新生成）
sqlite3 ~/Library/Application\ Support/com.recap.Recap/recap.db \
  "DELETE FROM work_summaries
   WHERE scale = 'hourly' AND period_start >= '2026-01-30'"

# 清除 snapshot 的 git_commits（讓系統重新捕獲）
sqlite3 ~/Library/Application\ Support/com.recap.Recap/recap.db \
  "UPDATE snapshot_raw_data SET git_commits = '[]'
   WHERE hour_bucket >= '2026-01-30'"
```

### 3. 驗證 git log 命令

```bash
# 測試時間範圍內是否有 commits
cd /path/to/project
git log --since="2026-01-30T09:00:00+08:00" \
        --until="2026-01-30T10:00:00+08:00" \
        --format="%H|%s|%aI"
```

### 4. 寫測試先驗證函數

```rust
#[test]
fn test_enrich_buckets_with_git_commits() {
    let crate_path = env!("CARGO_MANIFEST_DIR");
    let mut buckets = vec![HourlyBucket {
        hour_bucket: "2026-01-30T09:00:00".to_string(),
        git_commits: vec![],
        // ...
    }];
    enrich_buckets_with_git_commits(&mut buckets, crate_path);
    assert!(!buckets[0].git_commits.is_empty());
}
```

---

## 資料流程圖

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│  Claude Code     │     │  snapshot.rs     │     │ snapshot_raw_data│
│  Session Files   │────▶│  capture +       │────▶│  (git_commits    │
│  (.jsonl)        │     │  enrich_commits  │     │   完整 JSON)     │
└──────────────────┘     └──────────────────┘     └────────┬─────────┘
                                                           │
                                                           ▼
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│  Frontend        │     │  snapshots.rs    │     │  work_summaries  │
│  DayGanttChart   │◀────│  get_hourly_     │◀────│  (git_commits_   │
│  (顯示 markers)  │     │  breakdown       │     │   summary 字串)  │
└──────────────────┘     └──────────────────┘     └──────────────────┘
                                │
                                │ 🆕 fallback: 當 summary
                                │    commits 為空時從
                                │    snapshot_raw_data 取得
                                ▼
                         ┌──────────────────┐
                         │  snapshot_raw_   │
                         │  data (補充      │
                         │  timestamp)      │
                         └──────────────────┘
```

---

## 關鍵函數位置

| 函數 | 檔案 | 用途 |
|------|------|------|
| `enrich_buckets_with_git_commits` | `crates/recap-core/src/services/snapshot.rs` | 捕獲時添加 git commits |
| `resolve_git_root` | `crates/recap-core/src/services/sync.rs` | 找到真正的 .git 目錄 |
| `get_commits_in_time_range` | `crates/recap-core/src/services/worklog.rs` | 執行 git log 取得 commits |
| `get_hourly_breakdown` | `src-tauri/src/commands/snapshots.rs` | API：回傳小時明細 |
| `DayGanttChart` | `src/pages/ThisWeek/components/DayGanttChart.tsx` | 前端甘特圖顯示 |

---

## 學到的經驗

1. **時間格式要處理多種情況**：RFC3339 和 NaiveDateTime 都要支援
2. **路徑不等於 git root**：專案路徑可能是 git repo 的子目錄
3. **資料有多個來源時要有 fallback**：summary 沒資料就回 snapshot
4. **先寫測試再修 bug**：確保函數獨立運作正確
5. **用資料庫直接查詢驗證**：比看 log 更快找到問題
