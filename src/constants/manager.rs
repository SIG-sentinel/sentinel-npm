pub const PACKAGE_MANAGER_FIELD: &str = "packageManager";
pub const EMPTY_PACKAGE_MANAGER: &str = "";

pub const NPM_PREFIX_AT: &str = "npm@";
pub const NPM_PREFIX_SPACE: &str = "npm ";
pub const YARN_PREFIX_AT: &str = "yarn@";
pub const YARN_PREFIX_SPACE: &str = "yarn ";
pub const PNPM_PREFIX_AT: &str = "pnpm@";
pub const PNPM_PREFIX_SPACE: &str = "pnpm ";

pub const MULTIPLE_LOCKFILES_THRESHOLD: usize = 1;

pub const PACKAGE_MANAGER_NPM: &str = "npm";
pub const PACKAGE_MANAGER_YARN: &str = "yarn";
pub const PACKAGE_MANAGER_PNPM: &str = "pnpm";

pub const SETUP_AUTODETECT_FAILED_MESSAGE: &str = "[setup] package manager auto-detection failed.";
pub const SETUP_NO_LOCKFILE_CONTEXT_MESSAGE: &str =
    "No lockfile found in project root (package-lock.json, yarn.lock, pnpm-lock.yaml).";
pub const SETUP_EXPLICIT_COMMAND_HEADER: &str = "Run one command explicitly:";
pub const SETUP_NO_LOCKFILE_TIP: &str =
    "Tip: create a lockfile first (recommended): sentinel ci --init-lockfile";
pub const SETUP_SCRIPT_BLOCKED_TIP: &str =
    "Tip: lifecycle scripts are blocked by default. Use --allow-scripts only when required.";
