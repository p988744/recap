---
description: 檢查職責邊界，確保沒有越界修改其他模組
allowed-tools: Bash(git:*)
---

## 任務：檢查職責邊界

當前分支：
!`git branch --show-current`

修改的檔案：
!`git diff origin/develop --name-only 2>/dev/null`

### 職責邊界規則

| 角色 | 分支關鍵字 | 可修改 | 禁止修改 |
|------|-----------|--------|----------|
| Core 開發者 | `core` | `crates/recap-core/` | `src-tauri/`, `web/src/`, `crates/recap-cli/` |
| Desktop 開發者 | `desktop` | `web/src-tauri/`, `web/src/` | `crates/recap-core/`, `crates/recap-cli/` |
| CLI 開發者 | `cli` | `crates/recap-cli/` | `crates/recap-core/`, `src-tauri/`, `web/src/` |

### 請檢查

1. 根據分支名稱判斷當前角色
2. 檢查所有修改的檔案是否在允許範圍內
3. 如有越界修改，明確指出哪些檔案違規
4. 提供修正建議（開 Issue 給負責的開發者）
