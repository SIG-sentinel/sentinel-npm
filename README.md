# sentinel-npm

> Repository for the Sentinel npm ecosystem. Published CLI: `sentinel`. npm wrapper for `npx`: `sentinel-check`.

![build](https://img.shields.io/badge/build-passing-brightgreen)
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

| Capability | `npm ci` | `npm audit` | sentinel |
| --- | --- | --- | --- |
| Verify tarball vs lockfile | ✅ | ❌ | ✅ |
| Cross-check registry metadata | ❌ | ❌ | ✅ |
| Verify all packages before installing any | ❌ | ❌ | ✅ |
| TOCTOU protection (lockfile re-check) | ❌ | ❌ | ✅ |
| Audit without installing | ❌ | ✅ | ✅ |
| Auto-detect npm/yarn/pnpm | ❌ | ❌ | ✅ |
| Machine-readable CI output (JSON/JUnit/GitHub) | ❌ | ✅ | ✅ |
| Zero SaaS / zero telemetry | ✅ | ❌ | ✅ |

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

| Command | When to use | What it does |
| --- | --- | --- |
| `sentinel check` | Local audit, PR review, debugging | Audits the current project without installing anything |
| `sentinel ci` | Pipeline, clean environment, strict gate | Verifies **every package in the lockfile** and, if all pass, runs the clean install command for the detected manager |
| `sentinel install package@version` | Adding a new package safely | Resolves the package in the lockfile, verifies the target and its transitive deps, then runs the manager-specific install command |
| `sentinel report package` | Manually report a suspicious package | Prints the evidence escalation flow for the given package |

> If your goal is "install the whole project from the lockfile", the right command is `sentinel ci`.

---

## Get started in 30 seconds

### Option A: no installation needed

Good for quick evaluation, ephemeral environments, and CI.

> Important: in clean environments, avoid `npx sentinel ...` because npm may resolve a different package named `sentinel`. Use `npx --yes sentinel-check ...`.

```bash
# verify the whole project and, if clean, run the detected manager clean install
npx --yes sentinel-check ci

# audit the project without installing anything
npx --yes sentinel-check check

# install a specific package with verification
npx --yes sentinel-check install lodash@4.17.21
```

### Option B: binary on PATH

Good for teams that will use Sentinel daily.

#### Linux and macOS

Standard install to `/usr/local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/SIG-sentinel/sentinel-npm/main/scripts/install.sh | sudo sh
```

Install to user directory:

```bash
curl -fsSL -o /tmp/install-sentinel.sh https://raw.githubusercontent.com/SIG-sentinel/sentinel-npm/main/scripts/install.sh
INSTALL_DIR="$HOME/.local/bin" sh /tmp/install-sentinel.sh
```

Pin a specific version:

```bash
curl -fsSL -o /tmp/install-sentinel.sh https://raw.githubusercontent.com/SIG-sentinel/sentinel-npm/main/scripts/install.sh
sh /tmp/install-sentinel.sh --version 1.1.1
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
```

---

## CI/CD integration

### GitHub Actions with npx

```yaml
- name: Verify dependency integrity
  run: npx --yes sentinel-check ci
```

### Package manager setup examples

```yaml
# npm lockfile
- run: npm install --package-lock-only
- run: npx --yes sentinel-check ci

# yarn lockfile
- run: corepack enable
- run: yarn install --mode=update-lockfile
- run: npx --yes sentinel-check ci

# pnpm lockfile
- run: corepack enable
- run: pnpm install --lockfile-only
- run: npx --yes sentinel-check ci
```

### GitHub Actions with installed binary

```yaml
- name: Install sentinel
  run: curl -fsSL https://raw.githubusercontent.com/SIG-sentinel/sentinel-npm/main/scripts/install.sh | sudo sh

- name: Verify dependency integrity
  run: sentinel ci
```

### Machine-readable output

```bash
sentinel check --format json
sentinel check --format junit
sentinel check --format github
sentinel ci --dry-run --format json --report sentinel-report.json
```

If no lockfile is present, Sentinel generates one with the detected manager when possible.

The secure order in CI is: generate/sync lockfile first, run `sentinel ci`, and let Sentinel perform the guarded install step.

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

| Requirement | Why it matters |
| --- | --- |
| **Lockfile committed to version control** | Sentinel compares lockfile integrity against registry and tarball. Without a committed lockfile, there is no trusted baseline to verify |
| **Frozen installs** (`npm ci` / `yarn --frozen-lockfile` / `pnpm --frozen-lockfile`) | `npm install` updates the lockfile on resolution, potentially recording a malicious version as the new baseline |
| **Review lockfile changes in PRs** | Lockfile injection attacks modify resolution URLs and integrity hashes directly — code review is the only defense ([details](THREAT_MODEL.md#lockfile-injection-via-pull-request)) |
| **Pin exact dependency versions** | Ranges like `^1.14.0` allow resolution to a newly published malicious version on the next `npm install` |

If these practices are not in place, Sentinel's protection window narrows significantly. See [THREAT_MODEL.md](THREAT_MODEL.md) for the full analysis including [real-world incidents](THREAT_MODEL.md#real-world-incidents-2025-2026).

---

## Evidence and trust docs

- [SECURITY.md](SECURITY.md): disclosure policy, guarantees, limitations, and operational security notes
- [THREAT_MODEL.md](THREAT_MODEL.md): attacker model, trust boundaries, registry trust root caveat, and why three-source verification differs from list-only approaches
- [ADOPTION_DISTRIBUTION.md](ADOPTION_DISTRIBUTION.md): rollout guidance for CI adoption and distribution roadmap (winget, scoop, choco, Homebrew)

---

## How to interpret results

| Status | Meaning | Effect |
| --- | --- | --- |
| `CLEAN` | integrity confirmed | installation allowed |
| `UNVERIFIABLE` | could not confirm the chain | installation blocked |
| `COMPROMISED` | divergence detected | installation blocked |

If Sentinel prints `dependency cycles detected`, the dependency graph contains circular chains. Sentinel will **continue verification and report cycles as a warning** (not a blocker). This allows you to see package integrity status despite cycles. For a safe first recovery step, remove `node_modules` and rerun `sentinel ci` (or `npx --yes sentinel-check ci`). If lockfile recovery is needed, remove the lockfile and rerun `sentinel ci` so Sentinel regenerates it in the guarded flow.
