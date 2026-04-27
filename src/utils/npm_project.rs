use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output};

use crate::constants::{
    NPM_ARG_ADD, NPM_ARG_CACHE_FOLDER, NPM_ARG_IGNORE_SCRIPTS, NPM_ARG_INSTALL,
    NPM_ARG_NO_LOCKFILE, NPM_ARG_NO_PACKAGE_LOCK, NPM_ARG_NO_SAVE, NPM_ARG_PREFER_OFFLINE,
    NPM_ARG_SILENT, NPM_ARG_STORE, NPM_HINT_COMMAND_NOT_FOUND, NPM_HINT_CONFLICT_DETAIL_TEMPLATE,
    NPM_HINT_ERESOLVE, NPM_HINT_GENERATE_LOCKFILE_MANUALLY_TEMPLATE, NPM_HINT_NETWORK_ERROR,
    NPM_LOCKFILE_FLAG, PNPM_HINT_ERESOLVE, PNPM_LOCKFILE_FLAG, STDERR_PATTERN_CONFLICT,
    STDERR_PATTERN_ECONNREFUSED, STDERR_PATTERN_ENOENT, STDERR_PATTERN_ENOTFOUND,
    STDERR_PATTERN_ERESOLVE, STDERR_PATTERN_ETIMEDOUT, STDERR_PATTERN_FETCH_FAILED,
    STDERR_PATTERN_NOT_FOUND, STDERR_PATTERN_PEER, STDERR_PATTERN_UNABLE_RESOLVE,
    YARN_HINT_ERESOLVE, YARN_LOCKFILE_FLAG, render_template,
};
use crate::ecosystem::{CommandPlan, InstallExecutor, PackageManager, detect_package_manager};
use crate::types::PackageManagerExecutor;
use crate::types::{
    DiagnoseLockfileFailureParams, InstallPackageParams, InstallPackageSourceParams,
    LockfileFailureKind, LockfileGenerationResult, ResolvePackageIntoLockfileParams,
    RunCleanInstallParams, RunPackageSourceInstallParams, RunPackageSourcePreloadParams,
};
use crate::verifier::artifact_cleanup::{cleanup_artifact, register_artifact, unregister_artifact};

const YARN_PRELOAD_TEMP_PREFIX: &str = "sentinel-yarn-preload";
const PRELOAD_PROJECT_DIR_NAME: &str = "project";
const PRELOAD_CACHE_DIR_NAME: &str = "cache";
const TEMP_ROOT_RETRY_ATTEMPTS: usize = 8;
const PRELOAD_PACKAGE_JSON_FILE: &str = "package.json";
const PRELOAD_PACKAGE_JSON_CONTENT: &str =
    "{\"name\":\"sentinel-preload\",\"version\":\"1.0.0\"}\n";

fn detected_manager(current_working_directory: &Path) -> PackageManager {
    detect_package_manager(current_working_directory).unwrap_or(PackageManager::Npm)
}

fn run_output_plan(current_working_directory: &Path, plan: CommandPlan) -> std::io::Result<Output> {
    let mut command = Command::new(plan.program);

    command.current_dir(current_working_directory);
    command.args(plan.args);
    command.output()
}

fn run_status_plan(
    current_working_directory: &Path,
    plan: CommandPlan,
) -> std::io::Result<ExitStatus> {
    let mut command = Command::new(plan.program);

    command.current_dir(current_working_directory);
    command.args(plan.args);
    command.status()
}

pub fn generate_lockfile_with_manager(
    current_working_directory: &Path,
    manager: PackageManager,
) -> std::io::Result<LockfileGenerationResult> {
    let executor = PackageManagerExecutor::new(manager);
    let plan = executor.generate_lockfile_plan();

    run_output_plan(current_working_directory, plan)
        .map(|output| LockfileGenerationResult { output, manager })
}

pub fn generate_lockfile(
    current_working_directory: &Path,
) -> std::io::Result<LockfileGenerationResult> {
    let manager = detected_manager(current_working_directory);

    generate_lockfile_with_manager(current_working_directory, manager)
}

fn classify_lockfile_failure(stderr: &str) -> LockfileFailureKind {
    let stderr_lower = stderr.to_lowercase();

    let has_eresolve = stderr_lower.contains(STDERR_PATTERN_ERESOLVE)
        || stderr_lower.contains(STDERR_PATTERN_UNABLE_RESOLVE);

    let has_peer_conflict = stderr_lower.contains(STDERR_PATTERN_PEER)
        && stderr_lower.contains(STDERR_PATTERN_CONFLICT);

    let is_dependency_conflict = has_eresolve || has_peer_conflict;

    let is_command_not_found = stderr_lower.contains(STDERR_PATTERN_NOT_FOUND)
        || stderr_lower.contains(STDERR_PATTERN_ENOENT);

    let is_network_error = stderr_lower.contains(STDERR_PATTERN_ENOTFOUND)
        || stderr_lower.contains(STDERR_PATTERN_ECONNREFUSED)
        || stderr_lower.contains(STDERR_PATTERN_ETIMEDOUT)
        || stderr_lower.contains(STDERR_PATTERN_FETCH_FAILED);

    match () {
        () if is_dependency_conflict => LockfileFailureKind::DependencyConflict,
        () if is_command_not_found => LockfileFailureKind::CommandNotFound,
        () if is_network_error => LockfileFailureKind::NetworkError,
        () => LockfileFailureKind::Unknown,
    }
}

fn trim_unmatched_closing_parentheses(value: &mut String) {
    let has_trailing_parenthesis = value.ends_with(')');
    let has_no_open_parenthesis = !value.contains('(');
    let should_trim = has_trailing_parenthesis && has_no_open_parenthesis;

    if !should_trim {
        return;
    }

    loop {
        value.pop();

        let has_trailing_parenthesis = value.ends_with(')');
        let has_no_open_parenthesis = !value.contains('(');
        let should_continue = has_trailing_parenthesis && has_no_open_parenthesis;

        if !should_continue {
            break;
        }
    }
}

fn clean_conflict_value(value: &str) -> Option<String> {
    let cleaned = value.trim().trim_matches('"').trim_matches('`');
    let cleaned_replaced = cleaned.replace('"', "");
    let cleaned = cleaned_replaced
        .trim()
        .trim_end_matches('.')
        .trim_end_matches(',')
        .trim_end_matches(')')
        .trim();

    let mut cleaned_value = cleaned.to_string();

    trim_unmatched_closing_parentheses(&mut cleaned_value);

    let cleaned = cleaned_value.trim();

    if cleaned.is_empty() {
        return None;
    }

    Some(cleaned.to_string())
}

const PEER_FROM_SEPARATOR: &str = " from ";
const EXACT_CONFLICT_PATTERNS: [&str; 2] = [
    "Could not resolve dependency:",
    "Conflicting peer dependency:",
];
const PEER_PREFIX_PATTERN: &str = "peer ";

fn try_extract_from_exact_pattern(line: &str) -> Option<String> {
    for pattern in EXACT_CONFLICT_PATTERNS {
        let (_, value) = line.split_once(pattern)?;

        if let Some(requirement) = clean_conflict_value(value) {
            return Some(requirement);
        }
    }
    None
}

fn try_extract_from_peer_pattern(line: &str) -> Option<String> {
    let (_, after_peer) = line.split_once(PEER_PREFIX_PATTERN)?;
    let requirement_segment = after_peer
        .split_once(PEER_FROM_SEPARATOR)
        .map_or(after_peer, |(value, _)| value);

    clean_conflict_value(requirement_segment)
}

fn extract_conflicting_requirement(stderr: &str) -> Option<String> {
    stderr.lines().find_map(|line| {
        let trimmed = line.trim();
        try_extract_from_exact_pattern(trimmed).or_else(|| try_extract_from_peer_pattern(trimmed))
    })
}

pub fn diagnose_lockfile_failure(params: DiagnoseLockfileFailureParams<'_>) -> String {
    let DiagnoseLockfileFailureParams { stderr, manager } = params;

    match classify_lockfile_failure(stderr) {
        LockfileFailureKind::DependencyConflict => build_dependency_conflict_hint(stderr, manager),
        LockfileFailureKind::CommandNotFound => build_command_not_found_hint(manager),
        LockfileFailureKind::NetworkError => NPM_HINT_NETWORK_ERROR.to_string(),
        LockfileFailureKind::Unknown => build_unknown_lockfile_hint(manager),
    }
}

pub fn resolve_package_into_lockfile(
    params: ResolvePackageIntoLockfileParams<'_>,
) -> std::io::Result<Output> {
    let ResolvePackageIntoLockfileParams {
        current_working_directory,
        package_reference,
    } = params;

    let manager = detected_manager(current_working_directory);
    let executor = PackageManagerExecutor::new(manager);
    let plan = executor.resolve_package_lockfile_plan(&package_reference.to_string());

    run_output_plan(current_working_directory, plan)
}

pub fn install_package(params: InstallPackageParams<'_>) -> std::io::Result<ExitStatus> {
    let InstallPackageParams {
        current_working_directory,
        package_reference,
        ignore_scripts,
    } = params;

    let manager = detected_manager(current_working_directory);
    let executor = PackageManagerExecutor::new(manager);
    let plan = executor.install_package_plan(&package_reference.to_string(), ignore_scripts);

    run_status_plan(current_working_directory, plan)
}

pub fn install_package_source(
    params: InstallPackageSourceParams<'_>,
) -> std::io::Result<ExitStatus> {
    let manager = detected_manager(params.current_working_directory);

    match manager {
        PackageManager::Npm => install_package_source_npm(params),
        PackageManager::Yarn => install_package_source_yarn(params),
        PackageManager::Pnpm => install_package_source_pnpm(params),
    }
}

fn install_package_source_npm(
    params: InstallPackageSourceParams<'_>,
) -> std::io::Result<ExitStatus> {
    let InstallPackageSourceParams {
        current_working_directory,
        package_source,
        ignore_scripts,
        ..
    } = params;

    let mut command = Command::new(PackageManager::Npm.command());

    command.current_dir(current_working_directory);
    command.arg(NPM_ARG_INSTALL);
    command.arg(package_source);
    command.arg(NPM_ARG_NO_SAVE);
    command.arg(NPM_ARG_NO_PACKAGE_LOCK);

    if ignore_scripts {
        command.arg(NPM_ARG_IGNORE_SCRIPTS);
    }

    command.status()
}

fn install_package_source_yarn(
    params: InstallPackageSourceParams<'_>,
) -> std::io::Result<ExitStatus> {
    let InstallPackageSourceParams {
        current_working_directory,
        package_reference,
        package_source,
        ignore_scripts,
    } = params;
    let temp_root = create_private_temp_root(YARN_PRELOAD_TEMP_PREFIX)?;
    let preload_project = temp_root.join(PRELOAD_PROJECT_DIR_NAME);
    let cache_dir = temp_root.join(PRELOAD_CACHE_DIR_NAME);

    let result = (|| -> std::io::Result<ExitStatus> {
        std::fs::create_dir_all(&preload_project)?;
        std::fs::create_dir_all(&cache_dir)?;
        write_preload_package_json(&preload_project)?;

        let preload_params = RunPackageSourcePreloadParams {
            work_dir: &preload_project,
            package_source,
            cache_or_store_dir: &cache_dir,
        };
        let preload_status = run_package_source_preload(PackageManager::Yarn, preload_params)?;

        if !preload_status.success() {
            return Ok(preload_status);
        }

        let install_params = RunPackageSourceInstallParams {
            work_dir: current_working_directory,
            package_reference,
            cache_or_store_dir: &cache_dir,
            ignore_scripts,
        };

        run_package_source_install(PackageManager::Yarn, &install_params)
    })();

    cleanup_temp_root(&temp_root);
    result
}

fn install_package_source_pnpm(
    params: InstallPackageSourceParams<'_>,
) -> std::io::Result<ExitStatus> {
    let InstallPackageSourceParams {
        current_working_directory,
        package_reference,
        package_source,
        ignore_scripts,
    } = params;

    let preload_params = RunPackageSourcePreloadParams {
        work_dir: current_working_directory,
        package_source,
        cache_or_store_dir: current_working_directory,
    };
    let preload_status = run_package_source_preload(PackageManager::Pnpm, preload_params)?;

    if !preload_status.success() {
        return Ok(preload_status);
    }

    let install_params = RunPackageSourceInstallParams {
        work_dir: current_working_directory,
        package_reference,
        cache_or_store_dir: current_working_directory,
        ignore_scripts,
    };

    run_package_source_install(PackageManager::Pnpm, &install_params)
}

fn create_private_temp_root(prefix: &str) -> std::io::Result<PathBuf> {
    let process_id = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    for attempt in 0..TEMP_ROOT_RETRY_ATTEMPTS {
        let path = std::env::temp_dir().join(format!("{prefix}-{process_id}-{nanos}-{attempt}"));

        match std::fs::create_dir(&path) {
            Ok(()) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;

                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
                }

                register_artifact(path.clone());

                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "failed to create isolated temp directory for source install",
    ))
}

fn cleanup_temp_root(path: &Path) {
    let _ = cleanup_artifact(path);

    unregister_artifact(path);
}

fn write_preload_package_json(project_dir: &Path) -> std::io::Result<()> {
    std::fs::write(
        project_dir.join(PRELOAD_PACKAGE_JSON_FILE),
        PRELOAD_PACKAGE_JSON_CONTENT,
    )
}

pub fn run_clean_install(params: RunCleanInstallParams<'_>) -> std::io::Result<ExitStatus> {
    let RunCleanInstallParams {
        current_working_directory,
        ignore_scripts,
        omit_dev,
        omit_optional,
        silent_output,
    } = params;

    let manager = detected_manager(current_working_directory);
    let executor = PackageManagerExecutor::new(manager);
    let plan = executor.clean_install_plan(crate::types::CleanInstallPlanParams {
        ignore_scripts,
        omit_dev,
        omit_optional,
        silent_output,
    });

    run_status_plan(current_working_directory, plan)
}

fn build_dependency_conflict_hint(stderr: &str, manager: PackageManager) -> String {
    let base_hint = resolve_eresolve_hint(manager);

    let Some(requirement) = extract_conflicting_requirement(stderr) else {
        return base_hint;
    };

    let cleaned_requirement = requirement.trim_end_matches(')');
    let conflict_detail_template_args = vec![cleaned_requirement.to_string()];
    let conflict_detail = render_template(
        NPM_HINT_CONFLICT_DETAIL_TEMPLATE,
        &conflict_detail_template_args,
    );
    let conflict_hint = format!("{base_hint}\n{conflict_detail}");

    conflict_hint
}

fn resolve_eresolve_hint(manager: PackageManager) -> String {
    match manager {
        PackageManager::Npm => NPM_HINT_ERESOLVE.to_string(),
        PackageManager::Yarn => YARN_HINT_ERESOLVE.to_string(),
        PackageManager::Pnpm => PNPM_HINT_ERESOLVE.to_string(),
    }
}

fn build_command_not_found_hint(manager: PackageManager) -> String {
    let command_not_found_template_args = vec![manager.command().to_string()];

    render_template(NPM_HINT_COMMAND_NOT_FOUND, &command_not_found_template_args)
}

fn build_unknown_lockfile_hint(manager: PackageManager) -> String {
    let lockfile_flag = match manager {
        PackageManager::Npm => NPM_LOCKFILE_FLAG,
        PackageManager::Yarn => YARN_LOCKFILE_FLAG,
        PackageManager::Pnpm => PNPM_LOCKFILE_FLAG,
    };
    let generate_lockfile_template_args =
        vec![manager.command().to_string(), lockfile_flag.to_string()];

    render_template(
        NPM_HINT_GENERATE_LOCKFILE_MANUALLY_TEMPLATE,
        &generate_lockfile_template_args,
    )
}

fn run_package_source_preload(
    manager: PackageManager,
    params: RunPackageSourcePreloadParams<'_>,
) -> std::io::Result<ExitStatus> {
    let RunPackageSourcePreloadParams {
        work_dir,
        package_source,
        cache_or_store_dir,
    } = params;

    let mut preload = Command::new(manager.command());

    preload.current_dir(work_dir);

    match manager {
        PackageManager::Pnpm => {
            preload.arg(NPM_ARG_STORE);
            preload.arg(NPM_ARG_ADD);
            preload.arg(package_source);
        }
        PackageManager::Yarn => {
            preload.arg(NPM_ARG_SILENT);
            preload.arg(NPM_ARG_IGNORE_SCRIPTS);
            preload.arg(NPM_ARG_ADD);
            preload.arg(package_source);
            preload.arg(NPM_ARG_NO_LOCKFILE);
            preload.arg(NPM_ARG_CACHE_FOLDER);
            preload.arg(cache_or_store_dir);
        }
        PackageManager::Npm => {
            preload.arg(NPM_ARG_ADD);
            preload.arg(package_source);
            preload.arg(NPM_ARG_NO_LOCKFILE);
            preload.arg(NPM_ARG_CACHE_FOLDER);
            preload.arg(cache_or_store_dir);
        }
    }

    preload.status()
}

fn run_package_source_install(
    manager: PackageManager,
    params: &RunPackageSourceInstallParams<'_>,
) -> std::io::Result<ExitStatus> {
    let RunPackageSourceInstallParams {
        work_dir,
        package_reference,
        cache_or_store_dir,
        ignore_scripts,
    } = *params;

    let exact_arg = match manager {
        PackageManager::Npm | PackageManager::Yarn => crate::constants::NPM_ARG_EXACT,
        PackageManager::Pnpm => crate::constants::NPM_ARG_SAVE_EXACT,
    };
    let mut install = Command::new(manager.command());

    install.current_dir(work_dir);
    install.arg(NPM_ARG_ADD);
    install.arg(package_reference.to_string());
    install.arg(exact_arg);
    install.arg(NPM_ARG_PREFER_OFFLINE);

    if matches!(manager, PackageManager::Npm | PackageManager::Yarn) {
        install.arg(NPM_ARG_CACHE_FOLDER);
        install.arg(cache_or_store_dir);
    }

    if ignore_scripts {
        install.arg(NPM_ARG_IGNORE_SCRIPTS);
    }

    install.status()
}
