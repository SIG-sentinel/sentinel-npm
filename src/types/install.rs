use std::collections::HashMap;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

use crate::npm::LockfileEntry;
use crate::verifier::Verifier;

use super::{
    CiArgs, DependencyNode, DependencyTree, InstallArgs, InstallBlockReason, PackageRef, Report,
    VerifyResult,
};

pub struct ProjectFilesSnapshot {
    pub(crate) package_json: Option<Vec<u8>>,
    pub(crate) package_lock_json: Option<Vec<u8>>,
}

pub struct RestoreFileParams<'a> {
    pub current_working_directory: &'a Path,
    pub file_name: &'a str,
    pub original_contents: &'a Option<Vec<u8>>,
}

pub struct PrepareLockfileForInstallParams<'a> {
    pub current_working_directory: &'a Path,
    pub package_reference: &'a PackageRef,
    pub quiet: bool,
}

pub struct SaveCiReportParams<'a> {
    pub report: &'a Report,
    pub report_path: &'a Path,
    pub quiet: bool,
}

pub struct EnsureLockfileExistsParams<'a> {
    pub current_working_directory: &'a Path,
    pub quiet: bool,
}

pub struct CollectInstallPackagesParams<'a> {
    pub dependency_tree: &'a DependencyTree,
    pub package_reference: &'a PackageRef,
}

pub struct PreparedInstallState {
    pub package_ref: PackageRef,
    pub packages_to_verify: Vec<DependencyNode>,
    pub verifier: Arc<Verifier>,
    pub lockfile_entries: Arc<HashMap<String, LockfileEntry>>,
    pub lock_hash_before_verify: Option<String>,
    pub cycles: Vec<Vec<String>>,
}

pub struct PreparedCiState {
    pub packages_to_verify: Vec<DependencyNode>,
    pub verifier: Arc<Verifier>,
    pub lockfile_entries: Arc<HashMap<String, LockfileEntry>>,
    pub lock_hash_before_verify: Option<String>,
    pub cycles: Vec<Vec<String>>,
}

pub struct BlockedVerifyResults {
    pub compromised: Vec<VerifyResult>,
    pub unverifiable: Vec<VerifyResult>,
}

pub struct InstallExecutionOutcome {
    pub exit_code: ExitCode,
    pub should_restore_snapshot: bool,
}

impl InstallExecutionOutcome {
    pub fn success(should_restore_snapshot: bool) -> Self {
        Self {
            exit_code: ExitCode::SUCCESS,
            should_restore_snapshot,
        }
    }

    pub fn failure() -> Self {
        Self {
            exit_code: ExitCode::FAILURE,
            should_restore_snapshot: true,
        }
    }
}

pub struct ResolveInstallPolicyParams {
    pub compromised_count: usize,
    pub unverifiable_count: usize,
    pub allow_scripts: bool,
    pub no_scripts: bool,
}

pub struct PrintBlockReasonResultsParams<'a> {
    pub block_reason: InstallBlockReason,
    pub blocked: &'a BlockedVerifyResults,
}

pub struct FinalizeCiRunParams<'a> {
    pub args: &'a CiArgs,
    pub report: &'a Report,
    pub lock_hash_before_verify: &'a Option<String>,
}

pub struct FinalizeInstallRunParams<'a> {
    pub args: &'a InstallArgs,
    pub package_ref: &'a PackageRef,
    pub report: &'a Report,
    pub lock_hash_before_verify: &'a Option<String>,
}

pub struct RestoreProjectFilesSnapshotParams<'a> {
    pub snapshot: &'a ProjectFilesSnapshot,
    pub current_working_directory: &'a Path,
}

pub struct InstallPackageParams<'a> {
    pub current_working_directory: &'a Path,
    pub package_reference: &'a PackageRef,
    pub ignore_scripts: bool,
}

pub struct RunCleanInstallParams<'a> {
    pub current_working_directory: &'a Path,
    pub ignore_scripts: bool,
    pub omit_dev: bool,
    pub omit_optional: bool,
    pub silent_output: bool,
}

pub struct ResolvePackageIntoLockfileParams<'a> {
    pub current_working_directory: &'a Path,
    pub package_reference: &'a PackageRef,
}

pub struct PrintCiBlockingResultsParams<'a> {
    pub results: &'a [VerifyResult],
    pub args: &'a CiArgs,
}
