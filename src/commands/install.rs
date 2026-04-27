use futures_util::StreamExt;
use semver::{Version, VersionReq};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use super::shared::{
    build_report, load_command_state, validate_package_json_dependencies, verify_packages,
};

use crate::constants::cli::{CLI_COMMAND_HINT_CI, CLI_COMMAND_HINT_INSTALL};
use crate::constants::paths::{
    PACKAGE_JSON_FILE, PACKAGE_VERSION_LATEST, PACKAGE_VERSION_NEXT, PREVALIDATED_TARBALL_PREFIX,
};
use crate::constants::{
    FALLBACK_PROCESS_EXIT_CODE, HISTORY_COMMAND_CI, HISTORY_COMMAND_INSTALL,
    INSTALL_ERR_FAILED_BUILD_ASYNC_RUNTIME, INSTALL_ERR_FAILED_COMPUTE_RELATIVE_PATH,
    INSTALL_ERR_FAILED_DOWNLOAD_REGISTRY_TARBALL, INSTALL_ERR_FAILED_FETCH_REGISTRY_METADATA,
    INSTALL_ERR_FAILED_INIT_POST_VERIFY_VERIFIER, INSTALL_ERR_FAILED_PARSE_PACKAGE_MANIFEST,
    INSTALL_ERR_FAILED_READ_DIRECTORY, INSTALL_ERR_FAILED_READ_DIRECTORY_ENTRY,
    INSTALL_ERR_FAILED_READ_FILE, INSTALL_ERR_FAILED_READ_FILE_TYPE,
    INSTALL_ERR_FAILED_READ_PACKAGE_MANIFEST, INSTALL_ERR_FAILED_READ_TARBALL_RESPONSE,
    INSTALL_ERR_LOCKFILE_INIT_MISSING_AFTER_SUCCESS, INSTALL_ERR_NO_LOCKFILE_FOR_CI,
    INSTALL_ERR_NO_LOCKFILE_FOR_INSTALL, INSTALL_ERR_PACKAGE_NOT_FOUND_NODE_MODULES,
    INSTALL_MAX_CONCURRENCY, INSTALL_MSG_NOTHING_TO_INSTALL, INSTALL_PROGRESS_TEMPLATE,
    INSTALL_PROGRESS_VERIFY_MSG, NODE_MODULES_DIR, NPM_ERR_EXEC_FAILED_TEMPLATE,
    NPM_ERR_LOCKFILE_ONLY_FAILED_TEMPLATE, POST_VERIFY_CONTENT_MISMATCH_ERR_TEMPLATE,
    POST_VERIFY_GOOD_TERM_SECS, POST_VERIFY_LARGE_PACKAGE_WARN_THRESHOLD,
    POST_VERIFY_MAX_CONCURRENCY, POST_VERIFY_MISSING_PACKAGES_ERR_TEMPLATE,
    POST_VERIFY_PACKAGE_PREFIX, SEMVER_PINNED_EXTRA_CHARS, SEMVER_RANGE_CHARS, render_template,
};
use crate::ecosystem::{
    PackageManager, active_lockfile_path, detect_package_manager, resolve_package_manager,
};
use crate::history::ledger::{AppendHistoryEventsParams, append_history_events};
use crate::history::types::HistoryPackageMetadata;
use crate::output::{print_install_blocked, print_install_blocked_unverifiable, print_report};
use crate::policy::{DefaultSecurityPolicy, InstallPolicyDecision, SecurityPolicy};
use crate::types::{
    AnalyzeDependencyCyclesParams, AppendCiHistoryParams, AppendInstallHistoryParams,
    BlockedVerifyResults, CheckContentFingerprintMismatchesParams, CiArgs,
    CollectInstallPackagesParams, CompleteSuccessfulCiRunParams, CompleteSuccessfulInstallParams,
    DependencyNode, DependencyTree, DiagnoseLockfileFailureParams,
    EnsureLockfileExistsForInstallParams, ExecuteVerificationRunParams, FinalizeCiDryRunParams,
    FinalizeCiRunParams, FinalizeInstallDryRunParams, FinalizeInstallRunParams,
    FindContentMismatchPostVerifyPackagesParams, InstallArgs, InstallBlockReason,
    InstallExecutionOutcome, InstallFingerprintResult, InstallFromVerifiedSourceOrFailureParams,
    InstallFromVerifiedSourceParams, InstallPackageParams, InstallPackageRequest,
    InstallPackageSourceParams, InstallPolicyInput, OutputFormat, PackageRef, PrepareCiStateParams,
    PrepareInstallStateParams, PrepareLockfileForInstallParams, PreparedCiState,
    PreparedInstallState, PrintAndSaveCiReportParams, PrintBlockReasonResultsParams,
    PrintCiBlockingResultsParams, PrintInstallCandidateResolvedParams, PrintInstallReportParams,
    PrintPostVerifyElapsedWarningParams, PrintReportParams, ProcessDirectoryEntryParams,
    ProcessInstalledPackageIndexEntryParams, ProgressBarConfig, ResolveInstallPolicyParams,
    ResolveInstallTargetsParams, ResolvePackageIntoLockfileParams, ResolvePackageManagerParams,
    RestoreProjectFilesSnapshotParams, RunCiPostVerifyParams, RunCleanInstallOrFailureParams,
    RunCleanInstallParams, RunPostVerifyForPackagesParams, SaveCiReportParams, SharedCommandState,
    SharedCommandStateError, ShouldPrintReportParams, ShouldRenderProgressBarParams,
    SyncLockfileWithPackageJsonParams, VerifiedTarball, VerifierNewParams,
    VerifyPackagesExecutionParams, VerifyPackagesParams, VerifyResult, VersionSpecKind,
    WarnPostVerifyElapsedParams, WarnPostVerifyLargeScopeParams,
};
use crate::ui::command_feedback as ui;
use crate::utils::{
    build_install_command_hint, build_prevalidated_tarball_file_name,
    capture_project_files_snapshot, create_progress_bar, diagnose_lockfile_failure,
    format_err_for_package, format_err_for_path, format_err_in_path, format_err_with_path,
    format_err_with_reason, format_err_with_subject, format_prefixed_package_message,
    generate_lockfile_with_manager, install_package, install_package_source, lockfile_sha256,
    resolve_package_into_lockfile, restore_project_files_snapshot, run_clean_install,
    should_render_progress_bar,
};
use crate::verifier::compute_tarball_fingerprint_bytes;

fn materialize_verified_tarball(
    package_ref: &PackageRef,
    tarball: VerifiedTarball,
) -> Result<PathBuf, std::io::Error> {
    match tarball {
        VerifiedTarball::Spool(path) => Ok(path),
        VerifiedTarball::Memory(bytes) => {
            let process_id = std::process::id();
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default();

            let file_name = build_prevalidated_tarball_file_name(
                PREVALIDATED_TARBALL_PREFIX,
                process_id,
                nanos,
                &package_ref.name,
            );
            let path = std::env::temp_dir().join(file_name);

            std::fs::write(&path, bytes)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
            }

            crate::verifier::artifact_cleanup::register_artifact(path.clone());

            Ok(path)
        }
    }
}

fn cleanup_materialized_tarball(path: &Path) {
    let _ = crate::verifier::artifact_cleanup::cleanup_artifact(path);
    crate::verifier::artifact_cleanup::unregister_artifact(path);
}

#[cfg(test)]
fn parse_package_ref(spec: &str) -> Option<PackageRef> {
    let separator = spec.rfind('@')?;
    let package_name = &spec[..separator];
    let package_version = &spec[separator + 1..];

    let is_missing_package_name = package_name.is_empty();
    let is_missing_package_version = package_version.is_empty();
    let has_missing_package_parts = is_missing_package_name || is_missing_package_version;

    if has_missing_package_parts {
        return None;
    }

    Some(PackageRef::new(package_name, package_version))
}

fn parse_install_package_request(spec: &str) -> Option<InstallPackageRequest> {
    let trimmed = spec.trim();
    let is_empty_input = trimmed.is_empty();
    let has_whitespace = trimmed.chars().any(char::is_whitespace);
    let ends_with_separator = trimmed.ends_with('@');
    let has_invalid_input = is_empty_input || has_whitespace || ends_with_separator;

    if has_invalid_input {
        return None;
    }

    let separator = trimmed.rfind('@');
    let starts_scoped = trimmed.starts_with('@');

    let (package_name, version_spec) = match separator {
        Some(0) if starts_scoped => (trimmed.to_string(), None),
        Some(index) => {
            let package_name = &trimmed[..index];
            let package_version = &trimmed[index + 1..];

            let is_missing_package_name = package_name.is_empty();
            let is_missing_package_version = package_version.is_empty();
            let has_missing_package_parts = is_missing_package_name || is_missing_package_version;

            if has_missing_package_parts {
                return None;
            }

            (package_name.to_string(), Some(package_version.to_string()))
        }
        None => (trimmed.to_string(), None),
    };

    Some(InstallPackageRequest {
        package_name,
        version_spec,
    })
}

fn parse_requested_install(
    args: &InstallArgs,
) -> Result<(InstallPackageRequest, PackageRef), ExitCode> {
    let Some(install_request) = parse_install_package_request(&args.package) else {
        let package_name_hint = args
            .package
            .split('@')
            .find(|segment| !segment.is_empty())
            .unwrap_or("<package>");

        ui::print_invalid_install_package_input(&args.package, package_name_hint);

        return Err(ExitCode::FAILURE);
    };

    let candidate_spec = install_request
        .version_spec
        .clone()
        .unwrap_or_else(|| PACKAGE_VERSION_LATEST.to_string());
    let requested_package_ref = PackageRef::new(&install_request.package_name, &candidate_spec);

    Ok((install_request, requested_package_ref))
}

fn load_install_shared_state(
    current_working_directory: &Path,
    timeout: u64,
) -> Result<SharedCommandState, ExitCode> {
    match load_command_state(current_working_directory, timeout) {
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

fn resolve_install_targets(
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

fn print_and_save_ci_report(params: PrintAndSaveCiReportParams<'_>) {
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

#[allow(clippy::ref_option)]
fn append_ci_history(params: AppendCiHistoryParams<'_>) -> Result<(), String> {
    let AppendCiHistoryParams {
        args,
        report,
        lock_hash_before_verify,
    } = params;

    let ci_packages: Vec<HistoryPackageMetadata> = report
        .results
        .iter()
        .filter(|result| result.is_clean())
        .map(|result| HistoryPackageMetadata {
            name: result.package.name.clone(),
            version: result.package.version.clone(),
            direct: result.is_direct,
        })
        .collect();

    let lock_hash_after_install = lockfile_sha256(&args.cwd);
    let manager = detect_package_manager(&args.cwd).unwrap_or(PackageManager::Npm);

    let append_history_events_params = AppendHistoryEventsParams {
        current_working_directory: &args.cwd,
        package_manager: manager,
        command: HISTORY_COMMAND_CI,
        lockfile_path: manager.lockfile_name(),
        lock_hash_before: lock_hash_before_verify,
        lock_hash_after: &lock_hash_after_install,
        packages: &ci_packages,
    };

    append_history_events(append_history_events_params)
}

fn install_from_verified_source(
    params: InstallFromVerifiedSourceParams<'_>,
) -> std::io::Result<std::process::ExitStatus> {
    let InstallFromVerifiedSourceParams {
        args,
        package_ref,
        ignore_scripts,
        prevalidated_tarball,
    } = params;

    if let Some(tarball) = prevalidated_tarball
        && let Ok(tarball_path) = materialize_verified_tarball(package_ref, tarball)
    {
        let source = tarball_path.to_string_lossy().to_string();
        let install_package_source_params = InstallPackageSourceParams {
            current_working_directory: &args.cwd,
            package_reference: package_ref,
            package_source: &source,
            ignore_scripts,
        };
        let status = install_package_source(install_package_source_params);

        cleanup_materialized_tarball(&tarball_path);

        return status;
    }

    let install_package_params = InstallPackageParams {
        current_working_directory: &args.cwd,
        package_reference: package_ref,
        ignore_scripts,
    };

    install_package(install_package_params)
}

fn normalize_relative_path(path: &Path) -> Option<String> {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let invalid_path = normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.split('/').any(|segment| segment == "..");

    if invalid_path {
        return None;
    }

    Some(normalized)
}

fn package_key(package_ref: &PackageRef) -> String {
    package_ref.to_string()
}

fn read_package_ref_from_manifest(
    package_manifest: &Path,
) -> InstallFingerprintResult<Option<PackageRef>> {
    if !package_manifest.exists() {
        return Ok(None);
    }

    let package_manifest_contents = std::fs::read_to_string(package_manifest).map_err(|error| {
        format_err_with_subject(
            INSTALL_ERR_FAILED_READ_PACKAGE_MANIFEST,
            package_manifest,
            &error,
        )
    })?;

    let package_manifest_json =
        serde_json::from_str::<serde_json::Value>(&package_manifest_contents).map_err(|error| {
            format_err_with_subject(
                INSTALL_ERR_FAILED_PARSE_PACKAGE_MANIFEST,
                package_manifest,
                &error,
            )
        })?;

    let Some(package_name) = package_manifest_json
        .get("name")
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(None);
    };

    let Some(package_version) = package_manifest_json
        .get("version")
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(None);
    };

    Ok(Some(PackageRef::new(package_name, package_version)))
}

fn process_installed_package_index_entry(
    params: ProcessInstalledPackageIndexEntryParams<'_>,
) -> InstallFingerprintResult<()> {
    let ProcessInstalledPackageIndexEntryParams {
        path,
        entry_result,
        index,
        pending_paths,
    } = params;

    let entry = entry_result.map_err(|error| {
        format_err_in_path(INSTALL_ERR_FAILED_READ_DIRECTORY_ENTRY, path, &error)
    })?;

    let file_type = entry.file_type().map_err(|error| {
        let entry_path = entry.path();
        format_err_for_path(INSTALL_ERR_FAILED_READ_FILE_TYPE, &entry_path, &error)
    })?;

    let is_regular_dir = file_type.is_dir() && !file_type.is_symlink();

    if !is_regular_dir {
        return Ok(());
    }

    let entry_path = entry.path();

    pending_paths.push(entry_path.clone());

    let package_manifest = entry_path.join(PACKAGE_JSON_FILE);
    let package_ref = read_package_ref_from_manifest(&package_manifest)?;

    let Some(package_ref) = package_ref else {
        return Ok(());
    };

    index.insert(package_ref.to_string(), entry_path);

    Ok(())
}

fn build_installed_package_index(
    current_working_directory: &Path,
) -> InstallFingerprintResult<HashMap<String, PathBuf>> {
    let node_modules_root = current_working_directory.join(NODE_MODULES_DIR);

    if !node_modules_root.exists() {
        return Ok(HashMap::new());
    }

    let mut index = HashMap::new();
    let mut pending_paths = vec![node_modules_root];

    while let Some(path) = pending_paths.pop() {
        let entries = std::fs::read_dir(&path).map_err(|error| {
            format_err_with_path(INSTALL_ERR_FAILED_READ_DIRECTORY, &path, &error)
        })?;

        for entry_result in entries {
            let process_entry_params = ProcessInstalledPackageIndexEntryParams {
                path: &path,
                entry_result,
                index: &mut index,
                pending_paths: &mut pending_paths,
            };

            process_installed_package_index_entry(process_entry_params)?;
        }
    }

    Ok(index)
}

fn warn_post_verify_large_scope(params: WarnPostVerifyLargeScopeParams<'_>) {
    let WarnPostVerifyLargeScopeParams {
        command_name,
        package_count,
        should_warn,
    } = params;

    let is_large_scope = package_count >= POST_VERIFY_LARGE_PACKAGE_WARN_THRESHOLD;
    let should_emit_warning = should_warn && is_large_scope;

    if !should_emit_warning {
        return;
    }

    ui::print_warn_post_verify_large_scope(command_name, package_count);
}

fn warn_post_verify_elapsed(params: WarnPostVerifyElapsedParams<'_>) {
    let WarnPostVerifyElapsedParams {
        command_name,
        package_count,
        elapsed,
        should_warn,
    } = params;

    let is_elapsed_too_long = elapsed.as_secs() > POST_VERIFY_GOOD_TERM_SECS;

    if !should_warn || !is_elapsed_too_long {
        return;
    }

    let print_warn_post_verify_elapsed_params = PrintPostVerifyElapsedWarningParams {
        command_name,
        package_count,
        elapsed_secs: elapsed.as_secs(),
        good_term_secs: POST_VERIFY_GOOD_TERM_SECS,
    };

    ui::print_warn_post_verify_elapsed(print_warn_post_verify_elapsed_params);
}

fn process_directory_entry(params: ProcessDirectoryEntryParams<'_>) {
    let ProcessDirectoryEntryParams {
        entry_path,
        file_type,
        entries,
        pending_paths,
    } = params;

    let is_symlink = file_type.is_symlink();

    if is_symlink {
        return;
    }

    let is_directory = file_type.is_dir();

    if is_directory {
        pending_paths.push(entry_path);

        return;
    }

    let is_file = file_type.is_file();

    if is_file {
        entries.push(entry_path);
    }
}

fn collect_directory_entries(root: &Path) -> InstallFingerprintResult<Vec<PathBuf>> {
    let mut entries = Vec::new();
    let mut pending_paths = vec![root.to_path_buf()];

    while let Some(path) = pending_paths.pop() {
        let read_dir = std::fs::read_dir(&path).map_err(|error| {
            format_err_with_path(INSTALL_ERR_FAILED_READ_DIRECTORY, &path, &error)
        })?;

        for entry_result in read_dir {
            let entry = entry_result.map_err(|error| {
                format_err_in_path(INSTALL_ERR_FAILED_READ_DIRECTORY_ENTRY, &path, &error)
            })?;

            let file_type = entry.file_type().map_err(|error| {
                let entry_path = entry.path();

                format_err_for_path(INSTALL_ERR_FAILED_READ_FILE_TYPE, &entry_path, &error)
            })?;

            let entry_path = entry.path();

            let process_directory_entry_params = ProcessDirectoryEntryParams {
                entry_path,
                file_type: &file_type,
                entries: &mut entries,
                pending_paths: &mut pending_paths,
            };

            process_directory_entry(process_directory_entry_params);
        }
    }

    entries.sort();
    Ok(entries)
}

fn compute_directory_fingerprint(root: &Path) -> InstallFingerprintResult<String> {
    let entries = collect_directory_entries(root)?;
    let mut hasher = Sha256::new();

    for entry_path in entries {
        let relative_path = entry_path.strip_prefix(root).map_err(|error| {
            format_err_for_path(
                INSTALL_ERR_FAILED_COMPUTE_RELATIVE_PATH,
                &entry_path,
                &error,
            )
        })?;

        let Some(relative_path_text) = normalize_relative_path(relative_path) else {
            continue;
        };

        let file_bytes = std::fs::read(&entry_path).map_err(|error| {
            format_err_with_path(INSTALL_ERR_FAILED_READ_FILE, &entry_path, &error)
        })?;

        let file_digest = Sha256::digest(&file_bytes);

        hasher.update(relative_path_text.as_bytes());
        hasher.update([0]);
        hasher.update(file_digest);
        hasher.update([0]);
    }

    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(digest.len() * 2);

    for byte in digest {
        use std::fmt::Write as _;

        let _ = write!(&mut encoded, "{byte:02x}");
    }

    Ok(encoded)
}

fn compute_installed_package_fingerprint(
    installed_package_index: &HashMap<String, PathBuf>,
    package_ref: &PackageRef,
) -> InstallFingerprintResult<String> {
    let installed_root = installed_package_index
        .get(&package_key(package_ref))
        .cloned()
        .ok_or_else(|| {
            format_prefixed_package_message(
                POST_VERIFY_PACKAGE_PREFIX,
                package_ref,
                INSTALL_ERR_PACKAGE_NOT_FOUND_NODE_MODULES,
            )
        })?;

    compute_directory_fingerprint(&installed_root)
}

async fn compute_registry_package_fingerprint(
    verifier: &crate::verifier::Verifier,
    package_ref: &PackageRef,
) -> InstallFingerprintResult<String> {
    let metadata = verifier
        .registry
        .fetch_version(package_ref)
        .await
        .map_err(|error| {
            format_err_for_package(
                INSTALL_ERR_FAILED_FETCH_REGISTRY_METADATA,
                package_ref,
                &error,
            )
        })?;

    let response = verifier
        .registry
        .download_tarball(&metadata.dist.tarball)
        .await
        .map_err(|error| {
            format_err_for_package(
                INSTALL_ERR_FAILED_DOWNLOAD_REGISTRY_TARBALL,
                package_ref,
                &error,
            )
        })?;

    let tarball_bytes = response.bytes().await.map_err(|error| {
        format_err_for_package(
            INSTALL_ERR_FAILED_READ_TARBALL_RESPONSE,
            package_ref,
            &error,
        )
    })?;

    compute_tarball_fingerprint_bytes(&tarball_bytes, package_ref)
}

async fn find_content_mismatch_post_verify_packages(
    params: FindContentMismatchPostVerifyPackagesParams<'_>,
) -> InstallFingerprintResult<Vec<PackageRef>> {
    let FindContentMismatchPostVerifyPackagesParams {
        installed_package_index,
        packages,
        verifier,
        cached_fingerprints,
    } = params;

    let mismatch_results = futures_util::stream::iter(packages.iter().cloned())
        .map(|package_ref| async move {
            let installed_fingerprint =
                compute_installed_package_fingerprint(installed_package_index, &package_ref)?;
            let package_identifier = package_key(&package_ref);
            let registry_fingerprint = match cached_fingerprints.get(&package_identifier) {
                Some(cached) => cached.clone(),
                None => compute_registry_package_fingerprint(verifier, &package_ref).await?,
            };

            let is_mismatch = installed_fingerprint != registry_fingerprint;

            Ok::<Option<PackageRef>, String>(is_mismatch.then_some(package_ref))
        })
        .buffer_unordered(POST_VERIFY_MAX_CONCURRENCY)
        .collect::<Vec<_>>()
        .await;

    let mut mismatches = Vec::new();

    for mismatch_result in mismatch_results {
        let mismatch = mismatch_result?;

        if let Some(package_ref) = mismatch {
            mismatches.push(package_ref);
        }
    }

    mismatches.sort_by_key(package_key);

    Ok(mismatches)
}

fn run_future_in_context<T>(future: impl Future<Output = T>) -> InstallFingerprintResult<T> {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        return Ok(handle.block_on(future));
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format_err_with_reason(INSTALL_ERR_FAILED_BUILD_ASYNC_RUNTIME, &error))?;

    Ok(runtime.block_on(future))
}

fn collect_cached_tarball_fingerprints(results: &[VerifyResult]) -> HashMap<String, String> {
    results
        .iter()
        .filter_map(|result| {
            let fingerprint = result.tarball_fingerprint.as_ref()?;
            Some((package_key(&result.package), fingerprint.clone()))
        })
        .collect()
}

fn check_content_fingerprint_mismatches(
    params: CheckContentFingerprintMismatchesParams<'_>,
) -> InstallFingerprintResult<()> {
    let CheckContentFingerprintMismatchesParams {
        current_working_directory,
        timeout_ms,
        installed_package_index,
        packages,
        verify_results,
        command_name,
    } = params;

    let verifier = create_post_verify_verifier(current_working_directory, timeout_ms)?;
    let cached_fingerprints = collect_cached_tarball_fingerprints(verify_results);

    let mismatches = run_future_in_context(find_content_mismatch_post_verify_packages(
        FindContentMismatchPostVerifyPackagesParams {
            installed_package_index,
            packages,
            verifier: &verifier,
            cached_fingerprints: &cached_fingerprints,
        },
    ))??;

    if !mismatches.is_empty() {
        return Err(format_post_verify_content_mismatch_message(
            command_name,
            &mismatches,
        ));
    }

    Ok(())
}

fn run_post_verify_for_packages(
    params: RunPostVerifyForPackagesParams<'_>,
) -> InstallFingerprintResult<()> {
    let RunPostVerifyForPackagesParams {
        current_working_directory,
        timeout_ms,
        quiet,
        is_text_output,
        command_name,
        packages,
        verify_results,
    } = params;

    let should_warn = !quiet && is_text_output;

    let warn_post_verify_large_scope_params = WarnPostVerifyLargeScopeParams {
        command_name,
        package_count: packages.len(),
        should_warn,
    };

    warn_post_verify_large_scope(warn_post_verify_large_scope_params);

    let post_verify_started_at = Instant::now();
    let installed_package_index = build_installed_package_index(current_working_directory)?;
    let missing_packages =
        find_missing_post_verify_packages_from_index(&installed_package_index, packages);

    if !missing_packages.is_empty() {
        return Err(format_post_verify_missing_packages_message(
            command_name,
            &missing_packages,
        ));
    }

    let check_content_fingerprint_mismatches_params = CheckContentFingerprintMismatchesParams {
        current_working_directory,
        timeout_ms,
        installed_package_index: &installed_package_index,
        packages,
        verify_results,
        command_name,
    };

    check_content_fingerprint_mismatches(check_content_fingerprint_mismatches_params)?;

    let warn_post_verify_elapsed_params = WarnPostVerifyElapsedParams {
        command_name,
        package_count: packages.len(),
        elapsed: post_verify_started_at.elapsed(),
        should_warn,
    };

    warn_post_verify_elapsed(warn_post_verify_elapsed_params);

    Ok(())
}

fn create_post_verify_verifier(
    current_working_directory: &Path,
    timeout_ms: u64,
) -> InstallFingerprintResult<crate::verifier::Verifier> {
    let verifier_new_params = VerifierNewParams {
        timeout_ms,
        current_working_directory,
        cache_dir: None,
        artifact_store: crate::artifact_store_config::get(),
        max_memory_bytes: crate::verifier::memory_budget::detect_memory_budget(),
    };

    crate::verifier::Verifier::new(verifier_new_params).map_err(|error| {
        format_err_with_reason(INSTALL_ERR_FAILED_INIT_POST_VERIFY_VERIFIER, &error)
    })
}

#[cfg(test)]
fn find_missing_post_verify_packages(
    current_working_directory: &Path,
    packages: &[PackageRef],
) -> Vec<PackageRef> {
    let Ok(installed_package_index) = build_installed_package_index(current_working_directory)
    else {
        return packages.to_vec();
    };

    find_missing_post_verify_packages_from_index(&installed_package_index, packages)
}

fn find_missing_post_verify_packages_from_index(
    installed_package_index: &HashMap<String, PathBuf>,
    packages: &[PackageRef],
) -> Vec<PackageRef> {
    packages
        .iter()
        .filter(|package_ref| !installed_package_index.contains_key(&package_key(package_ref)))
        .cloned()
        .collect()
}

fn format_post_verify_missing_packages_message(
    command_name: &str,
    missing: &[PackageRef],
) -> String {
    let missing_list = missing
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");

    POST_VERIFY_MISSING_PACKAGES_ERR_TEMPLATE
        .replace("{command_name}", command_name)
        .replace("{missing_list}", &missing_list)
}

fn format_post_verify_content_mismatch_message(
    command_name: &str,
    mismatches: &[PackageRef],
) -> String {
    let mismatch_list = mismatches
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");

    POST_VERIFY_CONTENT_MISMATCH_ERR_TEMPLATE
        .replace("{command_name}", command_name)
        .replace("{mismatch_list}", &mismatch_list)
}

#[allow(clippy::ref_option)]
fn append_install_history(params: AppendInstallHistoryParams<'_>) -> Result<(), String> {
    let AppendInstallHistoryParams {
        args,
        package_ref,
        lock_hash_before_verify,
    } = params;

    let lock_hash_after_install = lockfile_sha256(&args.cwd);
    let manager = detect_package_manager(&args.cwd).unwrap_or(PackageManager::Npm);
    let install_packages = vec![HistoryPackageMetadata {
        name: package_ref.name.clone(),
        version: package_ref.version.clone(),
        direct: true,
    }];

    let append_history_events_params = AppendHistoryEventsParams {
        current_working_directory: &args.cwd,
        package_manager: manager,
        command: HISTORY_COMMAND_INSTALL,
        lockfile_path: manager.lockfile_name(),
        lock_hash_before: lock_hash_before_verify,
        lock_hash_after: &lock_hash_after_install,
        packages: &install_packages,
    };

    append_history_events(append_history_events_params)
}

fn is_exact_version_spec(spec: &str) -> bool {
    let has_range_tokens = spec.chars().any(|c| SEMVER_RANGE_CHARS.contains(&c));
    let has_digits = spec.chars().any(|c| c.is_ascii_digit());
    let has_only_valid_chars = spec
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || SEMVER_PINNED_EXTRA_CHARS.contains(&c));

    !has_range_tokens && has_digits && has_only_valid_chars
}

fn normalize_semver_input(value: &str) -> &str {
    value.trim().trim_start_matches('v')
}

fn parse_semver_version(value: &str) -> Option<Version> {
    Version::parse(normalize_semver_input(value)).ok()
}

fn is_tag_spec(spec: &str) -> bool {
    let normalized = spec.to_ascii_lowercase();

    normalized == PACKAGE_VERSION_LATEST || normalized == PACKAGE_VERSION_NEXT
}

fn resolve_exact_spec(spec: &str, matches: &mut Vec<PackageRef>) -> Option<PackageRef> {
    let exact_match = matches.iter().find(|p| p.version == *spec).cloned();
    let normalized_match = parse_semver_version(spec).and_then(|requested| {
        matches
            .iter()
            .find(|p| parse_semver_version(&p.version) == Some(requested.clone()))
            .cloned()
    });

    exact_match.or(normalized_match).or_else(|| {
        matches.sort_by(|left, right| left.version.cmp(&right.version));
        matches.pop()
    })
}

fn resolve_range_spec(spec: &str, matches: &[PackageRef]) -> Option<PackageRef> {
    let version_req = VersionReq::parse(spec).ok()?;

    matches
        .iter()
        .filter_map(|package_ref| {
            parse_semver_version(&package_ref.version).map(|version| (version, package_ref.clone()))
        })
        .filter(|(version, _)| version_req.matches(version))
        .max_by(|left, right| left.0.cmp(&right.0))
        .map(|(_, package_ref)| package_ref)
}

fn select_highest_semver_candidate(matches: &[PackageRef]) -> Option<PackageRef> {
    matches
        .iter()
        .filter_map(|package_ref| {
            parse_semver_version(&package_ref.version).map(|version| (version, package_ref.clone()))
        })
        .max_by(|left, right| left.0.cmp(&right.0))
        .map(|(_, package_ref)| package_ref)
}

fn classify_version_spec(version_spec: Option<&String>) -> VersionSpecKind<'_> {
    let Some(spec) = version_spec else {
        return VersionSpecKind::Unspecified;
    };

    if is_tag_spec(spec) {
        return VersionSpecKind::Tag;
    }

    if is_exact_version_spec(spec) {
        return VersionSpecKind::Exact(spec);
    }

    VersionSpecKind::Range(spec)
}

fn resolve_install_candidate_package(
    dependency_tree: &DependencyTree,
    request: &InstallPackageRequest,
) -> Option<PackageRef> {
    let InstallPackageRequest {
        package_name,
        version_spec,
    } = request;

    let mut direct_matches: Vec<PackageRef> = dependency_tree
        .nodes
        .values()
        .filter(|node| node.package.name == *package_name && node.is_direct)
        .map(|node| node.package.clone())
        .collect();

    if direct_matches.is_empty() {
        direct_matches = dependency_tree
            .nodes
            .values()
            .filter(|node| node.package.name == *package_name)
            .map(|node| node.package.clone())
            .collect();
    }

    if direct_matches.is_empty() {
        return None;
    }

    match classify_version_spec(version_spec.as_ref()) {
        VersionSpecKind::Unspecified | VersionSpecKind::Tag => {
            select_highest_semver_candidate(&direct_matches)
        }
        VersionSpecKind::Exact(spec) => resolve_exact_spec(spec, &mut direct_matches),
        VersionSpecKind::Range(spec) => resolve_range_spec(spec, &direct_matches).or_else(|| {
            direct_matches.sort_by(|left, right| left.version.cmp(&right.version));
            direct_matches.pop()
        }),
    }
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

    let resolve_package_into_lockfile_params = ResolvePackageIntoLockfileParams {
        current_working_directory,
        package_reference,
    };
    let output = resolve_package_into_lockfile(resolve_package_into_lockfile_params);

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

fn render_lockfile_generation_error(
    current_working_directory: &Path,
    result: Result<crate::types::LockfileGenerationResult, std::io::Error>,
) -> String {
    match result {
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.output.stderr);
            let manager = result.manager;
            let diagnose_lockfile_failure_params = DiagnoseLockfileFailureParams {
                stderr: &stderr,
                manager,
            };
            let hint = diagnose_lockfile_failure(diagnose_lockfile_failure_params);
            let lockfile_only_failed_template_args = vec![manager.command().to_string(), hint];

            render_template(
                NPM_ERR_LOCKFILE_ONLY_FAILED_TEMPLATE,
                &lockfile_only_failed_template_args,
            )
        }
        Err(error) => {
            let manager =
                detect_package_manager(current_working_directory).unwrap_or(PackageManager::Npm);
            let exec_failed_template_args = vec![manager.command().to_string(), error.to_string()];

            render_template(NPM_ERR_EXEC_FAILED_TEMPLATE, &exec_failed_template_args)
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
        .filter(|result| result.is_compromised())
        .cloned()
        .collect();
    let unverifiable = results
        .iter()
        .filter(|result| result.is_blocking_unverifiable())
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
            print_install_blocked_unverifiable(&blocked.unverifiable);
        }
    }
}

fn resolve_install_block_reason(blocked: &BlockedVerifyResults) -> Option<InstallBlockReason> {
    let has_compromised_results = !blocked.compromised.is_empty();
    let has_unverifiable_results = !blocked.unverifiable.is_empty();

    match (has_compromised_results, has_unverifiable_results) {
        (true, _) => Some(InstallBlockReason::Compromised),
        (false, true) => Some(InstallBlockReason::Unverifiable),
        (false, false) => None,
    }
}

fn resolve_install_policy(params: ResolveInstallPolicyParams) -> InstallPolicyDecision {
    let ResolveInstallPolicyParams {
        compromised_count,
        unverifiable_count,
        allow_scripts,
        post_verify,
    } = params;

    let install_policy_input = InstallPolicyInput {
        compromised_count,
        unverifiable_count,
        allow_scripts,
        post_verify,
    };

    DefaultSecurityPolicy.install_decision(install_policy_input)
}

fn should_print_report(params: ShouldPrintReportParams<'_>) -> bool {
    let ShouldPrintReportParams {
        output_format,
        quiet,
    } = params;

    !quiet || !matches!(output_format, OutputFormat::Text)
}

fn prepare_install_lockfiles(args: &InstallArgs, package_ref: &PackageRef) -> Result<(), ExitCode> {
    let ensure_params = EnsureLockfileExistsForInstallParams {
        current_working_directory: &args.cwd,
    };

    if !ensure_lockfile_exists(ensure_params) {
        return Err(ExitCode::FAILURE);
    }

    let prepare_params = PrepareLockfileForInstallParams {
        current_working_directory: &args.cwd,
        package_reference: package_ref,
        quiet: args.quiet,
    };

    if !prepare_lockfile_for_install(prepare_params) {
        return Err(ExitCode::FAILURE);
    }

    Ok(())
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

    let (install_request, requested_package_ref) = parse_requested_install(args)?;

    prepare_install_lockfiles(args, &requested_package_ref)?;

    let lock_hash_before_verify = lockfile_sha256(&args.cwd);
    let shared_state = load_install_shared_state(&args.cwd, args.timeout)?;
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

    let resolve_install_targets_params = ResolveInstallTargetsParams {
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

fn print_blocking_install_results(results: &[VerifyResult]) -> bool {
    let blocked = collect_blocked_verify_results(results);
    let block_reason = resolve_install_block_reason(&blocked);

    match block_reason {
        Some(block_reason) => {
            let print_block_reason_results_params = PrintBlockReasonResultsParams {
                block_reason,
                blocked: &blocked,
            };

            print_block_reason_results(print_block_reason_results_params);

            true
        }
        None => false,
    }
}

fn ensure_ci_lockfile_ready(args: &CiArgs, manager: PackageManager) -> Result<(), ExitCode> {
    let lockfile_path = active_lockfile_path(&args.cwd);
    let lockfile_exists = lockfile_path.exists();

    if lockfile_exists {
        return Ok(());
    }

    if !args.init_lockfile {
        ui::print_generic_error(INSTALL_ERR_NO_LOCKFILE_FOR_CI);

        return Err(ExitCode::FAILURE);
    }

    let is_text_output = matches!(args.format, OutputFormat::Text);
    let sync_lockfile_with_package_json_params = SyncLockfileWithPackageJsonParams {
        current_working_directory: &args.cwd,
        manager,
        lockfile_path: lockfile_path.as_path(),
        quiet: args.quiet,
        is_text_output,
    };
    let sync_result = sync_lockfile_with_package_json(sync_lockfile_with_package_json_params);

    if let Err(error) = sync_result {
        ui::print_generic_error(&error);

        return Err(ExitCode::FAILURE);
    }

    Ok(())
}

fn analyze_dependency_cycles(params: AnalyzeDependencyCyclesParams<'_>) -> Vec<Vec<String>> {
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

#[allow(clippy::unused_async)]
async fn prepare_ci_state(params: PrepareCiStateParams<'_>) -> Result<PreparedCiState, ExitCode> {
    let PrepareCiStateParams { args, manager } = params;
    let is_text_output = matches!(args.format, OutputFormat::Text);

    if let Err(error) = validate_package_json_dependencies(&args.cwd) {
        ui::print_invalid_package_json(&error);

        return Err(ExitCode::FAILURE);
    }

    ensure_ci_lockfile_ready(args, manager)?;

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

fn print_ci_blocking_results(params: PrintCiBlockingResultsParams<'_>) -> bool {
    let PrintCiBlockingResultsParams { results, args } = params;
    let blocked = collect_blocked_verify_results(results);

    let resolve_install_policy_params = ResolveInstallPolicyParams {
        compromised_count: blocked.compromised.len(),
        unverifiable_count: blocked.unverifiable.len(),
        allow_scripts: args.allow_scripts,
        post_verify: args.post_verify,
    };
    let policy_decision = resolve_install_policy(resolve_install_policy_params);

    match policy_decision.block_reason {
        Some(InstallBlockReason::Compromised) => {
            let print_block_reason_results_params = PrintBlockReasonResultsParams {
                block_reason: InstallBlockReason::Compromised,
                blocked: &blocked,
            };

            print_block_reason_results(print_block_reason_results_params);

            true
        }
        Some(InstallBlockReason::Unverifiable) => {
            let print_block_reason_results_params = PrintBlockReasonResultsParams {
                block_reason: InstallBlockReason::Unverifiable,
                blocked: &blocked,
            };

            print_block_reason_results(print_block_reason_results_params);

            true
        }

        None => false,
    }
}

fn finalize_ci_dry_run(params: FinalizeCiDryRunParams<'_>) -> Option<ExitCode> {
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

fn run_clean_install_or_failure(
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
    let install_status = run_clean_install(run_clean_install_params);

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

fn run_ci_post_verify(params: RunCiPostVerifyParams<'_>) -> Result<(), ExitCode> {
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
        quiet: args.quiet,
        is_text_output,
        command_name: HISTORY_COMMAND_CI,
        packages: &clean_packages,
        verify_results: &report.results,
    };

    if let Err(error) = run_post_verify_for_packages(post_verify_params) {
        ui::print_generic_error(&error);

        return Err(ExitCode::FAILURE);
    }

    Ok(())
}

fn complete_successful_ci_run(params: CompleteSuccessfulCiRunParams<'_>) -> ExitCode {
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

fn finalize_ci_run(params: FinalizeCiRunParams<'_>) -> ExitCode {
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
    let policy_decision = resolve_install_policy(resolve_install_policy_params);
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

    let pipeline = run_clean_install_or_failure(run_clean_install_or_failure_params)
        .and_then(|()| run_ci_post_verify(run_ci_post_verify_params));

    if let Err(exit_code) = pipeline {
        return exit_code;
    }

    let complete_successful_ci_run_params = CompleteSuccessfulCiRunParams {
        args,
        report,
        lock_hash_before_verify,
        is_text_output,
    };

    complete_successful_ci_run(complete_successful_ci_run_params)
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
    let should_print_install_report = should_print_report(should_print_report_params);

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
    let install_status = install_from_verified_source(install_from_verified_source_params);

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

fn complete_successful_install(
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
        quiet: args.quiet,
        is_text_output,
        command_name: HISTORY_COMMAND_INSTALL,
        packages: &target_packages,
        verify_results: &report.results,
    };

    let mut post_verify_result = Ok(());
    if args.post_verify {
        post_verify_result = run_post_verify_for_packages(post_verify_params).map_err(|error| {
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
        append_install_history(append_install_history_params).map_err(|error| {
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
fn finalize_install_run(params: FinalizeInstallRunParams<'_>) -> InstallExecutionOutcome {
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
    let policy_decision = resolve_install_policy(resolve_install_policy_params);

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
    let install_pipeline =
        install_from_verified_source_or_failure(install_from_verified_source_or_failure_params)
            .map(|()| {
                let complete_successful_install_params = CompleteSuccessfulInstallParams {
                    args,
                    package_ref,
                    report,
                    lock_hash_before_verify,
                    is_text_output,
                };

                complete_successful_install(complete_successful_install_params)
            });

    match install_pipeline {
        Err(outcome) | Ok(outcome) => outcome,
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

    finalize_install_run(finalize_install_run_params)
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

fn ensure_lockfile_exists(params: EnsureLockfileExistsForInstallParams<'_>) -> bool {
    let EnsureLockfileExistsForInstallParams {
        current_working_directory,
        ..
    } = params;

    let lockfile_path = active_lockfile_path(current_working_directory);

    if lockfile_path.exists() {
        return true;
    }

    ui::print_generic_error(INSTALL_ERR_NO_LOCKFILE_FOR_INSTALL);

    false
}

fn sync_lockfile_with_package_json(
    params: SyncLockfileWithPackageJsonParams<'_>,
) -> Result<(), String> {
    let SyncLockfileWithPackageJsonParams {
        current_working_directory,
        manager,
        lockfile_path,
        quiet,
        is_text_output,
    } = params;
    let should_print_sync_notices = !quiet && is_text_output;

    if should_print_sync_notices {
        ui::print_missing_lockfile_notice();
    }

    let result = generate_lockfile_with_manager(current_working_directory, manager);

    match result {
        Ok(result) if result.output.status.success() => {
            if !lockfile_path.exists() {
                return Err(INSTALL_ERR_LOCKFILE_INIT_MISSING_AFTER_SUCCESS.to_string());
            }

            if should_print_sync_notices {
                ui::print_lockfile_created_notice();
            }

            Ok(())
        }
        _ => Err(render_lockfile_generation_error(
            current_working_directory,
            result,
        )),
    }
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

    finalize_ci_run(finalize_ci_run_params)
}

#[cfg(test)]
#[path = "../../tests/internal/install_tests.rs"]
mod tests;
