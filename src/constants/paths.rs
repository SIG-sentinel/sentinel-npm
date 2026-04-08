pub const SENTINEL_HOME_DIR: &str = ".sentinel";
pub const SENTINEL_CACHE_DB_FILE: &str = "cache.db";
pub const SENTINEL_STAGING_DIR: &str = "sentinel-staging";

pub const PACKAGE_LOCK_FILE: &str = "package-lock.json";
pub const YARN_LOCK_FILE: &str = "yarn.lock";
pub const PNPM_LOCK_FILE: &str = "pnpm-lock.yaml";
pub const PACKAGE_JSON_FILE: &str = "package.json";
pub const PACKAGE_JSON_DEPENDENCIES_KEY: &str = "dependencies";
pub const PACKAGE_JSON_PEER_DEPENDENCIES_KEY: &str = "peerDependencies";
pub const PACKAGE_JSON_DEV_DEPENDENCIES_KEY: &str = "devDependencies";
pub const PACKAGE_VERSION_DEFAULT_RANGE: &str = "*";
pub const PACKAGE_VERSION_LATEST: &str = "latest";

pub const NODE_MODULES_PREFIX: &str = "node_modules/";
pub const FILE_URL_PREFIX: &str = "file://";

pub const LOCKFILE_JSON_KEY_PACKAGES: &str = "packages";
pub const LOCKFILE_JSON_KEY_DEPENDENCIES: &str = "dependencies";
pub const LOCKFILE_JSON_KEY_VERSION: &str = "version";
pub const LOCKFILE_JSON_KEY_INTEGRITY: &str = "integrity";
pub const LOCKFILE_JSON_KEY_DEV: &str = "dev";

pub const NPM_CONFIG_CACHE_ENV: &str = "NPM_CONFIG_CACHE";
pub const NPM_CACHE_SUBDIR: &str = ".npm";

pub const ENV_SENTINEL_LOG: &str = "SENTINEL_LOG";
pub const ENV_RUST_LOG: &str = "RUST_LOG";
pub const DEFAULT_LOG_FILTER: &str = "sentinel=info";
