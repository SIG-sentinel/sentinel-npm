#!/usr/bin/env bash
set -euo pipefail

BIN="${BIN:-$PWD/target/debug/sentinel}"
if [[ ! -x "$BIN" ]]; then
  echo "error: sentinel binary not found at $BIN" >&2
  echo "hint: cargo build" >&2
  exit 1
fi

WORKDIR="${WORKDIR:-/tmp/sentinel-smoke-all-options}"
rm -rf "$WORKDIR"
mkdir -p "$WORKDIR"

cat > "$WORKDIR/package.json" <<'JSON'
{
  "name": "sentinel-smoke-all-options",
  "version": "1.0.0",
  "packageManager": "npm@10.9.3",
  "dependencies": {
    "left-pad": "1.3.0"
  }
}
JSON

(cd "$WORKDIR" && npm install --package-lock-only >/dev/null)

echo "==> validating command help pages"
"$BIN" --help >/tmp/sentinel-help.out
"$BIN" check --help >/tmp/sentinel-check-help.out
"$BIN" ci --help >/tmp/sentinel-ci-help.out
"$BIN" install --help >/tmp/sentinel-install-help.out
"$BIN" history --help >/tmp/sentinel-history-help.out

grep -q -- "--artifact-store" /tmp/sentinel-help.out
grep -q -- "--omit-dev" /tmp/sentinel-check-help.out
grep -q -- "--init-lockfile" /tmp/sentinel-ci-help.out
grep -q -- "--allow-scripts" /tmp/sentinel-install-help.out
grep -q -- "--from" /tmp/sentinel-history-help.out
grep -q -- "--to" /tmp/sentinel-history-help.out

echo "==> check command option coverage"
"$BIN" check --cwd "$WORKDIR" --package-manager npm --format text >/tmp/check-text.out
"$BIN" check --cwd "$WORKDIR" --package-manager npm --format json >/tmp/check-json.out
"$BIN" check --cwd "$WORKDIR" --package-manager npm --format github >/tmp/check-github.out
"$BIN" check --cwd "$WORKDIR" --package-manager npm --format junit >/tmp/check-junit.out
"$BIN" check --cwd "$WORKDIR" --package-manager npm --omit-dev --omit-optional --timeout 7000 -q --format json >/tmp/check-all-flags.out

grep -q '"sentinel_version"' /tmp/check-json.out
grep -q '::' /tmp/check-github.out
grep -q '<testsuites' /tmp/check-junit.out
grep -q '"sentinel_version"' /tmp/check-all-flags.out

echo "==> ci command option coverage"
"$BIN" ci --cwd "$WORKDIR" --package-manager npm --dry-run --format text --report "$WORKDIR/ci-text-report.json" >/tmp/ci-text.out
"$BIN" ci --cwd "$WORKDIR" --package-manager npm --dry-run --format json --report "$WORKDIR/ci-json-report.json" >/tmp/ci-json.out
"$BIN" ci --cwd "$WORKDIR" --package-manager npm --dry-run --format github --report "$WORKDIR/ci-github-report.json" >/tmp/ci-github.out
"$BIN" ci --cwd "$WORKDIR" --package-manager npm --dry-run --format junit --report "$WORKDIR/ci-junit-report.json" >/tmp/ci-junit.out
"$BIN" ci --cwd "$WORKDIR" --package-manager npm --dry-run --omit-dev --omit-optional --allow-scripts --timeout 9000 -q --format json --report "$WORKDIR/ci-all-flags-report.json" >/tmp/ci-all-flags.out

test -f "$WORKDIR/ci-text-report.json"
test -f "$WORKDIR/ci-json-report.json"
test -f "$WORKDIR/ci-github-report.json"
test -f "$WORKDIR/ci-junit-report.json"
test -f "$WORKDIR/ci-all-flags-report.json"
grep -q '"sentinel_version"' /tmp/ci-json.out
grep -q '::' /tmp/ci-github.out
grep -q '<testsuites' /tmp/ci-junit.out

echo "==> ci init-lockfile option coverage"
INITDIR="$WORKDIR/init-lockfile"
mkdir -p "$INITDIR"
cat > "$INITDIR/package.json" <<'JSON'
{
  "name": "sentinel-smoke-init-lockfile",
  "version": "1.0.0",
  "packageManager": "npm@10.9.3",
  "dependencies": {}
}
JSON
"$BIN" ci --cwd "$INITDIR" --package-manager npm --init-lockfile --dry-run --format json --report "$INITDIR/report.json" >/tmp/ci-init-lockfile.out

test -f "$INITDIR/package-lock.json"

echo "==> install command option coverage"
"$BIN" install left-pad@1.3.0 --cwd "$WORKDIR" --package-manager npm --dry-run --format text >/tmp/install-text.out
"$BIN" install left-pad@1.3.0 --cwd "$WORKDIR" --package-manager npm --dry-run --format json >/tmp/install-json.out
"$BIN" install left-pad@1.3.0 --cwd "$WORKDIR" --package-manager npm --dry-run --format github >/tmp/install-github.out
"$BIN" install left-pad@1.3.0 --cwd "$WORKDIR" --package-manager npm --dry-run --format junit >/tmp/install-junit.out
"$BIN" install left-pad@1.3.0 --cwd "$WORKDIR" --package-manager npm --dry-run --allow-scripts --post-verify --timeout 9000 -q --format json >/tmp/install-all-flags.out
"$BIN" install left-pad@^1.3.0 --cwd "$WORKDIR" --package-manager npm --dry-run --format text >/tmp/install-range.out

grep -q '"sentinel_version"' /tmp/install-json.out
grep -q '::' /tmp/install-github.out
grep -q '<testsuites' /tmp/install-junit.out
grep -q 'resolved candidate:' /tmp/install-range.out

echo "==> history command option coverage"
mkdir -p "$WORKDIR/.sentinel"
cat > "$WORKDIR/.sentinel/install-history.ndjson" <<NDJSON
{"schema_version":1,"event_id":"evt-001","run":{"run_started_at":"2026-04-22T10:00:00Z","run_id":"run-001"},"occurred_at":"2026-04-22T10:00:01Z","project_root":"$WORKDIR","package_manager":"npm","command":"install","sentinel_version":"2.0.2","lockfile":{"path":"package-lock.json","sha256_before":"abc","sha256_after":"def"},"package":{"name":"left-pad","version":"1.3.0","direct":true},"result":"success"}
{"schema_version":1,"event_id":"evt-002","run":{"run_started_at":"2026-04-22T11:00:00Z","run_id":"run-002"},"occurred_at":"2026-04-22T11:00:01Z","project_root":"$WORKDIR","package_manager":"npm","command":"ci","sentinel_version":"2.0.2","lockfile":{"path":"package-lock.json","sha256_before":"def","sha256_after":"ghi"},"package":{"name":"left-pad","version":"1.3.0","direct":true},"result":"success"}
NDJSON

"$BIN" history --from "2026-04-22T00:00:00+00:00" --to now --cwd "$WORKDIR" --format text >/tmp/history-range-text.out
"$BIN" history --from "2026-04-22T00:00:00+00:00" --to now --package left-pad --version 1.3.0 --project "$WORKDIR" --package-manager npm --cwd "$WORKDIR" --format json -q >/tmp/history-package-json.out

grep -q 'events:' /tmp/history-range-text.out
grep -q '"found"' /tmp/history-package-json.out

echo "==> global artifact-store option coverage"
"$BIN" --artifact-store auto check --cwd "$WORKDIR" --package-manager npm --format json -q >/tmp/store-auto.out
"$BIN" --artifact-store memory check --cwd "$WORKDIR" --package-manager npm --format json -q >/tmp/store-memory.out
"$BIN" --artifact-store spool check --cwd "$WORKDIR" --package-manager npm --format json -q >/tmp/store-spool.out

grep -q '"sentinel_version"' /tmp/store-auto.out
grep -q '"sentinel_version"' /tmp/store-memory.out
grep -q '"sentinel_version"' /tmp/store-spool.out

echo "all-commands-options-smoke-ok"
