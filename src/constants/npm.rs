pub const DEFAULT_REGISTRY_TIMEOUT_MS: u64 = 5_000;
pub const CI_REGISTRY_TIMEOUT_MS: u64 = 10_000;
pub const DOWNLOAD_TARBALL_TIMEOUT_SECS: u64 = 120;

pub const NPM_REGISTRY_BASE_URL: &str = "https://registry.npmjs.org";
pub const NPM_SCOPED_SEPARATOR: &str = "%2F";
pub const NPM_USER_AGENT_PREFIX: &str = "sentinel/";

pub const NPM_CMD: &str = "npm";
pub const NPM_ARG_INSTALL: &str = "install";
pub const NPM_ARG_CACHE: &str = "cache";
pub const NPM_ARG_ADD: &str = "add";
pub const NPM_ARG_OFFLINE: &str = "--offline";
pub const NPM_ARG_PREFER_OFFLINE: &str = "--prefer-offline";
pub const NPM_ARG_NO_AUDIT: &str = "--no-audit";
pub const NPM_ARG_NO_FUND: &str = "--no-fund";
pub const NPM_ARG_IGNORE_SCRIPTS: &str = "--ignore-scripts";
