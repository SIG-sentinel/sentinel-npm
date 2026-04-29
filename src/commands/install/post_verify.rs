use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::constants::paths::{NODE_MODULES_DIR, PACKAGE_JSON_FILE};
use crate::constants::{
    INSTALL_ERR_FAILED_COMPUTE_RELATIVE_PATH, INSTALL_ERR_FAILED_DOWNLOAD_REGISTRY_TARBALL,
    INSTALL_ERR_FAILED_FETCH_REGISTRY_METADATA, INSTALL_ERR_FAILED_INIT_POST_VERIFY_VERIFIER,
    INSTALL_ERR_FAILED_PARSE_PACKAGE_MANIFEST,
    INSTALL_ERR_FAILED_PARSE_REGISTRY_TARBALL_ENTRY_POST_VERIFY, INSTALL_ERR_FAILED_READ_DIRECTORY,
    INSTALL_ERR_FAILED_READ_DIRECTORY_ENTRY, INSTALL_ERR_FAILED_READ_FILE,
    INSTALL_ERR_FAILED_READ_FILE_TYPE, INSTALL_ERR_FAILED_READ_PACKAGE_MANIFEST,
    INSTALL_ERR_FAILED_READ_REGISTRY_TARBALL_ENTRIES_POST_VERIFY,
    INSTALL_ERR_FAILED_READ_REGISTRY_TARBALL_FILE_CONTENT_POST_VERIFY,
    INSTALL_ERR_FAILED_READ_REGISTRY_TARBALL_PATH_POST_VERIFY,
    INSTALL_ERR_FAILED_READ_TARBALL_RESPONSE, INSTALL_ERR_PACKAGE_NOT_FOUND_NODE_MODULES,
    POST_VERIFY_CONTENT_MISMATCH_ERR_TEMPLATE, POST_VERIFY_GOOD_TERM_SECS,
    POST_VERIFY_LARGE_PACKAGE_WARN_THRESHOLD, POST_VERIFY_MAX_CONCURRENCY,
    POST_VERIFY_MISSING_PACKAGES_ERR_TEMPLATE, POST_VERIFY_PACKAGE_PREFIX,
    POST_VERIFY_TARBALL_PACKAGE_PREFIX,
};
use crate::types::{
    CheckContentFingerprintMismatchesParams, ComputePostVerifyRegistryFingerprintParams,
    ComputeRegistryPackageFingerprintFromSourceUrlParams, DirectoryEntryKind,
    EvaluatePostVerifyPackageMismatchParams, FindContentMismatchPostVerifyPackagesParams,
    FreshFingerprintPolicy, InitialRegistryFingerprintStrategy, InstallFingerprintResult,
    PackageRef, PrintPostVerifyElapsedWarningParams, ProcessDirectoryEntryParams,
    ProcessInstalledPackageIndexEntryParams, RegistryTarballMatchesInstalledContentParams,
    ResolveInitialRegistryFingerprintParams, RunPostVerifyForPackagesParams, VerifierNewParams,
    VerifyResult, WarnPostVerifyElapsedParams, WarnPostVerifyLargeScopeParams,
};
use crate::ui::command_feedback as ui;
use crate::utils::{
    format_err_for_package, format_err_for_path, format_err_in_path, format_err_with_path,
    format_err_with_reason, format_err_with_subject, format_prefixed_package_message,
};
use crate::verifier::{Verifier, compute_tarball_fingerprint_bytes};

use super::resolve::package_key;

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

fn normalize_tarball_relative_path(path: &str) -> Option<String> {
    let relative_path = path.trim_start_matches('/');
    let invalid_path =
        relative_path.is_empty() || relative_path.split('/').any(|segment| segment == "..");

    if invalid_path {
        return None;
    }

    Some(relative_path.to_string())
}

fn classify_initial_registry_fingerprint_strategy<'a>(
    cached_registry_fingerprint: Option<&'a String>,
    verified_source_url: Option<&'a str>,
) -> InitialRegistryFingerprintStrategy<'a> {
    match cached_registry_fingerprint {
        Some(fingerprint) if !fingerprint.is_empty() => {
            InitialRegistryFingerprintStrategy::UseCached(fingerprint.clone())
        }
        _ => InitialRegistryFingerprintStrategy::Compute(verified_source_url),
    }
}

fn classify_fresh_fingerprint_policy(
    cached_registry_fingerprint: Option<&String>,
) -> FreshFingerprintPolicy {
    match cached_registry_fingerprint {
        Some(_) => FreshFingerprintPolicy::RevalidateWithFreshFingerprint,
        None => FreshFingerprintPolicy::ReturnImmediately,
    }
}

async fn resolve_initial_registry_fingerprint(
    params: ResolveInitialRegistryFingerprintParams<'_>,
) -> InstallFingerprintResult<String> {
    let ResolveInitialRegistryFingerprintParams {
        verifier,
        package_ref,
        cached_registry_fingerprint,
        verified_source_url,
    } = params;

    let initial_registry_fingerprint_strategy = classify_initial_registry_fingerprint_strategy(
        cached_registry_fingerprint,
        verified_source_url,
    );

    match initial_registry_fingerprint_strategy {
        InitialRegistryFingerprintStrategy::UseCached(fingerprint) => Ok(fingerprint),
        InitialRegistryFingerprintStrategy::Compute(source_url) => {
            let compute_post_verify_registry_fingerprint_params =
                ComputePostVerifyRegistryFingerprintParams {
                    verifier,
                    package_ref,
                    verified_source_url: source_url,
                };

            compute_post_verify_registry_fingerprint(
                compute_post_verify_registry_fingerprint_params,
            )
            .await
        }
    }
}

async fn evaluate_post_verify_package_mismatch(
    params: EvaluatePostVerifyPackageMismatchParams<'_>,
) -> InstallFingerprintResult<Option<PackageRef>> {
    let EvaluatePostVerifyPackageMismatchParams {
        installed_package_index,
        verifier,
        cached_fingerprints,
        verified_source_urls,
        package_ref,
    } = params;
    let installed_fingerprint =
        compute_installed_package_fingerprint(installed_package_index, &package_ref)?;
    let package_identifier = package_key(&package_ref);

    let verified_source_url = verified_source_urls
        .get(&package_identifier)
        .map(String::as_str);

    let cached_registry_fingerprint = cached_fingerprints.get(&package_identifier).cloned();
    let resolve_initial_registry_fingerprint_params = ResolveInitialRegistryFingerprintParams {
        verifier,
        package_ref: &package_ref,
        cached_registry_fingerprint: cached_registry_fingerprint.as_ref(),
        verified_source_url,
    };
    let registry_fingerprint =
        resolve_initial_registry_fingerprint(resolve_initial_registry_fingerprint_params).await?;

    let mismatch_against_cached = installed_fingerprint != registry_fingerprint;

    if !mismatch_against_cached {
        return Ok(None);
    }

    let installed_root = installed_package_index
        .get(&package_identifier)
        .cloned()
        .ok_or_else(|| {
            format_prefixed_package_message(
                POST_VERIFY_PACKAGE_PREFIX,
                &package_ref,
                INSTALL_ERR_PACKAGE_NOT_FOUND_NODE_MODULES,
            )
        })?;

    let registry_tarball_matches_installed_content_params =
        RegistryTarballMatchesInstalledContentParams {
            verifier,
            package_ref: &package_ref,
            installed_root: &installed_root,
        };
    let content_matches = registry_tarball_matches_installed_content(
        registry_tarball_matches_installed_content_params,
    )
    .await?;

    if content_matches {
        return Ok(None);
    }

    let fresh_fingerprint_policy =
        classify_fresh_fingerprint_policy(cached_registry_fingerprint.as_ref());

    match fresh_fingerprint_policy {
        FreshFingerprintPolicy::ReturnImmediately => Ok(Some(package_ref)),
        FreshFingerprintPolicy::RevalidateWithFreshFingerprint => {
            let compute_post_verify_registry_fingerprint_params =
                ComputePostVerifyRegistryFingerprintParams {
                    verifier,
                    package_ref: &package_ref,
                    verified_source_url,
                };
            let fresh_registry_fingerprint = compute_post_verify_registry_fingerprint(
                compute_post_verify_registry_fingerprint_params,
            )
            .await?;
            let mismatch_against_fresh_registry =
                installed_fingerprint != fresh_registry_fingerprint;

            Ok(mismatch_against_fresh_registry.then_some(package_ref))
        }
    }
}

fn collect_sorted_post_verify_mismatches(
    mismatch_results: Vec<InstallFingerprintResult<Option<PackageRef>>>,
) -> InstallFingerprintResult<Vec<PackageRef>> {
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

pub(super) async fn compute_registry_package_fingerprint(
    verifier: &Verifier,
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

pub(super) async fn compute_registry_package_fingerprint_from_source_url(
    params: ComputeRegistryPackageFingerprintFromSourceUrlParams<'_>,
) -> InstallFingerprintResult<String> {
    let ComputeRegistryPackageFingerprintFromSourceUrlParams {
        verifier,
        package_ref,
        source_url,
    } = params;

    let response = verifier
        .registry
        .download_tarball(source_url)
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

pub(super) async fn compute_post_verify_registry_fingerprint(
    params: ComputePostVerifyRegistryFingerprintParams<'_>,
) -> InstallFingerprintResult<String> {
    let ComputePostVerifyRegistryFingerprintParams {
        verifier,
        package_ref,
        verified_source_url,
    } = params;

    let canonical_result = compute_registry_package_fingerprint(verifier, package_ref).await;

    if let Ok(fingerprint) = canonical_result {
        return Ok(fingerprint);
    }

    let Some(source_url) = verified_source_url else {
        return canonical_result;
    };

    let compute_registry_package_fingerprint_from_source_url_params =
        ComputeRegistryPackageFingerprintFromSourceUrlParams {
            verifier,
            package_ref,
            source_url,
        };

    compute_registry_package_fingerprint_from_source_url(
        compute_registry_package_fingerprint_from_source_url_params,
    )
    .await
}

pub(super) fn read_package_ref_from_manifest(
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

pub(super) fn process_installed_package_index_entry(
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

    if let Some(package_ref) = read_package_ref_from_manifest(&package_manifest)? {
        index.insert(package_key(&package_ref), entry_path);
    }

    Ok(())
}

pub(super) fn build_installed_package_index(
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

pub(super) fn process_directory_entry(params: ProcessDirectoryEntryParams<'_>) {
    let ProcessDirectoryEntryParams {
        entry_path,
        file_type,
        entries,
        pending_paths,
    } = params;

    let directory_entry_kind = classify_directory_entry_kind(*file_type);

    match directory_entry_kind {
        DirectoryEntryKind::Directory => {
            pending_paths.push(entry_path);
        }
        DirectoryEntryKind::File => {
            entries.push(entry_path);
        }
        DirectoryEntryKind::SkipSymlink | DirectoryEntryKind::Other => {}
    }
}

fn classify_directory_entry_kind(file_type: std::fs::FileType) -> DirectoryEntryKind {
    let is_symlink = file_type.is_symlink();
    let is_directory = file_type.is_dir();
    let is_file = file_type.is_file();

    match (is_symlink, is_directory, is_file) {
        (true, _, _) => DirectoryEntryKind::SkipSymlink,
        (false, true, _) => DirectoryEntryKind::Directory,
        (false, false, true) => DirectoryEntryKind::File,
        (false, false, false) => DirectoryEntryKind::Other,
    }
}

pub(super) fn collect_directory_entries(root: &Path) -> InstallFingerprintResult<Vec<PathBuf>> {
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

pub(super) fn compute_directory_fingerprint(root: &Path) -> InstallFingerprintResult<String> {
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

pub(super) fn compute_installed_package_fingerprint(
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

pub(super) fn collect_registry_tarball_file_hashes(
    package_ref: &PackageRef,
    tarball_bytes: &[u8],
) -> InstallFingerprintResult<HashMap<String, Vec<u8>>> {
    let decoder = flate2::read::GzDecoder::new(tarball_bytes);
    let mut archive = tar::Archive::new(decoder);
    let mut file_hashes = HashMap::new();

    let mut entries = archive.entries().map_err(|error| {
        format_err_for_package(
            INSTALL_ERR_FAILED_READ_REGISTRY_TARBALL_ENTRIES_POST_VERIFY,
            package_ref,
            &error,
        )
    })?;

    for entry_result in &mut entries {
        let mut entry = entry_result.map_err(|error| {
            format_err_for_package(
                INSTALL_ERR_FAILED_PARSE_REGISTRY_TARBALL_ENTRY_POST_VERIFY,
                package_ref,
                &error,
            )
        })?;

        if !entry.header().entry_type().is_file() {
            continue;
        }

        let entry_path = entry.path().map_err(|error| {
            format_err_for_package(
                INSTALL_ERR_FAILED_READ_REGISTRY_TARBALL_PATH_POST_VERIFY,
                package_ref,
                &error,
            )
        })?;

        let path_text = entry_path.to_string_lossy().replace('\\', "/");

        let raw_relative_path = path_text
            .strip_prefix(POST_VERIFY_TARBALL_PACKAGE_PREFIX)
            .unwrap_or(&path_text);

        let Some(relative_path) = normalize_tarball_relative_path(raw_relative_path) else {
            continue;
        };

        let mut bytes = Vec::new();

        entry.read_to_end(&mut bytes).map_err(|error| {
            format_err_for_package(
                INSTALL_ERR_FAILED_READ_REGISTRY_TARBALL_FILE_CONTENT_POST_VERIFY,
                package_ref,
                &error,
            )
        })?;

        let entry_hash = Sha256::digest(&bytes).to_vec();

        file_hashes.insert(relative_path, entry_hash);
    }

    Ok(file_hashes)
}

pub(super) fn collect_installed_file_hashes(
    installed_root: &Path,
) -> InstallFingerprintResult<HashMap<String, Vec<u8>>> {
    let entries = collect_directory_entries(installed_root)?;
    let mut file_hashes = HashMap::new();

    for entry_path in entries {
        let relative_path = entry_path.strip_prefix(installed_root).map_err(|error| {
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

        let file_digest = Sha256::digest(&file_bytes).to_vec();
        file_hashes.insert(relative_path_text, file_digest);
    }

    Ok(file_hashes)
}

pub(super) async fn registry_tarball_matches_installed_content(
    params: RegistryTarballMatchesInstalledContentParams<'_>,
) -> InstallFingerprintResult<bool> {
    let RegistryTarballMatchesInstalledContentParams {
        verifier,
        package_ref,
        installed_root,
    } = params;

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

    let registry_tarball_hashes =
        collect_registry_tarball_file_hashes(package_ref, &tarball_bytes)?;
    let installed_hashes = collect_installed_file_hashes(installed_root)?;

    Ok(installed_hashes == registry_tarball_hashes)
}

pub(super) async fn find_content_mismatch_post_verify_packages(
    params: FindContentMismatchPostVerifyPackagesParams<'_>,
) -> InstallFingerprintResult<Vec<PackageRef>> {
    let FindContentMismatchPostVerifyPackagesParams {
        installed_package_index,
        packages,
        verifier,
        cached_fingerprints,
        verified_source_urls,
    } = params;

    let mismatch_results = futures_util::stream::iter(packages.iter().cloned())
        .map(|package_ref| async move {
            let evaluate_post_verify_package_mismatch_params =
                EvaluatePostVerifyPackageMismatchParams {
                    installed_package_index,
                    verifier,
                    cached_fingerprints,
                    verified_source_urls,
                    package_ref,
                };

            evaluate_post_verify_package_mismatch(evaluate_post_verify_package_mismatch_params)
                .await
        })
        .buffer_unordered(POST_VERIFY_MAX_CONCURRENCY)
        .collect::<Vec<_>>()
        .await;

    collect_sorted_post_verify_mismatches(mismatch_results)
}

pub(super) fn create_post_verify_verifier(
    current_working_directory: &Path,
    timeout_ms: u64,
    registry_max_in_flight: Option<usize>,
) -> InstallFingerprintResult<Verifier> {
    let verifier_new_params = VerifierNewParams {
        timeout_ms,
        registry_max_in_flight,
        current_working_directory,
        cache_dir: None,
        artifact_store: crate::artifact_store_config::get(),
        max_memory_bytes: crate::verifier::memory_budget::detect_memory_budget(),
    };

    Verifier::new(verifier_new_params).map_err(|error| {
        format_err_with_reason(INSTALL_ERR_FAILED_INIT_POST_VERIFY_VERIFIER, &error)
    })
}

pub(super) fn collect_cached_tarball_fingerprints(
    results: &[VerifyResult],
) -> HashMap<String, String> {
    results
        .iter()
        .filter_map(|result| {
            let fingerprint = result.tarball_fingerprint.as_ref()?;

            Some((package_key(&result.package), fingerprint.clone()))
        })
        .collect()
}

pub(super) fn collect_verified_source_urls(results: &[VerifyResult]) -> HashMap<String, String> {
    results
        .iter()
        .filter_map(|result| {
            let source_url = result.evidence.source_url.as_ref()?;

            Some((package_key(&result.package), source_url.clone()))
        })
        .collect()
}

pub(super) fn warn_post_verify_large_scope(params: WarnPostVerifyLargeScopeParams<'_>) {
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

pub(super) fn warn_post_verify_elapsed(params: WarnPostVerifyElapsedParams<'_>) {
    let WarnPostVerifyElapsedParams {
        command_name,
        package_count,
        elapsed,
        should_warn,
    } = params;

    let is_elapsed_too_long = elapsed.as_secs() > POST_VERIFY_GOOD_TERM_SECS;
    let should_skip_elapsed_warning = !should_warn || !is_elapsed_too_long;

    if should_skip_elapsed_warning {
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

pub(super) fn find_missing_post_verify_packages_from_index(
    installed_package_index: &HashMap<String, PathBuf>,
    packages: &[PackageRef],
) -> Vec<PackageRef> {
    packages
        .iter()
        .filter(|package_ref| !installed_package_index.contains_key(&package_key(package_ref)))
        .cloned()
        .collect()
}

pub(super) fn format_post_verify_missing_packages_message(
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

pub(super) fn format_post_verify_content_mismatch_message(
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

pub(super) async fn check_content_fingerprint_mismatches(
    params: CheckContentFingerprintMismatchesParams<'_>,
) -> InstallFingerprintResult<()> {
    let CheckContentFingerprintMismatchesParams {
        current_working_directory,
        timeout_ms,
        registry_max_in_flight,
        installed_package_index,
        packages,
        verify_results,
        command_name,
    } = params;

    let verifier = create_post_verify_verifier(
        current_working_directory,
        timeout_ms,
        registry_max_in_flight,
    )?;
    let cached_fingerprints = collect_cached_tarball_fingerprints(verify_results);
    let verified_source_urls = collect_verified_source_urls(verify_results);

    let find_content_mismatch_post_verify_packages_params =
        FindContentMismatchPostVerifyPackagesParams {
            installed_package_index,
            packages,
            verifier: &verifier,
            cached_fingerprints: &cached_fingerprints,
            verified_source_urls: &verified_source_urls,
        };

    let mismatches = find_content_mismatch_post_verify_packages(
        find_content_mismatch_post_verify_packages_params,
    )
    .await?;

    if !mismatches.is_empty() {
        return Err(format_post_verify_content_mismatch_message(
            command_name,
            &mismatches,
        ));
    }

    Ok(())
}

pub(super) async fn run_post_verify_for_packages(
    params: RunPostVerifyForPackagesParams<'_>,
) -> InstallFingerprintResult<()> {
    let RunPostVerifyForPackagesParams {
        current_working_directory,
        timeout_ms,
        registry_max_in_flight,
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
        registry_max_in_flight,
        installed_package_index: &installed_package_index,
        packages,
        verify_results,
        command_name,
    };

    check_content_fingerprint_mismatches(check_content_fingerprint_mismatches_params).await?;

    let warn_post_verify_elapsed_params = WarnPostVerifyElapsedParams {
        command_name,
        package_count: packages.len(),
        elapsed: post_verify_started_at.elapsed(),
        should_warn,
    };

    warn_post_verify_elapsed(warn_post_verify_elapsed_params);

    Ok(())
}
