# Security Policy

## Overview

This repository, `sentinel-npm`, publishes two related artifacts:

- `sentinel`: the Rust CLI that performs verification and installation gating
- `sentinel-check`: the npm wrapper used with `npx` and Node-based automation

This document explains the security model of the Sentinel npm workflow and the operational guarantees that exist today.

For attacker model and trust-boundary details, see [THREAT_MODEL.md](THREAT_MODEL.md).

## Security Model

Sentinel performs three-source integrity verification before allowing any package installation:

```text
THREE-SOURCE VERIFICATION (every package, every check):
  1. Read lockfile integrity for pkg@version
  2. Query registry metadata for pkg@version dist.integrity
  3. Download tarball and compute SHA-512 in stream
  4. All three must agree → CLEAN
  5. Any divergence → COMPROMISED, install blocked

INSTALLATION GATE:
  1. All packages verified (parallel, bounded concurrency)
  2. Lockfile hash re-checked for TOCTOU protection
  3. Package manager install executed (npm ci / yarn --frozen-lockfile / pnpm --frozen-lockfile)
  4. Any check fails → block installation

RUNTIME NOTE:
  - node_modules is only touched after full verification completes.
  - Lockfile synchronization/resolution can still touch project files before verification.
```

### What this model catches

- Lockfile tampered locally (lockfile ≠ registry)
- CDN/MITM serving altered tarball (tarball ≠ registry)
- Post-publication tarball replacement (tarball ≠ lockfile)
- Zero-day integrity divergence before any threat feed indexes it

### What this model does NOT catch

**Registry trust root compromise**: if an attacker publishes a malicious version through a compromised maintainer account, the registry serves consistent metadata and tarball. All three sources agree on the malicious content, and Sentinel returns CLEAN. This is the scenario exploited by event-stream, ua-parser-js, and Codecov attacks. See [THREAT_MODEL.md](THREAT_MODEL.md) for details.

## Threat Model

| Threat | `npm ci` | sentinel | Notes |
| --- | --- | --- | --- |
| Tarball ≠ lockfile | ✅ | ✅ | Both verify tarball integrity against lockfile |
| Lockfile injection (hash + URL manipulated) | ❌ | ✅ | Sentinel cross-checks against registry metadata |
| CDN/MITM compromise | ✅ | ✅ | Both detect tarball ≠ expected hash |
| Pre-install isolation (all before any) | ❌ | ✅ | npm ci installs per-package; Sentinel gates the full tree |
| TOCTOU between verify and install | ❌ | ✅ | Lockfile hash re-checked before install |
| Cached stale result reuse | ⚠️ | ✅ | CLEAN TTL = 1h, UNVERIFIABLE TTL = 30s |
| Registry trust root compromise | ❌ | ❌ | Consistent malicious publish passes all hash checks |
| Malicious but consistent package | ❌ | ❌ | Requires static analysis (Socket, Phylum) |
| Developer social engineering | ⚠️ | ⚠️ | Technical verification does not solve human trust decisions |

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
| CLEAN | 1 hour | ✅ | Re-check after TTL to detect post-cache compromise |
| UNVERIFIABLE | 30 sec | ✅ | Re-check quickly, minimize exploit window |
| COMPROMISED | — | ❌ | Never cache (always block) |

Cache TTLs are intentionally short to reduce the window where a compromised package could be served from stale cache.

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

1. **Registry trust root compromise** — if an attacker publishes through a compromised maintainer account, registry metadata and tarball are both malicious but consistent. All three sources agree. This is the most dangerous supply chain scenario (event-stream, ua-parser-js, Codecov) and requires static analysis or provenance checks.
2. **Malicious but consistently published packages** — a package that is intentionally malicious from first publish will have matching hashes across all sources.
3. **Social engineering** — if a developer bypasses the tooling or trusts a malicious package intentionally.
4. **Packages without sufficient metadata** — old packages without integrity fields become `UNVERIFIABLE`.
5. **Post-installation runtime vulnerabilities** — Sentinel verifies integrity, not safety.

### Complementary tools (recommended)

- **Socket / Phylum** — static analysis of package behavior (closes the "consistent malicious publish" gap)
- **npm audit / Snyk / Dependabot** — known CVE detection
- **SLSA / Sigstore provenance** — build attestation verification
- **SBOM tools** — software bill of materials
- **Code review** — inspect source code before trusting

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

**A:** First run downloads and hashes every tarball (parallel, bounded concurrency). Subsequent runs within cache TTL (1 hour for CLEAN) use cached results and are near-instant. Typical CI time: 10-30 seconds depending on dependency count and network.

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
