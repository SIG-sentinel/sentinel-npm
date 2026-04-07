use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use indicatif::ProgressBar;

use crate::npm::LockfileEntry;
use crate::verifier::Verifier;

use super::{CheckArgs, DependencyNode};

pub struct PreparedCheckState {
    pub verifier: Arc<Verifier>,
    pub lockfile_entries: Arc<HashMap<String, LockfileEntry>>,
    pub packages_to_verify: Vec<DependencyNode>,
}

pub struct CollectPackagesToVerifyParams<'a> {
    pub check_args: &'a CheckArgs,
    pub dependency_nodes: &'a HashMap<String, DependencyNode>,
}

pub struct BuildLockfileEntryParams<'a> {
    pub dependency_node: &'a DependencyNode,
    pub lockfile_entries: &'a HashMap<String, LockfileEntry>,
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
}

pub struct EnsureLockfileExistsForCheckParams<'a> {
    pub current_working_directory: &'a Path,
    pub quiet: bool,
}
