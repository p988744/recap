#!/bin/bash
# 顯示團隊狀態
# 使用方式: ./scripts/team/status.sh

echo "=== 團隊開發狀態 ==="
echo ""

# 取得所有 worktree
echo "📁 Worktrees:"
git worktree list | while read -r line; do
    echo "   $line"
done

echo ""
echo "🌿 分支進度:"
echo ""

# Main
echo "main:"
MAIN_COMMIT=$(git log main -1 --format="%h %s" 2>/dev/null || echo "N/A")
echo "   $MAIN_COMMIT"

echo ""

# Develop
echo "develop:"
DEVELOP_AHEAD=$(git rev-list main..develop --count 2>/dev/null || echo "0")
DEVELOP_COMMIT=$(git log develop -1 --format="%h %s" 2>/dev/null || echo "N/A")
echo "   $DEVELOP_COMMIT"
echo "   (ahead of main by $DEVELOP_AHEAD commits)"

echo ""

# Feature branches
for branch in refactor/core-v2 refactor/desktop-v2 refactor/cli-v2; do
    if git rev-parse --verify "$branch" >/dev/null 2>&1; then
        echo "$branch:"
        AHEAD=$(git rev-list develop.."$branch" --count 2>/dev/null || echo "0")
        BEHIND=$(git rev-list "$branch"..develop --count 2>/dev/null || echo "0")
        LAST_COMMIT=$(git log "$branch" -1 --format="%h %s (%ar)" 2>/dev/null || echo "N/A")
        echo "   $LAST_COMMIT"
        echo "   (ahead: $AHEAD, behind: $BEHIND)"

        if [ "$BEHIND" != "0" ]; then
            echo "   ⚠️  需要 rebase develop"
        fi
        echo ""
    fi
done

# PRs
echo "📋 Open PRs:"
gh pr list --state open 2>/dev/null | while read -r line; do
    echo "   $line"
done || echo "   (無法取得 PR 列表)"

echo ""
echo "📊 Milestone 進度:"
gh api repos/:owner/:repo/milestones 2>/dev/null | jq -r '.[] | "   \(.title): \(.open_issues) open, \(.closed_issues) closed"' || echo "   (無法取得 Milestone)"
