use std::path::Path;
use std::process::{Command, ExitStatus, Output};

use crate::constants::{
    NPM_ARG_CI, NPM_ARG_IGNORE_SCRIPTS, NPM_ARG_INSTALL, NPM_ARG_NO_AUDIT, NPM_ARG_NO_FUND,
    NPM_ARG_OMIT_DEV, NPM_ARG_OMIT_OPTIONAL, NPM_ARG_PACKAGE_LOCK_ONLY, NPM_ARG_SAVE_EXACT,
    NPM_CMD,
};
use crate::types::{InstallPackageParams, ResolvePackageIntoLockfileParams, RunCleanInstallParams};

fn npm_command(current_working_directory: &Path) -> Command {
    let mut command = Command::new(NPM_CMD);

    command.current_dir(current_working_directory);
    command
}

fn append_enabled_args(command: &mut Command, optional_args: &[(bool, &str)]) {
    optional_args
        .iter()
        .filter_map(|(enabled, arg)| enabled.then_some(*arg))
        .for_each(|arg| {
            command.arg(arg);
        });
}

pub fn generate_lockfile(current_working_directory: &Path) -> std::io::Result<Output> {
    let mut command = npm_command(current_working_directory);

    command
        .arg(NPM_ARG_INSTALL)
        .arg(NPM_ARG_PACKAGE_LOCK_ONLY)
        .arg(NPM_ARG_IGNORE_SCRIPTS)
        .arg(NPM_ARG_NO_AUDIT)
        .arg(NPM_ARG_NO_FUND)
        .output()
}

pub fn resolve_package_into_lockfile(
    params: ResolvePackageIntoLockfileParams<'_>,
) -> std::io::Result<Output> {
    let ResolvePackageIntoLockfileParams {
        current_working_directory,
        package_reference,
    } = params;

    let mut command = npm_command(current_working_directory);

    command
        .arg(NPM_ARG_INSTALL)
        .arg(package_reference.to_string())
        .arg(NPM_ARG_SAVE_EXACT)
        .arg(NPM_ARG_PACKAGE_LOCK_ONLY)
        .arg(NPM_ARG_IGNORE_SCRIPTS)
        .arg(NPM_ARG_NO_AUDIT)
        .arg(NPM_ARG_NO_FUND)
        .output()
}

pub fn install_package(params: InstallPackageParams<'_>) -> std::io::Result<ExitStatus> {
    let InstallPackageParams {
        current_working_directory,
        package_reference,
        ignore_scripts,
    } = params;

    let mut command = npm_command(current_working_directory);

    command
        .arg(NPM_ARG_INSTALL)
        .arg(package_reference.to_string())
        .arg(NPM_ARG_SAVE_EXACT)
        .arg(NPM_ARG_NO_AUDIT)
        .arg(NPM_ARG_NO_FUND);

    append_enabled_args(&mut command, &[(ignore_scripts, NPM_ARG_IGNORE_SCRIPTS)]);

    command.status()
}

pub fn run_clean_install(params: RunCleanInstallParams<'_>) -> std::io::Result<ExitStatus> {
    let RunCleanInstallParams {
        current_working_directory,
        ignore_scripts,
        omit_dev,
        omit_optional,
    } = params;

    let mut command = npm_command(current_working_directory);

    command
        .arg(NPM_ARG_CI)
        .arg(NPM_ARG_NO_AUDIT)
        .arg(NPM_ARG_NO_FUND);

    append_enabled_args(
        &mut command,
        &[
            (omit_dev, NPM_ARG_OMIT_DEV),
            (omit_optional, NPM_ARG_OMIT_OPTIONAL),
            (ignore_scripts, NPM_ARG_IGNORE_SCRIPTS),
        ],
    );

    command.status()
}
