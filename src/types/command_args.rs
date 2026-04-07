use std::path::PathBuf;

use clap::Args;

use crate::constants::{CI_REGISTRY_TIMEOUT_MS, DEFAULT_REGISTRY_TIMEOUT_MS};
use crate::types::OutputFormat;

fn parse_exact_package_version(value: &str) -> Result<String, String> {
    let last_at = value
        .rfind('@')
        .ok_or_else(|| "expected format <package>@<exact-version>".to_string())?;

    let package_name = &value[..last_at];
    let package_version = &value[last_at + 1..];

    if package_name.is_empty() || package_version.is_empty() {
        return Err("expected format <package>@<exact-version>".to_string());
    }

    if matches!(package_version, "latest" | "next") {
        return Err("version tag is not allowed; provide an exact version".to_string());
    }

    let has_range_tokens = package_version
        .chars()
        .any(|c| matches!(c, '^' | '~' | '>' | '<' | '=' | '*' | 'x' | 'X' | '|'));
    if has_range_tokens {
        return Err("version range is not allowed; provide an exact version".to_string());
    }

    let valid_chars = package_version
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'));
    if !valid_chars {
        return Err("version contains invalid characters".to_string());
    }

    if !package_version.chars().any(|c| c.is_ascii_digit()) {
        return Err("version must contain numeric components".to_string());
    }

    Ok(value.to_string())
}

#[derive(Args, Debug)]
pub struct InstallArgs {
    #[arg(value_name = "PACKAGE@VERSION", value_parser = parse_exact_package_version)]
    pub package: String,

    #[arg(long)]
    pub allow_scripts: bool,

    #[arg(long)]
    pub no_scripts: bool,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long, default_value = "text", value_enum)]
    pub format: OutputFormat,

    #[arg(long, default_value = ".")]
    pub cwd: PathBuf,

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

    #[arg(long, default_value_t = DEFAULT_REGISTRY_TIMEOUT_MS)]
    pub timeout: u64,

    #[arg(long, short = 'q')]
    pub quiet: bool,
}

#[derive(Args, Debug)]
pub struct CiArgs {
    #[arg(long)]
    pub omit_dev: bool,

    #[arg(long)]
    pub omit_optional: bool,

    #[arg(long)]
    pub allow_scripts: bool,

    #[arg(long)]
    pub no_scripts: bool,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long, default_value = "json", value_enum)]
    pub format: OutputFormat,

    #[arg(long, default_value = "sentinel-report.json")]
    pub report: PathBuf,

    #[arg(long, default_value = ".")]
    pub cwd: PathBuf,

    #[arg(long, default_value_t = CI_REGISTRY_TIMEOUT_MS)]
    pub timeout: u64,

    #[arg(long, short = 'q')]
    pub quiet: bool,
}

#[derive(Args, Debug)]
pub struct ReportArgs {
    #[arg(value_name = "PACKAGE[@VERSION]")]
    pub package: String,

    #[arg(long)]
    pub evidence: Option<String>,

    #[arg(long, default_value = "suspicious activity")]
    pub reason: String,
}
