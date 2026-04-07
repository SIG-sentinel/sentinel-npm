pub const CHECK_MSG_NO_LOCKFILE_BODY: &str = "No package-lock.json found.\n\n  Run `npm install` first to generate the lockfile,\n\n  then run `sentinel check`.";
pub const CHECK_MSG_LOCKFILE_EMPTY: &str = "sentinel: lockfile is empty — nothing to check";
pub const CHECK_MSG_NO_PACKAGES_MATCHED: &str = "sentinel: no packages matched current filter";
pub const CHECK_MSG_NONE_REQUESTED_FOUND: &str = "sentinel: none of the requested packages were found in lockfile. Use 'name' or 'name@exact-version'.";
pub const CHECK_MSG_PROGRESS_TEMPLATE: &str = "\n  {} checking {} package(s)…";
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
