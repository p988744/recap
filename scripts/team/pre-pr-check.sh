#!/bin/bash
# PR 提交前檢查
# 使用方式: ./scripts/team/pre-pr-check.sh

set -e

echo "=== PR 提交前檢查 ==="
echo ""

ERRORS=0

# 1. 檢查是否已同步 develop
echo "[1/5] 檢查是否已同步 develop..."
BEHIND=$(git rev-list HEAD..origin/develop --count 2>/dev/null || echo "0")
if [ "$BEHIND" != "0" ]; then
    echo "  ❌ 落後 origin/develop $BEHIND 個 commits"
    echo "     請執行: ./scripts/team/sync-develop.sh"
    ERRORS=$((ERRORS + 1))
else
    echo "  ✅ 已同步 develop"
fi

# 2. 檢查 commits
echo ""
echo "[2/5] 檢查 commits..."
COMMITS=$(git log origin/develop..HEAD --oneline)
if [ -z "$COMMITS" ]; then
    echo "  ⚠️  沒有新的 commits"
else
    echo "  ✅ 你的 commits:"
    echo "$COMMITS" | sed 's/^/     /'
fi

# 3. 檢查修改的檔案
echo ""
echo "[3/5] 檢查修改的檔案..."
CHANGED_FILES=$(git diff origin/develop --name-only)
if [ -z "$CHANGED_FILES" ]; then
    echo "  ⚠️  沒有檔案變更"
else
    echo "  修改的檔案:"
    echo "$CHANGED_FILES" | sed 's/^/     /'
fi

# 4. 檢查職責邊界
echo ""
echo "[4/5] 檢查職責邊界..."
./scripts/team/check-boundaries.sh --quiet || ERRORS=$((ERRORS + 1))

# 5. 測試檢查提示
echo ""
echo "[5/5] 測試檢查 (請手動確認)"
echo "  請確保已執行:"
echo "     cargo test        # Rust 測試"
echo "     npm test          # Frontend 測試"
echo "     cargo build       # Rust 編譯"
echo "     npm run build     # Frontend 編譯"

# 結果
echo ""
echo "=========================================="
if [ $ERRORS -eq 0 ]; then
    echo "✅ 檢查通過，可以提交 PR"
    echo ""
    echo "提交 PR 指令:"
    echo "  git push -u origin $(git branch --show-current)"
    echo "  gh pr create --base develop"
else
    echo "❌ 發現 $ERRORS 個問題，請修正後再提交 PR"
    exit 1
fi
