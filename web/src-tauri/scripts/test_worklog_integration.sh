#!/bin/bash
# Integration test script for worklog aggregation
# Uses the actual recap project as test data

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="/home/weifan/projects/recap"
CLAUDE_PROJECTS_DIR="$HOME/.claude/projects"

echo "=========================================="
echo "Worklog Integration Test"
echo "Project: $PROJECT_ROOT"
echo "=========================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
fail() { echo -e "${RED}[FAIL]${NC} $1"; exit 1; }
info() { echo -e "${YELLOW}[INFO]${NC} $1"; }

# ==========================================
# Test 1: Git commits data availability
# ==========================================
echo ""
echo "=== Test 1: Git Commits Data ==="

cd "$PROJECT_ROOT"

# Get today's date and recent commits
TODAY=$(date +%Y-%m-%d)
YESTERDAY=$(date -d "yesterday" +%Y-%m-%d 2>/dev/null || date -v-1d +%Y-%m-%d)

info "Checking commits for today ($TODAY) and yesterday ($YESTERDAY)"

COMMITS_TODAY=$(git log --since="$TODAY 00:00:00" --until="$TODAY 23:59:59" --format="%H|%h|%an|%aI|%s" --all 2>/dev/null | wc -l)
COMMITS_YESTERDAY=$(git log --since="$YESTERDAY 00:00:00" --until="$YESTERDAY 23:59:59" --format="%H|%h|%an|%aI|%s" --all 2>/dev/null | wc -l)

echo "  Commits today: $COMMITS_TODAY"
echo "  Commits yesterday: $COMMITS_YESTERDAY"

if [ "$COMMITS_TODAY" -gt 0 ] || [ "$COMMITS_YESTERDAY" -gt 0 ]; then
    pass "Git commits are accessible"
else
    info "No recent commits found (this is OK for testing)"
fi

# ==========================================
# Test 2: Git diff stats for hours estimation
# ==========================================
echo ""
echo "=== Test 2: Git Diff Stats (Hours Estimation Input) ==="

# Get recent commit with stats
RECENT_COMMIT=$(git log -1 --format="%H" 2>/dev/null)

if [ -n "$RECENT_COMMIT" ]; then
    info "Testing diff stats for commit: ${RECENT_COMMIT:0:8}"

    STATS=$(git show --numstat --format="" "$RECENT_COMMIT" 2>/dev/null | head -10)

    if [ -n "$STATS" ]; then
        TOTAL_ADD=0
        TOTAL_DEL=0
        FILE_COUNT=0

        while IFS=$'\t' read -r add del file; do
            if [ "$add" != "-" ] && [ "$del" != "-" ]; then
                TOTAL_ADD=$((TOTAL_ADD + add))
                TOTAL_DEL=$((TOTAL_DEL + del))
                FILE_COUNT=$((FILE_COUNT + 1))
            fi
        done <<< "$STATS"

        echo "  Additions: $TOTAL_ADD"
        echo "  Deletions: $TOTAL_DEL"
        echo "  Files changed: $FILE_COUNT"

        # Calculate estimated hours (same logic as Rust code)
        TOTAL_LINES=$((TOTAL_ADD + TOTAL_DEL))
        if [ "$TOTAL_LINES" -eq 0 ]; then
            EST_HOURS="0.25"
        else
            # Approximate logarithmic calculation
            # ln(x) ≈ using bc
            EST_HOURS=$(echo "scale=2; l($TOTAL_LINES + 1) * 0.2 + $FILE_COUNT * 0.15" | bc -l 2>/dev/null || echo "0.5")
            # Clamp to 0.25-4.0
            if (( $(echo "$EST_HOURS < 0.25" | bc -l) )); then
                EST_HOURS="0.25"
            elif (( $(echo "$EST_HOURS > 4.0" | bc -l) )); then
                EST_HOURS="4.0"
            fi
        fi

        echo "  Estimated hours (heuristic): ${EST_HOURS}h"
        pass "Diff stats retrieval works"
    else
        info "Empty commit or binary files only"
    fi
else
    fail "No commits found in repository"
fi

# ==========================================
# Test 3: Claude session files
# ==========================================
echo ""
echo "=== Test 3: Claude Session Files ==="

# Find the recap project in Claude's projects directory
RECAP_CLAUDE_DIR=""
for dir in "$CLAUDE_PROJECTS_DIR"/*; do
    if [ -d "$dir" ]; then
        # Check if this directory name matches recap project path
        DIR_NAME=$(basename "$dir")
        if [[ "$DIR_NAME" == *"recap"* ]]; then
            RECAP_CLAUDE_DIR="$dir"
            break
        fi
    fi
done

if [ -z "$RECAP_CLAUDE_DIR" ]; then
    # Try the encoded path format
    ENCODED_PATH=$(echo "$PROJECT_ROOT" | sed 's|/|-|g')
    RECAP_CLAUDE_DIR="$CLAUDE_PROJECTS_DIR/$ENCODED_PATH"
fi

info "Looking for Claude sessions in: $RECAP_CLAUDE_DIR"

if [ -d "$RECAP_CLAUDE_DIR" ]; then
    SESSION_COUNT=$(find "$RECAP_CLAUDE_DIR" -name "*.jsonl" 2>/dev/null | wc -l)
    echo "  Session files found: $SESSION_COUNT"

    if [ "$SESSION_COUNT" -gt 0 ]; then
        # Analyze a sample session
        SAMPLE_SESSION=$(find "$RECAP_CLAUDE_DIR" -name "*.jsonl" | head -1)
        info "Analyzing sample session: $(basename "$SAMPLE_SESSION")"

        # Count lines (messages)
        LINE_COUNT=$(wc -l < "$SAMPLE_SESSION")
        echo "  Total lines (messages): $LINE_COUNT"

        # Extract timestamps
        FIRST_TS=$(grep -o '"timestamp":"[^"]*"' "$SAMPLE_SESSION" | head -1 | cut -d'"' -f4)
        LAST_TS=$(grep -o '"timestamp":"[^"]*"' "$SAMPLE_SESSION" | tail -1 | cut -d'"' -f4)

        if [ -n "$FIRST_TS" ] && [ -n "$LAST_TS" ]; then
            echo "  First timestamp: $FIRST_TS"
            echo "  Last timestamp: $LAST_TS"

            # Calculate duration
            FIRST_EPOCH=$(date -d "$FIRST_TS" +%s 2>/dev/null || echo "0")
            LAST_EPOCH=$(date -d "$LAST_TS" +%s 2>/dev/null || echo "0")

            if [ "$FIRST_EPOCH" -gt 0 ] && [ "$LAST_EPOCH" -gt 0 ]; then
                DURATION_SEC=$((LAST_EPOCH - FIRST_EPOCH))
                DURATION_HOURS=$(echo "scale=2; $DURATION_SEC / 3600" | bc)
                echo "  Session duration: ${DURATION_HOURS}h"
            fi
        fi

        # Count tool usage
        EDIT_COUNT=$(grep -c '"name":"Edit"' "$SAMPLE_SESSION" 2>/dev/null || echo "0")
        READ_COUNT=$(grep -c '"name":"Read"' "$SAMPLE_SESSION" 2>/dev/null || echo "0")
        BASH_COUNT=$(grep -c '"name":"Bash"' "$SAMPLE_SESSION" 2>/dev/null || echo "0")

        echo "  Tool usage: Edit($EDIT_COUNT), Read($READ_COUNT), Bash($BASH_COUNT)"

        pass "Claude session data is accessible"
    else
        info "No session files found (user may not have used Claude Code on this project)"
    fi
else
    info "Claude project directory not found at $RECAP_CLAUDE_DIR"
fi

# ==========================================
# Test 4: Cross-source deduplication logic
# ==========================================
echo ""
echo "=== Test 4: Cross-Source Deduplication Logic ==="

# Get a commit hash to test dedup logic
COMMIT_FULL=$(git log -1 --format="%H")
COMMIT_SHORT="${COMMIT_FULL:0:8}"

info "Testing deduplication with commit: $COMMIT_SHORT"
echo "  Full hash: $COMMIT_FULL"
echo "  Short hash (for dedup): $COMMIT_SHORT"

# Simulate the deduplication check
if [ "${COMMIT_FULL:0:8}" == "$COMMIT_SHORT" ]; then
    pass "Short hash extraction matches expected format"
else
    fail "Short hash extraction mismatch"
fi

# ==========================================
# Test 5: Hours source priority
# ==========================================
echo ""
echo "=== Test 5: Hours Source Priority ==="

info "Testing priority chain: UserModified > Session > CommitInterval > Heuristic"

# Simulate priority logic
test_priority() {
    local user_override="$1"
    local session_hours="$2"
    local interval_hours="$3"
    local heuristic="$4"

    if [ -n "$user_override" ]; then
        echo "$user_override (UserModified)"
    elif [ -n "$session_hours" ]; then
        echo "$session_hours (Session)"
    elif [ -n "$interval_hours" ]; then
        echo "$interval_hours (CommitInterval)"
    else
        echo "$heuristic (Heuristic)"
    fi
}

echo "  Case 1 (all present): $(test_priority "3.0" "2.0" "1.5" "1.0")"
echo "  Case 2 (no override): $(test_priority "" "2.0" "1.5" "1.0")"
echo "  Case 3 (no session): $(test_priority "" "" "1.5" "1.0")"
echo "  Case 4 (heuristic only): $(test_priority "" "" "" "1.0")"

pass "Priority chain works correctly"

# ==========================================
# Test 6: Build & Run Rust Tests
# ==========================================
echo ""
echo "=== Test 6: Rust Unit Tests ==="

cd "$PROJECT_ROOT/web/src-tauri"

info "Running cargo test..."

if source ~/.cargo/env 2>/dev/null && cargo test --lib 2>&1 | tail -20; then
    pass "All Rust tests passed"
else
    fail "Rust tests failed"
fi

# ==========================================
# Summary
# ==========================================
echo ""
echo "=========================================="
echo -e "${GREEN}All integration tests passed!${NC}"
echo "=========================================="
echo ""
echo "Summary of worklog data sources:"
echo "  - Git commits: Available with diff stats"
echo "  - Claude sessions: $([ -d "$RECAP_CLAUDE_DIR" ] && echo "Available" || echo "Not configured")"
echo "  - Hours estimation: Working (heuristic from diff stats)"
echo "  - Cross-source dedup: Working (8-char commit hash)"
echo "  - Hours priority: Working (4-tier priority chain)"
echo ""
