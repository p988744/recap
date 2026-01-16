---
description: PR 提交前完整檢查，確保符合團隊規範
allowed-tools: Bash(git:*), Read
---

## 任務：PR 提交前檢查

請執行以下檢查項目：

### 1. 檢查是否已同步 develop

當前分支落後 develop 的 commits：
!`git rev-list HEAD..origin/develop --count 2>/dev/null || echo "0"`

### 2. 檢查 commits

你的 commits（相對於 develop）：
!`git log origin/develop..HEAD --oneline 2>/dev/null || echo "無法取得"`

### 3. 檢查修改的檔案

修改的檔案清單：
!`git diff origin/develop --name-only 2>/dev/null || echo "無法取得"`

### 4. 檢查職責邊界

當前分支：
!`git branch --show-current`

根據分支名稱判斷角色：
- 包含 `core` → Core 開發者，只能改 `crates/recap-core/`
- 包含 `desktop` → Desktop 開發者，只能改 `web/src-tauri/` 和 `web/src/`
- 包含 `cli` → CLI 開發者，只能改 `crates/recap-cli/`

請檢查修改的檔案是否都在職責範圍內，如有越界修改請警告。

### 5. 提供結論

根據以上檢查結果，告訴用戶：
- ✅ 可以提交 PR 的條件
- ❌ 需要修正的問題
- 建議的下一步行動
