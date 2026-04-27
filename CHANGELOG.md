# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.1] - 2026-04-27

### Changed

- Release validation workflow now tolerates `ProvenanceMissing` outcomes during binary smoke validation.
- Release validation still blocks on real integrity/security failures (for example, compromised artifacts or operational errors).

### Security

- Preserved blocking behavior for security-significant failures while reducing false negatives from ecosystem provenance gaps.

## [2.0.0] - 2026-04-23

### Breaking Changes

#### CLI Script Policy (`ci` and `install`)

- Lifecycle scripts are now blocked by default in `sentinel ci` and `sentinel install`.
- To enable scripts for projects that require hooks, use `--allow-scripts` explicitly.
- This flips the default from opt-out to secure-by-default opt-in behavior.

#### Installation Script (`install.sh`)

**Explicit version required:**

- The `--version` flag is now **mandatory**. If omitted, the installer fails with an error.
- The implicit `latest` default has been removed entirely to prevent unexpected version drift.
- **Migration:** Pass `--version 2.0.0` or your desired version explicitly.

**Signature verification is now mandatory:**

- The installer now **requires** the release to have a `checksums.txt.sig` file signed with EC-256 private key.
- Without a valid signature, installation fails.
- This is a security hardening: bootstrap attacks on unsigned artifacts are now blocked.
- **Migration:** Ensure your CI/release process publishes `checksums.txt.sig` alongside binaries.

**Examples of new patterns:**

```bash
# ✅ New pattern (specify version explicitly)
curl -fsSL https://raw.githubusercontent.com/SIG-sentinel/sentinel-npm/main/scripts/install.sh | \
  sh -s -- --version 2.0.0

# ❌ Old pattern (no longer works — version is required)
curl -fsSL https://raw.githubusercontent.com/SIG-sentinel/sentinel-npm/main/scripts/install.sh | sh
```

#### Security Enhancements

- **EC-256 signature verification:** All release artifacts are signed with an EC private key stored in GitHub Actions secrets.
- **Checksum + signature:** The installer now validates both SHA-256 checksum **and** cryptographic signature.
- **No fallback mode:** Missing or invalid signatures cause hard failure, not graceful fallback.

### Changed

- Installation script (`scripts/install.sh`) now enforces explicit version and mandatory signature validation.
- Release workflow (`release.yml`) now generates EC-256 signatures for all releases.
- README and documentation updated with new installation patterns.
- Quick release script updated to include version in user-facing install commands.

### Added

- EC-256 signature files (`checksums.txt.sig`) are now published with every release.
- Signature verification step in `install.sh` using `openssl dgst -sha256 -verify`.

### Security

- **Bootstrap hardening:** Installation now requires explicit version pinning and cryptographic signature verification.
- **Zero new dependencies:** Verification uses standard `openssl`, no external tools required beyond what's already on most systems.
- **Transparency:** Release signatures are verifiable by the public against the hardcoded public key in the install script.

---

## [1.2.3] - 2026-04-20

### Features

- Post-verify cached fingerprint support (no re-download of tarballs during post-verify).
- Symlink asymmetry fix in fingerprint computation.
- Enhanced history command with ledger retention policies.
- Memory budget tracking for safety in large monorepos.
- Artifact cleanup and optimization.

### Fixed

- Corrected symlink handling in tarball fingerprint to match installed package fingerprint computation.

### Tested

- Full smoke test suite across npm, yarn, and pnpm.
- 152+ unit tests passing.
