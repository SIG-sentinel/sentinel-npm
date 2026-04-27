use std::path::{Path, PathBuf};

use crate::constants::{
    CLI_COMMAND_HINT_CI, CLI_COMMAND_HINT_INSTALL, PACKAGE_JSON_FILE, PACKAGE_LOCK_FILE,
    PARSE_INVALID_PACKAGE_MANAGER_TEMPLATE, PNPM_LOCK_FILE, SETUP_COMMAND_SUGGESTION_TEMPLATE,
    SETUP_DETECTED_LOCKFILES_TEMPLATE, YARN_LOCK_FILE, render_template,
};
use crate::types::{
    BuildResolveErrorMessageParams, PackageManager, ResolvePackageManagerParams,
    StartsWithManagerPrefixParams,
};

const PACKAGE_MANAGER_FIELD: &str = "packageManager";
const EMPTY_PACKAGE_MANAGER: &str = "";
const NPM_PREFIX_AT: &str = "npm@";
const NPM_PREFIX_SPACE: &str = "npm ";
const YARN_PREFIX_AT: &str = "yarn@";
const YARN_PREFIX_SPACE: &str = "yarn ";
const PNPM_PREFIX_AT: &str = "pnpm@";
const PNPM_PREFIX_SPACE: &str = "pnpm ";
const MULTIPLE_LOCKFILES_THRESHOLD: usize = 1;
const PACKAGE_MANAGER_NPM: &str = "npm";
const PACKAGE_MANAGER_YARN: &str = "yarn";
const PACKAGE_MANAGER_PNPM: &str = "pnpm";
const SETUP_AUTODETECT_FAILED_MESSAGE: &str = "[setup] package manager auto-detection failed.";
const SETUP_NO_LOCKFILE_CONTEXT_MESSAGE: &str =
    "No lockfile found in project root (package-lock.json, yarn.lock, pnpm-lock.yaml).";
const SETUP_EXPLICIT_COMMAND_HEADER: &str = "Run one command explicitly:";
const SETUP_NO_LOCKFILE_TIP: &str =
    "Tip: create a lockfile first (recommended): sentinel ci --init";
const SETUP_SCRIPT_BLOCKED_TIP: &str =
    "Tip: lifecycle scripts are blocked by default. Use --allow-scripts only when required.";

const LOCKFILE_MANAGER_MAPPINGS: [(&str, PackageManager); 3] = [
    (PACKAGE_LOCK_FILE, PackageManager::Npm),
    (YARN_LOCK_FILE, PackageManager::Yarn),
    (PNPM_LOCK_FILE, PackageManager::Pnpm),
];

fn starts_with_manager_prefix(params: StartsWithManagerPrefixParams<'_>) -> bool {
    let StartsWithManagerPrefixParams {
        value,
        prefix_at,
        prefix_space,
    } = params;
    let starts_with_at = value.starts_with(prefix_at);
    let starts_with_space = value.starts_with(prefix_space);

    starts_with_at || starts_with_space
}

fn read_package_manager_field(project_dir: &Path) -> Option<PackageManager> {
    let package_json_path = project_dir.join(PACKAGE_JSON_FILE);
    let content = std::fs::read_to_string(&package_json_path).ok()?;
    let package_json_value: serde_json::Value = serde_json::from_str(&content).ok()?;

    let package_manager_field_value = package_json_value
        .get(PACKAGE_MANAGER_FIELD)
        .and_then(|value| value.as_str())
        .unwrap_or(EMPTY_PACKAGE_MANAGER);

    let is_npm_prefix_params = StartsWithManagerPrefixParams {
        value: package_manager_field_value,
        prefix_at: NPM_PREFIX_AT,
        prefix_space: NPM_PREFIX_SPACE,
    };
    let is_npm_prefix = starts_with_manager_prefix(is_npm_prefix_params);

    let is_yarn_prefix_params = StartsWithManagerPrefixParams {
        value: package_manager_field_value,
        prefix_at: YARN_PREFIX_AT,
        prefix_space: YARN_PREFIX_SPACE,
    };
    let is_yarn_prefix = starts_with_manager_prefix(is_yarn_prefix_params);

    let has_pnpm_manager_prefix_params = StartsWithManagerPrefixParams {
        value: package_manager_field_value,
        prefix_at: PNPM_PREFIX_AT,
        prefix_space: PNPM_PREFIX_SPACE,
    };
    let has_pnpm_manager_prefix = starts_with_manager_prefix(has_pnpm_manager_prefix_params);

    match (is_npm_prefix, is_yarn_prefix, has_pnpm_manager_prefix) {
        (true, _, _) => Some(PackageManager::Npm),
        (false, true, _) => Some(PackageManager::Yarn),
        (false, false, true) => Some(PackageManager::Pnpm),
        _ => None,
    }
}

pub fn detect_package_manager(project_dir: &Path) -> Option<PackageManager> {
    let has_package_lock = project_dir.join(PACKAGE_LOCK_FILE).exists();
    let has_yarn_lock = project_dir.join(YARN_LOCK_FILE).exists();
    let has_pnpm_lock = project_dir.join(PNPM_LOCK_FILE).exists();

    let lockfile_count = [has_package_lock, has_yarn_lock, has_pnpm_lock]
        .iter()
        .filter(|&&x| x)
        .count();

    if lockfile_count > MULTIPLE_LOCKFILES_THRESHOLD {
        return None;
    }

    match (has_package_lock, has_yarn_lock, has_pnpm_lock) {
        (true, _, _) => Some(PackageManager::Npm),
        (false, true, _) => Some(PackageManager::Yarn),
        (false, false, true) => Some(PackageManager::Pnpm),
        _ => read_package_manager_field(project_dir),
    }
}

pub fn parse_package_manager(pm_str: &str) -> Option<PackageManager> {
    match pm_str {
        value if value.eq_ignore_ascii_case(PACKAGE_MANAGER_NPM) => Some(PackageManager::Npm),
        value if value.eq_ignore_ascii_case(PACKAGE_MANAGER_YARN) => Some(PackageManager::Yarn),
        value if value.eq_ignore_ascii_case(PACKAGE_MANAGER_PNPM) => Some(PackageManager::Pnpm),
        _ => None,
    }
}

fn detected_lockfiles(project_dir: &Path) -> Vec<&'static str> {
    LOCKFILE_MANAGER_MAPPINGS
        .iter()
        .filter_map(|(lockfile_name, _)| {
            let lockfile_exists = project_dir.join(lockfile_name).exists();
            lockfile_exists.then_some(*lockfile_name)
        })
        .collect()
}

fn suggested_package_managers(project_dir: &Path) -> Vec<PackageManager> {
    let lockfiles = detected_lockfiles(project_dir);

    if lockfiles.is_empty() {
        return vec![
            PackageManager::Npm,
            PackageManager::Yarn,
            PackageManager::Pnpm,
        ];
    }

    LOCKFILE_MANAGER_MAPPINGS
        .iter()
        .filter_map(|(lockfile_name, package_manager)| {
            let has_lockfile = lockfiles.contains(lockfile_name);
            has_lockfile.then_some(*package_manager)
        })
        .collect()
}

fn build_resolve_error_message(params: BuildResolveErrorMessageParams<'_>) -> String {
    let BuildResolveErrorMessageParams {
        project_dir,
        command_hint,
    } = params;
    let lockfiles = detected_lockfiles(project_dir);
    let suggestions = suggested_package_managers(project_dir);

    let mut lockfile_context = SETUP_NO_LOCKFILE_CONTEXT_MESSAGE.to_string();

    if !lockfiles.is_empty() {
        let detected_lockfiles = lockfiles.join(", ");

        lockfile_context =
            render_template(SETUP_DETECTED_LOCKFILES_TEMPLATE, &[detected_lockfiles]);
    }

    let mut lines = vec![
        SETUP_AUTODETECT_FAILED_MESSAGE.to_string(),
        lockfile_context,
        SETUP_EXPLICIT_COMMAND_HEADER.to_string(),
    ];

    for manager in suggestions {
        let suggestion = render_template(
            SETUP_COMMAND_SUGGESTION_TEMPLATE,
            &[command_hint.to_string(), manager.command().to_string()],
        );

        lines.push(suggestion);
    }

    if lockfiles.is_empty() {
        lines.push(String::new());
        lines.push(SETUP_NO_LOCKFILE_TIP.to_string());
    }

    let is_ci_command = command_hint.starts_with(CLI_COMMAND_HINT_CI);
    let is_install_command = command_hint.starts_with(CLI_COMMAND_HINT_INSTALL);
    let should_add_script_tip = is_ci_command || is_install_command;

    if should_add_script_tip {
        lines.push(SETUP_SCRIPT_BLOCKED_TIP.to_string());
    }

    lines.join("\n")
}

pub fn resolve_package_manager(
    params: &ResolvePackageManagerParams<'_>,
) -> Result<PackageManager, String> {
    let ResolvePackageManagerParams {
        project_dir,
        explicit_pm,
        command_hint,
    } = *params;

    match explicit_pm {
        Some(pm_name) => parse_package_manager(pm_name).ok_or_else(|| {
            render_template(
                PARSE_INVALID_PACKAGE_MANAGER_TEMPLATE,
                &[pm_name.to_string()],
            )
        }),
        None => detect_package_manager(project_dir).ok_or_else(|| {
            let build_resolve_error_message_params = BuildResolveErrorMessageParams {
                project_dir,
                command_hint,
            };

            build_resolve_error_message(build_resolve_error_message_params)
        }),
    }
}

pub fn active_lockfile_path(project_dir: &Path) -> PathBuf {
    match detect_package_manager(project_dir) {
        Some(manager) => project_dir.join(manager.lockfile_name()),
        None => project_dir.join(PACKAGE_LOCK_FILE),
    }
}
