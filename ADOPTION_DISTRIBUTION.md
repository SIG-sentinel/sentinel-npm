# Adoption and Distribution

This guide focuses on practical rollout for teams and clear packaging channels for users.

## Rolling out in CI (zero install)

Start in CI where policy is enforceable and measurable.

```yaml
- name: Verify dependency integrity
  run: npx --yes sentinel-check ci
```

Why this step first:

- no machine-level install required
- easy rollback in one workflow commit
- immediate evidence via JSON/JUnit/GitHub outputs

## Rolling out as team binary

For teams that run Sentinel daily, install the binary in PATH and use `sentinel` directly in scripts and local workflows.

### Team scripts in package.json

Standardize usage through npm scripts:

```json
{
  "scripts": {
    "sentinel:check": "npx --yes sentinel-check check",
    "sentinel:ci": "npx --yes sentinel-check ci"
  }
}
```

### Package manager note

The same `sentinel:ci` and `sentinel:check` scripts work in npm, Yarn, and pnpm projects because Sentinel detects lockfile/manager automatically.

## Airgap / restricted networks

Recommended approach for restricted environments:

1. Mirror release artifacts internally (binary + `checksums.txt`).
2. Set wrapper/binary configuration to use approved internal paths.
3. Run `sentinel check` and `sentinel ci` with local cache enabled.
4. If a controlled bootstrap is needed, use `sentinel ci --init` explicitly rather than relying on manual lockfile regeneration outside Sentinel.
5. Keep strict CI policy: block on `UNVERIFIABLE` and `COMPROMISED`.

Operational note:

- cache-assisted operation helps reduce registry dependency on repeated checks
- strict CI still fails when integrity cannot be verified

## Evidence checklist for enterprise reviews

Use this checklist during security/design reviews:

- [ ] threat model documented ([THREAT_MODEL.md](THREAT_MODEL.md))
- [ ] disclosure policy documented ([SECURITY.md](SECURITY.md))
- [ ] CI policy defined (`sentinel ci` blocks on `UNVERIFIABLE` and `COMPROMISED`)
- [ ] lockfile bootstrap policy defined (`sentinel ci --init` allowed only for controlled initialization/recovery)
- [ ] machine-readable reporting enabled (`--format json` and/or `--format junit`)
- [ ] release integrity verification process documented (checksums)

## Positioning guidance

Keep positioning short and concrete:

- Sentinel is a cryptographic integrity gate for lockfiles and tarballs.
- Sentinel supports npm, Yarn, and pnpm lockfiles with automatic detection.
- Sentinel supports cache-assisted operation, while strict CI still blocks unverifiable dependencies.
- Sentinel provides CI-native outputs (JSON, JUnit, GitHub annotations).

## Current distribution channels

- GitHub Releases (binary artifacts + checksums)
- npm wrapper package: `sentinel-check`
