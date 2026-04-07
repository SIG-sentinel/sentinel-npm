use super::cache_matches_lockfile;
use crate::types::{CacheMatchParams, Evidence, LockfileEntry, PackageRef, Verdict, VerifyResult};

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
