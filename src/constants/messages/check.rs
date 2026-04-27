pub const CHECK_MSG_NO_LOCKFILE_BODY: &str = "No lockfile found.\n\n  Prefer using Sentinel to initialize and verify in one flow:\n\n    sentinel ci --init\n\n  Or generate the lockfile manually, then run:\n\n    sentinel check";
pub const CHECK_MSG_LOCKFILE_REQUIRED: &str = "sentinel check requires a lockfile, but none was found.\n\nPrefer Sentinel-first setup:\n\n  sentinel ci --init\n\nOr generate one with your package manager, then re-run sentinel check:\n\n  npm install --package-lock-only   →  package-lock.json\n  yarn install --no-immutable       →  yarn.lock\n  pnpm install --lockfile-only      →  pnpm-lock.yaml";
pub const CHECK_MSG_LOCKFILE_EMPTY: &str = "sentinel: lockfile is empty — nothing to check";
pub const CHECK_MSG_NO_PACKAGES_MATCHED: &str = "sentinel: no packages matched current filter";
pub const CHECK_MSG_NONE_REQUESTED_FOUND: &str = "sentinel: none of the requested packages were found in lockfile. Use 'name' or 'name@exact-version'.";
pub const CHECK_MSG_PROGRESS_TEMPLATE: &str = "\n  {} checking {} package(s)…";
pub const CHECK_MSG_VERIFY_PROGRESS_TEMPLATE: &str = "  verifying packages: {}/{} ({}%)";
pub const CHECK_MSG_ABORTED_WORKER_TEMPLATE: &str =
    "sentinel: check aborted due to {} internal worker failure(s)";
pub const CHECK_MSG_REPORT_SAVED_TEMPLATE: &str = "  report saved to {}";
pub const CHECK_MSG_INIT_FAILED_TEMPLATE: &str = "sentinel: failed to initialize: {}";
pub const CHECK_MSG_READ_LOCKFILE_FAILED_TEMPLATE: &str = "sentinel: cannot read lockfile: {}";
pub const CHECK_MSG_WORKER_FAILED_TEMPLATE: &str = "sentinel: worker task failed: {}";
pub const CHECK_MSG_WRITE_REPORT_FAILED_TEMPLATE: &str =
    "sentinel: failed to write report to {}: {}";
pub const CHECK_MSG_SERIALIZE_REPORT_FAILED_TEMPLATE: &str =
    "sentinel: failed to serialize report: {}";
