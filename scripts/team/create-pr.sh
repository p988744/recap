#!/bin/bash
# 建立 PR
# 使用方式: ./scripts/team/create-pr.sh

set -e

echo "=== 建立 Pull Request ==="
echo ""

# 先執行檢查
echo "執行 PR 提交前檢查..."
echo ""
if ! ./scripts/team/pre-pr-check.sh; then
    echo ""
    echo "請修正問題後再執行此腳本"
    exit 1
fi

echo ""
echo "=========================================="
echo ""

# 取得當前分支
CURRENT_BRANCH=$(git branch --show-current)

# 推送分支
echo "推送分支到 origin..."
git push -u origin "$CURRENT_BRANCH"

echo ""

# 取得相關 Issue
echo "請輸入相關 Issue 編號 (例如: 2)，直接 Enter 跳過:"
read -r ISSUE_NUM

# 建立 PR body
PR_BODY="## Summary
-

## Changed Files
$(git diff origin/develop --name-only | sed 's/^/- /')

## Checklist
- [x] 已 rebase origin/develop
- [x] 只包含自己的 commits
- [x] 沒有修改其他模組的程式碼
- [ ] 測試通過
- [ ] 編譯通過
"

if [ -n "$ISSUE_NUM" ]; then
    PR_BODY="$PR_BODY
## Related Issue
Refs #$ISSUE_NUM"
fi

# 建立 PR
echo ""
echo "請輸入 PR 標題:"
read -r PR_TITLE

if [ -z "$PR_TITLE" ]; then
    PR_TITLE="feat($CURRENT_BRANCH): Update"
fi

echo ""
echo "建立 PR..."
gh pr create --base develop --title "$PR_TITLE" --body "$PR_BODY"

echo ""
echo "✅ PR 建立完成"
