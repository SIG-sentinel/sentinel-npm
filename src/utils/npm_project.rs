use std::path::Path;
use std::process::{Command, ExitStatus, Output};

use crate::constants::{
    NPM_HINT_COMMAND_NOT_FOUND, NPM_HINT_ERESOLVE, NPM_HINT_NETWORK_ERROR,
    NPM_HINT_GENERATE_LOCKFILE_MANUALLY_TEMPLATE, NPM_LOCKFILE_FLAG,
    PNPM_HINT_ERESOLVE, PNPM_LOCKFILE_FLAG, STDERR_PATTERN_CONFLICT,
    STDERR_PATTERN_ECONNREFUSED, STDERR_PATTERN_ENOENT, STDERR_PATTERN_ENOTFOUND,
    STDERR_PATTERN_ERESOLVE, STDERR_PATTERN_ETIMEDOUT, STDERR_PATTERN_FETCH_FAILED,
    STDERR_PATTERN_NOT_FOUND, STDERR_PATTERN_PEER, STDERR_PATTERN_UNABLE_RESOLVE,
    YARN_HINT_ERESOLVE, YARN_LOCKFILE_FLAG, render_template,
};
use crate::ecosystem::{
    CommandPlan, InstallExecutor, PackageManager, PackageManagerExecutor, detect_package_manager,
};
use crate::types::{
    InstallPackageParams, LockfileFailureKind, LockfileGenerationResult,
    ResolvePackageIntoLockfileParams, RunCleanInstallParams,
};

fn detected_manager(current_working_directory: &Path) -> PackageManager {
    detect_package_manager(current_working_directory).unwrap_or(PackageManager::Npm)
}

fn run_output_plan(current_working_directory: &Path, plan: CommandPlan) -> std::io::Result<Output> {
    let mut command = Command::new(plan.program);

    command.current_dir(current_working_directory);
    command.args(plan.args);
    command.output()
}

fn run_status_plan(
    current_working_directory: &Path,
    plan: CommandPlan,
) -> std::io::Result<ExitStatus> {
    let mut command = Command::new(plan.program);

    command.current_dir(current_working_directory);
    command.args(plan.args);
    command.status()
}

pub fn generate_lockfile(
    current_working_directory: &Path,
) -> std::io::Result<LockfileGenerationResult> {
    let manager = detected_manager(current_working_directory);
    let executor = PackageManagerExecutor::new(manager);
    let plan = executor.generate_lockfile_plan();

    run_output_plan(current_working_directory, plan)
        .map(|output| LockfileGenerationResult { output, manager })
}

fn classify_lockfile_failure(stderr: &str) -> LockfileFailureKind {
    let stderr_lower = stderr.to_lowercase();

    let has_eresolve = stderr_lower.contains(STDERR_PATTERN_ERESOLVE)
        || stderr_lower.contains(STDERR_PATTERN_UNABLE_RESOLVE);
    
    let has_peer_conflict =
        stderr_lower.contains(STDERR_PATTERN_PEER) && stderr_lower.contains(STDERR_PATTERN_CONFLICT);
    
    let is_dependency_conflict = has_eresolve || has_peer_conflict;

    let is_command_not_found = stderr_lower.contains(STDERR_PATTERN_NOT_FOUND)
        || stderr_lower.contains(STDERR_PATTERN_ENOENT);

    let is_network_error = stderr_lower.contains(STDERR_PATTERN_ENOTFOUND)
        || stderr_lower.contains(STDERR_PATTERN_ECONNREFUSED)
        || stderr_lower.contains(STDERR_PATTERN_ETIMEDOUT)
        || stderr_lower.contains(STDERR_PATTERN_FETCH_FAILED);

    match () {
        _ if is_dependency_conflict => LockfileFailureKind::DependencyConflict,
        _ if is_command_not_found => LockfileFailureKind::CommandNotFound,
        _ if is_network_error => LockfileFailureKind::NetworkError,
        _ => LockfileFailureKind::Unknown,
    }
}

pub fn diagnose_lockfile_failure(stderr: &str, manager: PackageManager) -> String {
    match classify_lockfile_failure(stderr) {
        LockfileFailureKind::DependencyConflict => match manager {
            PackageManager::Npm => NPM_HINT_ERESOLVE.to_string(),
            PackageManager::Yarn => YARN_HINT_ERESOLVE.to_string(),
            PackageManager::Pnpm => PNPM_HINT_ERESOLVE.to_string(),
        },
        LockfileFailureKind::CommandNotFound => render_template(
            NPM_HINT_COMMAND_NOT_FOUND,
            &[manager.command().to_string()],
        ),
        LockfileFailureKind::NetworkError => NPM_HINT_NETWORK_ERROR.to_string(),
        LockfileFailureKind::Unknown => {
            let lockfile_flag = match manager {
                PackageManager::Npm => NPM_LOCKFILE_FLAG,
                PackageManager::Yarn => YARN_LOCKFILE_FLAG,
                PackageManager::Pnpm => PNPM_LOCKFILE_FLAG,
            };

            render_template(
                NPM_HINT_GENERATE_LOCKFILE_MANUALLY_TEMPLATE,
                &[manager.command().to_string(), lockfile_flag.to_string()],
            )
        }
    }
}

pub fn resolve_package_into_lockfile(
    params: ResolvePackageIntoLockfileParams<'_>,
) -> std::io::Result<Output> {
    let ResolvePackageIntoLockfileParams {
        current_working_directory,
        package_reference,
    } = params;

    let manager = detected_manager(current_working_directory);
    let executor = PackageManagerExecutor::new(manager);
    let plan = executor.resolve_package_lockfile_plan(&package_reference.to_string());

    run_output_plan(current_working_directory, plan)
}

pub fn install_package(params: InstallPackageParams<'_>) -> std::io::Result<ExitStatus> {
    let InstallPackageParams {
        current_working_directory,
        package_reference,
        ignore_scripts,
    } = params;

    let manager = detected_manager(current_working_directory);
    let executor = PackageManagerExecutor::new(manager);
    let plan = executor.install_package_plan(&package_reference.to_string(), ignore_scripts);

    run_status_plan(current_working_directory, plan)
}

pub fn run_clean_install(params: RunCleanInstallParams<'_>) -> std::io::Result<ExitStatus> {
    let RunCleanInstallParams {
        current_working_directory,
        ignore_scripts,
        omit_dev,
        omit_optional,
        silent_output,
    } = params;

    let manager = detected_manager(current_working_directory);
    let executor = PackageManagerExecutor::new(manager);
    let plan = executor.clean_install_plan(crate::types::CleanInstallPlanParams {
        ignore_scripts,
        omit_dev,
        omit_optional,
        silent_output,
    });

    run_status_plan(current_working_directory, plan)
}
