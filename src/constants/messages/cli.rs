pub const CLI_PARSER_ERR_EMPTY_TIMESTAMP: &str = "timestamp cannot be empty";
pub const CLI_PARSER_ERR_INVALID_TIMESTAMP: &str =
    "expected RFC3339 (with timezone) or relative time like '7 days ago'";
pub const CLI_PARSER_ERR_DURATION_TOO_LARGE: &str = "relative duration is too large";
pub const CLI_PARSER_ERR_EMPTY_PACKAGE: &str = "package cannot be empty";
pub const CLI_PARSER_ERR_PACKAGE_SPACES: &str = "package cannot contain spaces";
pub const CLI_PARSER_ERR_MISSING_VERSION_AFTER_SEPARATOR: &str = "version is missing after '@'";
pub const CLI_PARSER_ERR_EXPECTED_POSITIVE_INTEGER: &str = "expected a positive integer";

pub const CLI_HELP_ALLOW_SCRIPTS: &str = "Allow npm lifecycle scripts (preinstall, postinstall, etc.). By default, sentinel blocks scripts for security.";
pub const CLI_HELP_REGISTRY_MAX_IN_FLIGHT: &str = "Max concurrent registry requests for this command. Overrides SENTINEL_REGISTRY_MAX_IN_FLIGHT when provided.";
