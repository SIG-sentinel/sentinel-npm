pub const INSTALL_MSG_NOTHING_TO_INSTALL: &str = "sentinel: nothing to install";
pub const INSTALL_MSG_VERIFYING_TEMPLATE: &str = "\n  {} verifying {} package(s) before install…";
pub const INSTALL_MSG_DRY_RUN_TEMPLATE: &str =
    "\n  {} dry-run complete — {} packages verified, nothing installed\n";
pub const INSTALL_MSG_SCRIPTS_DISABLED: &str =
    "\n  {} scripts disabled for unverified packages (use --allow-scripts to override)\n";
pub const INSTALL_MSG_SUCCESS_TEMPLATE: &str = "\n  {} {} package(s) installed and verified\n";
pub const INSTALL_MSG_CI_REPORT_TEMPLATE: &str = "\nsentinel ci: report saved to {}";
pub const INSTALL_ERR_PACKAGE_NOT_IN_LOCKFILE_TEMPLATE: &str =
    "target package {} not found in lockfile after resolution";
pub const INSTALL_ERR_LOCKFILE_GENERATE_FAILED: &str = "Failed to generate lockfile";
pub const INSTALL_MSG_OFFLINE_STRICT_FAIL: &str =
    "sentinel: offline mode cannot validate package integrity against npm registry in strict mode";
pub const INSTALL_MSG_NPM_FAILED_TEMPLATE: &str = "npm install failed with code {}";
