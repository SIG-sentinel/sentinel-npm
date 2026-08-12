# sentinel-check

> Thin npm wrapper for the `sentinel` CLI published from the [sentinel-npm](https://github.com/SIG-sentinel/sentinel-npm) repository.

Use `npx --yes sentinel-check ...` for one-shot runs with no manual binary setup.

Sentinel supports lockfile verification for npm, Yarn, and pnpm with automatic manager detection.

---

## Main commands (quick map)

Use these when you just want the shortest path:

```bash
# I want to audit the current project
npx --yes sentinel-check check

# I want to install one package safely
npx --yes sentinel-check install lodash@4.17.21

# I want to install multiple packages safely (single atomic command)
npx --yes sentinel-check install lodash@4.17.21 axios@1.11.0

# I want to create/recover a secure lockfile, then verify+install
npx --yes sentinel-check ci --init-lockfile --package-manager npm
npx --yes sentinel-check ci --init-lockfile --package-manager yarn
npx --yes sentinel-check ci --init-lockfile --package-manager pnpm
```

---

## Command reference

| Command                      | Purpose                                                                                                     | Common flags                                             | Example                                                                       |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `check`                      | Audit dependencies in lockfile without installing                                                           | `--format {text\|json\|github\|junit}`                   | `npx --yes sentinel-check check --format json`                                |
| `install <pkg[@version]>...` | Install one or more packages with atomic integrity verification (any package failure fails the whole chain) | `--format`, `--post-verify`                              | `npx --yes sentinel-check install lodash@4.17.21 axios@1.11.0 --post-verify`  |
| `ci`                         | Verify full lockfile then run clean install (CI mode)                                                       | `--init-lockfile`, `--package-manager {npm\|yarn\|pnpm}` | `npx --yes sentinel-check ci --init-lockfile --package-manager npm`           |
| `history`                    | Query local install/CI history ledger                                                                       | `--from`, `--to`, `--package`, `--format json`           | `npx --yes sentinel-check history --from "7 days ago" --to now --format json` |

### Flags (global)

| Flag                                   | Purpose                                                                 | Example                                                             |
| -------------------------------------- | ----------------------------------------------------------------------- | ------------------------------------------------------------------- |
| `--help`                               | Show command help and examples                                          | `npx --yes sentinel-check check --help`                             |
| `--format {text\|json\|github\|junit}` | Change output format (check, install, ci)                               | `npx --yes sentinel-check check --format github`                    |
| `--package-manager {npm\|yarn\|pnpm}`  | Explicitly set manager (auto-detected by default)                       | `npx --yes sentinel-check ci --package-manager pnpm`                |
| `--init-lockfile`                      | Regenerate lockfile securely (ci command, requires `--package-manager`) | `npx --yes sentinel-check ci --init-lockfile --package-manager npm` |

---

## Common workflows (encadeamentos)

### Scenario 1: Audit before CI

Audit first to review risks, then run CI:

```bash
# Step 1: Audit
npx --yes sentinel-check check --format json > audit.json

# Step 2: Review audit report, then install
npx --yes sentinel-check ci
```

### Scenario 2: Recover from broken lockfile

Regenerate lockfile securely, then verify everything:

```bash
# Step 1: Backup current lockfile
cp package-lock.json package-lock.json.bak

# Step 2: Regenerate with Sentinel
npx --yes sentinel-check ci --init-lockfile --package-manager npm

# Step 3: Verify new lockfile (optional second audit)
npx --yes sentinel-check check --format json
```

### Scenario 3: Add packages to protected project

Install one or more packages with full verification:

```bash
# Install several packages in one atomic chain
npx --yes sentinel-check install lodash@4.17.21 axios@1.11.0 express@4.18.2

# Chain semantics: if one package fails, the whole install fails and rolls back

# Then verify the full lockfile
npx --yes sentinel-check check
```

### Scenario 4: CI/CD with GitHub Actions

Full lockfile verification in GitHub Actions with GitHub-formatted output:

```bash
# Single command: verify and install with GitHub annotations
npx --yes sentinel-check ci --format github
```

### Scenario 5: Check for provenance anomalies in history

Review past installs and CI runs for workflow changes or missing signatures:

```bash
# Get recent history with JSON output
npx --yes sentinel-check history --from "30 days ago" --to now --format json

# Grep for anomalies in the output
npx --yes sentinel-check history --from "30 days ago" --to now --format json | grep -i "workflow\|provenance"
```

---

## Quick start

### Run directly with npx

```bash
# audit only
npx --yes sentinel-check check

# validate lockfile then install dependencies
npx --yes sentinel-check ci

# install one package with verification
npx --yes sentinel-check install lodash@4.17.21
```

### Add to package.json scripts (recommended)

Install once in the project and call `sentinel` from npm scripts:

```bash
npm install -D sentinel-check
```

```json
{
  "scripts": {
    "sentinel:check": "sentinel check",
    "sentinel:ci": "sentinel ci"
  }
}
```

```bash
npm run sentinel:check
npm run sentinel:ci
```

Need package install with verification? Run it directly:

```bash
npx --yes sentinel-check install lodash@4.17.21
```

---

## CI usage

GitHub Actions:

```yaml
- name: Verify dependency integrity
  run: npx --yes sentinel-check ci
```

If the workflow needs Sentinel to initialize the lockfile first:

```yaml
- name: Initialize lockfile and verify dependency integrity
  run: npx --yes sentinel-check ci --init-lockfile --package-manager npm
```

> Replace `npm` with `yarn` or `pnpm` based on your project's package manager.

---

## Notes

1. The wrapper downloads the matching Sentinel release binary on first use.
2. Downloaded binaries are cached locally.
3. Integrity is verified using release checksums before execution.
4. If you see `dependency cycles detected`, Sentinel found circular dependency chains in the lockfile graph. **Verification continues and cycles are reported as a warning.** You'll still see the integrity status of all packages. For a safe first recovery step, remove `node_modules` and rerun `npx --yes sentinel-check ci`. If lockfile recovery is needed, remove the lockfile and rerun `npx --yes sentinel-check ci --init-lockfile --package-manager <npm|yarn|pnpm>` so Sentinel regenerates it in the guarded flow.

## More documentation

- Security policy: [SECURITY.md](https://github.com/SIG-sentinel/sentinel-npm/blob/main/SECURITY.md)
- Threat model: [THREAT_MODEL.md](https://github.com/SIG-sentinel/sentinel-npm/blob/main/THREAT_MODEL.md)
- Adoption and distribution guide: [ADOPTION_DISTRIBUTION.md](https://github.com/SIG-sentinel/sentinel-npm/blob/main/ADOPTION_DISTRIBUTION.md)

---

## Useful environment variables

| Variable                   | Description                           |
| -------------------------- | ------------------------------------- |
| `SENTINEL_BIN`             | Use an existing local sentinel binary |
| `SENTINEL_VERSION`         | Pin a specific Sentinel version       |
| `SENTINEL_SKIP_DOWNLOAD=1` | Disable automatic binary download     |

See the [main README](https://github.com/SIG-sentinel/sentinel-npm#readme) for full CLI usage and binary installation options.
