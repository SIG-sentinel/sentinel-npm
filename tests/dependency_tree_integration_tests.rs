use std::fs;

use sentinel::npm::build_dependency_tree;
use sentinel::types::SentinelError;
use tempfile::tempdir;

#[test]
fn test_build_dependency_tree_v3_with_nested_paths_and_string_specs() {
    let tmp = tempdir().expect("tempdir");
    let lockfile = r#"
{
  "name": "demo",
  "lockfileVersion": 3,
  "packages": {
    "": {
      "name": "demo",
      "version": "1.0.0",
      "dependencies": {
        "a": "1.0.0"
      }
    },
    "node_modules/a": {
      "version": "1.0.0",
      "dependencies": {
        "b": "^1.0.0",
        "c": "1.0.0"
      }
    },
    "node_modules/a/node_modules/b": {
      "version": "1.1.0"
    },
    "node_modules/c": {
      "version": "1.0.0"
    }
  }
}
"#;

    fs::write(tmp.path().join("package-lock.json"), lockfile).expect("write lockfile");

    let tree = build_dependency_tree(tmp.path()).expect("tree should build");

    assert_eq!(tree.nodes.len(), 3);

    let a_node = tree.nodes.get("a@1.0.0").expect("a node should exist");
    assert_eq!(a_node.dependencies.len(), 2);
    assert!(a_node.dependencies.contains(&"b@1.1.0".to_string()));
    assert!(a_node.dependencies.contains(&"c@1.0.0".to_string()));

    let analysis = tree.analyze();
    assert_eq!(analysis.total_packages, 3);
    assert_eq!(analysis.max_depth, 2);
    assert!(analysis.cycles.is_empty());
}

#[test]
fn test_build_dependency_tree_falls_back_to_root_node_modules_dep() {
    let tmp = tempdir().expect("tempdir");
    let lockfile = r#"
{
  "name": "demo",
  "lockfileVersion": 3,
  "packages": {
    "node_modules/a": {
      "version": "1.0.0",
      "dependencies": {
        "lodash": "^4.17.0"
      }
    },
    "node_modules/lodash": {
      "version": "4.17.21"
    }
  }
}
"#;

    fs::write(tmp.path().join("package-lock.json"), lockfile).expect("write lockfile");

    let tree = build_dependency_tree(tmp.path()).expect("tree should build");
    let a_node = tree.nodes.get("a@1.0.0").expect("a node should exist");

    assert_eq!(a_node.dependencies, vec!["lodash@4.17.21".to_string()]);
}

#[test]
fn test_build_dependency_tree_supports_object_dependency_with_version() {
    let tmp = tempdir().expect("tempdir");
    let lockfile = r#"
{
  "name": "demo",
  "lockfileVersion": 3,
  "packages": {
    "node_modules/a": {
      "version": "1.0.0",
      "dependencies": {
        "b": {
          "version": "2.0.0"
        }
      }
    },
    "node_modules/b": {
      "version": "2.0.0"
    }
  }
}
"#;

    fs::write(tmp.path().join("package-lock.json"), lockfile).expect("write lockfile");

    let tree = build_dependency_tree(tmp.path()).expect("tree should build");
    let a_node = tree.nodes.get("a@1.0.0").expect("a node should exist");

    assert_eq!(a_node.dependencies, vec!["b@2.0.0".to_string()]);
}

#[test]
fn test_build_dependency_tree_missing_lockfile() {
    let tmp = tempdir().expect("tempdir");

    let result = build_dependency_tree(tmp.path());

    assert!(matches!(result, Err(SentinelError::LockfileNotFound)));
}

#[test]
fn test_build_dependency_tree_invalid_json() {
    let tmp = tempdir().expect("tempdir");
    fs::write(tmp.path().join("package-lock.json"), "{ invalid json").expect("write lockfile");

    let result = build_dependency_tree(tmp.path());

    match result {
        Err(SentinelError::LockfileParse(message)) => {
            assert!(!message.is_empty());
        }
        other => panic!("expected LockfileParse error, got: {other:?}"),
    }
}

#[test]
fn test_build_dependency_tree_ignores_missing_dependency_nodes() {
    let tmp = tempdir().expect("tempdir");
    let lockfile = r#"
{
  "name": "demo",
  "lockfileVersion": 3,
  "packages": {
    "node_modules/a": {
      "version": "1.0.0",
      "dependencies": {
        "ghost": "9.9.9"
      }
    }
  }
}
"#;

    fs::write(tmp.path().join("package-lock.json"), lockfile).expect("write lockfile");

    let tree = build_dependency_tree(tmp.path()).expect("tree should build");
    let a_node = tree.nodes.get("a@1.0.0").expect("a node should exist");

    assert!(a_node.dependencies.is_empty());
}
