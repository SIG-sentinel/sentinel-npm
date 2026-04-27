pub const DEFAULT_REGISTRY_TIMEOUT_MS: u64 = 5_000;
pub const CI_REGISTRY_TIMEOUT_MS: u64 = 10_000;
pub const DOWNLOAD_TARBALL_TIMEOUT_SECS: u64 = 120;
pub const REGISTRY_MAX_RETRIES: usize = 2;
pub const REGISTRY_RETRY_BASE_DELAY_MS: u64 = 200;

pub const NPM_REGISTRY_BASE_URL: &str = "https://registry.npmjs.org";
pub const NPM_SCOPED_SEPARATOR: &str = "%2F";
pub const NPM_USER_AGENT_PREFIX: &str = "sentinel/";

pub const NPM_CMD: &str = "npm";
pub const NPM_ARG_INSTALL: &str = "install";
pub const NPM_ARG_CACHE: &str = "cache";
pub const NPM_ARG_ADD: &str = "add";
pub const NPM_ARG_STORE: &str = "store";
pub const NPM_ARG_OFFLINE: &str = "--offline";
pub const NPM_ARG_PREFER_OFFLINE: &str = "--prefer-offline";
pub const NPM_ARG_NO_LOCKFILE: &str = "--no-lockfile";
pub const NPM_ARG_CACHE_FOLDER: &str = "--cache-folder";
pub const PNPM_ARG_STORE_DIR: &str = "--store-dir";
pub const NPM_ARG_NO_AUDIT: &str = "--no-audit";
pub const NPM_ARG_NO_FUND: &str = "--no-fund";
pub const NPM_ARG_SILENT: &str = "--silent";
pub const NPM_ARG_IGNORE_SCRIPTS: &str = "--ignore-scripts";

pub const YARN_LOCK_KEY_VERSION: &str = "version ";
pub const YARN_LOCK_KEY_INTEGRITY: &str = "integrity ";
pub const YARN_LOCK_KEY_DEPENDENCIES: &str = "dependencies:";
pub const YARN_LOCK_SELECTOR_SEPARATOR: char = ',';

pub const PNPM_LOCK_KEY_PACKAGES: &str = "packages";
pub const PNPM_LOCK_KEY_RESOLUTION: &str = "resolution";
pub const PNPM_LOCK_KEY_INTEGRITY: &str = "integrity";
pub const PNPM_LOCK_KEY_DEV: &str = "dev";
pub const PNPM_LOCK_KEY_DEPENDENCIES: &str = "dependencies";
pub const PNPM_LOCK_KEY_OPTIONAL_DEPENDENCIES: &str = "optionalDependencies";
pub const PNPM_LOCK_PEER_SEPARATOR: char = '(';
