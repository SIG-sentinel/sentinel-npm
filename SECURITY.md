# Security Policy

## Overview

This repository, `sentinel-npm`, publishes two related artifacts:

- `sentinel`: the Rust CLI that performs verification and installation gating
- `sentinel-check`: the npm wrapper used with `npx` and Node-based automation

This document explains the security model of the Sentinel npm workflow and the operational guarantees that exist today.

For attacker model and trust-boundary details, see [THREAT_MODEL.md](THREAT_MODEL.md).

## Security Model

Sentinel uses package-registry integrity metadata as source of truth for npm ecosystem lockfiles:

```text
DOWNLOAD VERIFICATION (happens for every install):
  1. Download package tarball from npm registry
  2. Compute SHA-512 hash of downloaded tarball
  3. Compare against npm's published dist.integrity → PASS or FAIL

LOCKFILE VERIFICATION (happens before install):
  1. Read detected lockfile entries (`package-lock.json`, `yarn.lock`, or `pnpm-lock.yaml`)
  2. Query npm registry for latest published hashes
  3. Compare lockfile hashes vs registry → MATCH, DIVERGE, or UNVERIFIABLE

INSTALLATION VERIFICATION (Sentinel: before/after):
  1. All above checks pass → permit manager-specific install
  2. Any check fails → block installation, prevent TOCTOU window
```

## Threat Model

| Threat | npm | sentinel | Notes |
| --- | --- | --- | --- |
| Tarball tampering | ❌ | ✅ | Hash mismatch blocks install |
| Lockfile tampering | ❌ | ✅ | Verified against registry |
| Man-in-the-middle over HTTPS | ✅ | ✅ | TLS is still required |
| Time-of-check-time-of-use (TOCTOU) between verify and install | ❌ | ✅ | lockfile hash is re-checked before install |
| Cached stale result reuse | ⚠️ | ✅ | cache policy limits reuse for unverifiable results |
| Developer social engineering | ⚠️ | ⚠️ | technical verification does not solve human trust decisions |

## Usage Recommendations

### Development

```bash
# Always verify before adding dependency
sentinel install express@4.21.2 --allow-scripts

# Or check current state
sentinel check
```

### CI/CD

```yaml
# GitHub Actions: gate on supply chain integrity
- name: Verify dependencies
  run: npx --yes sentinel-check ci
```

**CI mode enforces:**

- ❌ No `UNVERIFIABLE` packages
- ❌ No `COMPROMISED` packages
- ✅ JSON report for audit trail by default
- ✅ Non-zero exit code on any blocking result or failed guarded install

### Cache Behavior

Sentinel caches verification results locally at `~/.cache/sentinel/`:

| Status | TTL | Cache? | Behavior |
| --- | --- | --- | --- |
| CLEAN | ∞ | ✅ | Reuse indefinitely (immutable hash) |
| UNVERIFIABLE | 5 min | ✅ | Reuse briefly, re-check after TTL |
| COMPROMISED | — | ❌ | Never cache (always block) |

**Clear cache if you suspect corruption:**

```bash
rm -rf ~/.cache/sentinel/
```

## Installation Security

### Official Releases

All releases are published to GitHub with SHA-256 checksums:

```bash
# Verify binary before running
curl -fsSL https://github.com/SIG-sentinel/sentinel-npm/releases/latest/download/checksums.txt > /tmp/checksums.txt
sha256sum -c /tmp/checksums.txt sentinel-linux-x64
```

### npm Package (`sentinel-check`)

The npm wrapper (`sentinel-check`) does not implement verification itself. It resolves or downloads the `sentinel` binary and forwards all arguments to it:

- downloaded binaries are validated against published release checksums before execution
- wrapper execution fails when binary integrity validation fails

```bash
# Install via npm (recommended for CI)
npm install -g sentinel-check

# Use via npx (no installation)
npx --yes sentinel-check ci
```

## Reporting Security Issues

If you discover a vulnerability:

1. **Do NOT open a public GitHub issue**
2. Open a **private GitHub Security Advisory** in this repository (`Security` tab)
3. Include:

    - Description of vulnerability
    - Steps to reproduce
    - Suggested fix (if any)
    - Preferred response channel in the advisory thread

We will:

- Acknowledge receipt within 24 hours
- Investigate and confirm impact
- Prepare fix and release security patch
- Credit you (unless you prefer anonymity)

## Cryptographic Details

- **Hash Algorithm**: SHA-512 (via `sha2` crate, pure Rust)
- **Comparison**: Constant-time comparison (via `subtle` crate) to prevent timing attacks
- **Encoding**: Base64 (standard, RFC 4648)
- **TLS**: rustls only (no OpenSSL system dependency)
- **HTTP Client**: reqwest with Tokio async runtime

## Verification of Sentinel Itself

Sentinel is a supply chain security tool, so the repository aims to keep the trust surface small:

- **No unsafe Rust** in the CLI codebase
- **No telemetry** in the verification flow
- **Open source** code available for audit
- **Pinned release workflow actions** for GitHub Actions publishing

Operational note: the project invokes external package-manager commands (`npm`, `yarn`, `pnpm`) as part of the guarded workflow. That behavior is intentional and part of the trust boundary.

## Limitations

### What sentinel does NOT protect against

1. **Social engineering** — if a developer bypasses the tooling or trusts a malicious package intentionally
2. **Malicious but consistently published packages** — if registry metadata and tarball agree, Sentinel verifies integrity, not intent
3. **Registry/operator trust root** — Sentinel still relies on published registry metadata as part of the chain
4. **Packages without sufficient metadata** — these become `UNVERIFIABLE`
5. **Post-installation runtime vulnerabilities** — this is not a vulnerability scanner

### Complementary tools

- `npm audit` — vulnerability scanning in dependencies
- `snyk` / `Dependabot` — automated vulnerability monitoring
- `SBOM tools` — software bill of materials
- Code review — inspect source code before trusting

## FAQ

### Q: Why not use GPG signatures?

**A:** npm doesn't sign packages (only hashes). GPG would add trust assumptions without solving the core problem (which we solve via hash verification).

### Q: What if npm registry is down?

**A:** Sentinel marks affected packages as `UNVERIFIABLE`. In `sentinel ci`, this blocks installation by design.

### Q: Is sentinel production-ready?

**A:** Yes, for:

- ✅ Development environments
- ✅ CI/CD gates
- ✅ Audit workflows

### Q: What's the performance impact?

**A:** ~2-5 seconds per install, dominated by registry queries. Cached results are instant.

## Version History

- **v1.1.1** (2026-04-08)
  - Documentation hardening for multi-manager adoption and CI guidance
  - Security disclosure flow moved to private GitHub Security Advisory path
  - Wrapper binary-checksum verification explicitly documented

- **v1.1.0** (2026-04-08)
  - Multi-package-manager lockfile support for npm, Yarn, and pnpm
  - Stronger install and CI verification safeguards
  - Expanded adversarial and ecosystem test coverage

- **Pre-1.0 series** (2026-04-07 to 2026-04-08)
  - Early iterations and release hardening leading to the v1.1.0 baseline
  - See GitHub Releases for granular pre-1.0 tags and notes
