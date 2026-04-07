# sentinel-check

> Thin npm wrapper for the [Sentinel](https://github.com/SIG-sentinel/sentinel-npm) CLI.

Lets you run `sentinel` without installing anything — npx resolves and executes the binary automatically.

---

## Quick start

### No installation required (npx)

```bash
# check current project
npx -y -p sentinel-check ci

# install a package with integrity verification
npx -y -p sentinel-check install <package>@<version>

# audit only
npx -y -p sentinel-check check
```

### With sentinel binary in PATH

If you have the sentinel binary installed globally:

```bash
sentinel ci
sentinel install <package>@<version>
sentinel check
```

> See the [main README](../../README.md) for binary installation instructions (Linux, macOS, Windows).

### Recommended for end users (package.json scripts)

```json
{
	"scripts": {
		"sentinel:ci": "npx -y -p sentinel-check ci",
		"sentinel:check": "npx -y -p sentinel-check check",
		"sentinel:install": "npx -y -p sentinel-check install"
	}
}
```

```bash
npm run sentinel:ci
npm run sentinel:check
npm run sentinel:install -- lodash@4.17.21
```

---

## How it works

1. Looks for a local `sentinel` binary (via `SENTINEL_BIN` or `PATH`)
2. Falls back to a managed binary cached at `~/.cache/sentinel/bin/<version>/`
3. If not cached, downloads the release asset from GitHub and verifies its SHA-256 checksum against `checksums.txt`
4. Executes the resolved binary with all arguments forwarded

---

## Environment overrides

| Variable | Description |
|---|---|
| `SENTINEL_BIN` | path to an existing local binary |
| `SENTINEL_VERSION` | pin a specific release version |
| `SENTINEL_RELEASE_REPO` | override release repository (`owner/repo`) |
| `SENTINEL_RELEASE_BASE_URL` | override release base URL |
| `SENTINEL_SKIP_DOWNLOAD=1` | disable automatic binary download |
