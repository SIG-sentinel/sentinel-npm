use std::path::Path;
use std::process::{Command, ExitStatus, Output};

use crate::ecosystem::{
    CommandPlan, InstallExecutor, PackageManager, PackageManagerExecutor, detect_package_manager,
};
use crate::types::{InstallPackageParams, ResolvePackageIntoLockfileParams, RunCleanInstallParams};

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

pub fn generate_lockfile(current_working_directory: &Path) -> std::io::Result<Output> {
    let manager = detected_manager(current_working_directory);
    let executor = PackageManagerExecutor::new(manager);
    let plan = executor.generate_lockfile_plan();

    run_output_plan(current_working_directory, plan)
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
