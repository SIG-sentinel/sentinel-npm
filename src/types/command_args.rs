use std::path::PathBuf;

use clap::Args;

use crate::constants::{CI_REGISTRY_TIMEOUT_MS, DEFAULT_REGISTRY_TIMEOUT_MS};
use crate::types::HistoryOutputFormat;
use crate::types::OutputFormat;

fn parse_rfc3339_timestamp(value: &str) -> Result<String, String> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err("timestamp cannot be empty".to_string());
    }

    if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(trimmed) {
        return Ok(timestamp.to_rfc3339());
    }

    let normalized = trimmed.to_ascii_lowercase();
    if normalized == "now" {
        return Ok(chrono::Utc::now().to_rfc3339());
    }

    let relative_duration_text = normalized.strip_suffix(" ago").map_or(trimmed, str::trim);

    let duration = humantime::parse_duration(relative_duration_text).map_err(|_| {
        "expected RFC3339 (with timezone) or relative time like '7 days ago'".to_string()
    })?;

    let chrono_duration = chrono::Duration::from_std(duration)
        .map_err(|_| "relative duration is too large".to_string())?;

    Ok((chrono::Utc::now() - chrono_duration).to_rfc3339())
}

fn parse_exact_package_version(value: &str) -> Result<String, String> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err("package cannot be empty".to_string());
    }

    if trimmed.chars().any(char::is_whitespace) {
        return Err("package cannot contain spaces".to_string());
    }

    if trimmed.ends_with('@') {
        return Err("version is missing after '@'".to_string());
    }
    Ok(trimmed.to_string())
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Args, Debug)]
pub struct InstallArgs {
    #[arg(value_name = "PACKAGE[@VERSION]", value_parser = parse_exact_package_version)]
    pub package: String,

    #[arg(
        long,
        help = "Allow npm lifecycle scripts (preinstall, postinstall, etc.). By default, sentinel blocks scripts for security."
    )]
    pub allow_scripts: bool,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long)]
    pub post_verify: bool,

    #[arg(long, default_value = "text", value_enum)]
    pub format: OutputFormat,

    #[arg(long, default_value = ".")]
    pub cwd: PathBuf,

    #[arg(long, value_name = "npm|yarn|pnpm")]
    pub package_manager: Option<String>,

    #[arg(long, default_value_t = DEFAULT_REGISTRY_TIMEOUT_MS)]
    pub timeout: u64,

    #[arg(long, short = 'q')]
    pub quiet: bool,
}

#[derive(Args, Debug)]
pub struct CheckArgs {
    #[arg(long)]
    pub omit_dev: bool,

    #[arg(long)]
    pub omit_optional: bool,

    #[arg(long, default_value = "text", value_enum)]
    pub format: OutputFormat,

    #[arg(long, default_value = ".")]
    pub cwd: PathBuf,

    #[arg(long, value_name = "npm|yarn|pnpm")]
    pub package_manager: Option<String>,

    #[arg(long, default_value_t = DEFAULT_REGISTRY_TIMEOUT_MS)]
    pub timeout: u64,

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

    #[arg(
        long,
        help = "Allow npm lifecycle scripts (preinstall, postinstall, etc.). By default, sentinel blocks scripts for security."
    )]
    pub allow_scripts: bool,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long)]
    pub post_verify: bool,

    #[arg(long)]
    pub init_lockfile: bool,

    #[arg(long, default_value = "text", value_enum)]
    pub format: OutputFormat,

    #[arg(long, default_value = "sentinel-report.json")]
    pub report: PathBuf,

    #[arg(long, default_value = ".")]
    pub cwd: PathBuf,

    #[arg(long, value_name = "npm|yarn|pnpm")]
    pub package_manager: Option<String>,

    #[arg(long, default_value_t = CI_REGISTRY_TIMEOUT_MS)]
    pub timeout: u64,

    #[arg(long, short = 'q')]
    pub quiet: bool,
}

#[derive(Args, Debug)]
pub struct HistoryArgs {
    #[arg(long, value_name = "RFC3339|RELATIVE", value_parser = parse_rfc3339_timestamp)]
    pub from: String,

    #[arg(long, value_name = "RFC3339|RELATIVE", value_parser = parse_rfc3339_timestamp)]
    pub to: String,

    #[arg(long)]
    pub package: Option<String>,

    #[arg(long, requires = "package")]
    pub version: Option<String>,

    #[arg(long)]
    pub project: Option<PathBuf>,

    #[arg(long, value_name = "npm|yarn|pnpm")]
    pub package_manager: Option<String>,

    #[arg(long, default_value = "text", value_enum)]
    pub format: HistoryOutputFormat,

    #[arg(long, default_value = ".")]
    pub cwd: PathBuf,

    #[arg(long, short = 'q')]
    pub quiet: bool,
}

#[cfg(test)]
mod tests {
    use super::{parse_exact_package_version, parse_rfc3339_timestamp};

    #[test]
    fn install_spec_accepts_package_without_version_for_candidate_resolution() {
        let result = parse_exact_package_version("lodash");

        assert!(
            result.is_ok(),
            "install should accept package without explicit version to resolve a candidate"
        );
    }

    #[test]
    fn install_spec_accepts_latest_tag_for_candidate_resolution() {
        let result = parse_exact_package_version("lodash@latest");

        assert!(
            result.is_ok(),
            "install should accept latest tag to resolve and pin an exact candidate"
        );
    }

    #[test]
    fn history_timestamp_accepts_relative_ago_input() {
        let result = parse_rfc3339_timestamp("7 days ago");

        assert!(
            result.is_ok(),
            "history timestamp parser should accept relative 'ago' expressions"
        );
    }

    #[test]
    fn history_timestamp_accepts_now_keyword() {
        let result = parse_rfc3339_timestamp("now");

        assert!(
            result.is_ok(),
            "history timestamp parser should accept 'now'"
        );
    }

    #[test]
    fn history_timestamp_accepts_rfc3339_datetime() {
        let result = parse_rfc3339_timestamp("2026-04-23T12:34:56+00:00");

        assert!(
            result.is_ok(),
            "history timestamp parser should keep accepting explicit RFC3339 datetimes"
        );
    }
}
