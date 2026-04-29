use std::collections::HashMap;
use std::sync::Arc;

use crate::types::{DependencyTree, LockfileEntry, SentinelError};
use crate::verifier::Verifier;

pub struct SharedCommandState {
    pub dependency_tree: DependencyTree,
    pub lockfile_entries: Arc<HashMap<String, LockfileEntry>>,
    pub verifier: Arc<Verifier>,
}

pub enum SharedCommandStateError {
    DependencyTree(SentinelError),
    LockfileEntries(SentinelError),
    Verifier(SentinelError),
}
