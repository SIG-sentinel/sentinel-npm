mod history;
mod lockfile;
mod orchestration;
mod policy;
mod post_verify;
mod resolve;
mod source;

use std::collections::HashSet;
use std::process::ExitCode;

#[cfg(test)]
use std::path::Path;

use super::shared::{
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
#[cfg(test)]
use crate::types::EnsureLockfileExistsForInstallParams;
use crate::types::{
    AnalyzeDependencyCyclesParams, AppendInstallHistoryParams, CiArgs,
    CompleteSuccessfulCiRunParams, CompleteSuccessfulInstallParams, ExecuteVerificationRunParams,
    FinalizeCiDryRunParams, FinalizeCiRunParams, FinalizeInstallDryRunParams,
    FinalizeInstallRunParams, InstallArgs, InstallBlockReason, InstallExecutionOutcome,
    InstallFromVerifiedSourceOrFailureParams, InstallFromVerifiedSourceParams, OutputFormat,
    PackageRef, PrepareCiStateParams, PrepareInstallStateParams, PreparedCiState,
    PreparedInstallState, PrintBlockReasonResultsParams, PrintCiBlockingResultsParams,
    PrintInstallReportParams, PrintReportParams, ProgressBarConfig, ResolveInstallPolicyParams,
    ResolveInstallTargetsParams, ResolvePackageManagerParams, RestoreProjectFilesSnapshotParams,
    RunCiPostVerifyParams, RunCleanInstallOrFailureParams, RunPostVerifyForPackagesParams,
    SharedCommandState, SharedCommandStateError, ShouldPrintReportParams,
    ShouldRenderProgressBarParams, VerifyPackagesExecutionParams, VerifyPackagesParams,
    VerifyResult,
};
use crate::ui::command_feedback as ui;
use crate::utils::{
    build_install_command_hint, capture_project_files_snapshot, create_progress_bar,
    lockfile_sha256, restore_project_files_snapshot, should_render_progress_bar,
};

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
    orchestration::append_install_history(params)
}

#[allow(clippy::unused_async)]
async fn prepare_install_state(
    params: PrepareInstallStateParams<'_>,
) -> Result<PreparedInstallState, ExitCode> {
    let PrepareInstallStateParams { args, manager: _ } = params;

    if let Err(error) = validate_package_json_dependencies(&args.cwd) {
        ui::print_invalid_package_json(&error);

        return Err(ExitCode::FAILURE);
    }

    let (install_request, requested_package_ref) = orchestration::parse_requested_install(args)?;

    orchestration::prepare_install_lockfiles(args, &requested_package_ref)?;

    let lock_hash_before_verify = lockfile_sha256(&args.cwd);
    let shared_state = orchestration::load_install_shared_state(
        &args.cwd,
        args.timeout,
        args.registry_max_in_flight,
    )?;
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
    let cycles = orchestration::analyze_dependency_cycles(analyze_dependency_cycles_params);

    let resolve_install_targets_params = ResolveInstallTargetsParams {
        args,
        dependency_tree: &dependency_tree,
        install_request: &install_request,
        requested_package_ref: &requested_package_ref,
        is_text_output,
    };
    let (resolved_package_ref, packages_to_verify) =
        orchestration::resolve_install_targets(resolve_install_targets_params)?;

    Ok(PreparedInstallState {
        package_ref: resolved_package_ref,
        packages_to_verify,
        verifier,
        lockfile_entries,
        lock_hash_before_verify,
        cycles,
    })
}

fn print_blocking_install_results(results: &[VerifyResult]) -> bool {
    let blocked = orchestration::collect_blocked_verify_results(results);
    let block_reason = orchestration::resolve_install_block_reason(&blocked);

    match block_reason {
        Some(block_reason) => {
            let print_block_reason_results_params = PrintBlockReasonResultsParams {
                block_reason,
                blocked: &blocked,
            };

            orchestration::print_block_reason_results(print_block_reason_results_params);

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

    orchestration::ensure_ci_lockfile_ready(args, manager)?;

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
    let cycles = orchestration::analyze_dependency_cycles(analyze_dependency_cycles_params);

    let mut packages_to_verify: Vec<_> = dependency_tree.nodes.values().cloned().collect();

    if args.omit_dev {
        packages_to_verify.retain(|node| !node.is_dev);
    }

    let prepared_ci_state = PreparedCiState {
        packages_to_verify,
        verifier,
        lockfile_entries,
        lock_hash_before_verify,
        cycles,
    };

    Ok(prepared_ci_state)
}

fn print_ci_blocking_results(params: PrintCiBlockingResultsParams<'_>) -> bool {
    let PrintCiBlockingResultsParams { results, args } = params;
    let blocked = orchestration::collect_blocked_verify_results(results);

    let resolve_install_policy_params = ResolveInstallPolicyParams {
        compromised_count: blocked.compromised.len(),
        unverifiable_count: blocked.unverifiable.len(),
        allow_scripts: args.allow_scripts,
        post_verify: args.post_verify,
    };
    let policy_decision = orchestration::resolve_install_policy(resolve_install_policy_params);

    match policy_decision.block_reason {
        Some(InstallBlockReason::Compromised) => {
            let print_block_reason_results_params = PrintBlockReasonResultsParams {
                block_reason: InstallBlockReason::Compromised,
                blocked: &blocked,
            };

            orchestration::print_block_reason_results(print_block_reason_results_params);

            true
        }
        Some(InstallBlockReason::Unverifiable) => {
            let print_block_reason_results_params = PrintBlockReasonResultsParams {
                block_reason: InstallBlockReason::Unverifiable,
                blocked: &blocked,
            };

            orchestration::print_block_reason_results(print_block_reason_results_params);

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

    if let Err(error) = post_verify::run_post_verify_for_packages(post_verify_params).await {
        ui::print_generic_error(&error);

        return Err(ExitCode::FAILURE);
    }

    Ok(())
}

async fn finalize_ci_run(params: FinalizeCiRunParams<'_>) -> ExitCode {
    let FinalizeCiRunParams {
        args,
        report,
        lock_hash_before_verify,
    } = params;
    let is_text_output = matches!(args.format, OutputFormat::Text);

    if let Some(exit_code) = orchestration::finalize_ci_dry_run(FinalizeCiDryRunParams {
        args,
        report,
        is_text_output,
    }) {
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
    let policy_decision = orchestration::resolve_install_policy(resolve_install_policy_params);
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
        orchestration::run_clean_install_or_failure(run_clean_install_or_failure_params)?;
        run_ci_post_verify(run_ci_post_verify_params).await
    };

    match ci_pipeline.await {
        Ok(()) => orchestration::complete_successful_ci_run(complete_successful_ci_run_params),
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
    };

    verify_packages(verify_packages_execution_params).await
}

fn print_install_report_if_needed(params: PrintInstallReportParams<'_>) {
    let PrintInstallReportParams { args, report } = params;
    let should_print_report_params = ShouldPrintReportParams {
        output_format: &args.format,
        quiet: args.quiet,
    };
    let should_print_install_report =
        orchestration::should_print_report(should_print_report_params);

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
        orchestration::install_from_verified_source(install_from_verified_source_params);

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
        post_verify_result = post_verify::run_post_verify_for_packages(post_verify_params)
            .await
            .map_err(|error| {
                ui::print_generic_error(&error);
                InstallExecutionOutcome::failure()
            });
    }

    let append_install_history_params = AppendInstallHistoryParams {
        args,
        package_ref,
        lock_hash_before_verify,
    };

    let completion_pipeline = post_verify_result.and_then(|()| {
        orchestration::append_install_history(append_install_history_params).map_err(|error| {
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
async fn finalize_install_run(params: FinalizeInstallRunParams<'_>) -> InstallExecutionOutcome {
    let FinalizeInstallRunParams {
        args,
        package_ref,
        report,
        lock_hash_before_verify,
        prevalidated_tarball,
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
    let policy_decision = orchestration::resolve_install_policy(resolve_install_policy_params);

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
    };

    finalize_install_run(finalize_install_run_params).await
}

pub async fn run_install(args: &InstallArgs) -> ExitCode {
    let install_command_hint = build_install_command_hint(CLI_COMMAND_HINT_INSTALL, &args.package);
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
    let prepare_install_state_params = PrepareInstallStateParams { args, manager };
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

#[cfg(test)]
fn ensure_lockfile_exists(params: EnsureLockfileExistsForInstallParams<'_>) -> bool {
    lockfile::ensure_lockfile_exists(params)
}

pub async fn run_ci(args: &CiArgs) -> ExitCode {
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

#[cfg(test)]
pub(super) fn parse_package_ref(spec: &str) -> Option<PackageRef> {
    resolve::parse_package_ref(spec)
}

#[cfg(test)]
pub(super) fn parse_install_package_request(
    spec: &str,
) -> Option<crate::types::InstallPackageRequest> {
    orchestration::parse_install_package_request(spec)
}

#[cfg(test)]
pub(super) fn collect_install_packages_to_verify(
    params: crate::types::CollectInstallPackagesParams<'_>,
) -> Option<Vec<crate::types::DependencyNode>> {
    orchestration::collect_install_packages_to_verify(params)
}

#[cfg(test)]
pub(super) fn resolve_install_candidate_package(
    dependency_tree: &crate::types::DependencyTree,
    request: &crate::types::InstallPackageRequest,
) -> Option<PackageRef> {
    orchestration::resolve_install_candidate_package(dependency_tree, request)
}

#[cfg(test)]
pub(super) fn compute_directory_fingerprint(path: &Path) -> Result<String, String> {
    post_verify::compute_directory_fingerprint(path)
}

#[cfg(test)]
pub(super) fn resolve_install_policy(
    params: ResolveInstallPolicyParams,
) -> crate::policy::InstallPolicyDecision {
    orchestration::resolve_install_policy(params)
}

#[cfg(test)]
pub(super) fn should_print_report(params: ShouldPrintReportParams<'_>) -> bool {
    orchestration::should_print_report(params)
}

#[cfg(test)]
#[path = "../../../tests/internal/install_tests.rs"]
mod tests;
