use std::path::PathBuf;

use clap::Args;

use crate::cli_parsers::{
    parse_exact_package_version, parse_positive_usize, parse_rfc3339_timestamp,
};
use crate::constants::{
    CI_REGISTRY_TIMEOUT_MS, CLI_ARG_DEFAULT_CWD, CLI_ARG_DEFAULT_OUTPUT_FORMAT,
    CLI_ARG_DEFAULT_REPORT_PATH, CLI_ARG_VALUE_NAME_PACKAGE_MANAGER,
    CLI_ARG_VALUE_NAME_PACKAGE_WITH_VERSION, CLI_ARG_VALUE_NAME_POSITIVE_INTEGER,
    CLI_ARG_VALUE_NAME_TIMESTAMP_RANGE, CLI_HELP_ALLOW_SCRIPTS, CLI_HELP_REGISTRY_MAX_IN_FLIGHT,
    DEFAULT_REGISTRY_TIMEOUT_MS,
};
use crate::types::HistoryOutputFormat;
use crate::types::OutputFormat;

#[allow(clippy::struct_excessive_bools)]
#[derive(Args, Debug)]
pub struct InstallArgs {
    #[arg(value_name = CLI_ARG_VALUE_NAME_PACKAGE_WITH_VERSION, value_parser = parse_exact_package_version)]
    pub package: String,

    #[arg(long, help = CLI_HELP_ALLOW_SCRIPTS)]
    pub allow_scripts: bool,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long)]
    pub post_verify: bool,

    #[arg(long, default_value = CLI_ARG_DEFAULT_OUTPUT_FORMAT, value_enum)]
    pub format: OutputFormat,

    #[arg(long, default_value = CLI_ARG_DEFAULT_CWD)]
    pub cwd: PathBuf,

    #[arg(long, value_name = CLI_ARG_VALUE_NAME_PACKAGE_MANAGER)]
    pub package_manager: Option<String>,

    #[arg(long, default_value_t = DEFAULT_REGISTRY_TIMEOUT_MS)]
    pub timeout: u64,

    #[arg(
        long,
        value_name = CLI_ARG_VALUE_NAME_POSITIVE_INTEGER,
        value_parser = parse_positive_usize,
        help = CLI_HELP_REGISTRY_MAX_IN_FLIGHT
    )]
    pub registry_max_in_flight: Option<usize>,

    #[arg(long, short = 'q')]
    pub quiet: bool,
}

#[derive(Args, Debug)]
pub struct CheckArgs {
    #[arg(long)]
    pub omit_dev: bool,

    #[arg(long)]
    pub omit_optional: bool,

    #[arg(long, default_value = CLI_ARG_DEFAULT_OUTPUT_FORMAT, value_enum)]
    pub format: OutputFormat,

    #[arg(long, default_value = CLI_ARG_DEFAULT_CWD)]
    pub cwd: PathBuf,

    #[arg(long, value_name = CLI_ARG_VALUE_NAME_PACKAGE_MANAGER)]
    pub package_manager: Option<String>,

    #[arg(long, default_value_t = DEFAULT_REGISTRY_TIMEOUT_MS)]
    pub timeout: u64,

    #[arg(
        long,
        value_name = CLI_ARG_VALUE_NAME_POSITIVE_INTEGER,
        value_parser = parse_positive_usize,
        help = CLI_HELP_REGISTRY_MAX_IN_FLIGHT
    )]
    pub registry_max_in_flight: Option<usize>,

    #[arg(long, short = 'q')]
    pub quiet: bool,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Args, Debug)]
pub struct CiArgs {
    #[arg(long)]
    pub omit_dev: bool,

    #[arg(long)]
    pub omit_optional: bool,

    #[arg(long, help = CLI_HELP_ALLOW_SCRIPTS)]
    pub allow_scripts: bool,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long)]
    pub post_verify: bool,

    #[arg(long)]
    pub init_lockfile: bool,

    #[arg(long, default_value = CLI_ARG_DEFAULT_OUTPUT_FORMAT, value_enum)]
    pub format: OutputFormat,

    #[arg(long, default_value = CLI_ARG_DEFAULT_REPORT_PATH)]
    pub report: PathBuf,

    #[arg(long, default_value = CLI_ARG_DEFAULT_CWD)]
    pub cwd: PathBuf,

    #[arg(long, value_name = CLI_ARG_VALUE_NAME_PACKAGE_MANAGER)]
    pub package_manager: Option<String>,

    #[arg(long, default_value_t = CI_REGISTRY_TIMEOUT_MS)]
    pub timeout: u64,

    #[arg(
        long,
        value_name = CLI_ARG_VALUE_NAME_POSITIVE_INTEGER,
        value_parser = parse_positive_usize,
        help = CLI_HELP_REGISTRY_MAX_IN_FLIGHT
    )]
    pub registry_max_in_flight: Option<usize>,

    #[arg(long, short = 'q')]
    pub quiet: bool,
}

#[derive(Args, Debug)]
pub struct HistoryArgs {
    #[arg(long, value_name = CLI_ARG_VALUE_NAME_TIMESTAMP_RANGE, value_parser = parse_rfc3339_timestamp)]
    pub from: String,

    #[arg(long, value_name = CLI_ARG_VALUE_NAME_TIMESTAMP_RANGE, value_parser = parse_rfc3339_timestamp)]
    pub to: String,

    #[arg(long)]
    pub package: Option<String>,

    #[arg(long, requires = "package")]
    pub version: Option<String>,

    #[arg(long)]
    pub project: Option<PathBuf>,

    #[arg(long, value_name = CLI_ARG_VALUE_NAME_PACKAGE_MANAGER)]
    pub package_manager: Option<String>,

    #[arg(long, default_value = CLI_ARG_DEFAULT_OUTPUT_FORMAT, value_enum)]
    pub format: HistoryOutputFormat,

    #[arg(long, default_value = CLI_ARG_DEFAULT_CWD)]
    pub cwd: PathBuf,

    #[arg(long, short = 'q')]
    pub quiet: bool,
}

#[cfg(test)]
#[path = "../../tests/internal/command_args_tests.rs"]
mod tests;
