# Threat Model

This document defines what Sentinel protects, what it does not protect, and why its model differs from list-only package blocking.

## Scope

Sentinel is a pre-installation integrity gate for JavaScript dependency workflows.

Primary scope:

- Three-source integrity verification (lockfile, registry metadata, downloaded tarball)
- Guarded install flow for npm, Yarn, and pnpm
- TOCTOU protection between verification and installation

Out of scope:

- Source-code intent review or static analysis of package contents
- Runtime exploit detection in application code
- Full vulnerability management replacement (CVE feeds, advisory databases)

## Three-source verification model

Sentinel requires **three independent assertions** to agree before allowing install:

```text
1. lockfile says:      pkg@1.2.3 has integrity sha512-A
2. registry says:      pkg@1.2.3 has integrity sha512-A
3. downloaded tarball:  computed hash = sha512-A
```

If any source diverges, installation is blocked. The strength comes from cross-checking three independent sources — not from trusting any single one.

### What each check catches

| Divergence pattern | Caught? | Attack scenario |
| --- | --- | --- |
| lockfile ≠ registry | ✅ | Lockfile tampered locally |
| registry ≠ tarball | ✅ | CDN compromise, MITM, tarball replaced after publish |
| lockfile ≠ tarball | ✅ | Registry metadata updated but tarball swapped |
| all three agree on malicious content | ❌ | Maintainer compromise (see "Registry trust root" below) |

## Assets

Sentinel protects the integrity of:

- dependency lockfiles: `package-lock.json`, `yarn.lock`, `pnpm-lock.yaml`
- registry metadata used to validate lockfile entries
- package tarballs fetched for installation
- CI gate outputs used for enforcement and audits

## Trust boundaries

1. Local workspace files (`package.json`, lockfile)
2. Package manager invocation (`npm`, `yarn`, `pnpm`)
3. Remote metadata/tarball retrieval from registry endpoints
4. Sentinel local cache

Sentinel verifies consistency across these boundaries before allowing install.

## Threats and controls

| Threat | Example | Sentinel control |
| --- | --- | --- |
| Lockfile tampering | Lockfile integrity value changed to force poisoned artifact | Lockfile entry is checked against registry metadata and tarball |
| Tarball mismatch | Tarball content differs from expected integrity | Tarball hash is computed in-stream and compared against lockfile and registry |
| TOCTOU mutation | Lockfile changed after verification, before install | Sentinel re-checks lockfile hash before executing clean install |
| Registry outage/timeout | Metadata not available during check | Package marked `UNVERIFIABLE`, install is blocked in strict CI mode |
| Stale local state | Old cached state reused too long | CLEAN cache TTL = 1 hour; UNVERIFIABLE cache TTL = 30 seconds |
| CDN compromise | Registry metadata is correct but CDN serves altered tarball | Three-source check detects: computed hash ≠ lockfile/registry integrity |

## Registry trust root — explicit limitation

**The npm registry is still the trust root, and it can be compromised.**

If the registry itself is compromised — meaning an attacker gains control of a maintainer account and publishes a malicious version through the normal `npm publish` flow — then:

- The registry serves malicious tarball **and** updates `dist.integrity` to match
- The lockfile records the new integrity on `npm install`
- All three sources agree on the malicious content
- Sentinel sees consistency and returns `CLEAN`

**This is the scenario the Codecov/ua-parser-js/event-stream attacks exploited.** The attacker published through legitimate channels. Sentinel would not have blocked these specific attacks because the integrity chain was internally consistent.

### Why this matters

This is not a flaw in the verification model — it is a fundamental property of hash-based integrity checking. A hash proves "this is what was published" — it cannot prove "what was published is safe."

### What Sentinel adds beyond built-in verification

`npm ci` already verifies tarball integrity against the lockfile (2-source). Sentinel's third source (registry metadata) and operational model add value in these scenarios:

- **Lockfile injection** — a lockfile is modified to point to a different hash/URL than what the registry publishes. `npm ci` trusts the lockfile blindly; Sentinel cross-checks against registry metadata.
- **Pre-install isolation** — `npm ci` installs and runs lifecycle scripts per-package as each is verified. Sentinel verifies the entire tree before any install or script execution.
- **TOCTOU protection** — Sentinel re-hashes the lockfile between verification and install. `npm ci` does not.
- **CDN-only tarball replacement** — if a CDN serves an altered tarball but registry metadata still has the original hash, `npm ci` catches this (tarball ≠ lockfile). Sentinel also catches it and additionally detects if the lockfile was updated to match the altered tarball.

Blacklist-only tools (npm audit, Snyk) cannot close these gaps because they require prior knowledge of the compromise.

## Comparison of defense models

| Model | Detects | Does not detect |
| --- | --- | --- |
| Blacklist (npm audit, Snyk) | Known compromised versions (post-advisory) | Zero-day, integrity divergence |
| Built-in integrity (`npm ci`) | Tarball ≠ lockfile (2-source) | Lockfile injection, CDN-only replacement with metadata match |
| Three-source verification (Sentinel) | Any integrity divergence across lockfile, registry, and tarball | Malicious but consistently published packages |
| Static analysis (Socket, Phylum) | Suspicious behavior in code | Sophisticated obfuscation |
| Cooldown/age gate (Renovate) | Fast-published compromises (< 21d) | Malware in old packages |
| Provenance/SLSA | Un-attested builds | Compromised maintainer with valid attestation |

`npm ci` already provides 2-source integrity verification (tarball vs lockfile). Sentinel adds the registry metadata cross-check, the pre-install gate (all verified before any install), and TOCTOU protection. The strongest defense combines hash verification + static analysis + cooldown, because each closes the gap of the others.

## What Sentinel adds beyond `npm ci`

`npm ci` performs per-package tarball verification during install. Sentinel adds three things:

1. **Third cross-check** — the registry `dist.integrity` is compared against lockfile and tarball, catching lockfile injection where a manipulated lockfile points to a URL serving a tarball that matches the injected hash but differs from what the registry publishes.
2. **Pre-install gate** — `npm ci` verifies and installs atomically per package, meaning lifecycle scripts for package A can execute before package B is verified. Sentinel verifies **all** packages before installing **any**.
3. **TOCTOU re-check** — Sentinel re-hashes the lockfile between verification and install to detect concurrent mutation.

This means Sentinel can block integrity divergence even when there is no existing blocklist entry for that package version, and it prevents lifecycle scripts from executing on a partially-verified tree.

## Concrete scenario

Scenario:

1. `acme-lib@3.2.1` is not present in any public threat feed yet.
2. Lockfile expects integrity `sha512-A...`.
3. Retrieved metadata or tarball integrity resolves to `sha512-B...`.
4. Sentinel returns `COMPROMISED` and blocks install.

A list-only approach cannot block this unless that exact compromised version was previously reported and indexed.

## Real-world incidents (2025-2026)

The following supply chain attacks demonstrate where three-source verification provides protection and where it does not. Two defense mechanisms apply:

1. **Lockfile workflow enforcement** — `sentinel ci` uses frozen installs (`npm ci` / `yarn --frozen-lockfile` / `pnpm --frozen-lockfile`), so lockfile-pinned versions are installed regardless of what new versions exist in the registry.
2. **Hash divergence detection** — when an existing version's tarball is replaced, the lockfile integrity no longer matches the registry/tarball. Sentinel detects this and returns `COMPROMISED`.

Both mechanisms require a **committed lockfile that pre-dates the compromise** and **frozen installs**. If a developer runs `npm install` during the attack window, the lockfile records the malicious version and all three sources agree — Sentinel passes.

### Axios — March 2026

**Reference:** [Microsoft Security Blog, April 1 2026](https://www.microsoft.com/en-us/security/blog/2026/04/01/mitigating-the-axios-npm-supply-chain-compromise/)

Sapphire Sleet (North Korean state actor) compromised maintainer credentials and published `axios@1.14.1` and `axios@0.30.4` with a fake dependency (`plain-crypto-js@4.2.1`) that executed a RAT via `postinstall`. The Axios source code was unchanged — only the dependency list was modified.

**Defense mechanism:** Lockfile workflow enforcement. These were new version numbers, not modifications of existing versions. Projects with a pre-attack lockfile pinning a safe version (e.g., `1.14.0`) are protected because `sentinel ci` runs a frozen install that respects the lockfile pin. There is no hash divergence to detect — the safe version remains intact in the registry.

### Shai-Hulud — September 2025

**Reference:** CISA Alert, September 2025; Socket Research, September 2025

Attackers compromised maintainers via phishing, then used an automated tool to download existing tarballs, inject a malicious `bundle.js` into `package.json`, repackage, and republish. Over 500 packages were trojanized, including `@ctrl/tinycolor` and `ngx-bootstrap`.

**Defense mechanism:** Hash divergence detection. The republished tarball has a different hash from the original. Lockfile integrity ≠ registry `dist.integrity` → Sentinel returns `COMPROMISED` and blocks install before any lifecycle script runs.

### Chalk / Debug / ansi-styles — September 2025

**Reference:** Qualys Threat Research, September 12 2025

On September 8 2025, attackers compromised 18 widely-used packages — chalk, debug, ansi-styles, strip-ansi — with 2.6 billion combined weekly downloads. The payload intercepted cryptocurrency wallet transactions via obfuscated JavaScript.

**Defense mechanism:** Hash divergence detection. Republished tarballs produce different hashes. Incident response guidance confirms: check lockfiles for compromised versions and audit private registries for cached contaminated packages.

### SHA1-Hulud — November 2025

**Reference:** Snyk Research, November 26 2025

Second wave of the Shai-Hulud concept. This variant switched from `postinstall` to `preinstall` hooks to expand the attack surface.

**Defense mechanism:** Hash divergence detection. The shift from `postinstall` to `preinstall` does not bypass Sentinel — verification completes before any install command executes, so no lifecycle script runs.

### S1ngularity / Nx — August 2025

**Reference:** Sonatype Research, September 17 2025

Attackers stole the Nx project's publishing token and published malicious versions of multiple Nx packages to exfiltrate sensitive data during a four-hour window.

**Defense mechanism:** Lockfile workflow enforcement. The stolen token was used to publish new versions. Projects with a pre-attack lockfile pinning safe versions are protected by the frozen install flow.

### Lockfile injection via pull request

**Reference:** Snyk Research, 2019 (attack vector still active)

An attacker with PR access replaces a package's resolution URL in the lockfile to point to a controlled repository and adjusts the SHA-512 integrity field to match the malicious tarball.

**Sentinel verdict: not caught.** All three sources agree — the attacker controls the lockfile hash, the resolution endpoint, and the served tarball. Code review of lockfile diffs is the only defense against this vector.

### Coverage summary

| Incident | Date | Vector | Sentinel blocks? | Defense mechanism |
| --- | --- | --- | --- | --- |
| Axios | Mar 2026 | Maintainer compromise, new version | ✅ | Lockfile workflow (frozen install pins safe version) |
| Shai-Hulud | Sep 2025 | Phishing + tarball replacement | ✅ | Hash divergence (lockfile ≠ registry) |
| Chalk/Debug/ansi-styles | Sep 2025 | Account compromise, 18 packages | ✅ | Hash divergence (lockfile ≠ registry) |
| SHA1-Hulud | Nov 2025 | Variant with preinstall hooks | ✅ | Hash divergence (blocks before scripts) |
| S1ngularity/Nx | Aug 2025 | Stolen publishing token | ✅ | Lockfile workflow (frozen install pins safe version) |
| Lockfile injection via PR | Ongoing | Malicious lockfile modification | ❌ | — |
| Maintainer publishes consistent malicious version | Ongoing | Registry trust root compromise | ❌ | — |

**Note on SHA-1:** Older `package-lock.json` formats using SHA-1 integrity are vulnerable to collision attacks. Sentinel uses SHA-512 for all tarball verification — this class of attack does not apply.

## Residual risks

Sentinel does not solve:

- **Maintainer compromise with consistent publication** — if registry metadata and tarball both reflect the malicious content published through a compromised account, all three sources agree and Sentinel passes. This is the most dangerous scenario and requires complementary tools (static analysis, provenance checks, code review).
- **Malicious but internally consistent packages** — a package that is intentionally malicious from its first publish will have consistent hashes across all three sources.
- **Social engineering decisions by developers** — technical verification does not solve human trust decisions.
- **Downstream application vulnerabilities** — Sentinel verifies package integrity, not package safety.
- **Packages without sufficient metadata** — old packages without integrity fields become `UNVERIFIABLE`.

## Recommended complementary controls

Use Sentinel together with:

- **Static analysis** (Socket, Phylum) — detects suspicious behavior inside packages
- **Vulnerability scanners** (npm audit, Snyk, Dependabot) — known CVE detection
- **Provenance/SLSA** — build attestation verification
- **Code review** — inspect source code before trusting
- **Least-privilege runtime controls** — limit what installed packages can do

## Evidence outputs

For CI/CD and audit, Sentinel supports:

- text output for humans
- JSON output for machine processing
- JUnit output for test dashboards
- GitHub annotations format for workflow surfaces

This allows policy enforcement and post-run evidence without external SaaS requirements.
