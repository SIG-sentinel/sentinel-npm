pub const HISTORY_COMMAND_INSTALL: &str = "install";

pub const HISTORY_COMMAND_CI: &str = "ci";

pub const FALLBACK_PROCESS_EXIT_CODE: i32 = 1;

pub const POST_VERIFY_LARGE_PACKAGE_WARN_THRESHOLD: usize = 150;

pub const POST_VERIFY_GOOD_TERM_SECS: u64 = 30;

pub const POST_VERIFY_MAX_CONCURRENCY: usize = 8;

pub const POST_VERIFY_TARBALL_PACKAGE_PREFIX: &str = "package/";

pub const PACKAGE_SPEC_SEPARATOR: char = '@';

pub const SEMVER_VERSION_PREFIX: char = 'v';

pub const SEMVER_RANGE_CHARS: &[char] = &['^', '~', '>', '<', '=', '*', 'x', 'X', '|'];

pub const SEMVER_PINNED_EXTRA_CHARS: &[char] = &['.', '-', '+'];
