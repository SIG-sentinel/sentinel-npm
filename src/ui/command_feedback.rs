use std::fmt::Display;
use std::path::Path;

use colored::Colorize;

use crate::constants::{
    CHECK_MSG_PROGRESS_TEMPLATE, CHECK_MSG_VERIFY_PROGRESS_TEMPLATE, CLI_NAME, CLI_PREFIX_SENTINEL,
    INSTALL_MSG_CI_REPORT_TEMPLATE, INSTALL_MSG_DRY_RUN_TEMPLATE, INSTALL_MSG_SUCCESS_TEMPLATE,
    INSTALL_MSG_VERIFYING_TEMPLATE, OUTPUT_SYMBOL_ERROR,
    UI_MSG_BUILD_DEPENDENCY_TREE_FAILED_TEMPLATE, UI_MSG_DEPENDENCY_CYCLE_LINE_TEMPLATE,
    UI_MSG_DEPENDENCY_CYCLES_HEADER_TEMPLATE, UI_MSG_INSTALL_CANDIDATE_RESOLVED_TEMPLATE,
    UI_MSG_INVALID_INSTALL_PACKAGE_INPUT_TEMPLATE, UI_MSG_INVALID_PACKAGE_FORMAT,
    UI_MSG_INVALID_PACKAGE_JSON_TEMPLATE, UI_MSG_LOCKFILE_CHANGED_ABORT_CI,
    UI_MSG_LOCKFILE_CHANGED_ABORT_INSTALL, UI_MSG_LOCKFILE_CREATED_NOTICE,
    UI_MSG_LOCKFILE_MISSING_NOTICE, UI_MSG_NO_PACKAGES_TO_VERIFY,
    UI_MSG_NPM_CI_EXEC_FAILED_TEMPLATE, UI_MSG_NPM_CI_STATUS_FAILED_TEMPLATE,
    UI_MSG_NPM_INSTALL_EXEC_FAILED_TEMPLATE, UI_MSG_NPM_INSTALL_STATUS_FAILED_TEMPLATE,
    UI_MSG_READ_LOCKFILE_ENTRIES_FAILED_TEMPLATE,
    UI_MSG_RESOLVE_PACKAGE_INTO_LOCKFILE_FAILED_TEMPLATE, UI_MSG_RESOLVING_PACKAGE_TEMPLATE,
    UI_MSG_ROLLBACK_FAILED_TEMPLATE, UI_MSG_SAVE_REPORT_FAILED_TEMPLATE,
    UI_MSG_SCRIPTS_BLOCKED_BY_DEFAULT, UI_MSG_SERIALIZE_REPORT_FAILED_TEMPLATE,
    UI_MSG_TARGET_PACKAGE_NOT_FOUND_TEMPLATE, UI_MSG_VERIFIER_INIT_FAILED_TEMPLATE,
    WARN_POST_VERIFY_ELAPSED_TEMPLATE, WARN_POST_VERIFY_LARGE_SCOPE_TEMPLATE, render_template,
};
use crate::types::{
    PackageRef, PrintInstallCandidateResolvedParams, PrintPostVerifyElapsedWarningParams,
    PrintVerificationProgressParams,
};

fn print_stderr_template(template: &str, template_args: &[String]) {
    eprintln!("{}", render_template(template, template_args));
}

fn print_stdout_template(template: &str, template_args: &[String]) {
    println!("{}", render_template(template, template_args));
}

fn format_post_verify_large_scope_warning(command_name: &str, package_count: usize) -> String {
    WARN_POST_VERIFY_LARGE_SCOPE_TEMPLATE
        .replace("{command_name}", command_name)
        .replace("{package_count}", &package_count.to_string())
}

fn format_post_verify_elapsed_warning(params: PrintPostVerifyElapsedWarningParams<'_>) -> String {
    let PrintPostVerifyElapsedWarningParams {
        command_name,
        package_count,
        elapsed_secs,
        good_term_secs,
    } = params;

    WARN_POST_VERIFY_ELAPSED_TEMPLATE
        .replace("{command_name}", command_name)
        .replace("{elapsed}", &elapsed_secs.to_string())
        .replace("{package_count}", &package_count.to_string())
        .replace("{good_term_secs}", &good_term_secs.to_string())
}

pub fn print_missing_lockfile_notice() {
    let lockfile_missing_notice_template_args =
        vec![CLI_PREFIX_SENTINEL.yellow().bold().to_string()];

    print_stderr_template(
        UI_MSG_LOCKFILE_MISSING_NOTICE,
        &lockfile_missing_notice_template_args,
    );
}

pub fn print_lockfile_created_notice() {
    let lockfile_created_notice_template_args = vec!["✓".green().bold().to_string()];

    print_stderr_template(
        UI_MSG_LOCKFILE_CREATED_NOTICE,
        &lockfile_created_notice_template_args,
    );
}

pub fn print_resolving_package_into_lockfile(package_reference: &PackageRef) {
    let resolving_package_template_args = vec![
        CLI_PREFIX_SENTINEL.yellow().bold().to_string(),
        package_reference.to_string(),
    ];

    print_stderr_template(
        UI_MSG_RESOLVING_PACKAGE_TEMPLATE,
        &resolving_package_template_args,
    );
}

pub fn print_dependency_cycles(cycles: &[Vec<String>]) {
    let dependency_cycles_header_template_args = vec![
        CLI_PREFIX_SENTINEL.red().bold().to_string(),
        cycles.len().to_string(),
    ];

    print_stderr_template(
        UI_MSG_DEPENDENCY_CYCLES_HEADER_TEMPLATE,
        &dependency_cycles_header_template_args,
    );

    for (index, cycle) in cycles.iter().enumerate() {
        let dependency_cycle_line_template_args = vec![(index + 1).to_string(), cycle.join(" -> ")];

        print_stderr_template(
            UI_MSG_DEPENDENCY_CYCLE_LINE_TEMPLATE,
            &dependency_cycle_line_template_args,
        );
    }
}

pub fn print_verification_progress(params: PrintVerificationProgressParams) {
    let PrintVerificationProgressParams {
        completed,
        total,
        percentage,
    } = params;
    let verify_progress_template_args = vec![
        completed.to_string(),
        total.to_string(),
        percentage.to_string(),
    ];

    print_stderr_template(
        CHECK_MSG_VERIFY_PROGRESS_TEMPLATE,
        &verify_progress_template_args,
    );
}

pub fn print_check_progress(package_count: usize) {
    let check_progress_template_args = vec![
        CLI_NAME.cyan().bold().to_string(),
        package_count.to_string(),
    ];

    print_stdout_template(CHECK_MSG_PROGRESS_TEMPLATE, &check_progress_template_args);
}

pub fn print_install_verification_started(package_count: usize) {
    let install_verification_started_template_args = vec![
        CLI_NAME.cyan().bold().to_string(),
        package_count.to_string(),
    ];

    print_stdout_template(
        INSTALL_MSG_VERIFYING_TEMPLATE,
        &install_verification_started_template_args,
    );
}

pub fn print_ci_verification_started(package_count: usize) {
    let ci_verification_started_template_args = vec![
        CLI_NAME.cyan().bold().to_string(),
        package_count.to_string(),
    ];

    print_stdout_template(
        INSTALL_MSG_VERIFYING_TEMPLATE,
        &ci_verification_started_template_args,
    );
}

pub fn print_dry_run_complete(total_verified: usize) {
    let dry_run_complete_template_args = vec![
        CLI_PREFIX_SENTINEL.cyan().bold().to_string(),
        total_verified.to_string(),
    ];

    print_stdout_template(
        INSTALL_MSG_DRY_RUN_TEMPLATE,
        &dry_run_complete_template_args,
    );
}

pub fn print_install_success(clean_count: usize) {
    let install_success_template_args =
        vec!["✓".green().bold().to_string(), clean_count.to_string()];

    print_stdout_template(INSTALL_MSG_SUCCESS_TEMPLATE, &install_success_template_args);
}

pub fn print_ci_report_saved(report_path: &Path) {
    let ci_report_saved_template_args = vec![report_path.display().to_string()];

    print_stdout_template(
        INSTALL_MSG_CI_REPORT_TEMPLATE,
        &ci_report_saved_template_args,
    );
}

pub fn print_scripts_blocked_by_default_notice() {
    eprintln!("{UI_MSG_SCRIPTS_BLOCKED_BY_DEFAULT}");
}

pub fn print_post_verify_large_scope_warning(command_name: &str, package_count: usize) {
    let warning_message = format_post_verify_large_scope_warning(command_name, package_count);

    eprintln!("{warning_message}");
}

pub fn print_post_verify_elapsed_warning(params: PrintPostVerifyElapsedWarningParams<'_>) {
    let warning_message = format_post_verify_elapsed_warning(params);

    eprintln!("{warning_message}");
}

pub fn print_invalid_package_json(error: &dyn Display) {
    let invalid_package_json_template_args = vec![error.to_string()];

    print_stderr_template(
        UI_MSG_INVALID_PACKAGE_JSON_TEMPLATE,
        &invalid_package_json_template_args,
    );
}

pub fn print_invalid_package_format() {
    eprintln!("{UI_MSG_INVALID_PACKAGE_FORMAT}");
}

pub fn print_invalid_install_package_input(input: &str, package_name_hint: &str) {
    let invalid_install_package_input_template_args =
        vec![input.to_string(), package_name_hint.to_string()];

    print_stderr_template(
        UI_MSG_INVALID_INSTALL_PACKAGE_INPUT_TEMPLATE,
        &invalid_install_package_input_template_args,
    );
}

pub fn print_install_candidate_resolved(params: PrintInstallCandidateResolvedParams<'_>) {
    let PrintInstallCandidateResolvedParams {
        requested_spec,
        resolved_candidate,
        transitive_count,
    } = params;

    let install_candidate_resolved_template_args = vec![
        CLI_PREFIX_SENTINEL.cyan().bold().to_string(),
        requested_spec.to_string(),
        resolved_candidate.to_string(),
        transitive_count.to_string(),
    ];

    print_stdout_template(
        UI_MSG_INSTALL_CANDIDATE_RESOLVED_TEMPLATE,
        &install_candidate_resolved_template_args,
    );
}

pub fn print_resolve_package_into_lockfile_failed(package_reference: &PackageRef) {
    let resolve_package_failed_template_args = vec![package_reference.to_string()];

    print_stderr_template(
        UI_MSG_RESOLVE_PACKAGE_INTO_LOCKFILE_FAILED_TEMPLATE,
        &resolve_package_failed_template_args,
    );
}

pub fn print_failed_to_read_lockfile_entries(error: &dyn Display) {
    let read_lockfile_entries_failed_template_args = vec![error.to_string()];

    print_stderr_template(
        UI_MSG_READ_LOCKFILE_ENTRIES_FAILED_TEMPLATE,
        &read_lockfile_entries_failed_template_args,
    );
}

pub fn print_failed_to_build_dependency_tree(error: &dyn Display) {
    let build_dependency_tree_failed_template_args = vec![error.to_string()];

    print_stderr_template(
        UI_MSG_BUILD_DEPENDENCY_TREE_FAILED_TEMPLATE,
        &build_dependency_tree_failed_template_args,
    );
}

pub fn print_target_package_not_found(package_reference: &PackageRef) {
    let target_package_not_found_template_args = vec![package_reference.to_string()];

    print_stderr_template(
        UI_MSG_TARGET_PACKAGE_NOT_FOUND_TEMPLATE,
        &target_package_not_found_template_args,
    );
}

pub fn print_verifier_init_failed(error: &dyn Display) {
    let verifier_init_failed_template_args = vec![error.to_string()];

    print_stderr_template(
        UI_MSG_VERIFIER_INIT_FAILED_TEMPLATE,
        &verifier_init_failed_template_args,
    );
}

pub fn print_install_lockfile_changed_abort() {
    eprintln!("{UI_MSG_LOCKFILE_CHANGED_ABORT_INSTALL}");
}

pub fn print_ci_lockfile_changed_abort() {
    eprintln!("{UI_MSG_LOCKFILE_CHANGED_ABORT_CI}");
}

pub fn print_npm_install_failed_status(status_code: i32) {
    let npm_install_status_failed_template_args = vec![status_code.to_string()];

    print_stderr_template(
        UI_MSG_NPM_INSTALL_STATUS_FAILED_TEMPLATE,
        &npm_install_status_failed_template_args,
    );
}

pub fn print_npm_ci_failed_status(status_code: i32) {
    let npm_ci_status_failed_template_args = vec![status_code.to_string()];

    print_stderr_template(
        UI_MSG_NPM_CI_STATUS_FAILED_TEMPLATE,
        &npm_ci_status_failed_template_args,
    );
}

pub fn print_npm_install_exec_failed(error: &dyn Display) {
    let npm_install_exec_failed_template_args = vec![error.to_string()];

    print_stderr_template(
        UI_MSG_NPM_INSTALL_EXEC_FAILED_TEMPLATE,
        &npm_install_exec_failed_template_args,
    );
}

pub fn print_npm_ci_exec_failed(error: &dyn Display) {
    let npm_ci_exec_failed_template_args = vec![error.to_string()];

    print_stderr_template(
        UI_MSG_NPM_CI_EXEC_FAILED_TEMPLATE,
        &npm_ci_exec_failed_template_args,
    );
}

pub fn print_rollback_failed(error: &dyn Display) {
    let rollback_failed_template_args = vec![error.to_string()];

    print_stderr_template(
        UI_MSG_ROLLBACK_FAILED_TEMPLATE,
        &rollback_failed_template_args,
    );
}

pub fn print_save_report_failed(error: &dyn Display) {
    let save_report_failed_template_args = vec![error.to_string()];

    print_stderr_template(
        UI_MSG_SAVE_REPORT_FAILED_TEMPLATE,
        &save_report_failed_template_args,
    );
}

pub fn print_serialize_report_failed(error: &dyn Display) {
    let serialize_report_failed_template_args = vec![error.to_string()];

    print_stderr_template(
        UI_MSG_SERIALIZE_REPORT_FAILED_TEMPLATE,
        &serialize_report_failed_template_args,
    );
}

pub fn print_no_packages_to_verify() {
    println!("{UI_MSG_NO_PACKAGES_TO_VERIFY}");
}

pub fn print_generic_error(message: &str) {
    eprintln!("{} {}", OUTPUT_SYMBOL_ERROR.red().bold(), message);
}

pub fn print_warn_post_verify_large_scope(command_name: &str, package_count: usize) {
    print_post_verify_large_scope_warning(command_name, package_count);
}

pub fn print_warn_post_verify_elapsed(params: PrintPostVerifyElapsedWarningParams<'_>) {
    print_post_verify_elapsed_warning(params);
}
