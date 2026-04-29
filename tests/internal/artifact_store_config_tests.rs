use super::{get, init};
use crate::types::ArtifactStore;

#[test]
fn get_returns_auto_when_not_initialized() {
    let current = get();
    let is_auto = matches!(current, ArtifactStore::Auto);

    assert!(is_auto);
}

#[test]
fn get_returns_valid_store_variant() {
    let current = get();
    let is_known_variant = matches!(
        current,
        ArtifactStore::Memory | ArtifactStore::Spool | ArtifactStore::Auto
    );

    assert!(is_known_variant);
}

#[test]
fn init_twice_returns_error() {
    let first_result = init(ArtifactStore::Auto);
    let second_result = init(ArtifactStore::Memory);
    let second_is_err = second_result.is_err();

    assert!(
        second_is_err,
        "expected second init to fail, but first_result was: {first_result:?}",
    );
}
