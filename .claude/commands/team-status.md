---
description: 顯示團隊開發狀態，包含所有分支進度和 PR 狀態
allowed-tools: Bash(git:*), Bash(gh:*)
---

## 任務：顯示團隊開發狀態

### Worktrees

!`git worktree list 2>/dev/null || echo "無 worktree"`

### 分支狀態

**main 分支：**
!`git log main -1 --format="%h %s (%ar)" 2>/dev/null || echo "N/A"`

**develop 分支：**
!`git log develop -1 --format="%h %s (%ar)" 2>/dev/null || echo "N/A"`

develop 領先 main：
!`git rev-list main..develop --count 2>/dev/null || echo "0"` commits

### Feature 分支進度

**refactor/core-v2：**
!`git log develop..refactor/core-v2 --oneline 2>/dev/null || echo "無此分支或無新 commits"`

**refactor/desktop-v2：**
!`git log develop..refactor/desktop-v2 --oneline 2>/dev/null || echo "無此分支或無新 commits"`

**refactor/cli-v2：**
!`git log develop..refactor/cli-v2 --oneline 2>/dev/null || echo "無此分支或無新 commits"`

### GitHub PRs

!`gh pr list --state all --limit 10 2>/dev/null || echo "無法取得 PR 列表"`

### 請提供

1. 各分支的進度摘要
2. 是否有分支需要 rebase develop
3. 待處理的 PR
4. 建議的下一步行動
