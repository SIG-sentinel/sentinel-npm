use super::{cache_matches_lockfile, cache_requires_tarball_revalidation};
use crate::types::{
    ArtifactStore, CacheMatchParams, Evidence, LockfileEntry, PackageRef, Verdict, VerifyResult,
};
use crate::types::{UnverifiableReason, VerifierNewParams};
use crate::verifier::Verifier;

const INTEGRITY_SHA512_A: &str = "sha512-aaa";
const INTEGRITY_SHA512_B: &str = "sha512-bbb";
const INTEGRITY_SHA1_LEFT_PAD: &str = "sha1-W4o6d2Xf4AEmHd6RVYnngvjJTR4=";
const INTEGRITY_SHA512_TEST: &str = "sha512-test-integrity";
const PKG_TARBALL_URL: &str = "https://registry.npmjs.org/pkg/-/pkg-1.0.0.tgz";
const MAX_MEMORY_BYTES: usize = 512 * 1024 * 1024;
const TIMEOUT_MS_FAST: u64 = 1;
const TIMEOUT_MS_NORMAL: u64 = 5_000;

fn pkg_lockfile_entry(integrity: &str) -> LockfileEntry {
    LockfileEntry {
        package: PackageRef::new("pkg", "1.0.0"),
        integrity: Some(integrity.to_string()),
        is_dev: false,
        dependencies: Vec::new(),
    }
}

fn clean_cached_result(lockfile_integrity: &str) -> VerifyResult {
    VerifyResult {
        package: PackageRef::new("pkg", "1.0.0"),
        verdict: Verdict::Clean,
        detail: String::new(),
        evidence: Evidence {
            lockfile_integrity: Some(lockfile_integrity.to_string()),
            ..Evidence::empty()
        },
        is_direct: false,
        direct_parent: None,
        tarball_fingerprint: None,
    }
}

fn clean_cached_result_full(computed_sha512: Option<&str>) -> VerifyResult {
    VerifyResult {
        package: PackageRef::new("pkg", "1.0.0"),
        verdict: Verdict::Clean,
        detail: String::new(),
        evidence: Evidence {
            lockfile_integrity: Some(INTEGRITY_SHA512_A.to_string()),
            registry_integrity: Some(INTEGRITY_SHA512_A.to_string()),
            computed_sha512: computed_sha512.map(ToString::to_string),
            source_url: Some(PKG_TARBALL_URL.to_string()),
            ..Evidence::empty()
        },
        is_direct: false,
        direct_parent: None,
        tarball_fingerprint: None,
    }
}

fn left_pad_entry(integrity: &str) -> LockfileEntry {
    LockfileEntry {
        package: PackageRef::new("left-pad", "1.3.0"),
        integrity: Some(integrity.to_string()),
        is_dev: false,
        dependencies: Vec::new(),
    }
}

fn make_verifier(timeout_ms: u64, temp_dir: &std::path::Path) -> Verifier {
    let cache_dir = temp_dir
        .to_str()
        .expect("tempdir path should be valid utf-8");

    Verifier::new(VerifierNewParams {
        timeout_ms,
        current_working_directory: temp_dir,
        cache_dir: Some(cache_dir),
        artifact_store: ArtifactStore::Auto,
        max_memory_bytes: MAX_MEMORY_BYTES,
    })
    .expect("verifier should be created")
}

#[test]
fn test_cache_matches_lockfile_when_integrity_is_same() {
    assert!(cache_matches_lockfile(CacheMatchParams {
        entry: &pkg_lockfile_entry(INTEGRITY_SHA512_A),
        cached_result: &clean_cached_result(INTEGRITY_SHA512_A),
    }));
}

#[test]
fn test_cache_matches_lockfile_when_integrity_drifted() {
    assert!(!cache_matches_lockfile(CacheMatchParams {
        entry: &pkg_lockfile_entry(INTEGRITY_SHA512_B),
        cached_result: &clean_cached_result(INTEGRITY_SHA512_A),
    }));
}

#[test]
fn test_cache_requires_tarball_revalidation_for_clean_without_computed_sha512() {
    assert!(cache_requires_tarball_revalidation(
        &clean_cached_result_full(None)
    ));
}

#[test]
fn test_cache_does_not_require_tarball_revalidation_for_clean_with_computed_sha512() {
    assert!(!cache_requires_tarball_revalidation(
        &clean_cached_result_full(Some(INTEGRITY_SHA512_A))
    ));
}

#[tokio::test]
async fn test_check_from_lockfile_forced_tiny_timeout_returns_unverifiable_registry_issue() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let verifier = make_verifier(TIMEOUT_MS_FAST, temp_dir.path());
    let result = verifier
        .check_from_lockfile(&left_pad_entry(INTEGRITY_SHA512_TEST))
        .await;

    match result.verdict {
        Verdict::Unverifiable { reason } => {
            let is_expected_reason = matches!(
                reason,
                UnverifiableReason::RegistryOffline | UnverifiableReason::RegistryTimeout
            );
            assert!(
                is_expected_reason,
                "unexpected unverifiable reason: {reason:?}"
            );
        }
        _ => panic!("expected unverifiable verdict for forced tiny timeout"),
    }
}

#[tokio::test]
async fn test_check_from_lockfile_sha1_integrity_returns_unverifiable_legacy_sha1() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let verifier = make_verifier(TIMEOUT_MS_NORMAL, temp_dir.path());
    let result = verifier
        .check_from_lockfile(&left_pad_entry(INTEGRITY_SHA1_LEFT_PAD))
        .await;

    match result.verdict {
        Verdict::Unverifiable { reason } => {
            assert_eq!(reason, UnverifiableReason::LegacySha1Lockfile);
            assert!(result.detail.contains("sentinel ci"));
        }
        _ => panic!("expected unverifiable verdict for legacy sha1 lockfile"),
    }
}
