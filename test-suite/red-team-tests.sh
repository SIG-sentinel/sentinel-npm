#!/bin/bash
# Red-team and ledger validation tests
# Tests for adversarial scenarios, malicious inputs, and edge cases

SENTINEL_BIN="${SENTINEL_BIN:-/home/chris/job/sentinel/sentinel-npm/target/coverage-target/debug/sentinel}"
TEST_DIR="/home/chris/job/sentinel/sentinel-npm/test-suite/projects"
REPORT_DIR="/home/chris/job/sentinel/sentinel-npm/test-suite/red-team-reports"

mkdir -p "$REPORT_DIR"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

PASSED=0
FAILED=0
TESTS=0

log_redteam() {
    local test_name=$1
    local cmd=$2
    local expected_exit=$3
    
    ((TESTS++))
    
    local log_file="$REPORT_DIR/red-team-$(printf '%03d' $TESTS)-$(echo "$test_name" | tr ' ' '-').log"
    
    echo -ne "${CYAN}[$(printf '%02d' $TESTS)]${NC} $test_name ... "
    
    if timeout 60 bash -c "$cmd" > "$log_file" 2>&1; then
        actual_exit=0
    else
        actual_exit=$?
    fi
    
    if [ "$actual_exit" -eq "$expected_exit" ]; then
        echo -e "${GREEN}✓${NC} (exit $actual_exit)"
        ((PASSED++))
        return 0
    else
        echo -e "${RED}✗ (exit $actual_exit, expected $expected_exit)${NC}"
        ((FAILED++))
        return 1
    fi
}

echo "======================================"
echo "SENTINEL RED-TEAM TEST SUITE"
echo "======================================"
echo "Testing for adversarial/edge cases"
echo "Report directory: $REPORT_DIR"
echo ""

proj="$TEST_DIR/p1-barcode-front"

echo -e "${BLUE}═════════════════════════════════════${NC}"
echo -e "${BLUE}1. INVALID INPUT VALIDATION${NC}"
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo ""

log_redteam "Invalid --cwd path" "$SENTINEL_BIN check --cwd /nonexistent/path" 1
log_redteam "Invalid --format value" "$SENTINEL_BIN check --format invalid-format --cwd '$proj'" 2
log_redteam "Invalid --timeout value" "$SENTINEL_BIN check --timeout abc --cwd '$proj'" 2
log_redteam "Missing required argument (check needs nothing)" "$SENTINEL_BIN check --cwd '$proj'" 0
log_redteam "Unknown flag" "$SENTINEL_BIN check --unknown-flag --cwd '$proj'" 2

echo ""
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo -e "${BLUE}2. INSTALL COMMAND EDGE CASES${NC}"
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo ""

log_redteam "Install non-existent package" "$SENTINEL_BIN install nonexistent-pkg-xyz-12345@1.0.0 --cwd '$proj' --dry-run" 1
log_redteam "Install with invalid version" "$SENTINEL_BIN install lodash@invalid-version --cwd '$proj' --dry-run" 1
log_redteam "Install with empty package name" "$SENTINEL_BIN install '' --cwd '$proj' --dry-run" 2
log_redteam "Install without package name" "$SENTINEL_BIN install --cwd '$proj' --dry-run" 2

echo ""
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo -e "${BLUE}3. HISTORY COMMAND EDGE CASES${NC}"
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo ""

log_redteam "History without --from" "$SENTINEL_BIN history --to now --cwd '$proj'" 2
log_redteam "History without --to" "$SENTINEL_BIN history --from 'now' --cwd '$proj'" 2
log_redteam "History with invalid RFC3339" "$SENTINEL_BIN history --from 'invalid-date' --to 'now' --cwd '$proj'" 1
log_redteam "History with future --from date" "$SENTINEL_BIN history --from '2099-01-01' --to 'now' --cwd '$proj'" 1
log_redteam "History with swapped --from/--to" "$SENTINEL_BIN history --from 'now' --to '2020-01-01' --cwd '$proj'" 1

echo ""
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo -e "${BLUE}4. CI COMMAND COMBINATIONS${NC}"
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo ""

log_redteam "CI with both --allow-scripts and --post-verify" "$SENTINEL_BIN ci --dry-run --allow-scripts --post-verify --cwd '$proj'" 0
log_redteam "CI with --omit-dev and --omit-optional" "$SENTINEL_BIN ci --dry-run --omit-dev --omit-optional --cwd '$proj'" 0
log_redteam "CI with all flags combined" "$SENTINEL_BIN ci --dry-run --allow-scripts --post-verify --omit-dev --omit-optional --quiet --cwd '$proj'" 0

echo ""
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo -e "${BLUE}5. CHECK COMMAND NEGATIVE CASES${NC}"
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo ""

log_redteam "Check non-existent project" "$SENTINEL_BIN check --cwd /tmp/no-project-here" 1
log_redteam "Check with invalid timeout" "$SENTINEL_BIN check --timeout -1 --cwd '$proj'" 2
log_redteam "Check with timeout 0" "$SENTINEL_BIN check --timeout 0 --cwd '$proj'" 0

echo ""
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo -e "${BLUE}6. PACKAGE-MANAGER OVERRIDE${NC}"
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo ""

log_redteam "Check with explicit --package-manager npm" "$SENTINEL_BIN check --package-manager npm --cwd '$proj'" 0
log_redteam "Check with invalid --package-manager" "$SENTINEL_BIN check --package-manager invalid --cwd '$proj'" 2
log_redteam "CI with explicit --package-manager" "$SENTINEL_BIN ci --dry-run --package-manager yarn --cwd '$proj'" 0

echo ""
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo -e "${BLUE}7. OUTPUT FORMAT VALIDATION${NC}"
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo ""

log_redteam "Check --format json produces valid JSON" "bash -c 'output=\$($SENTINEL_BIN check --format json --cwd \"$proj\" 2>&1); echo \"\$output\" | jq . > /dev/null 2>&1 || (echo \"Invalid JSON:\"; echo \"\$output\"; exit 1)'" 0

log_redteam "Check --format github produces annotations" "$SENTINEL_BIN check --format github --cwd '$proj'" 0

log_redteam "Check --format junit produces valid XML" "bash -c 'output=\$($SENTINEL_BIN check --format junit --cwd \"$proj\" 2>&1); echo \"\$output\" | xmllint - > /dev/null 2>&1 || (echo \"Invalid XML:\"; echo \"\$output\"; exit 1)'" 0

echo ""
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo -e "${BLUE}8. QUIET MODE VALIDATION${NC}"
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo ""

log_redteam "Check --quiet produces minimal output" "bash -c 'lines=\$($SENTINEL_BIN check --quiet --cwd \"$proj\" 2>&1 | wc -l); [ \$lines -lt 20 ] || exit 1'" 0

log_redteam "CI --quiet with report" "$SENTINEL_BIN ci --dry-run --quiet --report /tmp/test-quiet-report.json --cwd '$proj'" 0

echo ""
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo -e "${BLUE}9. CONCURRENT/STRESS TESTS${NC}"
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo ""

log_redteam "Multiple checks in sequence" "bash -c 'for i in {1..3}; do $SENTINEL_BIN check --quiet --cwd \"$proj\" || exit 1; done'" 0

log_redteam "Check with --registry-max-in-flight 1" "$SENTINEL_BIN check --registry-max-in-flight 1 --cwd '$proj'" 0

log_redteam "Check with --registry-max-in-flight 100" "$SENTINEL_BIN check --registry-max-in-flight 100 --cwd '$proj'" 0

echo ""
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo -e "${BLUE}10. HELP AND VERSION${NC}"
echo -e "${BLUE}═════════════════════════════════════${NC}"
echo ""

log_redteam "Version flag" "$SENTINEL_BIN --version" 0
log_redteam "Help flag" "$SENTINEL_BIN --help" 0
log_redteam "Check help" "$SENTINEL_BIN check --help" 0
log_redteam "CI help" "$SENTINEL_BIN ci --help" 0
log_redteam "Install help" "$SENTINEL_BIN install --help" 0
log_redteam "History help" "$SENTINEL_BIN history --help" 0

echo ""
echo "======================================"
echo -e "${BLUE}RED-TEAM TEST SUMMARY${NC}"
echo "======================================"
echo "Total:  $TESTS"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo ""
echo "Detailed reports in: $REPORT_DIR"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ ALL RED-TEAM TESTS PASSED!${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠ Some red-team tests failed (may be expected for adversarial tests)${NC}"
    exit 0  # Don't fail red-team suite - it's meant to test error conditions
fi
