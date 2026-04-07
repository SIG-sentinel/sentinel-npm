use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::npm::{LockfileEntry, build_dependency_tree, read_npm_lockfile, read_package_json_deps};
use crate::types::{
    BuildLockfileEntryParams, DependencyTree, ReadPackageJsonDepsParams, Report, RunMode,
    SentinelError, VerifierNewParams, VerifyPackagesExecutionParams, VerifyPackagesParams,
    VerifyResult,
};
use crate::verifier::Verifier;

pub(super) struct SharedCommandState {
    pub dependency_tree: DependencyTree,
    pub lockfile_entries: Arc<HashMap<String, LockfileEntry>>,
    pub verifier: Arc<Verifier>,
}

pub(super) enum SharedCommandStateError {
    DependencyTree(SentinelError),
    LockfileEntries(SentinelError),
    Verifier(SentinelError),
}

pub(super) fn validate_package_json_dependencies(
    current_working_directory: &Path,
) -> Result<(), SentinelError> {
    read_package_json_deps(ReadPackageJsonDepsParams {
        project_dir: current_working_directory,
        include_dev: true,
    })
    .map(|_| ())
}

pub(super) fn load_dependency_tree(
    current_working_directory: &Path,
) -> Result<DependencyTree, SentinelError> {
    build_dependency_tree(current_working_directory)
}

pub(super) fn load_lockfile_entries(
    current_working_directory: &Path,
) -> Result<Arc<HashMap<String, LockfileEntry>>, SentinelError> {
    read_npm_lockfile(current_working_directory).map(Arc::new)
}

pub(super) fn load_command_state(
    current_working_directory: &Path,
    timeout_ms: u64,
) -> Result<SharedCommandState, SharedCommandStateError> {
    let dependency_tree = load_dependency_tree(current_working_directory)
        .map_err(SharedCommandStateError::DependencyTree)?;
    let lockfile_entries = load_lockfile_entries(current_working_directory)
        .map_err(SharedCommandStateError::LockfileEntries)?;
    let verifier = Verifier::new(VerifierNewParams {
        timeout_ms,
        cache_dir: None,
    })
    .map(Arc::new)
    .map_err(SharedCommandStateError::Verifier)?;

    Ok(SharedCommandState {
        dependency_tree,
        lockfile_entries,
        verifier,
    })
}

fn build_lockfile_entry(params: BuildLockfileEntryParams<'_>) -> LockfileEntry {
    let BuildLockfileEntryParams {
        dependency_node,
        lockfile_entries,
    } = params;

    let package_key = dependency_node.package.to_string();
    let lock_integrity = lockfile_entries
        .get(&package_key)
        .and_then(|entry| entry.integrity.clone());

    LockfileEntry {
        package: dependency_node.package.clone(),
        integrity: lock_integrity,
        is_dev: dependency_node.is_dev,
    }
}

pub(super) async fn verify_packages(params: VerifyPackagesExecutionParams) -> Vec<VerifyResult> {
    let VerifyPackagesExecutionParams {
        verify_packages_params,
        max_concurrency,
        progress_bar,
    } = params;

    let VerifyPackagesParams {
        packages_to_verify,
        verifier,
        lockfile_entries,
    } = verify_packages_params;

    let concurrency_gate = Arc::new(tokio::sync::Semaphore::new(max_concurrency));

    let verify_futures: Vec<_> = packages_to_verify
        .into_iter()
        .map(|node| {
            let verifier_ref = verifier.clone();
            let gate_ref = concurrency_gate.clone();
            let progress_ref = progress_bar.clone();
            let lock_entries_ref = lockfile_entries.clone();

            async move {
                let permit = gate_ref.acquire().await.ok();
                let entry = build_lockfile_entry(BuildLockfileEntryParams {
                    dependency_node: &node,
                    lockfile_entries: lock_entries_ref.as_ref(),
                });
                let result = verifier_ref.check_from_lockfile(&entry).await;
                drop(permit);

                if let Some(progress_bar) = &progress_ref {
                    progress_bar.inc(1);
                }

                result
            }
        })
        .collect();

    let results = futures_util::future::join_all(verify_futures).await;

    if let Some(progress_bar) = progress_bar {
        progress_bar.finish_and_clear();
    }

    results
}

pub(super) fn build_report(mode: RunMode, results: Vec<VerifyResult>) -> Report {
    Report::from_results(mode, results)
}
