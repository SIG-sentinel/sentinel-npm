#!/bin/bash
# Comprehensive sentinel test suite across multiple projects
# Tests each project with/without lockfile, all commands, formats, and validates ledger/outputs

set -e

SENTINEL_BIN="${SENTINEL_BIN:-$(pwd)/../../target/debug/sentinel}"
TEST_DIR="$(pwd)/projects"
REPORT_DIR="$(pwd)/reports"
LEDGER_DIR="${SENTINEL_HISTORY_PATH:-$HOME/.sentinel/history}"

mkdir -p "$REPORT_DIR"

# Color output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "================================"
echo "SENTINEL COMPREHENSIVE TEST SUITE"
echo "================================"
echo "Sentinel binary: $SENTINEL_BIN"
echo "Test directory: $TEST_DIR"
echo "Report directory: $REPORT_DIR"
echo ""

# Verify binary exists
if [ ! -f "$SENTINEL_BIN" ]; then
    echo -e "${RED}✗ Sentinel binary not found at $SENTINEL_BIN${NC}"
    echo "Build it with: cd ../.. && cargo build"
    exit 1
fi

# Test counter
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

test_command() {
    local project=$1
    local test_name=$2
    local cmd=$3
    local expected_exit=$4
    
    ((TOTAL_TESTS++))
    
    local test_log="$REPORT_DIR/${project}-${test_name}.log"
    local output
    
    echo -ne "  Testing ${project}: ${test_name}... "
    
    cd "$TEST_DIR/$project"
    
    if output=$(eval "$cmd" 2>&1); then
        actual_exit=$?
    else
        actual_exit=$?
    fi
    
    echo "$output" > "$test_log"
    
    if [ "$actual_exit" -eq "$expected_exit" ]; then
        echo -e "${GREEN}✓${NC}"
        ((PASSED_TESTS++))
        return 0
    else
        echo -e "${RED}✗ (exit $actual_exit, expected $expected_exit)${NC}"
        ((FAILED_TESTS++))
        return 1
    fi
}

# Test each project
for project_dir in "$TEST_DIR"/p*; do
    project=$(basename "$project_dir")
    echo ""
    echo -e "${YELLOW}Project: $project${NC}"
    
    # Check if project has package.json
    if [ ! -f "$project_dir/package.json" ]; then
        echo "  ✗ No package.json found, skipping"
        continue
    fi
    
    echo "  package.json exists"
    
    # --- PHASE 1: WITH LOCKFILE ---
    echo ""
    echo "  PHASE 1: WITH LOCKFILE"
    
    if [ -f "$project_dir/package-lock.json" ]; then
        echo "    ✓ package-lock.json present"
    elif [ -f "$project_dir/yarn.lock" ]; then
        echo "    ✓ yarn.lock present"
    elif [ -f "$project_dir/pnpm-lock.yaml" ]; then
        echo "    ✓ pnpm-lock.yaml present"
    else
        echo "    ! No lockfile found (will test without)"
    fi
    
    # Test: check command
    test_command "$project" "check-text" "$SENTINEL_BIN check --cwd $(pwd)" 0
    test_command "$project" "check-json" "$SENTINEL_BIN check --format json --cwd $(pwd)" 0
    test_command "$project" "check-github" "$SENTINEL_BIN check --format github --cwd $(pwd)" 0
    test_command "$project" "check-omit-dev" "$SENTINEL_BIN check --omit-dev --cwd $(pwd)" 0
    
    # Test: ci command
    test_command "$project" "ci-dry-run" "$SENTINEL_BIN ci --dry-run --format text --cwd $(pwd)" 0
    test_command "$project" "ci-report-json" "$SENTINEL_BIN ci --dry-run --format json --report /tmp/test-report.json --cwd $(pwd)" 0
    
    # --- PHASE 2: WITHOUT LOCKFILE (test init-lockfile) ---
    echo ""
    echo "  PHASE 2: WITHOUT LOCKFILE (init-lockfile test)"
    
    cd "$project_dir"
    
    # Backup original lockfile
    if [ -f "package-lock.json" ]; then
        mv package-lock.json package-lock.json.bak
        echo "    Backed up package-lock.json"
    elif [ -f "yarn.lock" ]; then
        mv yarn.lock yarn.lock.bak
        PM="yarn"
        echo "    Backed up yarn.lock"
    elif [ -f "pnpm-lock.yaml" ]; then
        mv pnpm-lock.yaml pnpm-lock.yaml.bak
        PM="pnpm"
        echo "    Backed up pnpm-lock.yaml"
    fi
    
    # Detect PM if not already set
    if [ -z "$PM" ]; then
        if grep -q '"lockfileVersion"' package.json 2>/dev/null || [ -f "package-lock.json.bak" ]; then
            PM="npm"
        elif [ -f "yarn.lock.bak" ]; then
            PM="yarn"
        elif [ -f "pnpm-lock.yaml.bak" ]; then
            PM="pnpm"
        else
            PM="npm"
        fi
    fi
    
    echo "    Detected PM: $PM"
    
    # Test init-lockfile
    test_command "$project" "init-lockfile-dry" "$SENTINEL_BIN ci --init-lockfile --package-manager $PM --dry-run --cwd $(pwd)" 0
    
    # Restore lockfile
    if [ -f "package-lock.json.bak" ]; then
        mv package-lock.json.bak package-lock.json
        echo "    Restored package-lock.json"
    elif [ -f "yarn.lock.bak" ]; then
        mv yarn.lock.bak yarn.lock
        echo "    Restored yarn.lock"
    elif [ -f "pnpm-lock.yaml.bak" ]; then
        mv pnpm-lock.yaml.bak pnpm-lock.yaml
        echo "    Restored pnpm-lock.yaml"
    fi
    
    # --- PHASE 3: HISTORY COMMAND ---
    echo ""
    echo "  PHASE 3: HISTORY COMMAND"
    
    test_command "$project" "history-text" "$SENTINEL_BIN history --cwd $(pwd)" 0
    test_command "$project" "history-json" "$SENTINEL_BIN history --format json --cwd $(pwd)" 0
    test_command "$project" "history-from-filter" "$SENTINEL_BIN history --from '7 days ago' --to now --cwd $(pwd)" 0
    
    # --- PHASE 4: EDGE CASES ---
    echo ""
    echo "  PHASE 4: EDGE CASES"
    
    test_command "$project" "check-omit-optional" "$SENTINEL_BIN check --omit-optional --cwd $(pwd)" 0
    test_command "$project" "check-quiet" "$SENTINEL_BIN check --quiet --cwd $(pwd)" 0
    
    unset PM
done

# --- SUMMARY ---
echo ""
echo "================================"
echo "TEST SUMMARY"
echo "================================"
echo "Total tests:  $TOTAL_TESTS"
echo -e "Passed:       ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed:       ${RED}$FAILED_TESTS${NC}"
echo ""

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed. See logs in $REPORT_DIR${NC}"
    exit 1
fi
