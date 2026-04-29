use std::path::Path;
use std::process::ExitCode;

use crate::constants::{
    INSTALL_ERR_LOCKFILE_INIT_MISSING_AFTER_SUCCESS, INSTALL_ERR_NO_LOCKFILE_FOR_CI,
    INSTALL_ERR_NO_LOCKFILE_FOR_INSTALL, NPM_ERR_EXEC_FAILED_TEMPLATE,
    NPM_ERR_LOCKFILE_ONLY_FAILED_TEMPLATE, render_template,
};
use crate::ecosystem::{PackageManager, active_lockfile_path, detect_package_manager};
use crate::types::{
    CiArgs, DiagnoseLockfileFailureParams, EnsureLockfileExistsForInstallParams, InstallArgs,
    LockfileGenerationResult, OutputFormat, PackageRef, PrepareLockfileForInstallParams,
    SyncLockfileWithPackageJsonParams,
};
use crate::ui::command_feedback as ui;
use crate::utils::{
    diagnose_lockfile_failure, generate_lockfile_with_manager, resolve_package_into_lockfile,
};

pub(super) fn prepare_lockfile_for_install(params: PrepareLockfileForInstallParams<'_>) -> bool {
    let PrepareLockfileForInstallParams {
        current_working_directory,
        package_reference,
        quiet,
    } = params;

    if !quiet {
        ui::print_resolving_package_into_lockfile(package_reference);
    }

    let resolve_package_into_lockfile_params = crate::types::ResolvePackageIntoLockfileParams {
        current_working_directory,
        package_reference,
    };
    let output = resolve_package_into_lockfile(resolve_package_into_lockfile_params);

    match output {
        Ok(output) if output.status.success() => true,
        _ => {
            ui::print_resolve_package_into_lockfile_failed(package_reference);

            false
        }
    }
}

pub(super) fn render_lockfile_generation_error(
    current_working_directory: &Path,
    result: Result<LockfileGenerationResult, std::io::Error>,
) -> String {
    match result {
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.output.stderr);
            let manager = result.manager;
            let diagnose_lockfile_failure_params = DiagnoseLockfileFailureParams {
                stderr: &stderr,
                manager,
            };
            let hint = diagnose_lockfile_failure(diagnose_lockfile_failure_params);
            let lockfile_only_failed_template_args = vec![manager.command().to_string(), hint];

            render_template(
                NPM_ERR_LOCKFILE_ONLY_FAILED_TEMPLATE,
                &lockfile_only_failed_template_args,
            )
        }
        Err(error) => {
            let manager =
                detect_package_manager(current_working_directory).unwrap_or(PackageManager::Npm);
            let exec_failed_template_args = vec![manager.command().to_string(), error.to_string()];

            render_template(NPM_ERR_EXEC_FAILED_TEMPLATE, &exec_failed_template_args)
        }
    }
}

pub(super) fn ensure_lockfile_exists(params: EnsureLockfileExistsForInstallParams<'_>) -> bool {
    let EnsureLockfileExistsForInstallParams {
        current_working_directory,
        ..
    } = params;

    let lockfile_path = active_lockfile_path(current_working_directory);

    if lockfile_path.exists() {
        return true;
    }

    ui::print_generic_error(INSTALL_ERR_NO_LOCKFILE_FOR_INSTALL);

    false
}

pub(super) fn sync_lockfile_with_package_json(
    params: SyncLockfileWithPackageJsonParams<'_>,
) -> Result<(), String> {
    let SyncLockfileWithPackageJsonParams {
        current_working_directory,
        manager,
        lockfile_path,
        quiet,
        is_text_output,
    } = params;
    let should_print_sync_notices = !quiet && is_text_output;

    if should_print_sync_notices {
        ui::print_missing_lockfile_notice();
    }

    let result = generate_lockfile_with_manager(current_working_directory, manager);

    match result {
        Ok(result) if result.output.status.success() => {
            if !lockfile_path.exists() {
                return Err(INSTALL_ERR_LOCKFILE_INIT_MISSING_AFTER_SUCCESS.to_string());
            }

            if should_print_sync_notices {
                ui::print_lockfile_created_notice();
            }

            Ok(())
        }
        _ => Err(render_lockfile_generation_error(
            current_working_directory,
            result,
        )),
    }
}

pub(super) fn prepare_install_lockfiles(
    args: &InstallArgs,
    package_ref: &PackageRef,
) -> Result<(), ExitCode> {
    let ensure_params = EnsureLockfileExistsForInstallParams {
        current_working_directory: &args.cwd,
    };

    if !ensure_lockfile_exists(ensure_params) {
        return Err(ExitCode::FAILURE);
    }

    let prepare_params = PrepareLockfileForInstallParams {
        current_working_directory: &args.cwd,
        package_reference: package_ref,
        quiet: args.quiet,
    };

    if !prepare_lockfile_for_install(prepare_params) {
        return Err(ExitCode::FAILURE);
    }

    Ok(())
}

pub(super) fn ensure_ci_lockfile_ready(
    args: &CiArgs,
    manager: PackageManager,
) -> Result<(), ExitCode> {
    let lockfile_path = active_lockfile_path(&args.cwd);
    let lockfile_exists = lockfile_path.exists();

    if lockfile_exists {
        return Ok(());
    }

    if !args.init_lockfile {
        ui::print_generic_error(INSTALL_ERR_NO_LOCKFILE_FOR_CI);

        return Err(ExitCode::FAILURE);
    }

    let is_text_output = matches!(args.format, OutputFormat::Text);
    let sync_lockfile_with_package_json_params = SyncLockfileWithPackageJsonParams {
        current_working_directory: &args.cwd,
        manager,
        lockfile_path: lockfile_path.as_path(),
        quiet: args.quiet,
        is_text_output,
    };
    let sync_result = sync_lockfile_with_package_json(sync_lockfile_with_package_json_params);

    if let Err(error) = sync_result {
        ui::print_generic_error(&error);

        return Err(ExitCode::FAILURE);
    }

    Ok(())
}
