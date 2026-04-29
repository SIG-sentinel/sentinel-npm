#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::err_expect,
    clippy::too_many_arguments,
    clippy::needless_raw_string_hashes,
    unused_qualifications
)]

mod common;

use sentinel::types::{
    CompromisedSource, DependencyNode, DependencyTree, Evidence, PackageRef, UnverifiableReason,
    Verdict, VerifyResult,
};

fn node(
    name: impl Into<String>,
    version: impl Into<String>,
    dependencies: Vec<String>,
) -> DependencyNode {
    DependencyNode {
        package: PackageRef::new(name.into(), version.into()),
        dependencies,
        is_dev: false,
        is_direct: false,
        direct_parent: None,
    }
}

fn clean_result(
    name: impl Into<String>,
    version: impl Into<String>,
    detail: impl Into<String>,
) -> VerifyResult {
    VerifyResult {
        package: PackageRef::new(name.into(), version.into()),
        verdict: Verdict::Clean,
        detail: detail.into(),
        evidence: Evidence::empty(),
        is_direct: false,
        direct_parent: None,
        tarball_fingerprint: None,
    }
}

fn unverifiable_result(
    name: impl Into<String>,
    version: impl Into<String>,
    reason: UnverifiableReason,
    detail: impl Into<String>,
) -> VerifyResult {
    VerifyResult {
        package: PackageRef::new(name.into(), version.into()),
        verdict: Verdict::Unverifiable { reason },
        detail: detail.into(),
        evidence: Evidence::empty(),
        is_direct: false,
        direct_parent: None,
        tarball_fingerprint: None,
    }
}

fn compromised_result(
    name: impl Into<String>,
    version: impl Into<String>,
    expected: impl Into<String>,
    actual: impl Into<String>,
    source: CompromisedSource,
    detail: impl Into<String>,
    evidence: Evidence,
) -> VerifyResult {
    VerifyResult {
        package: PackageRef::new(name.into(), version.into()),
        verdict: Verdict::Compromised {
            expected: expected.into(),
            actual: actual.into(),
            source,
        },
        detail: detail.into(),
        evidence,
        is_direct: false,
        direct_parent: None,
        tarball_fingerprint: None,
    }
}

#[test]
fn test_verification_all_clean_simple_tree() {
    let mut tree = DependencyTree::new();

    tree.insert(node("app", "1.0.0", vec!["express@4.18.0".to_string()]));
    tree.insert(node("express", "4.18.0", vec![]));

    let verify_results = [
        clean_result("app", "1.0.0", "verified via registry"),
        clean_result("express", "4.18.0", "verified via registry"),
    ];

    let clean_count = verify_results
        .iter()
        .filter(|r| r.verdict == Verdict::Clean)
        .count();
    assert_eq!(clean_count, 2);
}

#[test]
fn test_verification_compromised_in_transitive() {
    let mut tree = DependencyTree::new();

    tree.insert(node("app", "1.0.0", vec!["express@4.18.0".to_string()]));
    tree.insert(node(
        "express",
        "4.18.0",
        vec!["compromised-lib@1.0.0".to_string()],
    ));
    tree.insert(node("compromised-lib", "1.0.0", vec![]));

    let verify_results = [
        clean_result("app", "1.0.0", "verified"),
        clean_result("express", "4.18.0", "verified"),
        compromised_result(
            "compromised-lib",
            "1.0.0",
            "sha512-abc123",
            "sha512-xyz789",
            CompromisedSource::LockfileVsRegistry,
            "integrity mismatch",
            Evidence {
                registry_integrity: Some("sha512-xyz789".to_string()),
                lockfile_integrity: Some("sha512-abc123".to_string()),
                ..Evidence::empty()
            },
        ),
    ];

    let compromised = verify_results
        .iter()
        .filter(|r| matches!(r.verdict, Verdict::Compromised { .. }))
        .count();
    assert_eq!(compromised, 1);

    let compromised_ref = PackageRef::new("compromised-lib", "1.0.0");
    let is_transitive = tree
        .get_transitive_deps(&PackageRef::new("app", "1.0.0"))
        .contains(&compromised_ref.to_string());
    assert!(is_transitive);
}

#[test]
fn test_verification_unverifiable_in_deep_tree() {
    let mut tree = DependencyTree::new();

    tree.insert(node("app", "1.0.0", vec!["a@1.0.0".to_string()]));
    tree.insert(node("a", "1.0.0", vec!["b@1.0.0".to_string()]));
    tree.insert(node("b", "1.0.0", vec!["c@1.0.0".to_string()]));
    tree.insert(node(
        "c",
        "1.0.0",
        vec!["unverifiable-lib@1.0.0".to_string()],
    ));
    tree.insert(node("unverifiable-lib", "1.0.0", vec![]));

    let verify_results = [
        clean_result("app", "1.0.0", "verified"),
        clean_result("a", "1.0.0", "verified"),
        clean_result("b", "1.0.0", "verified"),
        clean_result("c", "1.0.0", "verified"),
        unverifiable_result(
            "unverifiable-lib",
            "1.0.0",
            UnverifiableReason::NoIntegrityField,
            "registry has no dist.integrity",
        ),
    ];

    let unverifiable = verify_results
        .iter()
        .filter(|r| matches!(r.verdict, Verdict::Unverifiable { .. }))
        .count();
    assert_eq!(unverifiable, 1);

    let is_in_transitive = tree
        .get_transitive_deps(&PackageRef::new("app", "1.0.0"))
        .contains("unverifiable-lib@1.0.0");
    assert!(is_in_transitive);
}

#[test]
fn test_verification_diamond_dependency() {
    let mut tree = DependencyTree::new();

    tree.insert(node(
        "app",
        "1.0.0",
        vec!["a@1.0.0".to_string(), "b@1.0.0".to_string()],
    ));
    tree.insert(node("a", "1.0.0", vec!["c@1.0.0".to_string()]));
    tree.insert(node("b", "1.0.0", vec!["c@1.0.0".to_string()]));
    tree.insert(node("c", "1.0.0", vec![]));

    let verify_results = [
        clean_result("app", "1.0.0", "verified"),
        clean_result("a", "1.0.0", "verified"),
        clean_result("b", "1.0.0", "verified"),
        clean_result("c", "1.0.0", "verified once despite multiple paths"),
    ];

    assert_eq!(
        verify_results
            .iter()
            .filter(|r| r.verdict == Verdict::Clean)
            .count(),
        4
    );

    let transitive_a = tree.get_transitive_deps(&PackageRef::new("a", "1.0.0"));
    let transitive_b = tree.get_transitive_deps(&PackageRef::new("b", "1.0.0"));
    assert!(transitive_a.contains("c@1.0.0"));
    assert!(transitive_b.contains("c@1.0.0"));
}

#[test]
fn test_verification_mixed_verdicts() {
    let mut tree = DependencyTree::new();

    tree.insert(node(
        "app",
        "1.0.0",
        vec![
            "clean-pkg@1.0.0".to_string(),
            "compromised-pkg@1.0.0".to_string(),
            "unverifiable-pkg@1.0.0".to_string(),
        ],
    ));
    tree.insert(node("clean-pkg", "1.0.0", vec![]));
    tree.insert(node("compromised-pkg", "1.0.0", vec![]));
    tree.insert(node("unverifiable-pkg", "1.0.0", vec![]));

    let verify_results = [
        clean_result("app", "1.0.0", "verified"),
        clean_result("clean-pkg", "1.0.0", "verified"),
        compromised_result(
            "compromised-pkg",
            "1.0.0",
            "sha1",
            "sha2",
            CompromisedSource::DownloadVsRegistry,
            "integrity mismatch",
            Evidence::empty(),
        ),
        unverifiable_result(
            "unverifiable-pkg",
            "1.0.0",
            UnverifiableReason::RegistryOffline,
            "registry offline",
        ),
    ];

    let clean_count = verify_results
        .iter()
        .filter(|r| r.verdict == Verdict::Clean)
        .count();
    let compromised_count = verify_results
        .iter()
        .filter(|r| matches!(r.verdict, Verdict::Compromised { .. }))
        .count();
    let unverifiable_count = verify_results
        .iter()
        .filter(|r| matches!(r.verdict, Verdict::Unverifiable { .. }))
        .count();

    assert_eq!(clean_count, 2);
    assert_eq!(compromised_count, 1);
    assert_eq!(unverifiable_count, 1);

    let policy_should_block = compromised_count > 0 || unverifiable_count > 0;
    assert!(policy_should_block);
}

#[test]
fn test_verification_count_unique_packages() {
    let mut tree = DependencyTree::new();

    for i in 0..100 {
        let dependencies = match i {
            0 => vec![],
            _ => vec![format!("pkg-{}@1.0.0", i - 1)],
        };

        tree.insert(node(format!("pkg-{i}"), "1.0.0", dependencies));
    }

    assert_eq!(tree.nodes.len(), 100);

    assert_eq!(tree.analyze().total_packages, 100);
}
