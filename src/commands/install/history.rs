use crate::constants::{HISTORY_COMMAND_CI, HISTORY_COMMAND_INSTALL};
use crate::ecosystem::{PackageManager, detect_package_manager};
use crate::history::ledger::{AppendHistoryEventsParams, append_history_events};
use crate::history::types::HistoryPackageMetadata;
use crate::types::{AppendCiHistoryParams, AppendInstallHistoryParams};

#[allow(clippy::ref_option)]
pub(super) fn append_ci_history(params: AppendCiHistoryParams<'_>) -> Result<(), String> {
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

    let lock_hash_after_install = crate::utils::lockfile_sha256(&args.cwd);
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

#[allow(clippy::ref_option)]
pub(super) fn append_install_history(params: AppendInstallHistoryParams<'_>) -> Result<(), String> {
    let AppendInstallHistoryParams {
        args,
        package_ref,
        lock_hash_before_verify,
    } = params;

    let lock_hash_after_install = crate::utils::lockfile_sha256(&args.cwd);
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
