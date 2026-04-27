use std::collections::HashMap;

use super::build_lockfile_entry;
use crate::types::{BuildLockfileEntryParams, DependencyNode, LockfileEntry, PackageRef};

#[test]
fn build_lockfile_entry_reuses_integrity_when_entry_exists() {
    let node = DependencyNode {
        package: PackageRef::new("left-pad", "1.0.0"),
        dependencies: vec!["lodash@4.17.21".to_string()],
        is_dev: false,
        is_direct: true,
        direct_parent: None,
    };

    let mut lockfile_entries = HashMap::new();
    lockfile_entries.insert(
        node.package.to_string(),
        LockfileEntry {
            package: node.package.clone(),
            integrity: Some("sha512-abc".to_string()),
            is_dev: true,
            dependencies: vec![],
        },
    );

    let entry = build_lockfile_entry(BuildLockfileEntryParams {
        dependency_node: &node,
        lockfile_entries: &lockfile_entries,
    });

    assert_eq!(entry.package.to_string(), "left-pad@1.0.0");
    assert_eq!(entry.integrity.as_deref(), Some("sha512-abc"));
    assert_eq!(entry.is_dev, node.is_dev);
    assert_eq!(entry.dependencies, node.dependencies);
}

#[test]
fn build_lockfile_entry_keeps_none_integrity_when_not_found() {
    let node = DependencyNode {
        package: PackageRef::new("chalk", "5.3.0"),
        dependencies: vec![],
        is_dev: true,
        is_direct: false,
        direct_parent: Some("eslint@9.0.0".to_string()),
    };

    let lockfile_entries: HashMap<String, LockfileEntry> = HashMap::new();

    let entry = build_lockfile_entry(BuildLockfileEntryParams {
        dependency_node: &node,
        lockfile_entries: &lockfile_entries,
    });

    assert_eq!(entry.package.to_string(), "chalk@5.3.0");
    assert_eq!(entry.integrity, None);
    assert_eq!(entry.is_dev, node.is_dev);
    assert!(entry.dependencies.is_empty());
}
