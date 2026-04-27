use std::fs;

use tempfile::tempdir;

use super::*;

#[test]
fn yarn_lock_parser_collects_dependency_edges() {
    let yarn_lock = r#""react@^18.2.0":
  version "18.2.0"
  dependencies:
    loose-envify "^1.1.0"

"loose-envify@^1.1.0":
  version "1.4.0"
  integrity "sha512-loose"
"#;

    let entries = parse_yarn_lock_entries(yarn_lock);
    let react_entry = entries
        .get("react@18.2.0")
        .expect("react entry should exist");

    assert_eq!(react_entry.dependencies, vec!["loose-envify@1.4.0"]);
}

#[test]
fn pnpm_lock_parser_collects_dependency_edges() {
    let pnpm_lock = r"lockfileVersion: '9.0'
packages:
  /react@18.2.0:
    resolution:
      integrity: sha512-react
    dependencies:
      loose-envify: 1.4.0
  /loose-envify@1.4.0:
    resolution:
      integrity: sha512-loose
    dev: false
";

    let entries = parse_pnpm_lock_entries(pnpm_lock).expect("pnpm entries should parse");
    let react_entry = entries
        .get("react@18.2.0")
        .expect("react entry should exist");

    assert_eq!(react_entry.dependencies, vec!["loose-envify@1.4.0"]);
}

#[test]
fn pnpm_lock_parser_allows_empty_lockfile_without_packages_map() {
    let pnpm_lock = r"lockfileVersion: '9.0'
settings:
  autoInstallPeers: true
";

    let entries = parse_pnpm_lock_entries(pnpm_lock).expect("empty pnpm lockfile should parse");

    assert!(entries.is_empty());
}

#[test]
fn build_dependency_tree_for_yarn_sets_direct_parent_from_edges() {
    let temp = tempdir().expect("tempdir should be created");

    fs::write(
        temp.path().join("package.json"),
        r#"{
  "name": "demo",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.2.0"
  }
}"#,
    )
    .expect("package.json should be written");

    fs::write(
        temp.path().join("yarn.lock"),
        r#""react@^18.2.0":
  version "18.2.0"
  dependencies:
    loose-envify "^1.1.0"

"loose-envify@^1.1.0":
  version "1.4.0"
  integrity "sha512-loose"
"#,
    )
    .expect("yarn.lock should be written");

    let entries = read_lockfile_entries(temp.path()).expect("entries should parse");
    let tree = build_dependency_tree_for_manager(temp.path(), &entries).expect("tree should build");

    let transitive = tree
        .nodes
        .get("loose-envify@1.4.0")
        .expect("transitive node should exist");

    assert_eq!(transitive.direct_parent.as_deref(), Some("react@18.2.0"));
}

#[test]
fn build_dependency_tree_for_pnpm_sets_direct_parent_from_edges() {
    let temp = tempdir().expect("tempdir should be created");

    fs::write(
        temp.path().join("package.json"),
        r#"{
  "name": "demo",
  "version": "1.0.0",
  "dependencies": {
    "react": "18.2.0"
  }
}"#,
    )
    .expect("package.json should be written");

    fs::write(
        temp.path().join("pnpm-lock.yaml"),
        r"lockfileVersion: '9.0'
packages:
  /react@18.2.0:
    resolution:
      integrity: sha512-react
    dependencies:
      loose-envify: 1.4.0
  /loose-envify@1.4.0:
    resolution:
      integrity: sha512-loose
    dev: false
",
    )
    .expect("pnpm-lock.yaml should be written");

    let entries = read_lockfile_entries(temp.path()).expect("entries should parse");
    let tree = build_dependency_tree_for_manager(temp.path(), &entries).expect("tree should build");

    let transitive = tree
        .nodes
        .get("loose-envify@1.4.0")
        .expect("transitive node should exist");

    assert_eq!(transitive.direct_parent.as_deref(), Some("react@18.2.0"));
}
