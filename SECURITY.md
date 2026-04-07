# Security Policy

## Overview

Sentinel provides **supply chain security for npm** by verifying package integrity before installation. This document explains the security model and best practices.

## Security Model

Sentinel uses **npm's immutable dist.integrity field** as source of truth:

```
DOWNLOAD VERIFICATION (happens for every install):
  1. Download package tarball from npm registry
  2. Compute SHA-512 hash of downloaded tarball
  3. Compare against npm's published dist.integrity → PASS or FAIL

LOCKFILE VERIFICATION (happens before install):
  1. Read package-lock.json entries
  2. Query npm registry for latest published hashes
  3. Compare lockfile hashes vs registry → MATCH, DIVERGE, or UNVERIFIABLE

INSTALLATION VERIFICATION (Sentinel: before/after):
  1. All above checks pass → permit npm install
  2. Any check fails → block installation, prevent TOCTOU window
```

## Threat Model

| Threat | npm | sentinel | Notes |
|--------|-----|----------|-------|
| Tarball tampering | ❌ | ✅ | Hash mismatch blocks install |
| Registry compromise | ❌ | ✅ | Tarball verification independent |
| Man-in-the-middle (HTTPS) | ✅ | ✅ | TLS 1.2+ required (rustls only) |
| Lockfile tampering | ❌ | ✅ | Verified against registry |
| Time-of-check-time-of-use (TOCTOU) | ❌ | ✅ | atomic check+install window |
| Cached malware | ⚠️ | ✅ | Cache validation, TTL on UNVERIFIABLE |
| Developer social engineering | ⚠️ | ⚠️ | Blocks technical attacks, not social |

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
  run: npx -y -p sentinel-check ci
```

**CI mode enforces:**
- ❌ No UNVERIFIABLE packages (even for old/obscure packages)
- ❌ No installation without explicit allowance
- ✅ JSON report for audit trail
- ✅ Process exits non-zero on any failure

### Cache Behavior

Sentinel caches verification results locally at `~/.cache/sentinel/`:

| Status | TTL | Cache? | Behavior |
|--------|-----|--------|----------|
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

### npm Package (sentinel-check)

The npm wrapper (`sentinel-check`) ships a pre-compiled binary:

```bash
# Install via npm (recommended for CI)
npm install -g sentinel-check

# Use via npx (no installation)
npx -y -p sentinel-check ci
```

## Reporting Security Issues

If you discover a vulnerability:

1. **Do NOT open a public GitHub issue**
2. Email: [security@sig-sentinel.org](mailto:security@sig-sentinel.org)
3. Include:
   - Description of vulnerability
   - Steps to reproduce
   - Suggested fix (if any)
   - Your contact information

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

Sentinel is a supply chain security tool—we practice what we preach:

- **No unsafe code** — 100% safe Rust
- **No external processes** — no shell injection vectors
- **No log files** — no sensitive data leak via logs
- **No telemetry** — fully local computation
- **Open source** — code available for audit

## Limitations

### What sentinel does NOT protect against:

1. **Social engineering** — if developer manually installs malicious package
2. **Compromised npm account** — if package maintainer is hacked (but we catch the tarball diff)
3. **Registry operator compromise** — we only verify against their published hashes
4. **Old/obscure packages** — registry may not have integrity data (UNVERIFIABLE status)
5. **Post-installation exploits** — if package contains 0-day vulnerability

### Complementary tools:

- `npm audit` — vulnerability scanning in dependencies
- `snyk` / `Dependabot` — automated vulnerability monitoring
- `SBOM tools` — software bill of materials
- Code review — inspect source code before trusting

## FAQ

### Q: Why not use GPG signatures?

**A:** npm doesn't sign packages (only hashes). GPG would add trust assumptions without solving the core problem (which we solve via hash verification).

### Q: What if npm registry is down?

**A:** Sentinel marks packages as `UNVERIFIABLE`. In CI, this blocks install (safe-fail). You can `--allow-unverifiable` in development (local only).

### Q: Is sentinel production-ready?

**A:** Yes, for:
- ✅ Development environments (catch issues before commit)
- ✅ CI/CD gates (prevent supply chain attacks)
- ✅ Audit workflows (compliance + security)

### Q: What's the performance impact?

**A:** ~2-5 seconds per install, dominated by registry queries. Cached results are instant.

## Version History

- **v0.1.0** (2026-04-07)
  - Initial release
  - Lockfile + tarball verification
  - Cache with TTL
  - CI mode with strict enforcement
  - JSON/JUnit/GitHub output formats
