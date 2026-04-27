pub const NESTED_DEP_PATH_TEMPLATE: &str = "{}/node_modules/{}";
pub const ROOT_DEP_PATH_TEMPLATE: &str = "node_modules/{}";
pub const VERSIONED_DEP_KEY_TEMPLATE: &str = "{}@{}";

pub const REGISTRY_VERSION_URL_TEMPLATE: &str = "{}/{}/{}";
pub const HTTPS_PREFIXED_REGISTRY_TEMPLATE: &str = "https:{}";
pub const HTTP_PREFIXED_REGISTRY_TEMPLATE: &str = "http:{}";
pub const NPM_IDENTITY_PATH_TEMPLATE: &str = "{}/{}@{}";

pub const NPM_ERR_REGISTRY_RESPONSE_TEMPLATE: &str = "npm registry returned {} for {}";
pub const NPM_ERR_PARSE_RESPONSE_TEMPLATE: &str = "parse error for {}: {}";
pub const NPM_ERR_TIMEOUT_DOWNLOAD_TEMPLATE: &str = "timeout downloading {}";
pub const NPM_ERR_LOCKFILE_ONLY_FAILED_TEMPLATE: &str = "{} lockfile generation failed: {}";
pub const NPM_ERR_EXEC_FAILED_TEMPLATE: &str = "Failed to execute {}: {}";

pub const NPM_HINT_ERESOLVE: &str = "hint: peer dependency conflict — sentinel cannot generate a safe lockfile.\n\
     \x20 1. Fix incompatible versions in package.json\n\
     \x20 2. Generate lockfile without installing:\n\
     \x20      npm install --package-lock-only\n\
     \x20 3. Verify and install through sentinel:\n\
    \x20      npx --yes sentinel-check ci";
pub const NPM_HINT_CONFLICT_DETAIL_TEMPLATE: &str = "\x20 Detected conflicting requirement: {}";
pub const YARN_HINT_ERESOLVE: &str = "hint: dependency conflict — sentinel cannot generate a safe lockfile.\n\
     \x20 1. Fix incompatible versions in package.json\n\
     \x20 2. Generate lockfile without installing:\n\
     \x20      yarn install --mode=update-lockfile\n\
     \x20 3. Verify and install through sentinel:\n\
    \x20      npx --yes sentinel-check ci";
pub const PNPM_HINT_ERESOLVE: &str = "hint: dependency conflict — sentinel cannot generate a safe lockfile.\n\
     \x20 1. Fix incompatible versions in package.json\n\
     \x20 2. Generate lockfile without installing:\n\
     \x20      pnpm install --lockfile-only\n\
     \x20 3. Verify and install through sentinel:\n\
    \x20      npx --yes sentinel-check ci";
pub const NPM_HINT_COMMAND_NOT_FOUND: &str =
    "hint: {} is not installed or not in PATH. Install it, then rerun sentinel.";
pub const NPM_HINT_NETWORK_ERROR: &str =
    "hint: cannot reach the npm registry — check your network connection or proxy settings.";
pub const NPM_HINT_GENERATE_LOCKFILE_MANUALLY_TEMPLATE: &str = "hint: generate the lockfile manually with your package manager, then rerun sentinel:\n\
     \x20 {} install {}\n\
    Then run: sentinel ci";
pub const NPM_ERR_LOCKFILE_GENERATION_STDERR_TEMPLATE: &str = "\n  {} stderr:\n{}\n";

pub const STDERR_PATTERN_ERESOLVE: &str = "eresolve";
pub const STDERR_PATTERN_UNABLE_RESOLVE: &str = "unable to resolve dependency";
pub const STDERR_PATTERN_PEER: &str = "peer";
pub const STDERR_PATTERN_CONFLICT: &str = "conflict";
pub const STDERR_PATTERN_ENOTFOUND: &str = "enotfound";
pub const STDERR_PATTERN_ECONNREFUSED: &str = "econnrefused";
pub const STDERR_PATTERN_ETIMEDOUT: &str = "etimedout";
pub const STDERR_PATTERN_FETCH_FAILED: &str = "fetch failed";
pub const STDERR_PATTERN_NOT_FOUND: &str = "not found";
pub const STDERR_PATTERN_ENOENT: &str = "enoent";

pub const NPM_LOCKFILE_FLAG: &str = "--package-lock-only";
pub const YARN_LOCKFILE_FLAG: &str = "--mode=update-lockfile";
pub const PNPM_LOCKFILE_FLAG: &str = "--lockfile-only";
