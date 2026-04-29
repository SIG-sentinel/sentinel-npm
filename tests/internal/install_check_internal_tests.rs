use super::resolve_stream_storage_mode;
use crate::types::{ArtifactStore, StreamStorageMode, VerifierNewParams};
use crate::verifier::Verifier;

#[test]
fn auto_mode_falls_back_to_spool_under_memory_pressure() {
    let verifier_new_params = VerifierNewParams {
        timeout_ms: 500,
        registry_max_in_flight: None,
        current_working_directory: std::path::Path::new("."),
        cache_dir: None,
        artifact_store: ArtifactStore::Auto,
        max_memory_bytes: 1,
    };
    let verifier = Verifier::new(verifier_new_params).expect("verifier should be created");

    verifier.memory_budget.record_buffer(1);
    let StreamStorageMode {
        effective_mode,
        capture_buffer,
        spool_to_disk,
    } = resolve_stream_storage_mode(&verifier);

    assert_eq!(effective_mode, ArtifactStore::Spool);
    assert!(!capture_buffer);
    assert!(spool_to_disk);

    verifier.memory_budget.release_buffer(1);
}
