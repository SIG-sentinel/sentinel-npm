pub const CLI_NAME: &str = "sentinel";
pub const CLI_PREFIX_SENTINEL: &str = "sentinel:";
pub const CLI_PREFIX_WARNING: &str = "warning:";

pub const NPM_ARG_CI: &str = "ci";
pub const NPM_ARG_PACKAGE_LOCK_ONLY: &str = "--package-lock-only";
pub const NPM_ARG_SAVE_EXACT: &str = "--save-exact";
pub const NPM_ARG_OMIT_DEV: &str = "--omit=dev";
pub const NPM_ARG_OMIT_OPTIONAL: &str = "--omit=optional";

pub const NPM_ARG_FROZEN_LOCKFILE: &str = "--frozen-lockfile";
pub const NPM_ARG_EXACT: &str = "--exact";
pub const NPM_ARG_LOCKFILE_ONLY: &str = "--lockfile-only";
pub const NPM_ARG_MODE_UPDATE_LOCKFILE: &str = "--mode=update-lockfile";
pub const NPM_ARG_PROD: &str = "--prod";
pub const NPM_ARG_REPORTER_SILENT: &str = "--reporter=silent";

pub const CLI_LONG_ABOUT: &str = "
sentinel verifies npm packages by comparing their sha512 hash against
npm's own dist.integrity field — before or after installation.

COMMANDS:
  check    Audit installed packages (compares lockfile vs npm registry)
  install  Download, verify, then install (closes TOCTOU attack window)
  ci       CI mode: strict + fail-on-warn + JSON report
  report   Report a suspicious package

SECURITY MODEL:
  - CLEAN:        sha512(tarball) matches npm dist.integrity — safe
  - UNVERIFIABLE: cannot confirm (old package, registry offline) — block
  - COMPROMISED:  hash mismatch — NEVER install — report immediately

SOURCE OF TRUTH:
  npm dist.integrity is set at publish time and is immutable in practice.
  We verify the tarball you're installing matches what npm published.
  No proprietary database required.
";
