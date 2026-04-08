use std::fmt::Display;
use std::path::Path;

use colored::Colorize;

use crate::constants::{
    CHECK_MSG_PROGRESS_TEMPLATE, CHECK_MSG_VERIFY_PROGRESS_TEMPLATE, CLI_NAME, CLI_PREFIX_SENTINEL, INSTALL_MSG_CI_REPORT_TEMPLATE,
    INSTALL_MSG_DRY_RUN_TEMPLATE, INSTALL_MSG_SUCCESS_TEMPLATE, INSTALL_MSG_VERIFYING_TEMPLATE,
    OUTPUT_SYMBOL_ERROR, UI_MSG_BUILD_DEPENDENCY_TREE_FAILED_TEMPLATE,
    UI_MSG_DEPENDENCY_CYCLE_LINE_TEMPLATE, UI_MSG_DEPENDENCY_CYCLES_HEADER_TEMPLATE,
    UI_MSG_INVALID_PACKAGE_FORMAT, UI_MSG_INVALID_PACKAGE_JSON_TEMPLATE,
    UI_MSG_LOCKFILE_CHANGED_ABORT_CI, UI_MSG_LOCKFILE_CHANGED_ABORT_INSTALL,
    UI_MSG_LOCKFILE_CREATED_NOTICE, UI_MSG_LOCKFILE_MISSING_NOTICE, UI_MSG_NO_PACKAGES_TO_VERIFY,
    UI_MSG_NPM_CI_EXEC_FAILED_TEMPLATE, UI_MSG_NPM_CI_STATUS_FAILED_TEMPLATE,
    UI_MSG_NPM_INSTALL_EXEC_FAILED_TEMPLATE, UI_MSG_NPM_INSTALL_STATUS_FAILED_TEMPLATE,
    UI_MSG_READ_LOCKFILE_ENTRIES_FAILED_TEMPLATE,
    UI_MSG_RESOLVE_PACKAGE_INTO_LOCKFILE_FAILED_TEMPLATE, UI_MSG_RESOLVING_PACKAGE_TEMPLATE,
    UI_MSG_ROLLBACK_FAILED_TEMPLATE, UI_MSG_SAVE_REPORT_FAILED_TEMPLATE,
    UI_MSG_SERIALIZE_REPORT_FAILED_TEMPLATE, UI_MSG_TARGET_PACKAGE_NOT_FOUND_TEMPLATE,
    UI_MSG_VERIFIER_INIT_FAILED_TEMPLATE, render_template,
};
use crate::types::PackageRef;

pub fn print_missing_lockfile_notice() {
    eprintln!(
        "{}",
        render_template(
            UI_MSG_LOCKFILE_MISSING_NOTICE,
            &[CLI_PREFIX_SENTINEL.yellow().bold().to_string()]
        )
    );
}

pub fn print_lockfile_created_notice() {
    eprintln!(
        "{}",
        render_template(
            UI_MSG_LOCKFILE_CREATED_NOTICE,
            &["✓".green().bold().to_string()]
        )
    );
}

pub fn print_resolving_package_into_lockfile(package_reference: &PackageRef) {
    eprintln!(
        "{}",
        render_template(
            UI_MSG_RESOLVING_PACKAGE_TEMPLATE,
            &[
                CLI_PREFIX_SENTINEL.yellow().bold().to_string(),
                package_reference.to_string(),
            ]
        )
    );
}

pub fn print_dependency_cycles(cycles: &[Vec<String>]) {
    eprintln!(
        "{}",
        render_template(
            UI_MSG_DEPENDENCY_CYCLES_HEADER_TEMPLATE,
            &[
                CLI_PREFIX_SENTINEL.red().bold().to_string(),
                cycles.len().to_string(),
            ]
        )
    );

    for (index, cycle) in cycles.iter().enumerate() {
        eprintln!(
            "{}",
            render_template(
                UI_MSG_DEPENDENCY_CYCLE_LINE_TEMPLATE,
                &[(index + 1).to_string(), cycle.join(" -> ")]
            )
        );
    }
}

pub fn print_verification_progress(completed: usize, total: usize, percentage: usize) {
    eprintln!(
        "{}",
        render_template(
            CHECK_MSG_VERIFY_PROGRESS_TEMPLATE,
            &[
                completed.to_string(),
                total.to_string(),
                percentage.to_string(),
            ],
        )
    );
}

pub fn print_check_progress(package_count: usize) {
    println!(
        "{}",
        render_template(
            CHECK_MSG_PROGRESS_TEMPLATE,
            &[
                CLI_NAME.cyan().bold().to_string(),
                package_count.to_string(),
            ]
        )
    );
}

pub fn print_install_verification_started(package_count: usize) {
    println!(
        "{}",
        render_template(
            INSTALL_MSG_VERIFYING_TEMPLATE,
            &[
                CLI_NAME.cyan().bold().to_string(),
                package_count.to_string(),
            ]
        )
    );
}

pub fn print_dry_run_complete(total_verified: usize) {
    println!(
        "{}",
        render_template(
            INSTALL_MSG_DRY_RUN_TEMPLATE,
            &[
                CLI_PREFIX_SENTINEL.cyan().bold().to_string(),
                total_verified.to_string(),
            ],
        )
    );
}

pub fn print_install_success(clean_count: usize) {
    println!(
        "{}",
        render_template(
            INSTALL_MSG_SUCCESS_TEMPLATE,
            &["✓".green().bold().to_string(), clean_count.to_string()]
        )
    );
}

pub fn print_ci_report_saved(report_path: &Path) {
    println!(
        "{}",
        render_template(
            INSTALL_MSG_CI_REPORT_TEMPLATE,
            &[report_path.display().to_string()]
        )
    );
}

pub fn print_invalid_package_json(error: &dyn Display) {
    eprintln!(
        "{}",
        render_template(UI_MSG_INVALID_PACKAGE_JSON_TEMPLATE, &[error.to_string()])
    );
}

pub fn print_invalid_package_format() {
    eprintln!("{UI_MSG_INVALID_PACKAGE_FORMAT}");
}

pub fn print_resolve_package_into_lockfile_failed(package_reference: &PackageRef) {
    eprintln!(
        "{}",
        render_template(
            UI_MSG_RESOLVE_PACKAGE_INTO_LOCKFILE_FAILED_TEMPLATE,
            &[package_reference.to_string()]
        )
    );
}

pub fn print_failed_to_read_lockfile_entries(error: &dyn Display) {
    eprintln!(
        "{}",
        render_template(
            UI_MSG_READ_LOCKFILE_ENTRIES_FAILED_TEMPLATE,
            &[error.to_string()]
        )
    );
}

pub fn print_failed_to_build_dependency_tree(error: &dyn Display) {
    eprintln!(
        "{}",
        render_template(
            UI_MSG_BUILD_DEPENDENCY_TREE_FAILED_TEMPLATE,
            &[error.to_string()]
        )
    );
}

pub fn print_target_package_not_found(package_reference: &PackageRef) {
    eprintln!(
        "{}",
        render_template(
            UI_MSG_TARGET_PACKAGE_NOT_FOUND_TEMPLATE,
            &[package_reference.to_string()]
        )
    );
}

pub fn print_verifier_init_failed(error: &dyn Display) {
    eprintln!(
        "{}",
        render_template(UI_MSG_VERIFIER_INIT_FAILED_TEMPLATE, &[error.to_string()])
    );
}

pub fn print_install_lockfile_changed_abort() {
    eprintln!("{UI_MSG_LOCKFILE_CHANGED_ABORT_INSTALL}");
}

pub fn print_ci_lockfile_changed_abort() {
    eprintln!("{UI_MSG_LOCKFILE_CHANGED_ABORT_CI}");
}

pub fn print_npm_install_failed_status(status_code: i32) {
    eprintln!(
        "{}",
        render_template(
            UI_MSG_NPM_INSTALL_STATUS_FAILED_TEMPLATE,
            &[status_code.to_string()]
        )
    );
}

pub fn print_npm_ci_failed_status(status_code: i32) {
    eprintln!(
        "{}",
        render_template(
            UI_MSG_NPM_CI_STATUS_FAILED_TEMPLATE,
            &[status_code.to_string()]
        )
    );
}

pub fn print_npm_install_exec_failed(error: &dyn Display) {
    eprintln!(
        "{}",
        render_template(
            UI_MSG_NPM_INSTALL_EXEC_FAILED_TEMPLATE,
            &[error.to_string()]
        )
    );
}

pub fn print_npm_ci_exec_failed(error: &dyn Display) {
    eprintln!(
        "{}",
        render_template(UI_MSG_NPM_CI_EXEC_FAILED_TEMPLATE, &[error.to_string()])
    );
}

pub fn print_rollback_failed(error: &dyn Display) {
    eprintln!(
        "{}",
        render_template(UI_MSG_ROLLBACK_FAILED_TEMPLATE, &[error.to_string()])
    );
}

pub fn print_save_report_failed(error: &dyn Display) {
    eprintln!(
        "{}",
        render_template(UI_MSG_SAVE_REPORT_FAILED_TEMPLATE, &[error.to_string()])
    );
}

pub fn print_serialize_report_failed(error: &dyn Display) {
    eprintln!(
        "{}",
        render_template(
            UI_MSG_SERIALIZE_REPORT_FAILED_TEMPLATE,
            &[error.to_string()]
        )
    );
}

pub fn print_no_packages_to_verify() {
    println!("{UI_MSG_NO_PACKAGES_TO_VERIFY}");
}

pub fn print_generic_error(message: &str) {
    eprintln!("{} {}", OUTPUT_SYMBOL_ERROR.red().bold(), message);
}
