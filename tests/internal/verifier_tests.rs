use super::cache_matches_lockfile;
use crate::types::{CacheMatchParams, Evidence, LockfileEntry, PackageRef, Verdict, VerifyResult};
use crate::types::{UnverifiableReason, VerifierNewParams};
use crate::verifier::Verifier;

#[test]
fn test_cache_matches_lockfile_when_integrity_is_same() {
    let lockfile_entry = LockfileEntry {
        package: PackageRef::new("pkg", "1.0.0"),
        integrity: Some("sha512-aaa".to_string()),
        is_dev: false,
    };

    let cached_result = VerifyResult {
        package: PackageRef::new("pkg", "1.0.0"),
        verdict: Verdict::Clean,
        detail: String::new(),
        evidence: Evidence {
            lockfile_integrity: Some("sha512-aaa".to_string()),
            ..Evidence::empty()
        },
    };

    assert!(cache_matches_lockfile(CacheMatchParams {
        entry: &lockfile_entry,
        cached_result: &cached_result,
    }));
}

#[test]
fn test_cache_matches_lockfile_when_integrity_drifted() {
    let lockfile_entry = LockfileEntry {
        package: PackageRef::new("pkg", "1.0.0"),
        integrity: Some("sha512-bbb".to_string()),
        is_dev: false,
    };

    let cached_result = VerifyResult {
        package: PackageRef::new("pkg", "1.0.0"),
        verdict: Verdict::Clean,
        detail: String::new(),
        evidence: Evidence {
            lockfile_integrity: Some("sha512-aaa".to_string()),
            ..Evidence::empty()
        },
    };

    assert!(!cache_matches_lockfile(CacheMatchParams {
        entry: &lockfile_entry,
        cached_result: &cached_result,
    }));
}

#[tokio::test]
async fn test_check_from_lockfile_forced_tiny_timeout_returns_unverifiable_registry_issue() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let cache_dir = temp_dir
        .path()
        .to_str()
        .expect("tempdir path should be valid utf-8");

    let verifier = Verifier::new(VerifierNewParams {
        timeout_ms: 1,
        cache_dir: Some(cache_dir),
    })
    .expect("verifier should be created");

    let entry = LockfileEntry {
        package: PackageRef::new("left-pad", "1.3.0"),
        integrity: Some("sha512-test-integrity".to_string()),
        is_dev: false,
    };

    let result = verifier.check_from_lockfile(&entry).await;

    match result.verdict {
        Verdict::Unverifiable { reason } => {
            let is_expected_reason = matches!(
                reason,
                UnverifiableReason::RegistryOffline | UnverifiableReason::RegistryTimeout
            );
            assert!(is_expected_reason, "unexpected unverifiable reason: {reason:?}");
        }
        _ => panic!("expected unverifiable verdict for forced tiny timeout"),
    }
}
