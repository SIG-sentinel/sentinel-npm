use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::artifact_store_config;
use crate::ecosystem::{build_dependency_tree_for_manager, read_lockfile_entries};
use crate::npm::read_package_json_deps;
use crate::types::{
    BuildLockfileEntryParams, DependencyTree, LockfileEntry, PrintVerificationProgressParams,
    ReadPackageJsonDepsParams, Report, RunMode, SentinelError, SharedCommandState,
    SharedCommandStateError, UpdateVerificationProgressParams, VerifierNewParams,
    VerifyPackagesExecutionParams, VerifyPackagesParams, VerifyResult, VerifySinglePackageParams,
};
use crate::verifier::Verifier;
use crate::verifier::memory_budget::detect_memory_budget;

const FIRST_PROGRESS_UPDATE: usize = 1;
const PERCENT_SCALE: usize = 100;
const DEFAULT_PROGRESS_PARTITIONS: usize = 10;
const ZERO_REMAINDER: usize = 0;

pub(super) fn validate_package_json_dependencies(
    current_working_directory: &Path,
) -> Result<(), SentinelError> {
    let read_package_json_deps_params = ReadPackageJsonDepsParams {
        project_dir: current_working_directory,
        include_dev: true,
    };

    read_package_json_deps(read_package_json_deps_params).map(|_| ())
}

pub(super) fn load_dependency_tree(
    current_working_directory: &Path,
) -> Result<DependencyTree, SentinelError> {
    let lockfile_entries = read_lockfile_entries(current_working_directory)?;

    build_dependency_tree_for_manager(current_working_directory, &lockfile_entries)
}

pub(super) fn load_lockfile_entries(
    current_working_directory: &Path,
) -> Result<Arc<HashMap<String, LockfileEntry>>, SentinelError> {
    read_lockfile_entries(current_working_directory).map(Arc::new)
}

pub(super) fn load_command_state(
    current_working_directory: &Path,
    timeout_ms: u64,
    registry_max_in_flight: Option<usize>,
) -> Result<SharedCommandState, SharedCommandStateError> {
    let dependency_tree = load_dependency_tree(current_working_directory)
        .map_err(SharedCommandStateError::DependencyTree)?;
    let lockfile_entries = load_lockfile_entries(current_working_directory)
        .map_err(SharedCommandStateError::LockfileEntries)?;

    let artifact_store = artifact_store_config::get();
    let max_memory_bytes = detect_memory_budget();

    let verifier_new_params = VerifierNewParams {
        timeout_ms,
        registry_max_in_flight,
        current_working_directory,
        cache_dir: None,
        artifact_store,
        max_memory_bytes,
    };

    let verifier = Verifier::new(verifier_new_params)
        .map(Arc::new)
        .map_err(SharedCommandStateError::Verifier)?;

    let shared_command_state = SharedCommandState {
        dependency_tree,
        lockfile_entries,
        verifier,
    };

    Ok(shared_command_state)
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
        dependencies: dependency_node.dependencies.clone(),
    }
}

fn update_verification_progress(params: UpdateVerificationProgressParams<'_>) {
    let UpdateVerificationProgressParams {
        progress_bar,
        show_text_progress_fallback,
        completed_counter,
        total_packages,
        progress_step,
    } = params;

    if let Some(progress_bar) = progress_bar {
        progress_bar.inc(1);
        return;
    }

    if !show_text_progress_fallback {
        return;
    }

    let completed = completed_counter.fetch_add(1, Ordering::Relaxed) + FIRST_PROGRESS_UPDATE;
    let is_first_update = completed == FIRST_PROGRESS_UPDATE;
    let is_last_update = completed == total_packages;
    let reached_progress_step = completed % progress_step == ZERO_REMAINDER;
    let should_print_progress = is_first_update || is_last_update || reached_progress_step;

    if !should_print_progress {
        return;
    }

    let percentage = completed.saturating_mul(PERCENT_SCALE) / total_packages.max(1);

    let print_verification_progress_params = PrintVerificationProgressParams {
        completed,
        total: total_packages,
        percentage,
    };

    crate::ui::print_verification_progress(print_verification_progress_params);
}

async fn verify_single_package(params: VerifySinglePackageParams) -> VerifyResult {
    let VerifySinglePackageParams {
        node,
        verifier,
        concurrency_gate,
        lockfile_entries,
        progress_bar,
        completed_counter,
        show_text_progress_fallback,
        total_packages,
        progress_step,
    } = params;

    let permit = concurrency_gate.acquire().await.ok();

    let build_lockfile_entry_params = BuildLockfileEntryParams {
        dependency_node: &node,
        lockfile_entries: lockfile_entries.as_ref(),
    };
    let entry = build_lockfile_entry(build_lockfile_entry_params);
    let is_direct = node.is_direct;
    let direct_parent = node.direct_parent.clone();
    let mut result = verifier.check_from_lockfile(&entry).await;

    result.is_direct = is_direct;
    result.direct_parent = direct_parent;

    drop(permit);

    let update_verification_progress_params = UpdateVerificationProgressParams {
        progress_bar: progress_bar.as_deref(),
        show_text_progress_fallback,
        completed_counter: &completed_counter,
        total_packages,
        progress_step,
    };

    update_verification_progress(update_verification_progress_params);

    result
}

pub(super) async fn verify_packages(params: VerifyPackagesExecutionParams) -> Vec<VerifyResult> {
    let VerifyPackagesExecutionParams {
        verify_packages_params,
        max_concurrency,
        progress_bar,
        show_text_progress_fallback,
    } = params;

    let VerifyPackagesParams {
        packages_to_verify,
        verifier,
        lockfile_entries,
    } = verify_packages_params;

    let total_packages = packages_to_verify.len();
    let progress_step =
        total_packages.max(DEFAULT_PROGRESS_PARTITIONS) / DEFAULT_PROGRESS_PARTITIONS;
    let completed_counter = Arc::new(AtomicUsize::new(0));

    let concurrency_gate = Arc::new(tokio::sync::Semaphore::new(max_concurrency));

    let verify_futures: Vec<_> = packages_to_verify
        .into_iter()
        .map(|node| {
            let verifier_ref = verifier.clone();
            let gate_ref = concurrency_gate.clone();
            let progress_ref = progress_bar.as_ref().map(|pb| Arc::new(pb.clone()));
            let lock_entries_ref = lockfile_entries.clone();
            let completed_counter_ref = completed_counter.clone();

            let verify_single_package_params = VerifySinglePackageParams {
                node,
                verifier: verifier_ref,
                concurrency_gate: gate_ref,
                lockfile_entries: lock_entries_ref,
                progress_bar: progress_ref,
                completed_counter: completed_counter_ref,
                show_text_progress_fallback,
                total_packages,
                progress_step,
            };

            verify_single_package(verify_single_package_params)
        })
        .collect();

    let results = futures_util::future::join_all(verify_futures).await;

    if let Some(progress_bar) = progress_bar {
        progress_bar.finish_and_clear();
    }

    results
}

pub(super) fn build_report(
    mode: RunMode,
    results: Vec<VerifyResult>,
    cycles: Vec<Vec<String>>,
) -> Report {
    Report::from_results(mode, results, cycles)
}

#[cfg(test)]
#[path = "../../tests/internal/shared_command_tests.rs"]
mod tests;
