#!/bin/bash
# Sequential test runner: test each project with all commands one by one

SENTINEL_BIN="${SENTINEL_BIN:-/home/chris/job/sentinel/sentinel-npm/target/coverage-target/debug/sentinel}"
TEST_DIR="/home/chris/job/sentinel/sentinel-npm/test-suite/projects"
REPORT_DIR="/home/chris/job/sentinel/sentinel-npm/test-suite/reports"

mkdir -p "$REPORT_DIR"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PASSED=0
FAILED=0
TESTS=0

log_test() {
    local project=$1
    local cmd=$2
    local desc=$3
    
    ((TESTS++))
    
    echo -ne "${BLUE}[$(printf '%02d' $TESTS)]${NC} ${project} :: $desc ... "
    
    local log_file="$REPORT_DIR/${project}-$(echo "$cmd" | md5sum | cut -d' ' -f1).log"
    
    if timeout 120 bash -c "cd '$TEST_DIR/$project' && $cmd" > "$log_file" 2>&1; then
        echo -e "${GREEN}✓${NC} (log: $(basename $log_file))"
        ((PASSED++))
        return 0
    else
        local exit_code=$?
        if [ $exit_code -eq 124 ]; then
            echo -e "${RED}✗ TIMEOUT${NC}"
        else
            echo -e "${RED}✗ (exit $exit_code)${NC}"
        fi
        ((FAILED++))
        head -20 "$log_file" | sed 's/^/    /'
        return 1
    fi
}

echo "======================================"
echo "SENTINEL SEQUENTIAL PROJECT TEST SUITE"
echo "======================================"
echo "Sentinel: $SENTINEL_BIN"
echo "Projects: $TEST_DIR"
echo "Reports: $REPORT_DIR"
echo ""

if [ ! -f "$SENTINEL_BIN" ]; then
    echo -e "${RED}✗ Sentinel binary not found${NC}"
    exit 1
fi

# Test each project
for proj in "$TEST_DIR"/p*; do
    proj_name=$(basename "$proj")
    
    if [ ! -f "$proj/package.json" ]; then
        echo -e "${YELLOW}Skip${NC} $proj_name (no package.json)"
        continue
    fi
    
    echo ""
    echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}PROJECT: $proj_name${NC}"
    echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    
    # Detect lockfile type
    if [ -f "$proj/package-lock.json" ]; then
        LOCKFILE="package-lock.json"
        PM="npm"
    elif [ -f "$proj/yarn.lock" ]; then
        LOCKFILE="yarn.lock"
        PM="yarn"
    elif [ -f "$proj/pnpm-lock.yaml" ]; then
        LOCKFILE="pnpm-lock.yaml"
        PM="pnpm"
    else
        LOCKFILE="none"
        PM="npm"
    fi
    
    echo "Lockfile: $LOCKFILE (PM: $PM)"
    echo ""
    
    echo -e "${BLUE}PHASE 1: WITH LOCKFILE${NC}"
    
    # Test check command variants
    log_test "$proj_name" "$SENTINEL_BIN check" "check --text"
    log_test "$proj_name" "$SENTINEL_BIN check --format json" "check --json"
    log_test "$proj_name" "$SENTINEL_BIN check --format github" "check --github"
    log_test "$proj_name" "$SENTINEL_BIN check --format junit" "check --junit"
    log_test "$proj_name" "$SENTINEL_BIN check --omit-dev" "check --omit-dev"
    log_test "$proj_name" "$SENTINEL_BIN check --omit-optional" "check --omit-optional"
    log_test "$proj_name" "$SENTINEL_BIN check --quiet" "check --quiet"
    
    echo ""
    echo -e "${BLUE}PHASE 2: CI COMMAND VARIANTS${NC}"
    
    log_test "$proj_name" "$SENTINEL_BIN ci --dry-run" "ci --dry-run"
    log_test "$proj_name" "$SENTINEL_BIN ci --dry-run --format json" "ci --dry-run --json"
    log_test "$proj_name" "$SENTINEL_BIN ci --dry-run --post-verify" "ci --dry-run --post-verify"
    log_test "$proj_name" "$SENTINEL_BIN ci --dry-run --omit-dev" "ci --dry-run --omit-dev"
    log_test "$proj_name" "$SENTINEL_BIN ci --dry-run --quiet" "ci --dry-run --quiet"
    
    echo ""
    echo -e "${BLUE}PHASE 3: WITHOUT LOCKFILE (init-lockfile)${NC}"
    
    if [ "$LOCKFILE" != "none" ]; then
        # Backup lockfile
        cd "$proj"
        cp "$LOCKFILE" "${LOCKFILE}.backup"
        rm "$LOCKFILE"
        cd - > /dev/null
        
        log_test "$proj_name" "$SENTINEL_BIN ci --init-lockfile --package-manager $PM --dry-run" "init-lockfile --dry-run"
        log_test "$proj_name" "$SENTINEL_BIN ci --init-lockfile --package-manager $PM --dry-run --format json" "init-lockfile --dry-run --json"
        
        # Restore lockfile
        cd "$proj"
        mv "${LOCKFILE}.backup" "$LOCKFILE"
        cd - > /dev/null
    fi
    
    echo ""
    echo -e "${BLUE}PHASE 4: HISTORY COMMAND${NC}"
    
    log_test "$proj_name" "$SENTINEL_BIN history" "history --text"
    log_test "$proj_name" "$SENTINEL_BIN history --format json" "history --json"
    log_test "$proj_name" "$SENTINEL_BIN history --from '30 days ago' --to now" "history --from/to filter"
done

echo ""
echo "======================================"
echo -e "${BLUE}TEST SUMMARY${NC}"
echo "======================================"
echo "Total:  $TESTS"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ ALL TESTS PASSED!${NC}"
    exit 0
else
    echo -e "${RED}✗ SOME TESTS FAILED${NC}"
    exit 1
fi
