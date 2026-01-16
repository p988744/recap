---
description: 更新 GitHub Issues 進度，同步最新的分支狀態
allowed-tools: Bash(git:*), Bash(gh:*)
---

## 任務：更新 GitHub Issues 進度

### 收集當前狀態

**各分支 commits：**

Core (refactor/core-v2)：
!`git log main..refactor/core-v2 --oneline 2>/dev/null || echo "無新 commits"`

Desktop (refactor/desktop-v2)：
!`git log main..refactor/desktop-v2 --oneline 2>/dev/null || echo "無新 commits"`

CLI (refactor/cli-v2)：
!`git log main..refactor/cli-v2 --oneline 2>/dev/null || echo "無新 commits"`

**GitHub Issues：**
!`gh issue list --milestone "Desktop Refactoring v2" --state all 2>/dev/null || echo "無法取得"`

**GitHub PRs：**
!`gh pr list --state all 2>/dev/null || echo "無法取得"`

### 請執行

1. 分析各分支的最新進度
2. 比對 GitHub Issues 的狀態
3. 建議需要更新的 Issues：
   - 哪些 Issues 可以關閉
   - 哪些 Issues 需要更新進度
   - 是否需要建立新的 Issues
4. 提供更新 Issue 的具體指令（使用 `gh issue` 命令）
