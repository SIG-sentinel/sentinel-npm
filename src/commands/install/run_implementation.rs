use std::collections::HashSet;
use std::process::ExitCode;

use crate::commands::shared::{
    build_report, load_command_state, validate_package_json_dependencies, verify_packages,
};

use crate::constants::cli::{CLI_COMMAND_HINT_CI, CLI_COMMAND_HINT_INSTALL};
use crate::constants::{
    FALLBACK_PROCESS_EXIT_CODE, HISTORY_COMMAND_CI, HISTORY_COMMAND_INSTALL,
    INSTALL_MAX_CONCURRENCY, INSTALL_MSG_NOTHING_TO_INSTALL, INSTALL_PROGRESS_TEMPLATE,
    INSTALL_PROGRESS_VERIFY_MSG,
};
use crate::ecosystem::resolve_package_manager;
use crate::output::print_report;
use crate::types::{
    AnalyzeDependencyCyclesParams, CiArgs, CompleteSuccessfulCiRunParams,
    CompleteSuccessfulInstallParams, ExecuteVerificationRunParams, FinalizeCiDryRunParams,
    FinalizeCiRunParams, FinalizeInstallDryRunParams, FinalizeInstallRunParams, InstallArgs,
    InstallBlockReason, InstallExecutionOutcome, InstallFromVerifiedSourceOrFailureParams,
    InstallFromVerifiedSourceParams, OutputFormat, PackageRef, PrepareCiStateParams,
    PrepareInstallStateParams, PreparedCiState, PreparedInstallState,
    PrintBlockReasonResultsParams, PrintCiBlockingResultsParams, PrintInstallReportParams,
    PrintReportParams, ProgressBarConfig, ResolveInstallPolicyParams, ResolvePackageManagerParams,
    RestoreProjectFilesSnapshotParams, RunCiPostVerifyParams, RunCleanInstallOrFailureParams,
    RunPostVerifyForPackagesParams, SharedCommandState, SharedCommandStateError,
    ShouldPrintReportParams, ShouldRenderProgressBarParams, VerifyPackagesExecutionParams,
    VerifyPackagesParams, VerifyResult,
};
use crate::ui::command_feedback as ui;
use crate::utils::{
    build_install_command_hint, capture_project_files_snapshot, create_progress_bar,
    lockfile_sha256, restore_project_files_snapshot, should_render_progress_bar,
};

fn analyze_dependency_cycles(params: AnalyzeDependencyCyclesParams<'_>) -> Vec<Vec<String>> {
    super::helpers::analyze_dependency_cycles(params)
}

pub(super) fn should_print_report_impl(params: ShouldPrintReportParams<'_>) -> bool {
    super::helpers::should_print_report(params)
}

fn finalize_ci_dry_run(params: FinalizeCiDryRunParams<'_>) -> Option<ExitCode> {
    super::helpers::finalize_ci_dry_run(params)
}

fn run_clean_install_or_failure(
    params: RunCleanInstallOrFailureParams<'_>,
) -> Result<(), ExitCode> {
    super::helpers::run_clean_install_or_failure(params)
}

fn complete_successful_ci_run(params: CompleteSuccessfulCiRunParams<'_>) -> ExitCode {
    super::helpers::complete_successful_ci_run(params)
}

fn load_install_shared_state(
    current_working_directory: &std::path::Path,
    timeout: u64,
    registry_max_in_flight: Option<usize>,
) -> Result<SharedCommandState, ExitCode> {
    super::helpers::load_install_shared_state(
        current_working_directory,
        timeout,
        registry_max_in_flight,
    )
}

fn resolve_install_targets(
    params: crate::types::ResolveInstallTargetsParams<'_>,
) -> Result<(PackageRef, Vec<crate::types::DependencyNode>), ExitCode> {
    super::helpers::resolve_install_targets(params)
}

#[allow(clippy::unused_async)]
async fn prepare_install_state(
    params: PrepareInstallStateParams<'_>,
) -> Result<PreparedInstallState, ExitCode> {
    let PrepareInstallStateParams {
        args,
        manager: _,
        package_spec,
    } = params;

    if let Err(error) = validate_package_json_dependencies(&args.cwd) {
        ui::print_invalid_package_json(&error);

        return Err(ExitCode::FAILURE);
    }

    let (install_request, requested_package_ref) =
        super::helpers::parse_requested_install(package_spec)?;

    super::lockfile::prepare_install_lockfiles(args, &requested_package_ref)?;

    let lock_hash_before_verify = lockfile_sha256(&args.cwd);
    let shared_state =
        load_install_shared_state(&args.cwd, args.timeout, args.registry_max_in_flight)?;
    let SharedCommandState {
        dependency_tree,
        lockfile_entries,
        verifier,
    } = shared_state;
    let is_text_output = matches!(args.format, OutputFormat::Text);
    let analyze_dependency_cycles_params = AnalyzeDependencyCyclesParams {
        dependency_tree: &dependency_tree,
        quiet: args.quiet,
        is_text_output,
    };
    let cycles = analyze_dependency_cycles(analyze_dependency_cycles_params);

    let resolve_install_targets_params = crate::types::ResolveInstallTargetsParams {
        args,
        dependency_tree: &dependency_tree,
        install_request: &install_request,
        requested_package_ref: &requested_package_ref,
        is_text_output,
    };
    let (resolved_package_ref, packages_to_verify) =
        resolve_install_targets(resolve_install_targets_params)?;

    Ok(PreparedInstallState {
        package_ref: resolved_package_ref,
        packages_to_verify,
        verifier,
        lockfile_entries,
        lock_hash_before_verify,
        cycles,
    })
}

pub(super) fn print_blocking_install_results(results: &[VerifyResult]) -> bool {
    let blocked = super::policy::collect_blocked_verify_results(results);
    let block_reason = super::policy::resolve_install_block_reason(&blocked);

    match block_reason {
        Some(block_reason) => {
            let print_block_reason_results_params = PrintBlockReasonResultsParams {
                block_reason,
                blocked: &blocked,
            };

            super::policy::print_block_reason_results(print_block_reason_results_params);

            true
        }
        None => false,
    }
}

#[allow(clippy::unused_async)]
async fn prepare_ci_state(params: PrepareCiStateParams<'_>) -> Result<PreparedCiState, ExitCode> {
    let PrepareCiStateParams { args, manager } = params;
    let is_text_output = matches!(args.format, OutputFormat::Text);

    if let Err(error) = validate_package_json_dependencies(&args.cwd) {
        ui::print_invalid_package_json(&error);

        return Err(ExitCode::FAILURE);
    }

    super::lockfile::ensure_ci_lockfile_ready(args, manager)?;

    let lock_hash_before_verify = lockfile_sha256(&args.cwd);

    let shared_state =
        match load_command_state(&args.cwd, args.timeout, args.registry_max_in_flight) {
            Ok(shared_state) => shared_state,
            Err(SharedCommandStateError::DependencyTree(error)) => {
                ui::print_failed_to_build_dependency_tree(&error);

                return Err(ExitCode::FAILURE);
            }
            Err(SharedCommandStateError::LockfileEntries(error)) => {
                ui::print_failed_to_read_lockfile_entries(&error);

                return Err(ExitCode::FAILURE);
            }
            Err(SharedCommandStateError::Verifier(error)) => {
                ui::print_verifier_init_failed(&error);

                return Err(ExitCode::FAILURE);
            }
        };

    let SharedCommandState {
        dependency_tree,
        lockfile_entries,
        verifier,
    } = shared_state;
    let analyze_dependency_cycles_params = AnalyzeDependencyCyclesParams {
        dependency_tree: &dependency_tree,
        quiet: args.quiet,
        is_text_output,
    };
    let cycles = analyze_dependency_cycles(analyze_dependency_cycles_params);

    let mut packages_to_verify: Vec<_> = dependency_tree.nodes.values().cloned().collect();

    if args.omit_dev {
        packages_to_verify.retain(|node| !node.is_dev);
    }

    Ok(PreparedCiState {
        packages_to_verify,
        verifier,
        lockfile_entries,
        lock_hash_before_verify,
        cycles,
    })
}

pub(super) fn print_ci_blocking_results(params: PrintCiBlockingResultsParams<'_>) -> bool {
    let PrintCiBlockingResultsParams { results, args } = params;
    let blocked = super::policy::collect_blocked_verify_results(results);

    let resolve_install_policy_params = ResolveInstallPolicyParams {
        compromised_count: blocked.compromised.len(),
        unverifiable_count: blocked.unverifiable.len(),
        allow_scripts: args.allow_scripts,
        post_verify: args.post_verify,
    };
    let policy_decision = super::policy::resolve_install_policy(resolve_install_policy_params);

    match policy_decision.block_reason {
        Some(InstallBlockReason::Compromised) => {
            let print_block_reason_results_params = PrintBlockReasonResultsParams {
                block_reason: InstallBlockReason::Compromised,
                blocked: &blocked,
            };

            super::policy::print_block_reason_results(print_block_reason_results_params);
            true
        }
        Some(InstallBlockReason::Unverifiable) => {
            let print_block_reason_results_params = PrintBlockReasonResultsParams {
                block_reason: InstallBlockReason::Unverifiable,
                blocked: &blocked,
            };

            super::policy::print_block_reason_results(print_block_reason_results_params);
            true
        }
        None => false,
    }
}

async fn run_ci_post_verify(params: RunCiPostVerifyParams<'_>) -> Result<(), ExitCode> {
    let RunCiPostVerifyParams {
        args,
        report,
        is_text_output,
    } = params;

    if !args.post_verify {
        return Ok(());
    }

    let mut seen_clean_packages = HashSet::new();
    let clean_packages: Vec<PackageRef> = report
        .results
        .iter()
        .filter(|result| result.is_clean())
        .map(|result| result.package.clone())
        .filter(|package_ref| seen_clean_packages.insert(package_ref.to_string()))
        .collect();

    let post_verify_params = RunPostVerifyForPackagesParams {
        current_working_directory: &args.cwd,
        timeout_ms: args.timeout,
        registry_max_in_flight: args.registry_max_in_flight,
        quiet: args.quiet,
        is_text_output,
        command_name: HISTORY_COMMAND_CI,
        packages: &clean_packages,
        verify_results: &report.results,
    };

    if let Err(error) = super::post_verify::run_post_verify_for_packages(post_verify_params).await {
        ui::print_generic_error(&error);
        return Err(ExitCode::FAILURE);
    }

    Ok(())
}

pub(super) async fn finalize_ci_run(params: FinalizeCiRunParams<'_>) -> ExitCode {
    let FinalizeCiRunParams {
        args,
        report,
        lock_hash_before_verify,
    } = params;
    let is_text_output = matches!(args.format, OutputFormat::Text);

    let finalize_ci_dry_run_params = FinalizeCiDryRunParams {
        args,
        report,
        is_text_output,
    };

    if let Some(exit_code) = finalize_ci_dry_run(finalize_ci_dry_run_params) {
        return exit_code;
    }

    let lock_hash_before_install = lockfile_sha256(&args.cwd);
    let lockfile_unchanged = lock_hash_before_install == *lock_hash_before_verify;

    if !lockfile_unchanged {
        ui::print_ci_lockfile_changed_abort();

        return ExitCode::FAILURE;
    }

    let resolve_install_policy_params = ResolveInstallPolicyParams {
        compromised_count: report.summary.compromised as usize,
        unverifiable_count: report.summary.unverifiable as usize,
        allow_scripts: args.allow_scripts,
        post_verify: args.post_verify,
    };
    let policy_decision = super::policy::resolve_install_policy(resolve_install_policy_params);
    let should_print_scripts_default_notice =
        is_text_output && !args.quiet && policy_decision.ignore_scripts;

    if should_print_scripts_default_notice {
        ui::print_scripts_blocked_by_default_notice();
    }

    let run_clean_install_or_failure_params = RunCleanInstallOrFailureParams {
        args,
        ignore_scripts: policy_decision.ignore_scripts,
    };
    let run_ci_post_verify_params = RunCiPostVerifyParams {
        args,
        report,
        is_text_output,
    };
    let complete_successful_ci_run_params = CompleteSuccessfulCiRunParams {
        args,
        report,
        lock_hash_before_verify,
        is_text_output,
    };

    let ci_pipeline = async {
        run_clean_install_or_failure(run_clean_install_or_failure_params)?;
        run_ci_post_verify(run_ci_post_verify_params).await
    };

    match ci_pipeline.await {
        Ok(()) => complete_successful_ci_run(complete_successful_ci_run_params),
        Err(exit_code) => exit_code,
    }
}

async fn execute_verification_run(params: ExecuteVerificationRunParams<'_>) -> Vec<VerifyResult> {
    let ExecuteVerificationRunParams {
        output_format,
        quiet,
        packages_to_verify,
        verifier,
        lockfile_entries,
        cwd,
    } = params;
    let is_text_output = matches!(output_format, OutputFormat::Text);
    let should_print_verification_started = !quiet && is_text_output;

    if should_print_verification_started {
        ui::print_install_verification_started(packages_to_verify.len());
    }

    let should_render_progress_bar_params = ShouldRenderProgressBarParams {
        output_format,
        quiet,
    };
    let verify_progress_bar =
        should_render_progress_bar(should_render_progress_bar_params).then(|| {
            let progress_bar_config = ProgressBarConfig {
                length: packages_to_verify.len(),
                message: INSTALL_PROGRESS_VERIFY_MSG,
                template: INSTALL_PROGRESS_TEMPLATE,
            };

            create_progress_bar(progress_bar_config)
        });
    let show_text_progress_fallback = verify_progress_bar.is_none() && !quiet && is_text_output;

    let verify_packages_params = VerifyPackagesParams {
        packages_to_verify,
        verifier,
        lockfile_entries,
    };
    let verify_packages_execution_params = VerifyPackagesExecutionParams {
        verify_packages_params,
        max_concurrency: INSTALL_MAX_CONCURRENCY,
        progress_bar: verify_progress_bar,
        show_text_progress_fallback,
        ledger_path: crate::history::path::resolve_project_root(cwd)
            .ok()
            .map(|root| {
                std::sync::Arc::new(crate::history::path::resolve_history_ledger_path(&root))
            }),
    };

    verify_packages(verify_packages_execution_params).await
}

fn print_install_report_if_needed(params: PrintInstallReportParams<'_>) {
    let PrintInstallReportParams { args, report } = params;
    let should_print_report_params = ShouldPrintReportParams {
        output_format: &args.format,
        quiet: args.quiet,
    };
    let should_print_install_report = should_print_report_impl(should_print_report_params);

    if !should_print_install_report {
        return;
    }

    let print_report_params = PrintReportParams {
        report,
        output_format: &args.format,
    };

    print_report(print_report_params);
}

fn finalize_install_dry_run(
    params: FinalizeInstallDryRunParams<'_>,
) -> Option<InstallExecutionOutcome> {
    let FinalizeInstallDryRunParams {
        args,
        report,
        is_text_output,
    } = params;

    if !args.dry_run {
        return None;
    }

    let should_print_dry_run_complete = !args.quiet && is_text_output;

    if should_print_dry_run_complete {
        ui::print_dry_run_complete(report.summary.total as usize);
    }

    Some(InstallExecutionOutcome::success(true))
}

fn install_from_verified_source_or_failure(
    params: InstallFromVerifiedSourceOrFailureParams<'_>,
) -> Result<(), InstallExecutionOutcome> {
    let InstallFromVerifiedSourceOrFailureParams {
        args,
        package_ref,
        ignore_scripts,
        prevalidated_tarball,
    } = params;

    let install_from_verified_source_params = InstallFromVerifiedSourceParams {
        args,
        package_ref,
        ignore_scripts,
        prevalidated_tarball,
    };
    let install_status =
        super::source::install_from_verified_source(install_from_verified_source_params);

    let status = match install_status {
        Ok(status) => status,
        Err(error) => {
            ui::print_npm_install_exec_failed(&error);
            return Err(InstallExecutionOutcome::failure());
        }
    };

    if !status.success() {
        ui::print_npm_install_failed_status(status.code().unwrap_or(FALLBACK_PROCESS_EXIT_CODE));

        return Err(InstallExecutionOutcome::failure());
    }

    Ok(())
}

async fn complete_successful_install(
    params: CompleteSuccessfulInstallParams<'_>,
) -> InstallExecutionOutcome {
    let CompleteSuccessfulInstallParams {
        args,
        package_ref,
        report,
        lock_hash_before_verify,
        is_text_output,
    } = params;

    let target_packages = vec![package_ref.clone()];
    let post_verify_params = RunPostVerifyForPackagesParams {
        current_working_directory: &args.cwd,
        timeout_ms: args.timeout,
        registry_max_in_flight: args.registry_max_in_flight,
        quiet: args.quiet,
        is_text_output,
        command_name: HISTORY_COMMAND_INSTALL,
        packages: &target_packages,
        verify_results: &report.results,
    };

    let mut post_verify_result = Ok(());

    if args.post_verify {
        post_verify_result = super::post_verify::run_post_verify_for_packages(post_verify_params)
            .await
            .map_err(|error| {
                ui::print_generic_error(&error);
                InstallExecutionOutcome::failure()
            });
    }

    let append_install_history_params = crate::types::AppendInstallHistoryParams {
        args,
        package_ref,
        report,
        lock_hash_before_verify,
    };

    let completion_pipeline = post_verify_result.and_then(|()| {
        super::history::append_install_history(append_install_history_params).map_err(|error| {
            ui::print_generic_error(&error);
            InstallExecutionOutcome::failure()
        })
    });

    if let Err(outcome) = completion_pipeline {
        return outcome;
    }

    let should_print_install_success = !args.quiet && is_text_output;

    if should_print_install_success {
        ui::print_install_success(report.summary.clean as usize);
    }

    InstallExecutionOutcome::success(false)
}

#[allow(clippy::too_many_lines)]
pub(super) async fn finalize_install_run(
    params: FinalizeInstallRunParams<'_>,
) -> InstallExecutionOutcome {
    let FinalizeInstallRunParams {
        args,
        package_ref,
        report,
        lock_hash_before_verify,
        prevalidated_tarball,
        cwd: _cwd,
    } = params;
    let is_text_output = matches!(args.format, OutputFormat::Text);
    let print_install_report_if_needed_params = PrintInstallReportParams { args, report };

    print_install_report_if_needed(print_install_report_if_needed_params);

    let finalize_install_dry_run_params = FinalizeInstallDryRunParams {
        args,
        report,
        is_text_output,
    };

    if let Some(outcome) = finalize_install_dry_run(finalize_install_dry_run_params) {
        return outcome;
    }

    let lock_hash_before_install = lockfile_sha256(&args.cwd);
    let lockfile_unchanged = lock_hash_before_install == *lock_hash_before_verify;

    if !lockfile_unchanged {
        ui::print_install_lockfile_changed_abort();

        return InstallExecutionOutcome::failure();
    }

    let resolve_install_policy_params = ResolveInstallPolicyParams {
        compromised_count: report.summary.compromised as usize,
        unverifiable_count: report.summary.unverifiable as usize,
        allow_scripts: args.allow_scripts,
        post_verify: args.post_verify,
    };
    let policy_decision = super::policy::resolve_install_policy(resolve_install_policy_params);

    let should_print_scripts_default_notice =
        is_text_output && !args.quiet && policy_decision.ignore_scripts;

    if should_print_scripts_default_notice {
        ui::print_scripts_blocked_by_default_notice();
    }

    let install_from_verified_source_or_failure_params = InstallFromVerifiedSourceOrFailureParams {
        args,
        package_ref,
        ignore_scripts: policy_decision.ignore_scripts,
        prevalidated_tarball,
    };

    let complete_successful_install_params = CompleteSuccessfulInstallParams {
        args,
        package_ref,
        report,
        lock_hash_before_verify,
        is_text_output,
    };

    let install_pipeline = async {
        install_from_verified_source_or_failure(install_from_verified_source_or_failure_params)?;
        Ok::<_, InstallExecutionOutcome>(
            complete_successful_install(complete_successful_install_params).await,
        )
    };

    match install_pipeline.await {
        Ok(outcome) | Err(outcome) => outcome,
    }
}

async fn run_install_with_prepared_state(
    args: &InstallArgs,
    prepared_state: PreparedInstallState,
) -> InstallExecutionOutcome {
    let PreparedInstallState {
        package_ref,
        packages_to_verify,
        verifier,
        lockfile_entries,
        lock_hash_before_verify,
        cycles,
    } = prepared_state;

    let execute_verification_run_params = ExecuteVerificationRunParams {
        output_format: &args.format,
        quiet: args.quiet,
        packages_to_verify,
        verifier: verifier.clone(),
        lockfile_entries,
        cwd: &args.cwd,
    };

    let results = execute_verification_run(execute_verification_run_params).await;

    if print_blocking_install_results(&results) {
        return InstallExecutionOutcome::failure();
    }

    let report = build_report(crate::types::RunMode::Install, results, cycles);
    let verify_result_with_tarball = verifier.verify_before_install(&package_ref).await;
    let prevalidated_tarball = verify_result_with_tarball
        .result
        .is_clean()
        .then_some(verify_result_with_tarball.tarball)
        .flatten();

    let finalize_install_run_params = FinalizeInstallRunParams {
        args,
        package_ref: &package_ref,
        report: &report,
        lock_hash_before_verify: &lock_hash_before_verify,
        prevalidated_tarball,
        cwd: &args.cwd,
    };

    finalize_install_run(finalize_install_run_params).await
}

pub(super) async fn run_install(args: &InstallArgs) -> ExitCode {
    if args.packages.is_empty() {
        ui::print_generic_error("At least one package must be provided");
        return ExitCode::FAILURE;
    }

    if args.packages.len() == 1 {
        return run_install_single_package(args).await;
    }

    run_install_multiple_packages(args).await
}

async fn run_install_single_package(args: &InstallArgs) -> ExitCode {
    let package_hint = &args.packages[0];
    let install_command_hint = build_install_command_hint(CLI_COMMAND_HINT_INSTALL, package_hint);
    let resolve_install_package_manager_params = ResolvePackageManagerParams {
        project_dir: &args.cwd,
        explicit_pm: args.package_manager.as_deref(),
        command_hint: &install_command_hint,
    };
    let manager = match resolve_package_manager(&resolve_install_package_manager_params) {
        Ok(manager) => manager,
        Err(error) => {
            ui::print_generic_error(&error);
            return ExitCode::FAILURE;
        }
    };

    let snapshot = capture_project_files_snapshot(&args.cwd);
    let prepare_install_state_params = PrepareInstallStateParams {
        args,
        manager,
        package_spec: &args.packages[0],
    };
    let outcome = match prepare_install_state(prepare_install_state_params).await {
        Ok(prepared_state) => run_install_with_prepared_state(args, prepared_state).await,
        Err(exit_code) => InstallExecutionOutcome {
            exit_code,
            should_restore_snapshot: true,
        },
    };

    let restore_project_files_snapshot_params = RestoreProjectFilesSnapshotParams {
        snapshot: &snapshot,
        current_working_directory: &args.cwd,
    };

    if outcome.should_restore_snapshot
        && let Err(error) = restore_project_files_snapshot(restore_project_files_snapshot_params)
    {
        ui::print_rollback_failed(&error);
        return ExitCode::FAILURE;
    }

    outcome.exit_code
}

async fn run_install_multiple_packages(args: &InstallArgs) -> ExitCode {
    let initial_snapshot = capture_project_files_snapshot(&args.cwd);
    let is_text_output = matches!(args.format, OutputFormat::Text);
    let should_print_progress = !args.quiet && is_text_output;

    let ledger_path = crate::history::path::resolve_project_root(&args.cwd)
        .ok()
        .map(|root| crate::history::path::resolve_history_ledger_path(&root));
    let initial_ledger_snapshot = ledger_path
        .as_ref()
        .and_then(|path| std::fs::read(path).ok());

    let install_command_hint = format!("{} packages", args.packages.len());
    let resolve_install_package_manager_params = ResolvePackageManagerParams {
        project_dir: &args.cwd,
        explicit_pm: args.package_manager.as_deref(),
        command_hint: &install_command_hint,
    };
    let manager = match resolve_package_manager(&resolve_install_package_manager_params) {
        Ok(manager) => manager,
        Err(error) => {
            ui::print_generic_error(&error);
            return ExitCode::FAILURE;
        }
    };

    let total_packages = args.packages.len();
    let mut any_failed = false;
    let mut should_restore_snapshot = false;

    for (idx, package_spec) in args.packages.iter().enumerate() {
        let step = idx + 1;
        if should_print_progress {
            eprintln!("[{step}/{total_packages}] Installing {package_spec}...");
        }

        let prepare_install_state_params = PrepareInstallStateParams {
            args,
            manager,
            package_spec,
        };

        let prepared_state = match prepare_install_state(prepare_install_state_params).await {
            Ok(state) => state,
            Err(_exit_code) => {
                ui::print_generic_error(&format!("Failed to prepare install for {package_spec}"));
                any_failed = true;
                break;
            }
        };

        let outcome = run_install_with_prepared_state(args, prepared_state).await;

        if outcome.should_restore_snapshot {
            should_restore_snapshot = true;
        }

        if outcome.exit_code != ExitCode::SUCCESS {
            if should_print_progress {
                eprintln!("[{step}/{total_packages}] ✗ Installation failed for {package_spec}");
            }

            any_failed = true;
            break;
        }

        if should_print_progress {
            eprintln!("[{step}/{total_packages}] ✓ Installation succeeded for {package_spec}");
        }
    }

    let should_rollback = any_failed || should_restore_snapshot;

    if should_rollback {
        if should_print_progress {
            eprintln!("Rolling back project files (package.json and lockfile)...");
        }

        let restore_project_files_snapshot_params = RestoreProjectFilesSnapshotParams {
            snapshot: &initial_snapshot,
            current_working_directory: &args.cwd,
        };

        if let Err(error) = restore_project_files_snapshot(restore_project_files_snapshot_params) {
            ui::print_rollback_failed(&error);
            return ExitCode::FAILURE;
        }

        match (&ledger_path, &initial_ledger_snapshot) {
            (Some(path), Some(contents)) => {
                let _ = std::fs::write(path, contents);
            }
            (Some(path), None) => {
                let _ = std::fs::remove_file(path);
            }
            _ => {}
        }

        if should_print_progress {
            eprintln!("✓ Project files rolled back");
        }

        if any_failed {
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

pub(super) async fn run_ci(args: &CiArgs) -> ExitCode {
    let resolve_ci_package_manager_params = ResolvePackageManagerParams {
        project_dir: &args.cwd,
        explicit_pm: args.package_manager.as_deref(),
        command_hint: CLI_COMMAND_HINT_CI,
    };
    let manager = match resolve_package_manager(&resolve_ci_package_manager_params) {
        Ok(manager) => manager,
        Err(error) => {
            ui::print_generic_error(&error);
            return ExitCode::FAILURE;
        }
    };

    let prepare_ci_state_params = PrepareCiStateParams { args, manager };
    let prepared_state = match prepare_ci_state(prepare_ci_state_params).await {
        Ok(prepared_state) => prepared_state,
        Err(exit_code) => return exit_code,
    };

    let PreparedCiState {
        packages_to_verify,
        verifier,
        lockfile_entries,
        lock_hash_before_verify,
        cycles,
    } = prepared_state;

    if packages_to_verify.is_empty() {
        ui::print_generic_error(INSTALL_MSG_NOTHING_TO_INSTALL);
        return ExitCode::SUCCESS;
    }

    let execute_verification_run_params = ExecuteVerificationRunParams {
        output_format: &args.format,
        quiet: args.quiet,
        packages_to_verify,
        verifier,
        lockfile_entries,
        cwd: &args.cwd,
    };

    let results = execute_verification_run(execute_verification_run_params).await;
    let print_ci_blocking_results_params = PrintCiBlockingResultsParams {
        results: &results,
        args,
    };
    let ci_blocked = print_ci_blocking_results(print_ci_blocking_results_params);

    if ci_blocked {
        return ExitCode::FAILURE;
    }

    let report = build_report(crate::types::RunMode::Ci, results, cycles);
    let finalize_ci_run_params = FinalizeCiRunParams {
        args,
        report: &report,
        lock_hash_before_verify: &lock_hash_before_verify,
    };

    finalize_ci_run(finalize_ci_run_params).await
}
