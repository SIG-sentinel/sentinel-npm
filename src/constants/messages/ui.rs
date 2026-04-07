pub const UI_LABEL_NEXT: &str = "Next:";
pub const UI_LABEL_TIP: &str = "Tip:";
pub const UI_LABEL_SENTINEL: &str = "sentinel:";

pub const UI_MSG_LOCKFILE_MISSING_NOTICE: &str =
    "\n  {}  package-lock.json not found. Running `npm install --package-lock-only`...\n";
pub const UI_MSG_LOCKFILE_CREATED_NOTICE: &str = "  {} package-lock.json created successfully\n";
pub const UI_MSG_RESOLVING_PACKAGE_TEMPLATE: &str =
    "\n  {}  resolving {} into package-lock.json before verification...\n";
pub const UI_MSG_RESOLVE_PACKAGE_INTO_LOCKFILE_FAILED_TEMPLATE: &str =
    "sentinel: Failed to resolve {} into lockfile";
pub const UI_MSG_DEPENDENCY_CYCLES_HEADER_TEMPLATE: &str = "{}  {} dependency cycles detected!";
pub const UI_MSG_DEPENDENCY_CYCLE_LINE_TEMPLATE: &str = "  Cycle {}: {}";
pub const UI_MSG_NO_PACKAGES_TO_VERIFY: &str = "No packages to verify";

pub const UI_MSG_INVALID_PACKAGE_JSON_TEMPLATE: &str = "sentinel: invalid package.json: {}";
pub const UI_MSG_INVALID_PACKAGE_FORMAT: &str =
    "sentinel: invalid package format: use <package>@<exact-version>";
pub const UI_MSG_READ_LOCKFILE_ENTRIES_FAILED_TEMPLATE: &str =
    "sentinel: failed to read lockfile entries: {}";
pub const UI_MSG_BUILD_DEPENDENCY_TREE_FAILED_TEMPLATE: &str =
    "sentinel: Failed to build dependency tree: {}";
pub const UI_MSG_TARGET_PACKAGE_NOT_FOUND_TEMPLATE: &str =
    "sentinel: target package {} not found in lockfile after resolution";
pub const UI_MSG_VERIFIER_INIT_FAILED_TEMPLATE: &str = "sentinel: init failed: {}";
pub const UI_MSG_LOCKFILE_CHANGED_ABORT_INSTALL: &str =
    "sentinel: package-lock.json changed after verification. Aborting install.";
pub const UI_MSG_LOCKFILE_CHANGED_ABORT_CI: &str =
    "sentinel: package-lock.json changed after verification. Aborting npm ci.";
pub const UI_MSG_NPM_INSTALL_STATUS_FAILED_TEMPLATE: &str =
    "sentinel: npm install failed with exit code {}";
pub const UI_MSG_NPM_CI_STATUS_FAILED_TEMPLATE: &str = "sentinel: npm ci failed with exit code {}";
pub const UI_MSG_NPM_INSTALL_EXEC_FAILED_TEMPLATE: &str =
    "sentinel: failed to execute npm install: {}";
pub const UI_MSG_NPM_CI_EXEC_FAILED_TEMPLATE: &str = "sentinel: failed to execute npm ci: {}";
pub const UI_MSG_ROLLBACK_FAILED_TEMPLATE: &str = "sentinel: failed to rollback project files: {}";
pub const UI_MSG_SAVE_REPORT_FAILED_TEMPLATE: &str = "sentinel: failed to save report: {}";
pub const UI_MSG_SERIALIZE_REPORT_FAILED_TEMPLATE: &str =
    "sentinel: failed to serialize report: {}";

pub const UI_GITHUB_WARNING_FORMAT: &str = "::warning title={},file={}::{} — {:?}";
pub const UI_JUNIT_SYSTEM_OUT_TEMPLATE: &str =
    "      <system-out>UNVERIFIABLE: {:?} — {}</system-out>";

pub const UI_REPORT_HEADER_SYMBOL_TEMPLATE: &str = "\n  {} Thank you for reporting {}.";
