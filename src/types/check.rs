use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use indicatif::ProgressBar;

use crate::npm::LockfileEntry;
use crate::verifier::Verifier;

use super::{CheckArgs, DependencyNode};

pub struct PreparedCheckState {
    pub verifier: Arc<Verifier>,
    pub lockfile_entries: Arc<HashMap<String, LockfileEntry>>,
    pub packages_to_verify: Vec<DependencyNode>,
    pub cycles: Vec<Vec<String>>,
}

#[derive(Clone, Copy)]
pub struct BuildLockfileEntryParams<'a> {
    pub dependency_node: &'a DependencyNode,
    pub lockfile_entries: &'a HashMap<String, LockfileEntry>,
}

#[derive(Clone, Copy)]
pub struct CollectPackagesToVerifyParams<'a> {
    pub check_args: &'a CheckArgs,
    pub dependency_nodes: &'a HashMap<String, DependencyNode>,
}

#[derive(Clone, Copy)]
pub struct EnsureLockfileExistsForCheckParams<'a> {
    pub current_working_directory: &'a Path,
}

pub struct PrepareCheckStateParams<'a> {
    pub args: &'a CheckArgs,
    pub manager: super::manager::PackageManager,
}

#[derive(Clone, Copy)]
pub struct UpdateVerificationProgressParams<'a> {
    pub progress_bar: Option<&'a ProgressBar>,
    pub show_text_progress_fallback: bool,
    pub completed_counter: &'a AtomicUsize,
    pub total_packages: usize,
    pub progress_step: usize,
}

pub struct VerifyPackagesParams {
    pub packages_to_verify: Vec<DependencyNode>,
    pub verifier: Arc<Verifier>,
    pub lockfile_entries: Arc<HashMap<String, LockfileEntry>>,
}

pub struct VerifyPackagesExecutionParams {
    pub verify_packages_params: VerifyPackagesParams,
    pub max_concurrency: usize,
    pub progress_bar: Option<ProgressBar>,
    pub show_text_progress_fallback: bool,
}

pub struct VerifySinglePackageParams {
    pub node: DependencyNode,
    pub verifier: Arc<Verifier>,
    pub concurrency_gate: Arc<tokio::sync::Semaphore>,
    pub lockfile_entries: Arc<HashMap<String, LockfileEntry>>,
    pub progress_bar: Option<Arc<ProgressBar>>,
    pub completed_counter: Arc<AtomicUsize>,
    pub show_text_progress_fallback: bool,
    pub total_packages: usize,
    pub progress_step: usize,
}
