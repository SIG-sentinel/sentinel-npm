use super::shared::{
    SharedCommandState, SharedCommandStateError, build_report, load_command_state, verify_packages,
};
use crate::constants::{
    CHECK_MAX_CONCURRENCY, CHECK_MSG_INIT_FAILED_TEMPLATE, NPM_ERR_EXEC_FAILED_TEMPLATE,
    NPM_ERR_LOCKFILE_ONLY_FAILED_TEMPLATE, PACKAGE_LOCK_FILE, render_template,
};
use crate::output::print_report;
use crate::types::{
    CheckArgs, CollectPackagesToVerifyParams, DependencyNode, EnsureLockfileExistsForCheckParams,
    PreparedCheckState, PrintReportParams, SentinelError, VerifyPackagesExecutionParams,
    VerifyPackagesParams,
};
use crate::ui::command_feedback as ui;
use crate::utils::generate_lockfile;
use std::process::ExitCode;

async fn ensure_lockfile_exists(
    params: EnsureLockfileExistsForCheckParams<'_>,
) -> Result<bool, SentinelError> {
    let EnsureLockfileExistsForCheckParams {
        current_working_directory,
        quiet,
    } = params;

    let lockfile_path = current_working_directory.join(PACKAGE_LOCK_FILE);

    if lockfile_path.exists() {
        return Ok(true);
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

            Ok(true)
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);

            Err(SentinelError::LockfileParse(render_template(
                NPM_ERR_LOCKFILE_ONLY_FAILED_TEMPLATE,
                &[stderr.to_string()],
            )))
        }
        Err(e) => Err(SentinelError::LockfileParse(render_template(
            NPM_ERR_EXEC_FAILED_TEMPLATE,
            &[e.to_string()],
        ))),
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

    let _omit_optional_requested = check_args.omit_optional;

    packages_to_verify
}

async fn prepare_check_state(args: &CheckArgs) -> Result<PreparedCheckState, ExitCode> {
    if let Err(error) = ensure_lockfile_exists(EnsureLockfileExistsForCheckParams {
        current_working_directory: &args.cwd,
        quiet: args.quiet,
    })
    .await
    {
        ui::print_generic_error(&render_template(
            CHECK_MSG_INIT_FAILED_TEMPLATE,
            &[error.to_string()],
        ));

        return Err(ExitCode::FAILURE);
    }

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
            ui::print_generic_error(&render_template(
                CHECK_MSG_INIT_FAILED_TEMPLATE,
                &[error.to_string()],
            ));

            return Err(ExitCode::FAILURE);
        }
    };

    let SharedCommandState {
        dependency_tree,
        lockfile_entries,
        verifier,
    } = shared_state;

    let analysis = dependency_tree.analyze();

    if !args.quiet {
        ui::print_check_progress(analysis.total_packages);
    }

    let cycles = analysis.cycles.clone();

    if !cycles.is_empty() && !args.quiet {
        ui::print_dependency_cycles(&cycles);
    }

    let packages_to_verify = collect_packages_to_verify(CollectPackagesToVerifyParams {
        check_args: args,
        dependency_nodes: &dependency_tree.nodes,
    });

    Ok(PreparedCheckState {
        verifier,
        lockfile_entries,
        packages_to_verify,
        cycles,
    })
}

pub async fn run(args: &CheckArgs) -> ExitCode {
    let prepared_state = match prepare_check_state(args).await {
        Ok(prepared_state) => prepared_state,
        Err(exit_code) => return exit_code,
    };

    let PreparedCheckState {
        verifier,
        lockfile_entries,
        packages_to_verify,
        cycles,
    } = prepared_state;

    if packages_to_verify.is_empty() {
        if !args.quiet {
            ui::print_no_packages_to_verify();
        }

        return ExitCode::SUCCESS;
    }

    let results = verify_packages(VerifyPackagesExecutionParams {
        verify_packages_params: VerifyPackagesParams {
            packages_to_verify,
            verifier,
            lockfile_entries,
        },
        max_concurrency: CHECK_MAX_CONCURRENCY,
        progress_bar: None,
    })
    .await;

    let report = build_report(crate::types::RunMode::Check, results, cycles);

    print_report(PrintReportParams {
        report: &report,
        output_format: &args.format,
    });

    ExitCode::from(report.summary.exit_code as u8)
}
