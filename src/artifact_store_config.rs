use crate::types::ArtifactStore;
use std::sync::OnceLock;

static ARTIFACT_STORE: OnceLock<ArtifactStore> = OnceLock::new();
const ARTIFACT_STORE_ALREADY_INITIALIZED_ERROR: &str =
    "artifact_store already initialized (init must be called exactly once in main)";

pub fn init(store: ArtifactStore) -> Result<(), &'static str> {
    ARTIFACT_STORE
        .set(store)
        .map_err(|_| ARTIFACT_STORE_ALREADY_INITIALIZED_ERROR)
}

pub fn get() -> ArtifactStore {
    ARTIFACT_STORE.get().copied().unwrap_or(ArtifactStore::Auto)
}

#[cfg(test)]
#[path = "../tests/internal/artifact_store_config_tests.rs"]
mod tests;
