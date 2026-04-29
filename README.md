# sentinel-npm

> Repository for the Sentinel npm ecosystem. Published CLI: `sentinel`. npm wrapper for `npx`: `sentinel-check`.

[![release](https://github.com/SIG-sentinel/sentinel-npm/actions/workflows/release.yml/badge.svg)](https://github.com/SIG-sentinel/sentinel-npm/actions/workflows/release.yml)
![license](https://img.shields.io/badge/license-MIT-blue)
![platforms](https://img.shields.io/badge/platforms-linux%20%7C%20macos%20%7C%20windows-lightgrey)
![outputs](https://img.shields.io/badge/output-text%20%7C%20json%20%7C%20junit%20%7C%20github-informational)
![npm version](https://img.shields.io/npm/v/sentinel-check)
![npm downloads](https://img.shields.io/npm/dm/sentinel-check)

Package managers already verify tarball integrity against the lockfile during install. **sentinel** adds a defense-in-depth layer on top: it cross-checks lockfile integrity, registry metadata, and the downloaded tarball hash — three independent sources — before any package is installed or any lifecycle script runs.

Sentinel automatically works with `package-lock.json`, `yarn.lock`, and `pnpm-lock.yaml`.

When using `sentinel-check`, the wrapper downloads the matching Sentinel binary and verifies it against release checksums before execution.

This repository has two entry points:

- `sentinel`: the main CLI binary
- `sentinel-check`: the npm wrapper for use with `npx` and automation

---

## What you get

| Capability                                     | `npm ci` | `npm audit` | sentinel |
| ---------------------------------------------- | -------- | ----------- | -------- |
| Verify tarball vs lockfile                     | ✅       | ❌          | ✅       |
| Cross-check registry metadata                  | ❌       | ❌          | ✅       |
| Verify all packages before installing any      | ❌       | ❌          | ✅       |
| TOCTOU protection (lockfile re-check)          | ❌       | ❌          | ✅       |
| Audit without installing                       | ❌       | ✅          | ✅       |
| Auto-detect npm/yarn/pnpm                      | ❌       | ❌          | ✅       |
| Machine-readable CI output (JSON/JUnit/GitHub) | ❌       | ✅          | ✅       |
| Zero SaaS / zero telemetry                     | ✅       | ❌          | ✅       |

### Lockfile detection flow

```text
yarn.lock / pnpm-lock.yaml / package-lock.json
        |
        v
      sentinel auto-detects manager
        |
        v
   sentinel ci executes manager-specific clean install
  npm ci | yarn install --frozen-lockfile | pnpm install --frozen-lockfile
```

---

## Pick the right command

| Command                              | When to use                              | What it does                                                                                                                      |
| ------------------------------------ | ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `sentinel check`                     | Local audit, PR review, debugging        | Audits the current project without installing anything                                                                            |
| `sentinel ci`                        | Pipeline, clean environment, strict gate | Verifies **every package in the lockfile** and, if all pass, runs the clean install command for the detected manager              |
| `sentinel install package[@version]` | Adding a new package safely              | Resolves the package in the lockfile, verifies the target and its transitive deps, then runs the manager-specific install command |
| `sentinel history`                   | Trace recent installs and CI runs        | Queries the local install history ledger by time range, package, project, or package manager                                      |

> If your goal is "install the whole project from the lockfile", the right command is `sentinel ci`.

> Security default in v2+: lifecycle scripts are blocked by default in `sentinel ci` and `sentinel install`.
> Use `--allow-scripts` only when your project requires lifecycle hooks.

## CLI reference (flags and advanced features)

This section reflects the current CLI help output.

### Global options (all commands)

| Flag                                     | Description                                                  |
| ---------------------------------------- | ------------------------------------------------------------ |
| `--artifact-store <memory\|spool\|auto>` | Chooses where verified tarballs are staged. Default: `auto`. |
| `-h, --help`                             | Prints help.                                                 |
| `-V, --version`                          | Prints version.                                              |

### `sentinel check` options

| Flag                                   | Description                                                       |
| -------------------------------------- | ----------------------------------------------------------------- |
| `--omit-dev`                           | Skips dev dependencies in verification.                           |
| `--omit-optional`                      | Skips optional dependencies in verification.                      |
| `--format <text\|json\|github\|junit>` | Output format (default: `text`).                                  |
| `--cwd <CWD>`                          | Project directory (default: `.`).                                 |
| `--package-manager <npm\|yarn\|pnpm>`  | Forces package manager instead of auto-detection.                 |
| `--timeout <TIMEOUT>`                  | Registry timeout in milliseconds (default: `5000`).               |
| `--registry-max-in-flight <N>`         | Max concurrent registry requests for this command (CLI override). |
| `-q, --quiet`                          | Reduces non-essential output.                                     |

### `sentinel ci` options

| Flag                                   | Description                                                                             |
| -------------------------------------- | --------------------------------------------------------------------------------------- |
| `--omit-dev`                           | Skips dev dependencies in verification/install flow.                                    |
| `--omit-optional`                      | Skips optional dependencies in verification/install flow.                               |
| `--allow-scripts`                      | Enables lifecycle scripts (`preinstall`, `postinstall`, etc.). Default is blocked.      |
| `--dry-run`                            | Verifies and prepares flow without executing final install step.                        |
| `--post-verify`                        | After install, validates installed package content against verified registry artifacts. |
| `--init-lockfile`                      | Initializes/refreshes lockfile in guarded flow when missing or recovery is required.    |
| `--format <text\|json\|github\|junit>` | Output format (default: `text`).                                                        |
| `--report <REPORT>`                    | Report path for CI output (default: `sentinel-report.json`).                            |
| `--cwd <CWD>`                          | Project directory (default: `.`).                                                       |
| `--package-manager <npm\|yarn\|pnpm>`  | Forces package manager instead of auto-detection.                                       |
| `--timeout <TIMEOUT>`                  | Registry timeout in milliseconds (default: `10000`).                                    |
| `--registry-max-in-flight <N>`         | Max concurrent registry requests for this command (CLI override).                       |
| `-q, --quiet`                          | Reduces non-essential output.                                                           |

### `sentinel install <PACKAGE[@VERSION]>` options

| Flag                                   | Description                                                                             |
| -------------------------------------- | --------------------------------------------------------------------------------------- |
| `--allow-scripts`                      | Enables lifecycle scripts (`preinstall`, `postinstall`, etc.). Default is blocked.      |
| `--dry-run`                            | Resolves/verifies candidate without applying package manager changes.                   |
| `--post-verify`                        | After install, validates installed package content against verified registry artifacts. |
| `--format <text\|json\|github\|junit>` | Output format (default: `text`).                                                        |
| `--cwd <CWD>`                          | Project directory (default: `.`).                                                       |
| `--package-manager <npm\|yarn\|pnpm>`  | Forces package manager instead of auto-detection.                                       |
| `--timeout <TIMEOUT>`                  | Registry timeout in milliseconds (default: `5000`).                                     |
| `--registry-max-in-flight <N>`         | Max concurrent registry requests for this command (CLI override).                       |
| `-q, --quiet`                          | Reduces non-essential output.                                                           |

### `sentinel history` options

| Flag                                  | Description                                                              |
| ------------------------------------- | ------------------------------------------------------------------------ |
| `--from <RFC3339\|RELATIVE>`          | Start time filter. Accepts RFC3339 or relative values like `7 days ago`. |
| `--to <RFC3339\|RELATIVE>`            | End time filter. Accepts RFC3339 or relative values like `now`.          |
| `--package <PACKAGE>`                 | Filters history by package name.                                         |
| `--version <VERSION>`                 | Filters by version (requires `--package`).                               |
| `--project <PROJECT>`                 | Filters by project path recorded in history events.                      |
| `--package-manager <npm\|yarn\|pnpm>` | Filters by package manager.                                              |
| `--format <text\|json>`               | Output format (default: `text`).                                         |
| `--cwd <CWD>`                         | Project directory (default: `.`).                                        |
| `-q, --quiet`                         | Reduces non-essential output.                                            |

### Advanced feature notes

1. Artifact store modes:
   `memory` keeps verified tarballs in memory; `spool` persists verified tarballs to temporary disk artifacts; `auto` (default) starts in memory and falls back to spool when memory budget is exceeded.
2. Registry concurrency control:
   CLI flag `--registry-max-in-flight` has highest precedence; if not passed, Sentinel reads `SENTINEL_REGISTRY_MAX_IN_FLIGHT`; if neither is set, the built-in default is used.
3. Provenance verification:
   Sentinel performs provenance checks during verification automatically (no separate `provenance verify` command), and provenance fields are included in evidence/report output when available.
4. Post-verify safety pass:
   `--post-verify` performs a second integrity pass after install by validating installed content against verified artifacts, useful for extra assurance in CI and release workflows.

### Useful environment variables

| Variable                          | Description                                                        |
| --------------------------------- | ------------------------------------------------------------------ |
| `SENTINEL_REGISTRY_MAX_IN_FLIGHT` | Default max concurrent registry requests when CLI flag is not set. |
| `SENTINEL_HISTORY_PATH`           | Custom path for install history ledger file.                       |
| `SENTINEL_LOG`                    | Sentinel log level/filter configuration.                           |
| `RUST_LOG`                        | Rust tracing/log filter override (advanced debugging).             |

---

## Get started in 30 seconds

### Option A: no installation needed

Good for quick evaluation, ephemeral environments, and CI.

> Important: in clean environments, avoid `npx sentinel ...` because npm may resolve a different package named `sentinel`. Use `npx --yes sentinel-check ...`.

```bash
# verify the whole project and, if clean, run the detected manager clean install
npx --yes sentinel-check ci

# enable lifecycle scripts explicitly only if required by your dependencies
npx --yes sentinel-check ci --allow-scripts

# audit the project without installing anything
npx --yes sentinel-check check

# install a specific package with verification
npx --yes sentinel-check install lodash@4.17.21

# enable lifecycle scripts explicitly only if required by your dependencies
npx --yes sentinel-check install lodash@4.17.21 --allow-scripts

# let sentinel resolve the latest safe candidate already pinned into the lockfile flow
npx --yes sentinel-check install lodash
```

### Option B: binary on PATH

Good for teams that will use Sentinel daily.

#### Linux and macOS

Standard install to `/usr/local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/SIG-sentinel/sentinel-npm/main/scripts/install.sh | sudo sh -s -- --version 2.1.1
```

Install to user directory:

```bash
curl -fsSL -o /tmp/install-sentinel.sh https://raw.githubusercontent.com/SIG-sentinel/sentinel-npm/main/scripts/install.sh
INSTALL_DIR="$HOME/.local/bin" sh /tmp/install-sentinel.sh --version 2.1.0
```

Pin a specific version:

```bash
curl -fsSL -o /tmp/install-sentinel.sh https://raw.githubusercontent.com/SIG-sentinel/sentinel-npm/main/scripts/install.sh
sh /tmp/install-sentinel.sh --version 2.1.0
```

Confirm installation:

```bash
sentinel --version
```

#### Windows

Use manual binary download from [github.com/SIG-sentinel/sentinel-npm/releases](https://github.com/SIG-sentinel/sentinel-npm/releases), then validate with `checksums.txt`.

---

## Add to package.json scripts

### Using npx wrapper scripts

```json
{
  "scripts": {
    "sentinel:ci": "npx --yes sentinel-check ci",
    "sentinel:check": "npx --yes sentinel-check check"
  }
}
```

Usage:

```bash
npm run sentinel:ci
npm run sentinel:check
npx --yes sentinel-check install lodash@4.17.21
npx --yes sentinel-check install lodash

# Same scripts in Yarn/pnpm projects
yarn sentinel:ci
pnpm sentinel:ci
```

### Using sentinel binary on PATH

```json
{
  "scripts": {
    "sentinel:ci": "sentinel ci",
    "sentinel:check": "sentinel check"
  }
}
```

Usage:

```bash
npm run sentinel:ci
npm run sentinel:check
sentinel install lodash@4.17.21
sentinel install lodash

# opt in when your dependency requires lifecycle scripts
sentinel ci --allow-scripts
sentinel install lodash@4.17.21 --allow-scripts
```

### Install and history examples

```bash
# let sentinel resolve the candidate and show the pinned version before install
sentinel install axios

# explicit pin still works when you already know the target version
sentinel install axios@1.11.0

# query recent install activity with human-friendly timestamps
sentinel history --from "7 days ago" --to now

# narrow to a single package
sentinel history --from "30 days ago" --to now --package axios
```

### Registry configuration via `.npmrc`

Sentinel resolves npm registries from the project directory first (`--cwd`), then falls back to `~/.npmrc`. That means a project-local `.npmrc` can safely override a broken or unrelated home-level registry configuration.

```ini
registry=https://registry.npmjs.org/
@acme:registry=https://npm.pkg.github.com/
//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}
```

Use this when a workspace depends on a private scope or when CI injects per-project registry credentials.

---

## CI/CD integration

### GitHub Actions with npx

```yaml
- name: Verify dependency integrity
  run: npx --yes sentinel-check ci

- name: Verify dependency integrity (project requires lifecycle scripts)
  run: npx --yes sentinel-check ci --allow-scripts
```

If the workflow may start without a lockfile, use:

```yaml
- name: Initialize lockfile and verify dependency integrity
  run: npx --yes sentinel-check ci --init-lockfile
```

### Package manager setup examples

```yaml
# npm lockfile
- run: npx --yes sentinel-check ci --init-lockfile

# yarn lockfile
- run: corepack enable
- run: npx --yes sentinel-check ci --init-lockfile

# pnpm lockfile
- run: corepack enable
- run: npx --yes sentinel-check ci --init-lockfile
```

If your repository already commits a trusted lockfile, prefer plain `sentinel ci` and reserve `--init-lockfile` for controlled recovery or first-time setup.

### GitHub Actions with installed binary

```yaml
- name: Install sentinel
  run: curl -fsSL https://raw.githubusercontent.com/SIG-sentinel/sentinel-npm/main/scripts/install.sh | sudo sh -s -- --version 2.1.0

- name: Verify dependency integrity
  run: sentinel ci

- name: Verify dependency integrity (project requires lifecycle scripts)
  run: sentinel ci --allow-scripts
```

If the workflow needs Sentinel to initialize the lockfile first:

```yaml
- name: Initialize lockfile and verify dependency integrity
  run: sentinel ci --init-lockfile
```

### Machine-readable output

```bash
sentinel check --format json
sentinel check --format junit
sentinel check --format github
sentinel ci --dry-run --format json --report sentinel-report.json
```

If no lockfile is present, use `sentinel ci --init-lockfile` to let Sentinel create or refresh it in the guarded flow.

The secure order in CI is: commit and review a trusted lockfile when possible, run `sentinel ci` for normal enforcement, and use `sentinel ci --init-lockfile` only for controlled initialization or recovery.

### Using sentinel alongside npm audit

`sentinel ci` and `npm audit` address different threat surfaces and should run together in CI pipelines — they are complementary, not alternatives.

| Tool          | What it catches                                                             |
| ------------- | --------------------------------------------------------------------------- |
| `sentinel ci` | Integrity: tampered tarballs, lockfile injection, CDN compromise, TOCTOU    |
| `npm audit`   | CVEs: known vulnerabilities in published versions via the advisory database |

Neither replaces the other. A package can have a clean hash and a published CVE; another can have no CVE and a compromised tarball. Run both:

```yaml
- name: Verify dependency integrity (sentinel)
  run: npx --yes sentinel-check ci

- name: Audit for known vulnerabilities (npm audit)
  run: npm audit --audit-level=high
```

For projects that must not fail CI on audit findings yet (e.g., vulnerabilities in dev-only deps with no fix available), use `--audit-level=critical` or `npm audit --production` while tracking remediation separately.

---

## Why this model

`npm ci` already verifies tarball integrity against the lockfile. Sentinel adds value in three areas:

1. **Third source (registry metadata)** — `npm ci` checks tarball vs lockfile (2 sources). Sentinel adds the registry's `dist.integrity` as a third cross-check, catching lockfile injection where the lockfile points to a malicious URL that serves a tarball matching the injected hash.
2. **Pre-install gate** — `npm ci` verifies and installs per-package atomically: if package A passes, its lifecycle scripts run before package B is verified. Sentinel verifies **all** packages before installing **any**, so no lifecycle script executes until the entire tree is clean.
3. **TOCTOU protection** — Sentinel re-checks the lockfile hash between verification and install. No other tool does this.

```text
lockfile says:      pkg@1.2.3 has sha512-A
registry says:      pkg@1.2.3 has sha512-A
downloaded tarball:  computed hash = sha512-A
→ All agree → CLEAN
→ Any divergence → COMPROMISED, install blocked
```

**Explicit limitation:** If an attacker publishes through a compromised maintainer account, the registry serves consistent metadata and tarball. All three sources agree on the malicious content, and Sentinel passes. This scenario requires complementary tools (static analysis, provenance checks). See [THREAT_MODEL.md](THREAT_MODEL.md) for full details.

---

## Security layer requirements

Sentinel is a **verification layer**, not a standalone solution. Its effectiveness depends on these practices:

| Requirement                                                                          | Why it matters                                                                                                                                                                     |
| ------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Lockfile committed to version control**                                            | Sentinel compares lockfile integrity against registry and tarball. Without a committed lockfile, there is no trusted baseline to verify                                            |
| **Frozen installs** (`npm ci` / `yarn --frozen-lockfile` / `pnpm --frozen-lockfile`) | `npm install` updates the lockfile on resolution, potentially recording a malicious version as the new baseline                                                                    |
| **Review lockfile changes in PRs**                                                   | Lockfile injection attacks modify resolution URLs and integrity hashes directly — code review is the only defense ([details](THREAT_MODEL.md#lockfile-injection-via-pull-request)) |
| **Pin exact dependency versions**                                                    | Ranges like `^1.14.0` allow resolution to a newly published malicious version on the next `npm install`                                                                            |

If these practices are not in place, Sentinel's protection window narrows significantly. See [THREAT_MODEL.md](THREAT_MODEL.md) for the full analysis including [real-world incidents](THREAT_MODEL.md#real-world-incidents-2025-2026).

---

## Evidence and trust docs

- [SECURITY.md](SECURITY.md): disclosure policy, guarantees, limitations, and operational security notes
- [THREAT_MODEL.md](THREAT_MODEL.md): attacker model, trust boundaries, registry trust root caveat, and why three-source verification differs from list-only approaches
- [ADOPTION_DISTRIBUTION.md](ADOPTION_DISTRIBUTION.md): rollout guidance for CI adoption and distribution roadmap (winget, scoop, choco, Homebrew)

---

## How to interpret results

| Status         | Meaning                     | Effect               |
| -------------- | --------------------------- | -------------------- |
| `CLEAN`        | integrity confirmed         | installation allowed |
| `UNVERIFIABLE` | could not confirm the chain | installation blocked |
| `COMPROMISED`  | divergence detected         | installation blocked |

If Sentinel prints `dependency cycles detected`, the dependency graph contains circular chains. Sentinel will **continue verification and report cycles as a warning** (not a blocker). This allows you to see package integrity status despite cycles. For a safe first recovery step, remove `node_modules` and rerun `sentinel ci` (or `npx --yes sentinel-check ci`). If lockfile recovery is needed, remove the lockfile and rerun `sentinel ci --init-lockfile` so Sentinel regenerates it in the guarded flow.
