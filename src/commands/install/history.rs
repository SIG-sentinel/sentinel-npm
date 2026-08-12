use crate::constants::{HISTORY_COMMAND_CI, HISTORY_COMMAND_INSTALL};
use crate::ecosystem::{PackageManager, detect_package_manager};
use crate::history::ledger::{AppendHistoryEventsParams, append_history_events};
use crate::history::types::{HistoryEventPackageInput, HistoryPackageMetadata};
use crate::types::{AppendCiHistoryParams, AppendInstallHistoryParams};

#[allow(clippy::ref_option)]
pub(super) fn append_ci_history(params: AppendCiHistoryParams<'_>) -> Result<(), String> {
    let AppendCiHistoryParams {
        args,
        report,
        lock_hash_before_verify,
    } = params;

    let ci_packages: Vec<HistoryEventPackageInput> = report
        .results
        .iter()
        .filter(|result| result.is_clean())
        .map(|result| {
            let had_provenance = result.evidence.provenance_workflow_path.is_some()
                || result.evidence.provenance_identity.is_some();

            HistoryEventPackageInput {
                package: HistoryPackageMetadata {
                    name: result.package.name.clone(),
                    version: result.package.version.clone(),
                    direct: result.is_direct,
                },
                had_provenance,
                provenance_workflow_path: result.evidence.provenance_workflow_path.clone(),
            }
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
        report,
        lock_hash_before_verify,
    } = params;

    let lock_hash_after_install = crate::utils::lockfile_sha256(&args.cwd);
    let manager = detect_package_manager(&args.cwd).unwrap_or(PackageManager::Npm);
    let install_result = report.results.iter().find(|result| {
        result.is_clean()
            && result.package.name == package_ref.name
            && result.package.version == package_ref.version
    });

    let (had_provenance, provenance_workflow_path) = install_result
        .map_or((false, None), |result| {
            let current_had_provenance = result.evidence.provenance_workflow_path.is_some()
                || result.evidence.provenance_identity.is_some();

            (
                current_had_provenance,
                result.evidence.provenance_workflow_path.clone(),
            )
        });

    let install_packages = vec![HistoryEventPackageInput {
        package: HistoryPackageMetadata {
            name: package_ref.name.clone(),
            version: package_ref.version.clone(),
            direct: true,
        },
        had_provenance,
        provenance_workflow_path,
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
