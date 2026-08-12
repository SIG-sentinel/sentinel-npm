mod helpers;
mod history;
mod lockfile;
mod policy;
mod post_verify;
mod post_verify_core;
mod post_verify_index;
mod resolve;
mod run_implementation;
mod source;

use std::process::ExitCode;

#[cfg(test)]
use std::path::Path;

#[cfg(test)]
use crate::types::EnsureLockfileExistsForInstallParams;
#[cfg(test)]
use crate::types::{
    AppendInstallHistoryParams, PackageRef, ResolveInstallPolicyParams, ShouldPrintReportParams,
};
use crate::types::{CiArgs, InstallArgs};

#[allow(clippy::ref_option)]
#[cfg(test)]
pub(super) fn find_missing_post_verify_packages(
    current_working_directory: &Path,
    packages: &[PackageRef],
) -> Vec<PackageRef> {
    let Ok(installed_package_index) =
        post_verify::build_installed_package_index(current_working_directory)
    else {
        return packages.to_vec();
    };

    post_verify::find_missing_post_verify_packages_from_index(&installed_package_index, packages)
}

#[allow(clippy::ref_option)]
#[cfg(test)]
#[allow(dead_code)]
pub(super) fn append_install_history(params: AppendInstallHistoryParams<'_>) -> Result<(), String> {
    history::append_install_history(params)
}

pub async fn run_install(args: &InstallArgs) -> ExitCode {
    run_implementation::run_install(args).await
}

#[cfg(test)]
fn ensure_lockfile_exists(params: EnsureLockfileExistsForInstallParams<'_>) -> bool {
    lockfile::ensure_lockfile_exists(params)
}

pub async fn run_ci(args: &CiArgs) -> ExitCode {
    run_implementation::run_ci(args).await
}

#[cfg(test)]
pub(super) fn parse_package_ref(spec: &str) -> Option<PackageRef> {
    resolve::parse_package_ref(spec)
}

#[cfg(test)]
pub(super) fn parse_install_package_request(
    spec: &str,
) -> Option<crate::types::InstallPackageRequest> {
    resolve::parse_install_package_request(spec)
}

#[cfg(test)]
pub(super) fn collect_install_packages_to_verify(
    params: crate::types::CollectInstallPackagesParams<'_>,
) -> Option<Vec<crate::types::DependencyNode>> {
    resolve::collect_install_packages_to_verify(params)
}

#[cfg(test)]
pub(super) fn resolve_install_candidate_package(
    dependency_tree: &crate::types::DependencyTree,
    request: &crate::types::InstallPackageRequest,
) -> Option<PackageRef> {
    resolve::resolve_install_candidate_package(dependency_tree, request)
}

#[cfg(test)]
pub(super) fn compute_directory_fingerprint(path: &Path) -> Result<String, String> {
    post_verify_index::compute_directory_fingerprint(path)
}

#[cfg(test)]
async fn finalize_ci_run(params: crate::types::FinalizeCiRunParams<'_>) -> ExitCode {
    run_implementation::finalize_ci_run(params).await
}

#[cfg(test)]
async fn finalize_install_run(
    params: crate::types::FinalizeInstallRunParams<'_>,
) -> crate::types::InstallExecutionOutcome {
    run_implementation::finalize_install_run(params).await
}

#[cfg(test)]
fn print_blocking_install_results(results: &[crate::types::VerifyResult]) -> bool {
    run_implementation::print_blocking_install_results(results)
}

#[cfg(test)]
fn print_ci_blocking_results(params: crate::types::PrintCiBlockingResultsParams<'_>) -> bool {
    run_implementation::print_ci_blocking_results(params)
}

#[cfg(test)]
pub(super) fn resolve_install_policy(
    params: ResolveInstallPolicyParams,
) -> crate::policy::InstallPolicyDecision {
    policy::resolve_install_policy(params)
}

#[cfg(test)]
pub(super) fn should_print_report(params: ShouldPrintReportParams<'_>) -> bool {
    run_implementation::should_print_report_impl(params)
}

#[cfg(test)]
#[path = "../../../tests/internal/install_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/internal/install_multiple_packages_tests.rs"]
mod install_multiple_packages_tests;
