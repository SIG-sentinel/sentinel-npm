pub const OUTPUT_STATUS_ALL_CLEAN: &str = "✓ all clean";
pub const OUTPUT_STATUS_WARNINGS: &str = "⚠ warnings";
pub const OUTPUT_STATUS_BLOCKED: &str = "✗ blocked";
pub const OUTPUT_SUMMARY_LINE_TEMPLATE: &str =
    "  {}  total: {}  clean: {}  unverifiable: {}  compromised: {}";
pub const OUTPUT_PROVENANCE_SUMMARY_TEMPLATE: &str =
    "  provenance: trusted={}  warning={}  inconsistent={}  coverage={}%  availability={}%";
pub const OUTPUT_PROVENANCE_MISSING_SUPPRESSED_TEMPLATE: &str =
    "  ... +{} packages without provenance hidden (showing top {})";

pub const OUTPUT_REASON_NO_INTEGRITY_FIELD: &str = "no integrity field (old package)";
pub const OUTPUT_REASON_LEGACY_SHA1_LOCKFILE: &str = "legacy sha1 lockfile integrity";
pub const OUTPUT_REASON_REGISTRY_OFFLINE: &str = "registry offline";
pub const OUTPUT_REASON_REGISTRY_TIMEOUT: &str = "registry timeout";
pub const OUTPUT_REASON_MISSING_FROM_LOCKFILE: &str = "not in lockfile";
pub const OUTPUT_REASON_TARBALL_TOO_LARGE: &str = "tarball too large";
pub const OUTPUT_REASON_PROVENANCE_MISSING: &str = "provenance not available";
pub const OUTPUT_REASON_PROVENANCE_INCONSISTENT: &str = "provenance inconsistent";

pub const OUTPUT_NEXT_ACTION_COMPROMISED: &str =
    "  {} escalate each compromised package above to your security process with evidence.";
pub const OUTPUT_NEXT_ACTION_GITHUB_CI: &str =
    "  {} run 'sentinel check --format github' to get inline CI annotations.";
pub const OUTPUT_NEXT_ACTION_OLD_PACKAGE_DIRECT: &str = "  {} [direct] remove the flagged package(s) from package.json, regenerate the lockfile, then run 'sentinel check'.";
pub const OUTPUT_NEXT_ACTION_OLD_PACKAGE_TRANSITIVE: &str = "  {} [transitive] run 'npm ls <package>' to find which direct dep pulls it in, then upgrade that dep and run 'sentinel check'.";
pub const OUTPUT_NEXT_ACTION_LEGACY_SHA1_LOCKFILE: &str = "  {} lockfile uses legacy sha1 integrity entries — delete lockfile, reinstall dependencies, then run 'sentinel ci'.";
pub const OUTPUT_NEXT_ACTION_LOCKFILE_STALE_DIRECT: &str = "  {} [direct] lockfile is out of sync — prefer 'sentinel ci --init'.\n  {} manual fallback: 'npm install --package-lock-only'. Then run 'sentinel check'.";
pub const OUTPUT_NEXT_ACTION_LOCKFILE_STALE_TRANSITIVE: &str = "  {} [transitive] lockfile is out of sync — prefer 'sentinel ci --init'.\n  {} manual fallback: 'npm install --package-lock-only'. Then run 'sentinel check'.";
pub const OUTPUT_NEXT_ACTION_REGISTRY_UNAVAILABLE: &str =
    "  {} registry unreachable — retry later, then run 'sentinel check'.";
pub const OUTPUT_NEXT_ACTION_PROVENANCE_MISSING: &str = "  {} provenance is missing for one or more packages — this is a warning in this phase; track coverage and prefer trusted publishers when possible.";
pub const OUTPUT_NEXT_ACTION_PROVENANCE_INCONSISTENT: &str = "  {} provenance is inconsistent — block and investigate package publisher identity or artifact mismatch before proceeding.";
pub const OUTPUT_NEXT_ACTION_STRICT_CI: &str = "  {} unverifiable packages are always blocked — fix the dependency state, then run 'sentinel check' again.";
pub const OUTPUT_NEXT_ACTION_INSTALL_DEFAULT: &str =
    "  {} use 'sentinel install <package>@<exact-version>' for safer single-package installs.";
pub const OUTPUT_NEXT_ACTION_CI_DEFAULT: &str =
    "  {} use 'sentinel ci' in pipelines for strict verification and reports.";

pub const OUTPUT_JSON_SERIALIZATION_ERROR_TEMPLATE: &str =
    "{\"error\": \"serialization failed: {}\"}";

pub const OUTPUT_GITHUB_SUMMARY_COMPROMISED_TEMPLATE: &str =
    "::error title=sentinel-summary::sentinel found {} compromised package(s)";
pub const OUTPUT_GITHUB_SUMMARY_UNVERIFIABLE_TEMPLATE: &str =
    "::notice title=sentinel-summary::sentinel: {} package(s) could not be verified";
pub const OUTPUT_GITHUB_SUMMARY_CLEAN_TEMPLATE: &str =
    "::notice title=sentinel-summary::sentinel: all {} packages verified clean";

pub const OUTPUT_INSTALL_BLOCKED_TEMPLATE: &str = "  {} sentinel is blocking this install:\n";
pub const OUTPUT_INSTALL_BLOCKED_HINT_COMPROMISED: &str =
    "    escalate package: {}@{} (open security incident + attach sentinel output)";
pub const OUTPUT_INSTALL_BLOCKED_NEXT_HEADER: &str = "\n  To proceed, review each package above:";
pub const OUTPUT_LABEL_DIRECT: &str = "direct";
pub const OUTPUT_LABEL_TRANSITIVE: &str = "transitive";
pub const OUTPUT_LABEL_PARENT_HINT: &str = "└─ via";

pub const OUTPUT_INSTALL_BLOCKED_GUIDANCE_NO_INTEGRITY_DIRECT: &str = "  This is a DIRECT dependency that predates npm integrity fields and cannot be verified.\n\
    Recommended:\n\
    1. Replace or remove the package in package.json\n\
    2. Delete the lockfile\n\
    3. Reinstall with your package manager to regenerate the lockfile\n\
    4. Re-run: sentinel install <package>@<exact-version>";
pub const OUTPUT_INSTALL_BLOCKED_GUIDANCE_NO_INTEGRITY_TRANSITIVE: &str = "  This is a TRANSITIVE dependency that predates npm integrity fields and cannot be verified.\n\
    It was pulled in by another package — you cannot remove it directly.\n\
    Recommended:\n\
    1. Identify which direct dependency requires it (use 'npm ls <package>')\n\
    2. Replace or upgrade that direct dependency\n\
    3. Delete the lockfile and reinstall to regenerate it\n\
    4. Re-run: sentinel install <package>@<exact-version>";
pub const OUTPUT_INSTALL_BLOCKED_GUIDANCE_NOT_IN_LOCKFILE_DIRECT: &str = "  This DIRECT dependency is missing from the lockfile. Regenerate it:\n\
    sentinel ci --init\n\
    (manual fallback: npm install --package-lock-only)\n\
    Then re-run: sentinel install <package>@<exact-version>";
pub const OUTPUT_INSTALL_BLOCKED_GUIDANCE_NOT_IN_LOCKFILE_TRANSITIVE: &str = "  This TRANSITIVE dependency is missing from the lockfile.\n\
    The lockfile may be out of sync with package.json.\n\
    Run: sentinel ci --init\n\
    (manual fallback: npm install --package-lock-only)\n\
    Then re-run: sentinel install <package>@<exact-version>";

pub const OUTPUT_INSTALL_BLOCKED_GUIDANCE_NO_INTEGRITY: &str = "  This package predates npm integrity fields and cannot be verified.\n\
    Recommended:\n\
    1. Remove the package from package.json\n\
    2. Delete the lockfile\n\
    3. Reinstall with your package manager to regenerate the lockfile\n\
    4. Re-run: sentinel install <package>@<exact-version>";
pub const OUTPUT_INSTALL_BLOCKED_GUIDANCE_LEGACY_SHA1_LOCKFILE: &str = "  This lockfile contains legacy sha1 integrity entries and sentinel does not validate sha1 packages.\n\
    Recommended:\n\
    1. Delete the lockfile\n\
    2. Reinstall dependencies to regenerate sha512 integrity entries\n\
    3. Run: sentinel ci";
pub const OUTPUT_INSTALL_BLOCKED_GUIDANCE_REGISTRY_UNAVAILABLE: &str = "  Cannot verify — npm registry is unreachable.\n\
    Retry when the registry is available, then re-run: sentinel install <package>@<exact-version>";
pub const OUTPUT_INSTALL_BLOCKED_GUIDANCE_NOT_IN_LOCKFILE: &str = "  Package not tracked in lockfile. Regenerate lockfile first:\n\
    sentinel ci --init\n\
    (manual fallback: npm install --package-lock-only)\n\
    Then re-run: sentinel install <package>@<exact-version>";
pub const OUTPUT_INSTALL_BLOCKED_GUIDANCE_PROVENANCE_INCONSISTENT: &str = "  Provenance exists but is inconsistent with the downloaded artifact.\n\
    Treat as potential supply-chain tampering until proven otherwise.\n\
    Verify package publisher identity and re-run after remediation.";
pub const OUTPUT_INSTALL_BLOCKED_GUIDANCE_COMPROMISED: &str = "  Integrity mismatch — this package may have been tampered with.\n\
    Investigate:";
pub const OUTPUT_INSTALL_DETAIL_LINES: usize = 3;

pub const OUTPUT_LABEL_COMPROMISED: &str = "COMPROMISED";
pub const OUTPUT_SYMBOL_WARNING: &str = "⚠";
pub const OUTPUT_SYMBOL_ERROR: &str = "✗";
pub const OUTPUT_GITHUB_ERROR_TITLE: &str = "sentinel-compromised";
pub const OUTPUT_GITHUB_WARNING_TITLE: &str = "sentinel-unverifiable";
pub const OUTPUT_GITHUB_LOCKFILE_REF: &str = "lockfile";
pub const OUTPUT_GITHUB_ERROR_FORMAT: &str = "::error title={},file={}::{} — {}";
pub const OUTPUT_XML_HEADER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>"#;
pub const OUTPUT_XML_TESTSUITES: &str =
    r#"<testsuites name="sentinel" tests="{}" errors="{}" failures="0" warnings="{}">"#;
pub const OUTPUT_XML_TESTSUITE: &str =
    r#"  <testsuite name="supply-chain-integrity" tests="{}" errors="{}">"#;
pub const OUTPUT_XML_TESTCASE_CLEAN: &str =
    r#"    <testcase classname="sentinel.integrity" name="{}"/>"#;
pub const OUTPUT_XML_TESTCASE_UNVERIFIABLE: &str =
    r#"    <testcase classname="sentinel.integrity" name="{}">"#;
pub const OUTPUT_XML_TESTCASE_COMPROMISED: &str =
    r#"    <testcase classname="sentinel.integrity" name="{}">"#;
pub const OUTPUT_XML_ERROR: &str =
    r#"      <error message="COMPROMISED" type="SupplyChainAttack">{}</error>"#;
pub const OUTPUT_XML_TESTCASE_CLOSE: &str = r"    </testcase>";
pub const OUTPUT_XML_TESTSUITE_CLOSE: &str = "  </testsuite>";
pub const OUTPUT_XML_TESTSUITES_CLOSE: &str = "</testsuites>";
