use super::*;

#[test]
fn test_dependency_tree_empty() {
    let tree = DependencyTree::new();

    assert_eq!(tree.nodes.len(), 0);
}

#[test]
fn test_dependency_tree_insert() {
    let mut tree = DependencyTree::new();
    let package_reference = PackageRef::new("foo", "1.0.0");
    let dependency_node = DependencyNode {
        package: package_reference.clone(),
        dependencies: vec![],
        is_dev: false,
        is_direct: false,
        direct_parent: None,
    };

    tree.insert(dependency_node);

    assert_eq!(tree.nodes.len(), 1);
    assert!(tree.nodes.contains_key(&package_reference.to_string()));
}
