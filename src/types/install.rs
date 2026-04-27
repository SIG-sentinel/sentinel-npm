use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{ExitCode, Output};
use std::sync::Arc;

use crate::ecosystem::PackageManager;

use crate::npm::LockfileEntry;
use crate::verifier::Verifier;

use super::{
    CiArgs, DependencyNode, DependencyTree, InstallArgs, InstallBlockReason, OutputFormat,
    PackageRef, Report, VerifiedTarball, VerifyResult,
};

pub struct ProjectFilesSnapshot {
    pub(crate) package_json: Option<Vec<u8>>,
    pub(crate) lockfile_name: String,
    pub(crate) lockfile_contents: Option<Vec<u8>>,
}

#[derive(Clone, Copy)]
pub struct RestoreFileParams<'a> {
    pub current_working_directory: &'a Path,
    pub file_name: &'a str,
    pub original_contents: &'a Option<Vec<u8>>,
}

#[derive(Clone, Copy)]
pub struct PrepareLockfileForInstallParams<'a> {
    pub current_working_directory: &'a Path,
    pub package_reference: &'a PackageRef,
    pub quiet: bool,
}

#[derive(Clone, Copy)]
pub struct SaveCiReportParams<'a> {
    pub report: &'a Report,
    pub report_path: &'a Path,
    pub quiet: bool,
    pub is_text_output: bool,
}

pub struct LockfileGenerationResult {
    pub output: Output,
    pub manager: PackageManager,
}

pub type InstallFingerprintResult<T> = Result<T, String>;

pub enum LockfileFailureKind {
    DependencyConflict,
    CommandNotFound,
    NetworkError,
    Unknown,
}

pub struct EnsureLockfileExistsParams<'a> {
    pub current_working_directory: &'a Path,
    pub quiet: bool,
}

#[derive(Clone, Copy)]
pub struct PrintAndSaveCiReportParams<'a> {
    pub args: &'a CiArgs,
    pub report: &'a Report,
    pub is_text_output: bool,
}

#[derive(Clone, Copy)]
pub struct WarnPostVerifyLargeScopeParams<'a> {
    pub command_name: &'a str,
    pub package_count: usize,
    pub should_warn: bool,
}

#[derive(Clone, Copy)]
pub struct WarnPostVerifyElapsedParams<'a> {
    pub command_name: &'a str,
    pub package_count: usize,
    pub elapsed: std::time::Duration,
    pub should_warn: bool,
}

#[derive(Clone, Copy)]
pub struct SyncLockfileWithPackageJsonParams<'a> {
    pub current_working_directory: &'a Path,
    pub manager: PackageManager,
    pub lockfile_path: &'a Path,
    pub quiet: bool,
    pub is_text_output: bool,
}

#[derive(Clone, Copy)]
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

pub struct ExecuteVerificationRunParams<'a> {
    pub output_format: &'a OutputFormat,
    pub quiet: bool,
    pub packages_to_verify: Vec<DependencyNode>,
    pub verifier: Arc<Verifier>,
    pub lockfile_entries: Arc<HashMap<String, LockfileEntry>>,
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

#[derive(Clone, Copy)]
pub struct ResolveInstallPolicyParams {
    pub compromised_count: usize,
    pub unverifiable_count: usize,
    pub allow_scripts: bool,
    pub post_verify: bool,
}

#[derive(Clone, Copy)]
pub struct PrintBlockReasonResultsParams<'a> {
    pub block_reason: InstallBlockReason,
    pub blocked: &'a BlockedVerifyResults,
}

#[derive(Clone, Copy)]
pub struct FinalizeCiRunParams<'a> {
    pub args: &'a CiArgs,
    pub report: &'a Report,
    pub lock_hash_before_verify: &'a Option<String>,
}

#[derive(Clone, Copy)]
pub struct FinalizeCiDryRunParams<'a> {
    pub args: &'a CiArgs,
    pub report: &'a Report,
    pub is_text_output: bool,
}

#[derive(Clone, Copy)]
pub struct RunCleanInstallOrFailureParams<'a> {
    pub args: &'a CiArgs,
    pub ignore_scripts: bool,
}

#[derive(Clone, Copy)]
pub struct RunCiPostVerifyParams<'a> {
    pub args: &'a CiArgs,
    pub report: &'a Report,
    pub is_text_output: bool,
}

#[derive(Clone, Copy)]
pub struct CompleteSuccessfulCiRunParams<'a> {
    pub args: &'a CiArgs,
    pub report: &'a Report,
    pub lock_hash_before_verify: &'a Option<String>,
    pub is_text_output: bool,
}

pub struct FinalizeInstallRunParams<'a> {
    pub args: &'a InstallArgs,
    pub package_ref: &'a PackageRef,
    pub report: &'a Report,
    pub lock_hash_before_verify: &'a Option<String>,
    pub prevalidated_tarball: Option<VerifiedTarball>,
}

#[derive(Clone, Copy)]
pub struct PrintInstallReportParams<'a> {
    pub args: &'a InstallArgs,
    pub report: &'a Report,
}

#[derive(Clone, Copy)]
pub struct FinalizeInstallDryRunParams<'a> {
    pub args: &'a InstallArgs,
    pub report: &'a Report,
    pub is_text_output: bool,
}

pub struct InstallFromVerifiedSourceOrFailureParams<'a> {
    pub args: &'a InstallArgs,
    pub package_ref: &'a PackageRef,
    pub ignore_scripts: bool,
    pub prevalidated_tarball: Option<VerifiedTarball>,
}

#[derive(Clone, Copy)]
pub struct CompleteSuccessfulInstallParams<'a> {
    pub args: &'a InstallArgs,
    pub package_ref: &'a PackageRef,
    pub report: &'a Report,
    pub lock_hash_before_verify: &'a Option<String>,
    pub is_text_output: bool,
}

#[derive(Clone, Copy)]
pub struct RestoreProjectFilesSnapshotParams<'a> {
    pub snapshot: &'a ProjectFilesSnapshot,
    pub current_working_directory: &'a Path,
}

#[derive(Clone, Copy)]
pub struct InstallPackageParams<'a> {
    pub current_working_directory: &'a Path,
    pub package_reference: &'a PackageRef,
    pub ignore_scripts: bool,
}

#[derive(Clone, Copy)]
pub struct InstallPackageSourceParams<'a> {
    pub current_working_directory: &'a Path,
    pub package_reference: &'a PackageRef,
    pub package_source: &'a str,
    pub ignore_scripts: bool,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy)]
pub struct RunCleanInstallParams<'a> {
    pub current_working_directory: &'a Path,
    pub ignore_scripts: bool,
    pub omit_dev: bool,
    pub omit_optional: bool,
    pub silent_output: bool,
}

#[allow(clippy::struct_excessive_bools)]
pub struct CleanInstallPlanParams {
    pub ignore_scripts: bool,
    pub omit_dev: bool,
    pub omit_optional: bool,
    pub silent_output: bool,
}

#[derive(Clone, Copy)]
pub struct ResolvePackageIntoLockfileParams<'a> {
    pub current_working_directory: &'a Path,
    pub package_reference: &'a PackageRef,
}

#[derive(Clone, Copy)]
pub struct PrintCiBlockingResultsParams<'a> {
    pub results: &'a [VerifyResult],
    pub args: &'a CiArgs,
}

#[derive(Clone, Copy)]
pub struct ShouldPrintReportParams<'a> {
    pub output_format: &'a OutputFormat,
    pub quiet: bool,
}

pub struct PrepareInstallStateParams<'a> {
    pub args: &'a InstallArgs,
    pub manager: PackageManager,
}

pub struct PrepareCiStateParams<'a> {
    pub args: &'a CiArgs,
    pub manager: PackageManager,
}

#[derive(Clone, Copy)]
pub struct AnalyzeDependencyCyclesParams<'a> {
    pub dependency_tree: &'a DependencyTree,
    pub quiet: bool,
    pub is_text_output: bool,
}

pub enum VersionSpecKind<'a> {
    Unspecified,
    Tag,
    Exact(&'a str),
    Range(&'a str),
}

pub struct InstallPackageRequest {
    pub package_name: String,
    pub version_spec: Option<String>,
}

#[derive(Clone, Copy)]
pub struct AppendCiHistoryParams<'a> {
    pub args: &'a CiArgs,
    pub report: &'a Report,
    pub lock_hash_before_verify: &'a Option<String>,
}

pub struct InstallFromVerifiedSourceParams<'a> {
    pub args: &'a InstallArgs,
    pub package_ref: &'a PackageRef,
    pub ignore_scripts: bool,
    pub prevalidated_tarball: Option<VerifiedTarball>,
}

pub struct ProcessDirectoryEntryParams<'a> {
    pub entry_path: PathBuf,
    pub file_type: &'a std::fs::FileType,
    pub entries: &'a mut Vec<PathBuf>,
    pub pending_paths: &'a mut Vec<PathBuf>,
}

pub struct ProcessInstalledPackageIndexEntryParams<'a> {
    pub path: &'a Path,
    pub entry_result: Result<std::fs::DirEntry, std::io::Error>,
    pub index: &'a mut HashMap<String, PathBuf>,
    pub pending_paths: &'a mut Vec<PathBuf>,
}

#[derive(Clone, Copy)]
pub struct FindContentMismatchPostVerifyPackagesParams<'a> {
    pub installed_package_index: &'a HashMap<String, PathBuf>,
    pub packages: &'a [PackageRef],
    pub verifier: &'a Verifier,
    pub cached_fingerprints: &'a HashMap<String, String>,
}

#[derive(Clone, Copy)]
pub struct CheckContentFingerprintMismatchesParams<'a> {
    pub current_working_directory: &'a Path,
    pub timeout_ms: u64,
    pub installed_package_index: &'a HashMap<String, PathBuf>,
    pub packages: &'a [PackageRef],
    pub verify_results: &'a [VerifyResult],
    pub command_name: &'a str,
}

#[derive(Clone, Copy)]
pub struct RunPostVerifyForPackagesParams<'a> {
    pub current_working_directory: &'a Path,
    pub timeout_ms: u64,
    pub quiet: bool,
    pub is_text_output: bool,
    pub command_name: &'a str,
    pub packages: &'a [PackageRef],
    pub verify_results: &'a [VerifyResult],
}

#[derive(Clone, Copy)]
pub struct AppendInstallHistoryParams<'a> {
    pub args: &'a InstallArgs,
    pub package_ref: &'a PackageRef,
    pub lock_hash_before_verify: &'a Option<String>,
}

#[derive(Clone, Copy)]
pub struct ResolveInstallTargetsParams<'a> {
    pub args: &'a InstallArgs,
    pub dependency_tree: &'a DependencyTree,
    pub install_request: &'a InstallPackageRequest,
    pub requested_package_ref: &'a PackageRef,
    pub is_text_output: bool,
}

#[derive(Clone, Copy)]
pub struct EnsureLockfileExistsForInstallParams<'a> {
    pub current_working_directory: &'a Path,
}
