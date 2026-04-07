mod common;

use sentinel::types::{DependencyNode, DependencyTree, PackageRef};

#[test]
fn test_tree_insert_single_package() {
    let mut tree = DependencyTree::new();
    let pkg = PackageRef::new("express", "4.18.0");
    let node = DependencyNode {
        package: pkg.clone(),
        dependencies: vec![],
        is_dev: false,
    };
    tree.insert(node);

    assert_eq!(tree.nodes.len(), 1);
    assert!(tree.nodes.contains_key("express@4.18.0"));
}

#[test]
fn test_tree_build_simple_chain() {
    let mut tree = DependencyTree::new();

    tree.insert(DependencyNode {
        package: PackageRef::new("express", "4.18.0"),
        dependencies: vec!["body-parser@1.20.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("body-parser", "1.20.0"),
        dependencies: vec!["bytes@3.1.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("bytes", "3.1.0"),
        dependencies: vec![],
        is_dev: false,
    });

    assert_eq!(tree.nodes.len(), 3);
    let analysis = tree.analyze();
    assert_eq!(analysis.max_depth, 3);
}

#[test]
fn test_tree_transitive_dependencies() {
    let mut tree = DependencyTree::new();

    tree.insert(DependencyNode {
        package: PackageRef::new("app", "1.0.0"),
        dependencies: vec!["dep-a@1.0.0".to_string(), "dep-b@1.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("dep-a", "1.0.0"),
        dependencies: vec!["dep-c@1.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("dep-b", "1.0.0"),
        dependencies: vec!["dep-c@1.0.0".to_string(), "dep-d@1.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("dep-c", "1.0.0"),
        dependencies: vec![],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("dep-d", "1.0.0"),
        dependencies: vec![],
        is_dev: false,
    });

    assert_eq!(tree.nodes.len(), 5);

    let analysis = tree.analyze();
    assert_eq!(analysis.direct_packages.len(), 1);
    assert_eq!(analysis.transitive_packages.len(), 4);
    assert_eq!(analysis.cycles.len(), 0);
}

#[test]
fn test_tree_detect_simple_cycle() {
    let mut tree = DependencyTree::new();

    tree.insert(DependencyNode {
        package: PackageRef::new("dep-a", "1.0.0"),
        dependencies: vec!["dep-b@1.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("dep-b", "1.0.0"),
        dependencies: vec!["dep-a@1.0.0".to_string()],
        is_dev: false,
    });

    let cycles = tree.detect_cycles();
    assert!(!cycles.is_empty(), "Expected cycles to be detected");
}

#[test]
fn test_tree_detect_complex_cycle() {
    let mut tree = DependencyTree::new();

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
        dependencies: vec!["a@1.0.0".to_string()],
        is_dev: false,
    });

    let cycles = tree.detect_cycles();
    assert!(!cycles.is_empty(), "Expected cycles to be detected");
    assert!(cycles.iter().any(|cycle| cycle.len() == 3));
}

#[test]
fn test_tree_topological_sort() {
    let mut tree = DependencyTree::new();

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
        dependencies: vec![],
        is_dev: false,
    });

    let sorted = tree.topological_sort().expect("Should not have cycles");

    assert_eq!(sorted.len(), 3);
    let c_pos = sorted.iter().position(|s| s == "c@1.0.0").unwrap();
    let b_pos = sorted.iter().position(|s| s == "b@1.0.0").unwrap();
    let a_pos = sorted.iter().position(|s| s == "a@1.0.0").unwrap();

    assert!(
        c_pos < b_pos,
        "c should come before b. Got c_pos={}, b_pos={}",
        c_pos,
        b_pos
    );
    assert!(
        b_pos < a_pos,
        "b should come before a. Got b_pos={}, a_pos={}",
        b_pos,
        a_pos
    );
}

#[test]
fn test_tree_topological_sort_with_cycle() {
    let mut tree = DependencyTree::new();

    tree.insert(DependencyNode {
        package: PackageRef::new("a", "1.0.0"),
        dependencies: vec!["b@1.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("b", "1.0.0"),
        dependencies: vec!["a@1.0.0".to_string()],
        is_dev: false,
    });

    let result = tree.topological_sort();
    assert!(result.is_err(), "Should return error when cycle exists");
}

#[test]
fn test_tree_get_transitive_deps() {
    let mut tree = DependencyTree::new();

    tree.insert(DependencyNode {
        package: PackageRef::new("app", "1.0.0"),
        dependencies: vec!["a@1.0.0".to_string(), "d@1.0.0".to_string()],
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
        dependencies: vec![],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("d", "1.0.0"),
        dependencies: vec!["e@1.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("e", "1.0.0"),
        dependencies: vec![],
        is_dev: false,
    });

    let transitive = tree.get_transitive_deps(&PackageRef::new("a", "1.0.0"));
    assert!(transitive.contains("b@1.0.0"));
    assert!(transitive.contains("c@1.0.0"));
    assert!(!transitive.contains("d@1.0.0"));
    assert_eq!(transitive.len(), 2);
}

#[test]
fn test_tree_analysis_comprehensive() {
    let mut tree = DependencyTree::new();

    tree.insert(DependencyNode {
        package: PackageRef::new("app", "1.0.0"),
        dependencies: vec!["express@4.18.0".to_string(), "react@18.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("express", "4.18.0"),
        dependencies: vec!["body-parser@1.20.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("body-parser", "1.20.0"),
        dependencies: vec!["bytes@3.1.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("bytes", "3.1.0"),
        dependencies: vec![],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("react", "18.0.0"),
        dependencies: vec!["react-dom@18.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("react-dom", "18.0.0"),
        dependencies: vec![],
        is_dev: false,
    });

    let analysis = tree.analyze();

    assert_eq!(analysis.total_packages, 6);
    assert_eq!(analysis.direct_packages.len(), 1);
    assert_eq!(analysis.transitive_packages.len(), 5);
    assert_eq!(analysis.cycles.len(), 0);
    assert_eq!(analysis.orphaned.len(), 0);
    assert_eq!(analysis.max_depth, 4);
}

#[test]
fn test_tree_dev_dependencies() {
    let mut tree = DependencyTree::new();

    tree.insert(DependencyNode {
        package: PackageRef::new("app", "1.0.0"),
        dependencies: vec!["mocha@10.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("mocha", "10.0.0"),
        dependencies: vec!["should@13.0.0".to_string()],
        is_dev: true,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("should", "13.0.0"),
        dependencies: vec![],
        is_dev: true,
    });

    let analysis = tree.analyze();
    assert_eq!(analysis.total_packages, 3);

    let mocha = tree.nodes.get("mocha@10.0.0").unwrap();
    assert!(mocha.is_dev);
}

#[test]
fn test_tree_shared_transitive_deps() {
    let mut tree = DependencyTree::new();

    tree.insert(DependencyNode {
        package: PackageRef::new("app", "1.0.0"),
        dependencies: vec!["a@1.0.0".to_string(), "b@1.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("a", "1.0.0"),
        dependencies: vec!["shared@1.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("b", "1.0.0"),
        dependencies: vec!["shared@1.0.0".to_string()],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("shared", "1.0.0"),
        dependencies: vec![],
        is_dev: false,
    });

    assert_eq!(tree.nodes.len(), 4);

    let transitive_a = tree.get_transitive_deps(&PackageRef::new("a", "1.0.0"));
    let transitive_b = tree.get_transitive_deps(&PackageRef::new("b", "1.0.0"));

    assert_eq!(transitive_a.len(), 1);
    assert_eq!(transitive_b.len(), 1);
    assert_eq!(transitive_a, transitive_b);
}

#[test]
fn test_tree_multiple_versions_same_package() {
    let mut tree = DependencyTree::new();

    tree.insert(DependencyNode {
        package: PackageRef::new("webpack", "4.46.0"),
        dependencies: vec![],
        is_dev: false,
    });

    tree.insert(DependencyNode {
        package: PackageRef::new("webpack", "5.88.0"),
        dependencies: vec![],
        is_dev: false,
    });

    assert_eq!(tree.nodes.len(), 2);
    assert!(tree.nodes.contains_key("webpack@4.46.0"));
    assert!(tree.nodes.contains_key("webpack@5.88.0"));
}
