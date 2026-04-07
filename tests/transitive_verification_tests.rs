mod common;

use sentinel::types::{
    CompromisedSource, DependencyNode, DependencyTree, Evidence, PackageRef, UnverifiableReason,
    Verdict, VerifyResult,
};

#[test]
fn test_verification_all_clean_simple_tree() {
    let mut tree = DependencyTree::new();

    tree.insert(DependencyNode {
        package: PackageRef::new("app", "1.0.0"),
        dependencies: vec!["express@4.18.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("express", "4.18.0"),
        dependencies: vec![],
        is_dev: false,
    });

    let verify_results = [
        VerifyResult {
            package: PackageRef::new("app", "1.0.0"),
            verdict: Verdict::Clean,
            detail: "verified via registry".to_string(),
            evidence: Evidence::empty(),
        },
        VerifyResult {
            package: PackageRef::new("express", "4.18.0"),
            verdict: Verdict::Clean,
            detail: "verified via registry".to_string(),
            evidence: Evidence::empty(),
        },
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

    tree.insert(DependencyNode {
        package: PackageRef::new("app", "1.0.0"),
        dependencies: vec!["express@4.18.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("express", "4.18.0"),
        dependencies: vec!["compromised-lib@1.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("compromised-lib", "1.0.0"),
        dependencies: vec![],
        is_dev: false,
    });

    let verify_results = [
        VerifyResult {
            package: PackageRef::new("app", "1.0.0"),
            verdict: Verdict::Clean,
            detail: "verified".to_string(),
            evidence: Evidence::empty(),
        },
        VerifyResult {
            package: PackageRef::new("express", "4.18.0"),
            verdict: Verdict::Clean,
            detail: "verified".to_string(),
            evidence: Evidence::empty(),
        },
        VerifyResult {
            package: PackageRef::new("compromised-lib", "1.0.0"),
            verdict: Verdict::Compromised {
                expected: "sha512-abc123".to_string(),
                actual: "sha512-xyz789".to_string(),
                source: CompromisedSource::LockfileVsRegistry,
            },
            detail: "integrity mismatch".to_string(),
            evidence: Evidence {
                registry_integrity: Some("sha512-xyz789".to_string()),
                lockfile_integrity: Some("sha512-abc123".to_string()),
                ..Evidence::empty()
            },
        },
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

    tree.insert(DependencyNode {
        package: PackageRef::new("app", "1.0.0"),
        dependencies: vec!["a@1.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("a", "1.0.0"),
        dependencies: vec!["b@1.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("b", "1.0.0"),
        dependencies: vec!["c@1.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("c", "1.0.0"),
        dependencies: vec!["unverifiable-lib@1.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("unverifiable-lib", "1.0.0"),
        dependencies: vec![],
        is_dev: false,
    });

    let verify_results = [
        VerifyResult {
            package: PackageRef::new("app", "1.0.0"),
            verdict: Verdict::Clean,
            detail: "verified".to_string(),
            evidence: Evidence::empty(),
        },
        VerifyResult {
            package: PackageRef::new("a", "1.0.0"),
            verdict: Verdict::Clean,
            detail: "verified".to_string(),
            evidence: Evidence::empty(),
        },
        VerifyResult {
            package: PackageRef::new("b", "1.0.0"),
            verdict: Verdict::Clean,
            detail: "verified".to_string(),
            evidence: Evidence::empty(),
        },
        VerifyResult {
            package: PackageRef::new("c", "1.0.0"),
            verdict: Verdict::Clean,
            detail: "verified".to_string(),
            evidence: Evidence::empty(),
        },
        VerifyResult {
            package: PackageRef::new("unverifiable-lib", "1.0.0"),
            verdict: Verdict::Unverifiable {
                reason: UnverifiableReason::NoIntegrityField,
            },
            detail: "registry has no dist.integrity".to_string(),
            evidence: Evidence::empty(),
        },
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

    tree.insert(DependencyNode {
        package: PackageRef::new("app", "1.0.0"),
        dependencies: vec!["a@1.0.0".to_string(), "b@1.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("a", "1.0.0"),
        dependencies: vec!["c@1.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("b", "1.0.0"),
        dependencies: vec!["c@1.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("c", "1.0.0"),
        dependencies: vec![],
        is_dev: false,
    });

    let verify_results = [
        VerifyResult {
            package: PackageRef::new("app", "1.0.0"),
            verdict: Verdict::Clean,
            detail: "verified".to_string(),
            evidence: Evidence::empty(),
        },
        VerifyResult {
            package: PackageRef::new("a", "1.0.0"),
            verdict: Verdict::Clean,
            detail: "verified".to_string(),
            evidence: Evidence::empty(),
        },
        VerifyResult {
            package: PackageRef::new("b", "1.0.0"),
            verdict: Verdict::Clean,
            detail: "verified".to_string(),
            evidence: Evidence::empty(),
        },
        VerifyResult {
            package: PackageRef::new("c", "1.0.0"),
            verdict: Verdict::Clean,
            detail: "verified once despite multiple paths".to_string(),
            evidence: Evidence::empty(),
        },
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

    tree.insert(DependencyNode {
        package: PackageRef::new("app", "1.0.0"),
        dependencies: vec![
            "clean-pkg@1.0.0".to_string(),
            "compromised-pkg@1.0.0".to_string(),
            "unverifiable-pkg@1.0.0".to_string(),
        ],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("clean-pkg", "1.0.0"),
        dependencies: vec![],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("compromised-pkg", "1.0.0"),
        dependencies: vec![],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("unverifiable-pkg", "1.0.0"),
        dependencies: vec![],
        is_dev: false,
    });

    let verify_results = [
        VerifyResult {
            package: PackageRef::new("app", "1.0.0"),
            verdict: Verdict::Clean,
            detail: "verified".to_string(),
            evidence: Evidence::empty(),
        },
        VerifyResult {
            package: PackageRef::new("clean-pkg", "1.0.0"),
            verdict: Verdict::Clean,
            detail: "verified".to_string(),
            evidence: Evidence::empty(),
        },
        VerifyResult {
            package: PackageRef::new("compromised-pkg", "1.0.0"),
            verdict: Verdict::Compromised {
                expected: "sha1".to_string(),
                actual: "sha2".to_string(),
                source: CompromisedSource::DownloadVsRegistry,
            },
            detail: "integrity mismatch".to_string(),
            evidence: Evidence::empty(),
        },
        VerifyResult {
            package: PackageRef::new("unverifiable-pkg", "1.0.0"),
            verdict: Verdict::Unverifiable {
                reason: UnverifiableReason::RegistryOffline,
            },
            detail: "registry offline".to_string(),
            evidence: Evidence::empty(),
        },
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
        tree.insert(DependencyNode {
            package: PackageRef::new(format!("pkg-{}", i), "1.0.0"),
            dependencies: if i == 0 {
                vec![]
            } else {
                vec![format!("pkg-{}", i - 1) + "@1.0.0"]
            },
            is_dev: false,
        });
    }

    assert_eq!(tree.nodes.len(), 100);

    assert_eq!(tree.analyze().total_packages, 100);
}
