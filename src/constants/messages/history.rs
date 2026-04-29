pub const HISTORY_ERR_INVALID_FROM_TIMESTAMP: &str =
    "invalid --from timestamp, expected RFC3339 with timezone or relative time like '7 days ago'";
pub const HISTORY_ERR_INVALID_TO_TIMESTAMP: &str =
    "invalid --to timestamp, expected RFC3339 with timezone or relative time like '7 days ago'";
pub const HISTORY_ERR_INVALID_RANGE_FROM_GT_TO: &str =
    "invalid range: --from must be less than or equal to --to";
pub const HISTORY_ERR_RESOLVE_CWD: &str = "failed to resolve --cwd";
pub const HISTORY_ERR_RESOLVE_PROJECT: &str = "failed to resolve --project";

pub const HISTORY_TEXT_UNKNOWN_PACKAGE: &str = "<unknown>";
pub const HISTORY_TEXT_FOUND_YES: &str = "yes";
pub const HISTORY_TEXT_FOUND_NO: &str = "no";
pub const HISTORY_TEXT_LABEL_PACKAGE: &str = "package:";
pub const HISTORY_TEXT_LABEL_RANGE: &str = "range:";
pub const HISTORY_TEXT_LABEL_FOUND: &str = "found:";
pub const HISTORY_TEXT_LABEL_EVENTS: &str = "events:";
pub const HISTORY_TEXT_LABEL_PROJECTS: &str = "projects:";
pub const HISTORY_TEXT_LABEL_PACKAGES: &str = "packages:";
pub const HISTORY_TEXT_LABEL_UNIQUE: &str = "unique";

pub const LEDGER_MISSING_TIP: &str =
    "Tip: run sentinel install <package>@latest (without --dry-run) to initialize history.";
pub const LEDGER_ERR_NOT_FOUND_TEMPLATE: &str = "install history ledger not found at {}.\n{}";
pub const LEDGER_ERR_CORRUPTED_FIRST_LINE_TEMPLATE: &str =
    "install history ledger is corrupted at first line: {}";
pub const LEDGER_WARN_SKIPPED_LINES_TEMPLATE: &str =
    "sentinel: skipped {} corrupted history lines while querying {}";
pub const PACKAGE_AT_VERSION_TEMPLATE: &str = "{}@{}";
pub const RUN_ID_TEMPLATE: &str = "{}-{}";
pub const TEMP_COMPACTION_EXTENSION_TEMPLATE: &str = "ndjson.tmp.{}";

pub const LEDGER_ERR_RESOLVE_PROJECT_ROOT: &str = "failed to resolve project root: {}";
pub const LEDGER_ERR_CREATE_DIRECTORY: &str = "failed to create history directory: {}";
pub const LEDGER_ERR_CREATE_FILE: &str = "failed to create history ledger: {}";
pub const LEDGER_ERR_INSPECT_METADATA: &str = "failed to inspect history ledger: {}";
pub const LEDGER_ERR_OPEN_FILE: &str = "failed to open history ledger: {}";
pub const LEDGER_ERR_READ_FIRST_LINE: &str = "failed to read history ledger first line: {}";
pub const LEDGER_ERR_READ_FILE: &str = "failed to read history ledger: {}";
pub const LEDGER_ERR_READ_LINE: &str = "failed to read history ledger line: {}";
pub const LEDGER_ERR_INSPECT_BEFORE_COMPACTION: &str =
    "failed to inspect history ledger before compaction: {}";
pub const LEDGER_ERR_OPEN_FOR_COMPACTION: &str = "failed to open history ledger for compaction: {}";
pub const LEDGER_ERR_READ_DURING_COMPACTION: &str =
    "failed to read history ledger during compaction: {}";
pub const LEDGER_ERR_CREATE_COMPACTED: &str = "failed to create compacted history ledger: {}";
pub const LEDGER_ERR_SERIALIZE_DURING_COMPACTION: &str =
    "failed to serialize history event during compaction: {}";
pub const LEDGER_ERR_WRITE_COMPACTED: &str = "failed to write compacted history ledger: {}";
pub const LEDGER_ERR_WRITE_COMPACTED_NEWLINE: &str =
    "failed to write compacted history newline: {}";
pub const LEDGER_ERR_REPLACE_WITH_COMPACTED: &str =
    "failed to replace history ledger with compacted ledger: {}";
pub const LEDGER_ERR_OPEN_FOR_APPEND: &str = "failed to open history ledger for append: {}";
pub const LEDGER_ERR_SERIALIZE_EVENT: &str = "failed to serialize history event: {}";
pub const LEDGER_ERR_APPEND_EVENT: &str = "failed to append history event: {}";
pub const LEDGER_ERR_APPEND_NEWLINE: &str = "failed to append history newline: {}";
