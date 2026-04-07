mod common;

use sha2::{Digest, Sha256};

use common::constants::{
    EMPTY_INPUT, INVALID_BASE64_INTEGRITY, ORIGINAL_TARBALL, SAMPLE_DATA, SHA256_EMPTY_HEX,
    SHA512_EMPTY_HEX, TAMPERED_TARBALL, TEST_TARBALL_CONTENT,
};
use common::fixtures::{hex_sha512, integrity_fixture, wrong_prefix_integrity};
use sentinel::crypto::verify_integrity;
use sentinel::types::VerifyIntegrityParams;

#[test]
fn sha256_empty_string() {
    let mut hasher = Sha256::new();
    hasher.update(EMPTY_INPUT);
    let digest = hasher.finalize();
    let hash_hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();

    assert_eq!(hash_hex, SHA256_EMPTY_HEX);
}

#[test]
fn sha512_empty_string() {
    let hash_hex = hex_sha512(EMPTY_INPUT);
    assert_eq!(hash_hex, SHA512_EMPTY_HEX);
}

#[test]
fn verify_integrity_correct() {
    let fixture = integrity_fixture(TEST_TARBALL_CONTENT);
    assert_eq!(
        verify_integrity(VerifyIntegrityParams {
            sha512_bytes: &fixture.bytes,
            integrity_field: &fixture.integrity,
        }),
        Ok(true)
    );
}

#[test]
fn verify_integrity_tampered() {
    let original_fixture = integrity_fixture(ORIGINAL_TARBALL);
    let tampered_fixture = integrity_fixture(TAMPERED_TARBALL);

    assert_eq!(
        verify_integrity(VerifyIntegrityParams {
            sha512_bytes: &tampered_fixture.bytes,
            integrity_field: &original_fixture.integrity,
        }),
        Ok(false)
    );
}

#[test]
fn verify_integrity_wrong_prefix() {
    let fixture = wrong_prefix_integrity(SAMPLE_DATA);
    assert!(
        verify_integrity(VerifyIntegrityParams {
            sha512_bytes: &fixture.bytes,
            integrity_field: &fixture.integrity,
        })
        .is_err()
    );
}

#[test]
fn verify_integrity_invalid_base64() {
    let fixture = integrity_fixture(SAMPLE_DATA);
    assert!(
        verify_integrity(VerifyIntegrityParams {
            sha512_bytes: &fixture.bytes,
            integrity_field: INVALID_BASE64_INTEGRITY,
        })
        .is_err()
    );
}

#[test]
fn verify_integrity_empty_field() {
    let fixture = integrity_fixture(SAMPLE_DATA);
    assert!(
        verify_integrity(VerifyIntegrityParams {
            sha512_bytes: &fixture.bytes,
            integrity_field: "",
        })
        .is_err()
    );
}
