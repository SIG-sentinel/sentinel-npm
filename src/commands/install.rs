use std::process::ExitCode;

use super::shared::{
    SharedCommandState, SharedCommandStateError, build_report, load_command_state,
    validate_package_json_dependencies, verify_packages,
};

use crate::constants::{
    INSTALL_ERR_LOCKFILE_GENERATE_FAILED, INSTALL_MAX_CONCURRENCY, INSTALL_MSG_NOTHING_TO_INSTALL,
    INSTALL_PROGRESS_TEMPLATE, INSTALL_PROGRESS_VERIFY_MSG,
};
use crate::output::{print_install_blocked, print_install_blocked_unverifiable, print_report};
use crate::policy::{DefaultSecurityPolicy, InstallPolicyDecision, SecurityPolicy};
use crate::types::{
    BlockedVerifyResults, CiArgs, CollectInstallPackagesParams, DependencyNode,
    EnsureLockfileExistsParams, FinalizeCiRunParams, FinalizeInstallRunParams, InstallArgs,
    InstallBlockReason, InstallExecutionOutcome, InstallPackageParams, InstallPolicyInput,
    PackageRef, PrepareLockfileForInstallParams, PreparedCiState, PreparedInstallState,
    PrintBlockReasonResultsParams, PrintCiBlockingResultsParams, PrintReportParams,
    ProgressBarConfig, ResolveInstallPolicyParams, ResolvePackageIntoLockfileParams,
    RestoreProjectFilesSnapshotParams, RunCleanInstallParams, SaveCiReportParams, Verdict,
    VerifyPackagesExecutionParams, VerifyPackagesParams, VerifyResult,
};
use crate::ui::command_feedback as ui;
use crate::utils::{
    capture_project_files_snapshot, create_progress_bar, generate_lockfile, install_package,
    lockfile_sha256, resolve_package_into_lockfile, restore_project_files_snapshot,
    run_clean_install,
};

fn parse_package_ref(spec: &str) -> Option<PackageRef> {
    let separator = spec.rfind('@')?;
    let package_name = &spec[..separator];
    let package_version = &spec[separator + 1..];

    if package_name.is_empty() || package_version.is_empty() {
        return None;
    }

    Some(PackageRef::new(package_name, package_version))
}

fn prepare_lockfile_for_install(params: PrepareLockfileForInstallParams<'_>) -> bool {
    let PrepareLockfileForInstallParams {
        current_working_directory,
        package_reference,
        quiet,
    } = params;

    if !quiet {
        ui::print_resolving_package_into_lockfile(package_reference);
    }

    let output = resolve_package_into_lockfile(ResolvePackageIntoLockfileParams {
        current_working_directory,
        package_reference,
    });

    match output {
        Ok(output) if output.status.success() => true,
        _ => {
            ui::print_resolve_package_into_lockfile_failed(package_reference);

            false
        }
    }
}

fn save_ci_report(params: SaveCiReportParams<'_>) {
    let SaveCiReportParams {
        report,
        report_path,
        quiet,
    } = params;

    match serde_json::to_string_pretty(report) {
        Ok(json) => {
            let write_result = std::fs::write(report_path, &json);

            if let Err(error) = &write_result {
                ui::print_save_report_failed(error);
            }

            if write_result.is_ok() && !quiet {
                ui::print_ci_report_saved(report_path);
            }
        }

        Err(error) => {
            ui::print_serialize_report_failed(&error);
        }
    }
}

fn collect_install_packages_to_verify(
    params: CollectInstallPackagesParams<'_>,
) -> Option<Vec<DependencyNode>> {
    let CollectInstallPackagesParams {
        dependency_tree,
        package_reference,
    } = params;

    let target_key = package_reference.to_string();
    let target_node = dependency_tree.nodes.get(&target_key)?;

    let mut keys_to_verify = dependency_tree.get_transitive_deps(&target_node.package);

    keys_to_verify.insert(target_key);

    let mut packages_to_verify: Vec<_> = keys_to_verify
        .iter()
        .filter_map(|key| dependency_tree.nodes.get(key).cloned())
        .collect();

    packages_to_verify
        .sort_by(|left, right| left.package.to_string().cmp(&right.package.to_string()));

    Some(packages_to_verify)
}

fn collect_blocked_verify_results(results: &[VerifyResult]) -> BlockedVerifyResults {
    let compromised = results
        .iter()
        .filter(|result| matches!(result.verdict, Verdict::Compromised { .. }))
        .cloned()
        .collect();
    let unverifiable = results
        .iter()
        .filter(|result| matches!(result.verdict, Verdict::Unverifiable { .. }))
        .cloned()
        .collect();

    BlockedVerifyResults {
        compromised,
        unverifiable,
    }
}

fn print_block_reason_results(params: PrintBlockReasonResultsParams<'_>) {
    let PrintBlockReasonResultsParams {
        block_reason,
        blocked,
    } = params;

    match block_reason {
        InstallBlockReason::Compromised => print_install_blocked(&blocked.compromised),
        InstallBlockReason::Unverifiable => {
            print_install_blocked_unverifiable(&blocked.unverifiable)
        }
    }
}

fn resolve_install_policy(params: ResolveInstallPolicyParams) -> InstallPolicyDecision {
    let ResolveInstallPolicyParams {
        compromised_count,
        unverifiable_count,
        allow_scripts,
        no_scripts,
    } = params;

    DefaultSecurityPolicy.install_decision(InstallPolicyInput {
        compromised_count,
        unverifiable_count,
        allow_scripts,
        no_scripts,
    })
}

async fn prepare_install_state(args: &InstallArgs) -> Result<PreparedInstallState, ExitCode> {
    if let Err(error) = validate_package_json_dependencies(&args.cwd) {
        ui::print_invalid_package_json(&error);

        return Err(ExitCode::FAILURE);
    }

    let package_ref = match parse_package_ref(&args.package) {
        Some(package_ref) => package_ref,
        None => {
            ui::print_invalid_package_format();

            return Err(ExitCode::FAILURE);
        }
    };

    let lockfile_ready = ensure_lockfile_exists(EnsureLockfileExistsParams {
        current_working_directory: &args.cwd,
        quiet: args.quiet,
    })
    .await;

    if !lockfile_ready {
        return Err(ExitCode::FAILURE);
    }

    let lockfile_prepared = prepare_lockfile_for_install(PrepareLockfileForInstallParams {
        current_working_directory: &args.cwd,
        package_reference: &package_ref,
        quiet: args.quiet,
    });

    if !lockfile_prepared {
        return Err(ExitCode::FAILURE);
    }

    let lock_hash_before_verify = lockfile_sha256(&args.cwd);

    let shared_state = match load_command_state(&args.cwd, args.timeout) {
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

    let analysis = dependency_tree.analyze();
    let cycles = analysis.cycles.clone();

    if !cycles.is_empty() && !args.quiet {
        ui::print_dependency_cycles(&cycles);
    }

    let packages_to_verify =
        match collect_install_packages_to_verify(CollectInstallPackagesParams {
            dependency_tree: &dependency_tree,
            package_reference: &package_ref,
        }) {
            Some(packages_to_verify) => packages_to_verify,
            None => {
                ui::print_target_package_not_found(&package_ref);

                return Err(ExitCode::FAILURE);
            }
        };

    Ok(PreparedInstallState {
        package_ref,
        packages_to_verify,
        verifier,
        lockfile_entries,
        lock_hash_before_verify,
        cycles,
    })
}

fn print_blocking_install_results(results: &[VerifyResult]) -> bool {
    let blocked = collect_blocked_verify_results(results);

    let has_compromised_results = !blocked.compromised.is_empty();

    if has_compromised_results {
        print_block_reason_results(PrintBlockReasonResultsParams {
            block_reason: InstallBlockReason::Compromised,
            blocked: &blocked,
        });

        return true;
    }

    let has_unverifiable_results = !blocked.unverifiable.is_empty();
    if has_unverifiable_results {
        print_block_reason_results(PrintBlockReasonResultsParams {
            block_reason: InstallBlockReason::Unverifiable,
            blocked: &blocked,
        });

        return true;
    }

    false
}

async fn prepare_ci_state(args: &CiArgs) -> Result<PreparedCiState, ExitCode> {
    if let Err(error) = validate_package_json_dependencies(&args.cwd) {
        ui::print_invalid_package_json(&error);

        return Err(ExitCode::FAILURE);
    }

    let lockfile_ready = ensure_lockfile_exists(EnsureLockfileExistsParams {
        current_working_directory: &args.cwd,
        quiet: args.quiet,
    })
    .await;

    if !lockfile_ready {
        return Err(ExitCode::FAILURE);
    }

    let lock_hash_before_verify = lockfile_sha256(&args.cwd);

    let shared_state = match load_command_state(&args.cwd, args.timeout) {
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

    let analysis = dependency_tree.analyze();
    let cycles = analysis.cycles.clone();

    if !cycles.is_empty() && !args.quiet {
        ui::print_dependency_cycles(&cycles);
    }

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

fn print_ci_blocking_results(params: PrintCiBlockingResultsParams<'_>) -> bool {
    let PrintCiBlockingResultsParams { results, args } = params;
    let blocked = collect_blocked_verify_results(results);

    let policy_decision = resolve_install_policy(ResolveInstallPolicyParams {
        compromised_count: blocked.compromised.len(),
        unverifiable_count: blocked.unverifiable.len(),
        allow_scripts: args.allow_scripts,
        no_scripts: args.no_scripts,
    });

    match policy_decision.block_reason {
        Some(InstallBlockReason::Compromised) => {
            print_block_reason_results(PrintBlockReasonResultsParams {
                block_reason: InstallBlockReason::Compromised,
                blocked: &blocked,
            });

            true
        }
        Some(InstallBlockReason::Unverifiable) => {
            print_block_reason_results(PrintBlockReasonResultsParams {
                block_reason: InstallBlockReason::Unverifiable,
                blocked: &blocked,
            });

            true
        }

        None => false,
    }
}

fn finalize_ci_run(params: FinalizeCiRunParams<'_>) -> ExitCode {
    let FinalizeCiRunParams {
        args,
        report,
        lock_hash_before_verify,
    } = params;

    if args.dry_run {
        ui::print_dry_run_complete(report.results.len());

        return ExitCode::SUCCESS;
    }

    let lock_hash_before_install = lockfile_sha256(&args.cwd);
    let lockfile_unchanged = lock_hash_before_install == *lock_hash_before_verify;

    if !lockfile_unchanged {
        ui::print_ci_lockfile_changed_abort();

        return ExitCode::FAILURE;
    }

    let policy_decision = resolve_install_policy(ResolveInstallPolicyParams {
        compromised_count: report.summary.compromised as usize,
        unverifiable_count: report.summary.unverifiable as usize,
        allow_scripts: args.allow_scripts,
        no_scripts: args.no_scripts,
    });

    let install_status = run_clean_install(RunCleanInstallParams {
        current_working_directory: &args.cwd,
        ignore_scripts: policy_decision.ignore_scripts,
        omit_dev: args.omit_dev,
        omit_optional: args.omit_optional,
    });

    match install_status {
        Ok(status) if status.success() => {}
        Ok(status) => {
            ui::print_npm_ci_failed_status(status.code().unwrap_or(1));

            return ExitCode::FAILURE;
        }
        Err(error) => {
            ui::print_npm_ci_exec_failed(&error);

            return ExitCode::FAILURE;
        }
    }

    if !args.quiet {
        print_report(PrintReportParams {
            report,
            output_format: &args.format,
        });
    }

    save_ci_report(SaveCiReportParams {
        report,
        report_path: &args.report,
        quiet: args.quiet,
    });

    let clean_results_count = report
        .results
        .iter()
        .filter(|result| result.verdict == Verdict::Clean)
        .count();

    if !args.quiet {
        ui::print_install_success(clean_results_count);
    }

    ExitCode::SUCCESS
}

fn finalize_install_run(params: FinalizeInstallRunParams<'_>) -> InstallExecutionOutcome {
    let FinalizeInstallRunParams {
        args,
        package_ref,
        report,
        lock_hash_before_verify,
    } = params;

    if !args.quiet {
        print_report(PrintReportParams {
            report,
            output_format: &args.format,
        });
    }

    if args.dry_run {
        ui::print_dry_run_complete(report.summary.total as usize);

        return InstallExecutionOutcome::success(true);
    }

    let lock_hash_before_install = lockfile_sha256(&args.cwd);
    let lockfile_unchanged = lock_hash_before_install == *lock_hash_before_verify;

    if !lockfile_unchanged {
        ui::print_install_lockfile_changed_abort();

        return InstallExecutionOutcome::failure();
    }

    let policy_decision = resolve_install_policy(ResolveInstallPolicyParams {
        compromised_count: report.summary.compromised as usize,
        unverifiable_count: report.summary.unverifiable as usize,
        allow_scripts: args.allow_scripts,
        no_scripts: args.no_scripts,
    });

    let install_status = install_package(InstallPackageParams {
        current_working_directory: &args.cwd,
        package_reference: package_ref,
        ignore_scripts: policy_decision.ignore_scripts,
    });

    match install_status {
        Ok(status) if status.success() => {
            if !args.quiet {
                ui::print_install_success(report.summary.clean as usize);
            }

            InstallExecutionOutcome::success(false)
        }
        Ok(status) => {
            ui::print_npm_install_failed_status(status.code().unwrap_or(1));
            InstallExecutionOutcome::failure()
        }
        Err(error) => {
            ui::print_npm_install_exec_failed(&error);
            InstallExecutionOutcome::failure()
        }
    }
}

pub async fn run_install(args: &InstallArgs) -> ExitCode {
    let snapshot = capture_project_files_snapshot(&args.cwd);
    let outcome = match prepare_install_state(args).await {
        Ok(prepared_state) => {
            let PreparedInstallState {
                package_ref,
                packages_to_verify,
                verifier,
                lockfile_entries,
                lock_hash_before_verify,
                cycles,
            } = prepared_state;

            if !args.quiet {
                ui::print_install_verification_started(packages_to_verify.len());
            }

            let verify_progress_bar = create_progress_bar(ProgressBarConfig {
                length: packages_to_verify.len(),
                message: INSTALL_PROGRESS_VERIFY_MSG,
                template: INSTALL_PROGRESS_TEMPLATE,
            });

            let results = verify_packages(VerifyPackagesExecutionParams {
                verify_packages_params: VerifyPackagesParams {
                    packages_to_verify,
                    verifier,
                    lockfile_entries,
                },
                max_concurrency: INSTALL_MAX_CONCURRENCY,
                progress_bar: Some(verify_progress_bar),
            })
            .await;

            let install_blocked = print_blocking_install_results(&results);

            match install_blocked {
                true => InstallExecutionOutcome::failure(),
                false => {
                    let report = build_report(crate::types::RunMode::Install, results, cycles);

                    finalize_install_run(FinalizeInstallRunParams {
                        args,
                        package_ref: &package_ref,
                        report: &report,
                        lock_hash_before_verify: &lock_hash_before_verify,
                    })
                }
            }
        }
        Err(exit_code) => InstallExecutionOutcome {
            exit_code,
            should_restore_snapshot: true,
        },
    };

    if outcome.should_restore_snapshot
        && let Err(error) = restore_project_files_snapshot(RestoreProjectFilesSnapshotParams {
            snapshot: &snapshot,
            current_working_directory: &args.cwd,
        })
    {
        ui::print_rollback_failed(&error);

        return ExitCode::FAILURE;
    }

    outcome.exit_code
}

async fn ensure_lockfile_exists(params: EnsureLockfileExistsParams<'_>) -> bool {
    let EnsureLockfileExistsParams {
        current_working_directory,
        quiet,
    } = params;

    let lockfile_path = current_working_directory.join(crate::constants::PACKAGE_LOCK_FILE);

    if lockfile_path.exists() {
        return true;
    }

    if !quiet {
        ui::print_missing_lockfile_notice();
    }

    let output = generate_lockfile(current_working_directory);

    match output {
        Ok(output) if output.status.success() => {
            if !quiet {
                ui::print_lockfile_created_notice();
            }
            true
        }
        _ => {
            ui::print_generic_error(INSTALL_ERR_LOCKFILE_GENERATE_FAILED);

            false
        }
    }
}

pub async fn run_ci(args: &CiArgs) -> ExitCode {
    let prepared_state = match prepare_ci_state(args).await {
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

    if !args.quiet {
        ui::print_install_verification_started(packages_to_verify.len());
    }

    let verify_progress_bar = create_progress_bar(ProgressBarConfig {
        length: packages_to_verify.len(),
        message: INSTALL_PROGRESS_VERIFY_MSG,
        template: INSTALL_PROGRESS_TEMPLATE,
    });

    let results = verify_packages(VerifyPackagesExecutionParams {
        verify_packages_params: VerifyPackagesParams {
            packages_to_verify,
            verifier,
            lockfile_entries,
        },
        max_concurrency: INSTALL_MAX_CONCURRENCY,
        progress_bar: Some(verify_progress_bar),
    })
    .await;

    let ci_blocked = print_ci_blocking_results(PrintCiBlockingResultsParams {
        results: &results,
        args,
    });

    if ci_blocked {
        return ExitCode::FAILURE;
    }

    let report = build_report(crate::types::RunMode::Ci, results, cycles);

    finalize_ci_run(FinalizeCiRunParams {
        args,
        report: &report,
        lock_hash_before_verify: &lock_hash_before_verify,
    })
}

#[cfg(test)]
#[path = "../../tests/internal/install_tests.rs"]
mod tests;
