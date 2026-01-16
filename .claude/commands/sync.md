---
description: 同步 develop 分支，確保本地分支是最新的
allowed-tools: Bash(git:*)
---

## 任務：同步 develop 分支

請執行以下步驟：

1. 檢查是否有未提交的變更
2. 執行 `git fetch origin`
3. 執行 `git rebase origin/develop`
4. 顯示同步結果

當前狀態：
!`git status --short`

當前分支：
!`git branch --show-current`

如果有未提交的變更，提醒用戶先 commit 或 stash。
如果 rebase 有衝突，說明如何解決。
