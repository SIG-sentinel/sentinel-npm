use super::shared::{build_report, load_command_state, verify_packages};
use crate::constants::cli::CLI_COMMAND_HINT_CHECK;
use crate::constants::{
    CHECK_MAX_CONCURRENCY, CHECK_MSG_INIT_FAILED_TEMPLATE, CHECK_MSG_LOCKFILE_REQUIRED,
    CHECK_PROGRESS_TEMPLATE, CHECK_PROGRESS_VERIFY_MSG, render_template_from_iter,
};
use crate::ecosystem::{active_lockfile_path, resolve_package_manager};
use crate::output::print_report;
use crate::types::ResolvePackageManagerParams;
use crate::types::{
    CheckArgs, CollectPackagesToVerifyParams, DependencyNode, EnsureLockfileExistsForCheckParams,
    LoadCheckSharedStateParams, OutputFormat, PrepareCheckStateParams, PreparedCheckState,
    PrintReportParams, ProgressBarConfig, SentinelError, SharedCommandState,
    SharedCommandStateError, ShouldRenderProgressBarParams, VerifyPackagesExecutionParams,
    VerifyPackagesParams,
};
use crate::ui::command_feedback as ui;
use crate::utils::{create_progress_bar, should_render_progress_bar};
use std::process::ExitCode;

fn ensure_lockfile_exists(
    params: EnsureLockfileExistsForCheckParams<'_>,
) -> Result<bool, SentinelError> {
    let EnsureLockfileExistsForCheckParams {
        current_working_directory,
        ..
    } = params;

    let lockfile_path = active_lockfile_path(current_working_directory);

    if lockfile_path.exists() {
        return Ok(true);
    }

    Err(SentinelError::LockfileNotFound)
}

fn ensure_check_lockfile_ready(args: &CheckArgs) -> Result<(), ExitCode> {
    let ensure_lockfile_exists_params = EnsureLockfileExistsForCheckParams {
        current_working_directory: &args.cwd,
    };

    let ensure_result = ensure_lockfile_exists(ensure_lockfile_exists_params);

    match ensure_result {
        Ok(_) => Ok(()),
        Err(SentinelError::LockfileNotFound) => {
            ui::print_generic_error(CHECK_MSG_LOCKFILE_REQUIRED);

            Err(ExitCode::FAILURE)
        }
        Err(error) => {
            let init_failed_message =
                render_template_from_iter(CHECK_MSG_INIT_FAILED_TEMPLATE, [error.to_string()]);

            ui::print_generic_error(&init_failed_message);

            Err(ExitCode::FAILURE)
        }
    }
}

fn load_check_shared_state(
    params: LoadCheckSharedStateParams<'_>,
) -> Result<SharedCommandState, ExitCode> {
    let LoadCheckSharedStateParams {
        current_working_directory,
        timeout,
        registry_max_in_flight,
    } = params;
    let load_state_result =
        load_command_state(current_working_directory, timeout, registry_max_in_flight);

    match load_state_result {
        Ok(shared_state) => Ok(shared_state),
        Err(SharedCommandStateError::DependencyTree(error)) => {
            ui::print_failed_to_build_dependency_tree(&error);

            Err(ExitCode::FAILURE)
        }
        Err(SharedCommandStateError::LockfileEntries(error)) => {
            ui::print_failed_to_read_lockfile_entries(&error);

            Err(ExitCode::FAILURE)
        }
        Err(SharedCommandStateError::Verifier(error)) => {
            let init_failed_message =
                render_template_from_iter(CHECK_MSG_INIT_FAILED_TEMPLATE, [error.to_string()]);

            ui::print_generic_error(&init_failed_message);

            Err(ExitCode::FAILURE)
        }
    }
}

fn collect_packages_to_verify(params: CollectPackagesToVerifyParams<'_>) -> Vec<DependencyNode> {
    let CollectPackagesToVerifyParams {
        check_args,
        dependency_nodes,
    } = params;

    let mut packages_to_verify: Vec<_> = dependency_nodes.values().cloned().collect();

    if check_args.omit_dev {
        packages_to_verify.retain(|node| !node.is_dev);
    }

    packages_to_verify
}

fn load_prepared_check_shared_state(args: &CheckArgs) -> Result<SharedCommandState, ExitCode> {
    ensure_check_lockfile_ready(args).and_then(|()| {
        let load_check_shared_state_params = LoadCheckSharedStateParams {
            current_working_directory: &args.cwd,
            timeout: args.timeout,
            registry_max_in_flight: args.registry_max_in_flight,
        };

        load_check_shared_state(load_check_shared_state_params)
    })
}

fn print_check_analysis_feedback(
    args: &CheckArgs,
    analysis: &crate::types::DependencyTreeAnalysis,
) {
    let is_text_output = matches!(args.format, OutputFormat::Text);
    let should_print_check_progress = !args.quiet && is_text_output;

    if should_print_check_progress {
        ui::print_check_progress(analysis.total_packages);
    }

    let has_dependency_cycles = !analysis.cycles.is_empty();
    let should_display_cycles = has_dependency_cycles && !args.quiet && is_text_output;

    if should_display_cycles {
        ui::print_dependency_cycles(&analysis.cycles);
    }
}

fn resolve_check_package_manager(
    args: &CheckArgs,
) -> Result<crate::types::PackageManager, ExitCode> {
    let resolve_package_manager_params = ResolvePackageManagerParams {
        project_dir: &args.cwd,
        explicit_pm: args.package_manager.as_deref(),
        command_hint: CLI_COMMAND_HINT_CHECK,
    };

    resolve_package_manager(&resolve_package_manager_params).map_err(|error| {
        ui::print_generic_error(&error);

        ExitCode::FAILURE
    })
}

fn create_check_progress_bar_if_needed(
    args: &CheckArgs,
    packages_to_verify_len: usize,
) -> Option<indicatif::ProgressBar> {
    let should_render_progress_bar_params = ShouldRenderProgressBarParams {
        output_format: &args.format,
        quiet: args.quiet,
    };
    let should_render_bar = should_render_progress_bar(should_render_progress_bar_params);

    should_render_bar.then(|| {
        let progress_bar_config = ProgressBarConfig {
            length: packages_to_verify_len,
            message: CHECK_PROGRESS_VERIFY_MSG,
            template: CHECK_PROGRESS_TEMPLATE,
        };

        create_progress_bar(progress_bar_config)
    })
}

fn resolve_check_exit_code(report: &crate::types::Report) -> ExitCode {
    match u8::try_from(report.summary.exit_code) {
        Ok(exit_code) => ExitCode::from(exit_code),
        Err(_) => ExitCode::FAILURE,
    }
}

fn should_exit_check_without_packages(
    args: &CheckArgs,
    packages_to_verify: &[DependencyNode],
) -> bool {
    let has_no_packages_to_verify = packages_to_verify.is_empty();
    let should_print_no_packages_message = has_no_packages_to_verify && !args.quiet;

    if should_print_no_packages_message {
        ui::print_no_packages_to_verify();
    }

    has_no_packages_to_verify
}

async fn execute_check_verification(
    args: &CheckArgs,
    prepared_state: PreparedCheckState,
) -> crate::types::Report {
    let PreparedCheckState {
        verifier,
        lockfile_entries,
        packages_to_verify,
        cycles,
    } = prepared_state;

    let is_text_output = matches!(args.format, OutputFormat::Text);
    let progress_bar = create_check_progress_bar_if_needed(args, packages_to_verify.len());
    let should_render_bar = progress_bar.is_some();
    let show_text_progress_fallback = !should_render_bar && !args.quiet && is_text_output;

    let verify_packages_params = VerifyPackagesParams {
        packages_to_verify,
        verifier,
        lockfile_entries,
    };
    let verify_packages_execution_params = VerifyPackagesExecutionParams {
        verify_packages_params,
        max_concurrency: CHECK_MAX_CONCURRENCY,
        progress_bar,
        show_text_progress_fallback,
    };
    let results = verify_packages(verify_packages_execution_params).await;

    build_report(crate::types::RunMode::Check, results, cycles)
}

fn print_check_report(args: &CheckArgs, report: &crate::types::Report) {
    let print_report_params = PrintReportParams {
        report,
        output_format: &args.format,
    };

    print_report(print_report_params);
}

#[allow(clippy::unused_async)]
async fn prepare_check_state(
    params: PrepareCheckStateParams<'_>,
) -> Result<PreparedCheckState, ExitCode> {
    let PrepareCheckStateParams { args, manager: _ } = params;

    let shared_state = match load_prepared_check_shared_state(args) {
        Ok(shared_state) => shared_state,
        Err(exit_code) => return Err(exit_code),
    };

    let SharedCommandState {
        dependency_tree,
        lockfile_entries,
        verifier,
    } = shared_state;
    let analysis = dependency_tree.analyze();
    let cycles = analysis.cycles.clone();
    print_check_analysis_feedback(args, &analysis);

    let collect_packages_to_verify_params = CollectPackagesToVerifyParams {
        check_args: args,
        dependency_nodes: &dependency_tree.nodes,
    };
    let packages_to_verify = collect_packages_to_verify(collect_packages_to_verify_params);

    let prepared_check_state = PreparedCheckState {
        verifier,
        lockfile_entries,
        packages_to_verify,
        cycles,
    };

    Ok(prepared_check_state)
}

pub async fn run(args: &CheckArgs) -> ExitCode {
    let manager = match resolve_check_package_manager(args) {
        Ok(manager) => manager,
        Err(exit_code) => return exit_code,
    };

    let prepare_check_state_params = PrepareCheckStateParams { args, manager };
    let prepared_state = match prepare_check_state(prepare_check_state_params).await {
        Ok(prepared_state) => prepared_state,
        Err(exit_code) => return exit_code,
    };

    if should_exit_check_without_packages(args, &prepared_state.packages_to_verify) {
        return ExitCode::SUCCESS;
    }

    let report = execute_check_verification(args, prepared_state).await;

    print_check_report(args, &report);

    resolve_check_exit_code(&report)
}
