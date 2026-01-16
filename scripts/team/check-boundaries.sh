#!/bin/bash
# 檢查職責邊界
# 使用方式: ./scripts/team/check-boundaries.sh [--quiet]

QUIET=false
if [ "$1" == "--quiet" ]; then
    QUIET=true
fi

# 取得當前分支
CURRENT_BRANCH=$(git branch --show-current)

# 判斷角色
ROLE=""
case "$CURRENT_BRANCH" in
    *core*)
        ROLE="core"
        ALLOWED_PATHS="crates/recap-core/"
        FORBIDDEN_PATHS="web/src-tauri/|web/src/|crates/recap-cli/"
        ;;
    *desktop*)
        ROLE="desktop"
        ALLOWED_PATHS="web/src-tauri/|web/src/"
        FORBIDDEN_PATHS="crates/recap-core/|crates/recap-cli/"
        ;;
    *cli*)
        ROLE="cli"
        ALLOWED_PATHS="crates/recap-cli/"
        FORBIDDEN_PATHS="crates/recap-core/|web/src-tauri/|web/src/"
        ;;
    *)
        if [ "$QUIET" == "false" ]; then
            echo "⚠️  無法判斷角色 (分支: $CURRENT_BRANCH)"
            echo "   分支名稱應包含 'core', 'desktop', 或 'cli'"
        fi
        exit 0
        ;;
esac

if [ "$QUIET" == "false" ]; then
    echo "=== 檢查職責邊界 ==="
    echo "分支: $CURRENT_BRANCH"
    echo "角色: $ROLE"
    echo ""
fi

# 取得修改的檔案
CHANGED_FILES=$(git diff origin/develop --name-only 2>/dev/null)

if [ -z "$CHANGED_FILES" ]; then
    if [ "$QUIET" == "false" ]; then
        echo "沒有檔案變更"
    fi
    exit 0
fi

# 檢查是否有越界修改
VIOLATIONS=""
while IFS= read -r file; do
    if echo "$file" | grep -qE "$FORBIDDEN_PATHS"; then
        VIOLATIONS="$VIOLATIONS$file\n"
    fi
done <<< "$CHANGED_FILES"

if [ -n "$VIOLATIONS" ]; then
    if [ "$QUIET" == "false" ]; then
        echo "❌ 發現越界修改:"
        echo -e "$VIOLATIONS" | sed 's/^/   /'
        echo ""
        echo "你的角色 ($ROLE) 不應修改這些檔案"
        echo "如需跨模組修改，請開 Issue 給負責的開發者"
    else
        echo "  ❌ 發現越界修改 (執行 ./scripts/team/check-boundaries.sh 查看詳情)"
    fi
    exit 1
else
    if [ "$QUIET" == "false" ]; then
        echo "✅ 所有修改都在職責範圍內"
    else
        echo "  ✅ 職責邊界檢查通過"
    fi
    exit 0
fi
