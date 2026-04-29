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
pub const INSTALL_ERR_NO_LOCKFILE_FOR_INSTALL: &str = "sentinel install requires a lockfile, but none was found.\n\nPrefer Sentinel-first setup:\n\n  sentinel ci --init-lockfile\n\nThen re-run:\n\n  sentinel install <package>@<version>\n\nManual fallback:\n\n  npm install               →  package-lock.json\n  yarn                      →  yarn.lock\n  pnpm install              →  pnpm-lock.yaml";
pub const INSTALL_ERR_LOCKFILE_INIT_MISSING_AFTER_SUCCESS: &str =
    "sentinel: lockfile initialization finished but no lockfile was created";
pub const INSTALL_ERR_NO_LOCKFILE_FOR_CI: &str = "sentinel ci requires a lockfile, but none was found.\n\nPrefer Sentinel-first setup:\n\n  sentinel ci --init-lockfile\n\nManual fallback:\n\n  npm install --package-lock-only   →  package-lock.json\n  yarn install --no-immutable       →  yarn.lock\n  pnpm install --lockfile-only      →  pnpm-lock.yaml\n\nThen re-run sentinel ci.\n\nNote: --init-lockfile creates/updates lockfile before verification.";

pub const INSTALL_ERR_FAILED_READ_DIRECTORY: &str = "failed to read directory";
pub const INSTALL_ERR_FAILED_READ_DIRECTORY_ENTRY: &str = "failed to read directory entry";
pub const INSTALL_ERR_FAILED_READ_FILE_TYPE: &str = "failed to read file type";
pub const INSTALL_ERR_FAILED_READ_PACKAGE_MANIFEST: &str = "failed to read package manifest";
pub const INSTALL_ERR_FAILED_PARSE_PACKAGE_MANIFEST: &str = "failed to parse package manifest";

pub const INSTALL_ERR_FAILED_COMPUTE_RELATIVE_PATH: &str = "failed to compute relative path";
pub const INSTALL_ERR_FAILED_READ_FILE: &str = "failed to read file";

pub const INSTALL_ERR_PACKAGE_NOT_FOUND_NODE_MODULES: &str =
    "package was not found in node_modules";

pub const INSTALL_ERR_FAILED_FETCH_REGISTRY_METADATA: &str = "failed to fetch registry metadata";
pub const INSTALL_ERR_FAILED_DOWNLOAD_REGISTRY_TARBALL: &str =
    "failed to download registry tarball";
pub const INSTALL_ERR_FAILED_READ_TARBALL_RESPONSE: &str = "failed to read tarball response";

pub const INSTALL_ERR_FAILED_READ_REGISTRY_TARBALL_ENTRIES_POST_VERIFY: &str =
    "failed to read registry tarball entries for post-verify";
pub const INSTALL_ERR_FAILED_PARSE_REGISTRY_TARBALL_ENTRY_POST_VERIFY: &str =
    "failed to parse registry tarball entry for post-verify";
pub const INSTALL_ERR_FAILED_READ_REGISTRY_TARBALL_PATH_POST_VERIFY: &str =
    "failed to read registry tarball path for post-verify";
pub const INSTALL_ERR_FAILED_READ_REGISTRY_TARBALL_FILE_CONTENT_POST_VERIFY: &str =
    "failed to read registry tarball file content for post-verify";

pub const INSTALL_ERR_FAILED_BUILD_ASYNC_RUNTIME: &str =
    "failed to build async runtime for post-verify";

pub const INSTALL_ERR_FAILED_INIT_POST_VERIFY_VERIFIER: &str =
    "failed to initialize post-verify verifier";

pub const WARN_POST_VERIFY_LARGE_SCOPE_TEMPLATE: &str = "sentinel: warning: post-verify for {command_name} will validate {package_count} packages; total time/performance/resource usage may increase";

pub const WARN_POST_VERIFY_ELAPSED_TEMPLATE: &str = "sentinel: warning: post-verify for {command_name} took {elapsed}s for {package_count} packages (good-term target: {good_term_secs}s). Validation continued successfully.";

pub const POST_VERIFY_PACKAGE_PREFIX: &str = "package";

pub const POST_VERIFY_MISSING_PACKAGES_ERR_TEMPLATE: &str = "post-install verify failed for {command_name}: missing packages in node_modules: {missing_list}";

pub const POST_VERIFY_CONTENT_MISMATCH_ERR_TEMPLATE: &str = "post-install verify failed for {command_name}: content mismatch against registry tarball for: {mismatch_list}";
