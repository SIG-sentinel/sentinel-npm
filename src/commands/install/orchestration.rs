use std::path::Path;
use std::process::ExitCode;

use crate::constants::{FALLBACK_PROCESS_EXIT_CODE, PACKAGE_VERSION_LATEST};
use crate::output::print_report;
use crate::types::{
    AnalyzeDependencyCyclesParams, AppendCiHistoryParams, CollectInstallPackagesParams,
    CompleteSuccessfulCiRunParams, DependencyNode, FinalizeCiDryRunParams, InstallArgs,
    InstallPackageRequest, OutputFormat, PackageRef, PrintAndSaveCiReportParams,
    PrintInstallCandidateResolvedParams, PrintReportParams, ResolveInstallTargetsParams,
    RunCleanInstallOrFailureParams, RunCleanInstallParams, SaveCiReportParams, SharedCommandState,
    SharedCommandStateError, ShouldPrintReportParams,
};
use crate::ui::command_feedback as ui;

// Re-export from submodules for backward compatibility
pub(super) use super::history::{append_ci_history, append_install_history};
pub(super) use super::lockfile::{ensure_ci_lockfile_ready, prepare_install_lockfiles};
pub(super) use super::policy::{
    collect_blocked_verify_results, print_block_reason_results, resolve_install_block_reason,
    resolve_install_policy,
};
pub(super) use super::resolve::{
    collect_install_packages_to_verify, parse_install_package_request,
    resolve_install_candidate_package,
};
pub(super) use super::source::install_from_verified_source;

pub(super) fn analyze_dependency_cycles(
    params: AnalyzeDependencyCyclesParams<'_>,
) -> Vec<Vec<String>> {
    let AnalyzeDependencyCyclesParams {
        dependency_tree,
        quiet,
        is_text_output,
    } = params;
    let cycles = dependency_tree.analyze().cycles.clone();
    let should_print_cycles = !cycles.is_empty() && !quiet && is_text_output;

    if should_print_cycles {
        ui::print_dependency_cycles(&cycles);
    }

    cycles
}

pub(super) fn should_print_report(params: ShouldPrintReportParams<'_>) -> bool {
    let ShouldPrintReportParams {
        output_format,
        quiet,
    } = params;

    !quiet || !matches!(output_format, OutputFormat::Text)
}

pub(super) fn save_ci_report(params: SaveCiReportParams<'_>) {
    let SaveCiReportParams {
        report,
        report_path,
        quiet,
        is_text_output,
    } = params;

    match serde_json::to_string_pretty(report) {
        Ok(json) => {
            let write_result = std::fs::write(report_path, &json);

            if let Err(error) = &write_result {
                ui::print_save_report_failed(error);
            }

            let report_saved_successfully = write_result.is_ok();
            let should_print_saved_message = report_saved_successfully && !quiet && is_text_output;

            if should_print_saved_message {
                ui::print_ci_report_saved(report_path);
            }
        }
        Err(error) => {
            ui::print_serialize_report_failed(&error);
        }
    }
}

pub(super) fn print_and_save_ci_report(params: PrintAndSaveCiReportParams<'_>) {
    let PrintAndSaveCiReportParams {
        args,
        report,
        is_text_output,
    } = params;
    let should_print_report_params = ShouldPrintReportParams {
        output_format: &args.format,
        quiet: args.quiet,
    };
    let should_print_final_report = should_print_report(should_print_report_params);

    if should_print_final_report {
        let print_report_params = PrintReportParams {
            report,
            output_format: &args.format,
        };

        print_report(print_report_params);
    }

    let save_ci_report_params = SaveCiReportParams {
        report,
        report_path: &args.report,
        quiet: args.quiet,
        is_text_output,
    };

    save_ci_report(save_ci_report_params);
}

pub(super) fn finalize_ci_dry_run(params: FinalizeCiDryRunParams<'_>) -> Option<ExitCode> {
    let FinalizeCiDryRunParams {
        args,
        report,
        is_text_output,
    } = params;

    if !args.dry_run {
        return None;
    }

    let print_and_save_ci_report_params = PrintAndSaveCiReportParams {
        args,
        report,
        is_text_output,
    };

    print_and_save_ci_report(print_and_save_ci_report_params);

    let should_print_dry_run_complete = !args.quiet && is_text_output;

    if should_print_dry_run_complete {
        ui::print_dry_run_complete(report.results.len());
    }

    Some(ExitCode::SUCCESS)
}

pub(super) fn run_clean_install_or_failure(
    params: RunCleanInstallOrFailureParams<'_>,
) -> Result<(), ExitCode> {
    let RunCleanInstallOrFailureParams {
        args,
        ignore_scripts,
    } = params;

    let run_clean_install_params = RunCleanInstallParams {
        current_working_directory: &args.cwd,
        ignore_scripts,
        omit_dev: args.omit_dev,
        omit_optional: args.omit_optional,
        silent_output: !matches!(args.format, OutputFormat::Text),
    };
    let install_status = crate::utils::run_clean_install(run_clean_install_params);

    let status = match install_status {
        Ok(status) => status,
        Err(error) => {
            ui::print_npm_ci_exec_failed(&error);
            return Err(ExitCode::FAILURE);
        }
    };

    if !status.success() {
        ui::print_npm_ci_failed_status(status.code().unwrap_or(FALLBACK_PROCESS_EXIT_CODE));

        return Err(ExitCode::FAILURE);
    }

    Ok(())
}

pub(super) fn complete_successful_ci_run(params: CompleteSuccessfulCiRunParams<'_>) -> ExitCode {
    let CompleteSuccessfulCiRunParams {
        args,
        report,
        lock_hash_before_verify,
        is_text_output,
    } = params;

    let print_and_save_ci_report_params = PrintAndSaveCiReportParams {
        args,
        report,
        is_text_output,
    };

    print_and_save_ci_report(print_and_save_ci_report_params);

    let clean_results_count = report
        .results
        .iter()
        .filter(|result| result.is_clean())
        .count();
    let should_print_install_success = !args.quiet && is_text_output;

    if should_print_install_success {
        ui::print_install_success(clean_results_count);
    }

    let append_ci_history_params = AppendCiHistoryParams {
        args,
        report,
        lock_hash_before_verify,
    };

    if let Err(error) = append_ci_history(append_ci_history_params) {
        ui::print_generic_error(&error);

        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

pub(super) fn parse_requested_install(
    args: &InstallArgs,
) -> Result<(InstallPackageRequest, PackageRef), ExitCode> {
    let install_request = parse_install_package_request(&args.package).ok_or_else(|| {
        let package_name_hint = args
            .package
            .split('@')
            .find(|segment| !segment.is_empty())
            .unwrap_or("<package>");

        ui::print_invalid_install_package_input(&args.package, package_name_hint);

        ExitCode::FAILURE
    })?;

    let candidate_spec = install_request
        .version_spec
        .clone()
        .unwrap_or_else(|| PACKAGE_VERSION_LATEST.to_string());
    let requested_package_ref = PackageRef::new(&install_request.package_name, &candidate_spec);

    Ok((install_request, requested_package_ref))
}

pub(super) fn load_install_shared_state(
    current_working_directory: &Path,
    timeout: u64,
    registry_max_in_flight: Option<usize>,
) -> Result<SharedCommandState, ExitCode> {
    use super::super::shared::load_command_state;

    match load_command_state(current_working_directory, timeout, registry_max_in_flight) {
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
            ui::print_verifier_init_failed(&error);
            Err(ExitCode::FAILURE)
        }
    }
}

pub(super) fn resolve_install_targets(
    params: ResolveInstallTargetsParams<'_>,
) -> Result<(PackageRef, Vec<DependencyNode>), ExitCode> {
    let ResolveInstallTargetsParams {
        args,
        dependency_tree,
        install_request,
        requested_package_ref,
        is_text_output,
    } = params;

    let Some(resolved_package_ref) =
        resolve_install_candidate_package(dependency_tree, install_request)
    else {
        ui::print_target_package_not_found(requested_package_ref);

        return Err(ExitCode::FAILURE);
    };

    let collect_install_packages_params = CollectInstallPackagesParams {
        dependency_tree,
        package_reference: &resolved_package_ref,
    };

    let Some(packages_to_verify) =
        collect_install_packages_to_verify(collect_install_packages_params)
    else {
        ui::print_target_package_not_found(&resolved_package_ref);

        return Err(ExitCode::FAILURE);
    };

    let is_resolved_candidate_changed =
        requested_package_ref.to_string() != resolved_package_ref.to_string();
    let should_print_resolved_candidate =
        is_text_output && !args.quiet && is_resolved_candidate_changed;

    if should_print_resolved_candidate {
        let transitive_count = packages_to_verify.len().saturating_sub(1);
        let print_install_candidate_resolved_params = PrintInstallCandidateResolvedParams {
            requested_spec: &args.package,
            resolved_candidate: &resolved_package_ref,
            transitive_count,
        };

        ui::print_install_candidate_resolved(print_install_candidate_resolved_params);
    }

    Ok((resolved_package_ref, packages_to_verify))
}
