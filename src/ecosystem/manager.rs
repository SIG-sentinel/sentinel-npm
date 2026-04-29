use std::path::{Path, PathBuf};

use crate::constants::{
    CLI_COMMAND_HINT_CI, CLI_COMMAND_HINT_INSTALL, EMPTY_PACKAGE_MANAGER,
    MULTIPLE_LOCKFILES_THRESHOLD, NPM_PREFIX_AT, NPM_PREFIX_SPACE, PACKAGE_JSON_FILE,
    PACKAGE_LOCK_FILE, PACKAGE_MANAGER_FIELD, PACKAGE_MANAGER_NPM, PACKAGE_MANAGER_PNPM,
    PACKAGE_MANAGER_YARN, PARSE_INVALID_PACKAGE_MANAGER_TEMPLATE, PNPM_LOCK_FILE, PNPM_PREFIX_AT,
    PNPM_PREFIX_SPACE, SETUP_AUTODETECT_FAILED_MESSAGE, SETUP_COMMAND_SUGGESTION_TEMPLATE,
    SETUP_DETECTED_LOCKFILES_TEMPLATE, SETUP_EXPLICIT_COMMAND_HEADER,
    SETUP_NO_LOCKFILE_CONTEXT_MESSAGE, SETUP_NO_LOCKFILE_TIP, SETUP_SCRIPT_BLOCKED_TIP,
    YARN_LOCK_FILE, YARN_PREFIX_AT, YARN_PREFIX_SPACE, render_template,
};
use crate::types::{
    BuildResolveErrorMessageParams, PackageManager, ResolvePackageManagerParams,
    StartsWithManagerPrefixParams,
};

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

fn build_lockfile_context(lockfiles: &[&str]) -> String {
    if lockfiles.is_empty() {
        SETUP_NO_LOCKFILE_CONTEXT_MESSAGE.to_string()
    } else {
        let detected_lockfiles = lockfiles.join(", ");
        render_template(SETUP_DETECTED_LOCKFILES_TEMPLATE, &[detected_lockfiles])
    }
}

fn collect_lockfile_tip_lines(lockfiles: &[&str]) -> Vec<String> {
    if lockfiles.is_empty() {
        vec![String::new(), SETUP_NO_LOCKFILE_TIP.to_string()]
    } else {
        Vec::new()
    }
}

fn should_add_setup_script_tip(command_hint: &str) -> bool {
    let is_ci_command = command_hint.starts_with(CLI_COMMAND_HINT_CI);
    let is_install_command = command_hint.starts_with(CLI_COMMAND_HINT_INSTALL);

    is_ci_command || is_install_command
}

fn collect_script_tip_lines(command_hint: &str) -> Vec<String> {
    if should_add_setup_script_tip(command_hint) {
        vec![SETUP_SCRIPT_BLOCKED_TIP.to_string()]
    } else {
        Vec::new()
    }
}

fn build_resolve_error_message(params: BuildResolveErrorMessageParams<'_>) -> String {
    let BuildResolveErrorMessageParams {
        project_dir,
        command_hint,
    } = params;
    let lockfiles = detected_lockfiles(project_dir);
    let suggestions = suggested_package_managers(project_dir);
    let lockfile_context = build_lockfile_context(&lockfiles);

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

    lines.extend(collect_lockfile_tip_lines(&lockfiles));
    lines.extend(collect_script_tip_lines(command_hint));

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
