# Threat Model

This document defines what Sentinel protects, what it does not protect, and why its model differs from list-only package blocking.

## Scope

Sentinel is a pre-installation integrity gate for JavaScript dependency workflows.

Primary scope:

- Lockfile integrity verification
- Registry metadata consistency verification
- Tarball integrity verification
- Guarded install flow for npm, Yarn, and pnpm

Out of scope:

- Source-code intent review
- Runtime exploit detection in application code
- Full vulnerability management replacement

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
| Lockfile tampering | Lockfile integrity value changed to force poisoned artifact | Lockfile entry is checked against registry metadata |
| Tarball mismatch | Tarball content differs from expected integrity | Tarball hash verification blocks install |
| TOCTOU mutation | Lockfile changed after verification, before install | Sentinel re-checks lockfile hash before executing clean install |
| Registry outage/timeout | Metadata not available during check | Package marked `UNVERIFIABLE`, install is blocked in strict CI mode |
| Stale local state | Old cached state reused too long | Cache policy limits reuse for unverifiable outcomes |

## Why Sentinel is not list-only

A known-bad package list has one hard requirement: the package/version must already be known and listed.

Sentinel's model is different:

1. Verify what lockfile claims
2. Verify what registry metadata claims
3. Verify what tarball actually contains
4. Block if these claims diverge

This means Sentinel can block integrity divergence even when there is no existing blocklist entry for that package version.

## Concrete scenario

Scenario:

1. `acme-lib@3.2.1` is not present in any public threat feed yet.
2. Lockfile expects integrity `sha512-A...`.
3. Retrieved metadata or tarball integrity resolves to `sha512-B...`.
4. Sentinel returns `COMPROMISED` and blocks install.

A list-only approach cannot block this unless that exact compromised version was previously reported and indexed.

## Residual risks

Sentinel does not solve:

- malicious but internally consistent packages (metadata and tarball both malicious)
- social engineering decisions by developers
- downstream application vulnerabilities unrelated to package integrity

Use Sentinel together with vulnerability scanners, code review, and least-privilege runtime controls.

## Evidence outputs

For CI/CD and audit, Sentinel supports:

- text output for humans
- JSON output for machine processing
- JUnit output for test dashboards
- GitHub annotations format for workflow surfaces

This allows policy enforcement and post-run evidence without external SaaS requirements.
