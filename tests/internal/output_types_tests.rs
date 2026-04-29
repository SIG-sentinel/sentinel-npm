use super::ArtifactStore;

#[test]
fn artifact_store_as_env_value_roundtrip() {
    let cases = [
        ArtifactStore::Memory,
        ArtifactStore::Spool,
        ArtifactStore::Auto,
    ];

    for case in cases {
        let env_value = case.as_env_value();
        let parsed = ArtifactStore::from_env_value(env_value);
        assert_eq!(parsed, Some(case));
    }
}

#[test]
fn artifact_store_from_env_value_rejects_unknown_values() {
    assert_eq!(ArtifactStore::from_env_value("invalid"), None);
}
