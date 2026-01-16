#!/bin/bash
# 同步 develop 分支
# 使用方式: ./scripts/team/sync-develop.sh

set -e

echo "=== 同步 develop 分支 ==="

# 檢查是否有未提交的變更
if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "錯誤: 有未提交的變更，請先 commit 或 stash"
    git status --short
    exit 1
fi

# 取得最新的 remote
echo "Fetching origin..."
git fetch origin

# 取得當前分支
CURRENT_BRANCH=$(git branch --show-current)
echo "當前分支: $CURRENT_BRANCH"

# Rebase develop
echo "Rebasing on origin/develop..."
if git rebase origin/develop; then
    echo ""
    echo "=== 同步成功 ==="
    echo "你的分支已更新至最新的 develop"
    echo ""
    echo "你的 commits:"
    git log origin/develop..HEAD --oneline
else
    echo ""
    echo "=== Rebase 發生衝突 ==="
    echo "請手動解決衝突後執行:"
    echo "  git add <resolved-files>"
    echo "  git rebase --continue"
    echo ""
    echo "或放棄 rebase:"
    echo "  git rebase --abort"
    exit 1
fi
