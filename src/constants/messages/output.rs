pub const OUTPUT_STATUS_ALL_CLEAN: &str = "✓ all clean";
pub const OUTPUT_STATUS_WARNINGS: &str = "⚠ warnings";
pub const OUTPUT_STATUS_BLOCKED: &str = "✗ blocked";
pub const OUTPUT_SUMMARY_LINE_TEMPLATE: &str =
    "  {}  total: {}  clean: {}  unverifiable: {}  compromised: {}";

pub const OUTPUT_REASON_NO_INTEGRITY_FIELD: &str = "no integrity field (old package)";
pub const OUTPUT_REASON_REGISTRY_OFFLINE: &str = "registry offline";
pub const OUTPUT_REASON_REGISTRY_TIMEOUT: &str = "registry timeout";
pub const OUTPUT_REASON_MISSING_FROM_LOCKFILE: &str = "not in lockfile";
pub const OUTPUT_REASON_TARBALL_TOO_LARGE: &str = "tarball too large";

pub const OUTPUT_NEXT_ACTION_COMPROMISED: &str =
    "  {} run 'sentinel report <package>' for each compromised package.";
pub const OUTPUT_NEXT_ACTION_GITHUB_CI: &str =
    "  {} re-run with 'sentinel check --format github' in CI.";
pub const OUTPUT_NEXT_ACTION_STRICT_CI: &str =
    "  {} sentinel only installs verified packages — unverifiable packages are always blocked.";
pub const OUTPUT_NEXT_ACTION_STRICT_INSTALL: &str =
    "  {} use 'sentinel install' — unverifiable packages are always blocked.";
pub const OUTPUT_NEXT_ACTION_INSTALL_DEFAULT: &str =
    "  {} use 'sentinel install' as safer replacement for npm install.";
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
pub const OUTPUT_INSTALL_BLOCKED_HINT_COMPROMISED: &str = "    sentinel report {}@{}";
pub const OUTPUT_INSTALL_BLOCKED_NEXT_HEADER: &str = "\n  To proceed, review each package above:";
pub const OUTPUT_INSTALL_BLOCKED_GUIDANCE_NO_INTEGRITY: &str = "  This package predates npm integrity fields and cannot be verified.\n\
    Recommended:\n\
    1. Remove the package from package.json\n\
    2. Delete the lockfile\n\
    3. Reinstall with your package manager to regenerate the lockfile\n\
    4. sentinel install";
pub const OUTPUT_INSTALL_BLOCKED_GUIDANCE_REGISTRY_UNAVAILABLE: &str = "  Cannot verify — npm registry is unreachable.\n\
    Retry when the registry is available, then re-run: sentinel install";
pub const OUTPUT_INSTALL_BLOCKED_GUIDANCE_NOT_IN_LOCKFILE: &str = "  Package not tracked in lockfile. Regenerate lockfile first:\n\
    install with your package manager\n\
    Then re-run: sentinel install";
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
pub const OUTPUT_XML_TESTCASE_CLOSE: &str = r#"    </testcase>"#;
pub const OUTPUT_XML_TESTSUITE_CLOSE: &str = "  </testsuite>";
pub const OUTPUT_XML_TESTSUITES_CLOSE: &str = "</testsuites>";
