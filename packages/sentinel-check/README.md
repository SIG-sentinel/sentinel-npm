# sentinel-check

> Thin npm wrapper for the `sentinel` CLI published from the [sentinel-npm](https://github.com/SIG-sentinel/sentinel-npm) repository.

Run Sentinel with npx, no manual binary setup required.

---

## Quick start

### Run directly with npx

```bash
# audit only
npx -y -p sentinel-check check

# validate lockfile then install dependencies
npx -y -p sentinel-check ci

# install one package with verification
npx -y -p sentinel-check install lodash@4.17.21
```

### Add to package.json scripts (recommended)

```json
{
  "scripts": {
    "sentinel:check": "npx -y -p sentinel-check check",
    "sentinel:ci": "npx -y -p sentinel-check ci"
  }
}
```

```bash
npm run sentinel:check
npm run sentinel:ci
```

Need package install with verification? Run it directly:

```bash
npx -y -p sentinel-check install express@4.21.2
```

---

## CI usage

GitHub Actions:

```yaml
- name: Verify dependency integrity
  run: npx -y -p sentinel-check ci
```

---

## Notes

1. The wrapper downloads the matching Sentinel release binary on first use.
2. Downloaded binaries are cached locally.
3. Integrity is verified using release checksums before execution.

---

## Useful environment variables

| Variable | Description |
| --- | --- |
| `SENTINEL_BIN` | Use an existing local sentinel binary |
| `SENTINEL_VERSION` | Pin a specific Sentinel version |
| `SENTINEL_SKIP_DOWNLOAD=1` | Disable automatic binary download |

See the [main README](https://github.com/SIG-sentinel/sentinel-npm#readme) for full CLI usage and binary installation options.
