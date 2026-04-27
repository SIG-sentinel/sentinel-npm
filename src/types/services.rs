use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::types::ArtifactStore;
use crate::types::MemoryBudgetTracker;

pub struct Verifier {
    pub(crate) registry: NpmRegistry,
    pub(crate) cache: LocalCache,
    pub(crate) artifact_store: ArtifactStore,
    pub(crate) memory_budget: MemoryBudgetTracker,
}

pub struct NpmRegistry {
    pub(crate) client: reqwest::Client,
    pub(crate) timeout: Duration,
    pub(crate) default_registry_base: String,
    pub(crate) scoped_registry_bases: HashMap<String, String>,
    pub(crate) auth_token_prefixes: Vec<(String, String)>,
}

pub struct LocalCache {
    pub(crate) db_path: PathBuf,
}

#[derive(Clone, Copy)]
pub struct ProgressBarConfig {
    pub length: usize,
    pub message: &'static str,
    pub template: &'static str,
}
