use std::collections::HashMap;
use std::path::Path;

use crate::constants::{
    LOCKFILE_JSON_KEY_DEPENDENCIES, LOCKFILE_JSON_KEY_PACKAGES, LOCKFILE_JSON_KEY_VERSION,
    NODE_MODULES_PREFIX, PACKAGE_LOCK_FILE,
};
use crate::types::{
    DependencyNode, DependencyTree, ProcessLockfilePackageParams, ResolveDependencyKeyParams,
    SentinelError,
};

use super::lockfile::process_lockfile_package;

pub fn build_dependency_tree(project_dir: &Path) -> Result<DependencyTree, SentinelError> {
    let lock_path = project_dir.join(PACKAGE_LOCK_FILE);
    let lockfile_exists = lock_path.exists();
    if !lockfile_exists {
        return Err(SentinelError::LockfileNotFound);
    }

    let content = std::fs::read_to_string(&lock_path)?;
    let lock: serde_json::Value = serde_json::from_str(&content)
        .map_err(|error| SentinelError::LockfileParse(error.to_string()))?;

    let mut tree = DependencyTree::new();
    let mut all_packages: HashMap<String, DependencyNode> = HashMap::new();
    let mut package_key_by_path: HashMap<String, String> = HashMap::new();

    if let Some(packages) = lock
        .get(LOCKFILE_JSON_KEY_PACKAGES)
        .and_then(|p| p.as_object())
    {
        for (path, metadata) in packages {
            if let Some((key, entry)) = process_lockfile_package(ProcessLockfilePackageParams {
                package_path: path,
                package_metadata: metadata,
            }) {
                package_key_by_path.insert(path.clone(), key.clone());
                all_packages.insert(
                    key.clone(),
                    DependencyNode {
                        package: entry.package.clone(),
                        dependencies: Vec::new(),
                        is_dev: entry.is_dev,
                    },
                );
            }
        }
    }

    if let Some(packages) = lock
        .get(LOCKFILE_JSON_KEY_PACKAGES)
        .and_then(|p| p.as_object())
    {
        for (path, metadata) in packages {
            if let Some((key, _)) = process_lockfile_package(ProcessLockfilePackageParams {
                package_path: path,
                package_metadata: metadata,
            }) {
                let mut direct_deps = Vec::new();

                if let Some(deps_obj) = metadata
                    .get(LOCKFILE_JSON_KEY_DEPENDENCIES)
                    .and_then(|d| d.as_object())
                {
                    for (dep_name, dep_meta) in deps_obj {
                        if let Some(dep_key) = resolve_dep_key(ResolveDependencyKeyParams {
                            package_path: path,
                            dep_name,
                            dep_meta,
                            package_key_by_path: &package_key_by_path,
                            all_packages: &all_packages,
                        }) {
                            direct_deps.push(dep_key);
                        }
                    }
                }

                if let Some(node) = all_packages.get_mut(&key) {
                    direct_deps.sort();
                    direct_deps.dedup();
                    node.dependencies = direct_deps;
                }
            }
        }
    }

    for (_, node) in all_packages {
        tree.insert(node);
    }

    Ok(tree)
}

fn resolve_dep_key(params: ResolveDependencyKeyParams<'_>) -> Option<String> {
    let ResolveDependencyKeyParams {
        package_path,
        dep_name,
        dep_meta,
        package_key_by_path,
        all_packages,
    } = params;

    let nested_dep_path = match package_path.is_empty() {
        true => format!("{NODE_MODULES_PREFIX}{dep_name}"),
        false => format!("{package_path}/{NODE_MODULES_PREFIX}{dep_name}"),
    };

    if let Some(dep_key) = package_key_by_path.get(&nested_dep_path) {
        return Some(dep_key.clone());
    }

    let root_dep_path = format!("{NODE_MODULES_PREFIX}{dep_name}");
    if let Some(dep_key) = package_key_by_path.get(&root_dep_path) {
        return Some(dep_key.clone());
    }

    let dep_version = match dep_meta {
        serde_json::Value::Object(obj) => obj
            .get(LOCKFILE_JSON_KEY_VERSION)
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        serde_json::Value::String(version_range) => Some(version_range.to_string()),
        _ => None,
    };

    if let Some(version) = dep_version {
        let dep_key = format!("{dep_name}@{version}");
        let dep_key_exists = all_packages.contains_key(&dep_key);
        if dep_key_exists {
            return Some(dep_key);
        }
    }

    None
}
