# sentinel-check

> Thin npm wrapper for the `sentinel` CLI published from the [sentinel-npm](https://github.com/SIG-sentinel/sentinel-npm) repository.

Use `npx --yes sentinel-check ...` for one-shot runs with no manual binary setup.

Sentinel supports lockfile verification for npm, Yarn, and pnpm with automatic manager detection.

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
  run: npx --yes sentinel-check ci --init
```

---

## Notes

1. The wrapper downloads the matching Sentinel release binary on first use.
2. Downloaded binaries are cached locally.
3. Integrity is verified using release checksums before execution.
4. If you see `dependency cycles detected`, Sentinel found circular dependency chains in the lockfile graph. **Verification continues and cycles are reported as a warning.** You'll still see the integrity status of all packages. For a safe first recovery step, remove `node_modules` and rerun `npx --yes sentinel-check ci`. If lockfile recovery is needed, remove the lockfile and rerun `npx --yes sentinel-check ci --init` so Sentinel regenerates it in the guarded flow.

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
