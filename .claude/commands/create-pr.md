---
description: 建立 PR，包含自動檢查和標準格式
argument-hint: "[issue-number]"
allowed-tools: Bash(git:*), Bash(gh:*)
---

## 任務：建立 Pull Request

相關 Issue：$ARGUMENTS

### 步驟 1：執行 PR 前檢查

當前分支：
!`git branch --show-current`

是否已同步 develop：
!`git rev-list HEAD..origin/develop --count 2>/dev/null`

修改的檔案：
!`git diff origin/develop --name-only 2>/dev/null`

你的 commits：
!`git log origin/develop..HEAD --oneline 2>/dev/null`

### 步驟 2：檢查職責邊界

根據分支名稱和修改的檔案，確認沒有越界修改。

### 步驟 3：建立 PR

如果檢查通過，請：

1. 推送分支：`git push -u origin <branch-name>`
2. 使用以下格式建立 PR：

```
gh pr create --base develop --title "<type>(<scope>): <description>" --body "## Summary
- 簡述完成的功能

## Changed Files
- 列出修改的檔案

## Checklist
- [x] 已 rebase origin/develop
- [x] 只包含自己的 commits
- [x] 沒有修改其他模組的程式碼
- [ ] 測試通過
- [ ] 編譯通過

## Related Issue
Refs #<issue-number>"
```

### 注意事項

- 如果有任何檢查未通過，請先修正再建立 PR
- PR title 使用 conventional commit 格式
- 確保關聯正確的 Issue
