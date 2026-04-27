use std::fmt::Display;
use std::path::Path;

use crate::types::PackageRef;

pub fn build_prevalidated_tarball_file_name(
    prevalidated_tarball_prefix: &str,
    process_id: u32,
    nanos: u128,
    package_name: &str,
) -> String {
    let sanitized_package_name = package_name.replace(['/', '@'], "_");

    format!("{prevalidated_tarball_prefix}-{process_id}-{nanos}-{sanitized_package_name}.tgz")
}

pub fn format_err_with_path(prefix: &str, path: &Path, error: &impl Display) -> String {
    format!("{prefix}: {}: {error}", path.display())
}

pub fn format_err_in_path(prefix: &str, path: &Path, error: &impl Display) -> String {
    format!("{prefix} in {}: {error}", path.display())
}

pub fn format_err_for_path(prefix: &str, path: &Path, error: &impl Display) -> String {
    format!("{prefix} for {}: {error}", path.display())
}

pub fn format_err_with_subject(prefix: &str, subject: &Path, error: &impl Display) -> String {
    format!("{prefix} {}: {error}", subject.display())
}

pub fn format_err_for_package(
    prefix: &str,
    package_ref: &PackageRef,
    error: &impl Display,
) -> String {
    format!("{prefix} for {package_ref}: {error}")
}

pub fn format_prefixed_package_message(
    prefix: &str,
    package_ref: &PackageRef,
    suffix: &str,
) -> String {
    format!("{prefix} {package_ref} {suffix}")
}

pub fn format_err_with_reason(prefix: &str, error: &impl Display) -> String {
    format!("{prefix}: {error}")
}

pub fn build_install_command_hint(command_hint: &str, package: &str) -> String {
    format!("{command_hint} {package}")
}
