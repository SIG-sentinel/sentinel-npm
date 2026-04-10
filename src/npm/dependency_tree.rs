use std::collections::HashMap;
use std::path::Path;

use crate::constants::{
    LOCKFILE_JSON_KEY_DEPENDENCIES, LOCKFILE_JSON_KEY_PACKAGES, LOCKFILE_JSON_KEY_VERSION,
    NODE_MODULES_PREFIX, PACKAGE_LOCK_FILE,
};
use crate::types::{
    DependencyNode, DependencyTree, ProcessLockfilePackageParams, ResolveDependencyKeyParams,
    SentinelError, WireDependenciesParams,
};

use super::lockfile::process_lockfile_package;

pub fn build_dependency_tree(project_dir: &Path) -> Result<DependencyTree, SentinelError> {
    let lock_path = project_dir.join(PACKAGE_LOCK_FILE);

    if !lock_path.exists() {
        return Err(SentinelError::LockfileNotFound);
    }

    let content = std::fs::read_to_string(&lock_path)?;
    let lock: serde_json::Value = serde_json::from_str(&content)
        .map_err(|error| SentinelError::LockfileParse(error.to_string()))?;

    let packages = lock
        .get(LOCKFILE_JSON_KEY_PACKAGES)
        .and_then(|p| p.as_object());

    let Some(packages) = packages else {
        return Ok(DependencyTree::new());
    };

    let (mut all_packages, package_key_by_path) = collect_packages(packages);
    
    wire_dependencies(WireDependenciesParams {
        packages,
        package_key_by_path: &package_key_by_path,
        all_packages: &mut all_packages,
    });

    let mut tree = DependencyTree::new();

    for (_, node) in all_packages {
        tree.insert(node);
    }

    Ok(tree)
}

fn collect_packages(
    packages: &serde_json::Map<String, serde_json::Value>,
) -> (HashMap<String, DependencyNode>, HashMap<String, String>) {
    let mut all_packages: HashMap<String, DependencyNode> = HashMap::new();
    let mut package_key_by_path: HashMap<String, String> = HashMap::new();

    for (path, metadata) in packages {
        let Some((key, entry)) = process_lockfile_package(ProcessLockfilePackageParams {
            package_path: path,
            package_metadata: metadata,
        }) else {
            continue;
        };

        package_key_by_path.insert(path.clone(), key.clone());
        all_packages.insert(
            key,
            DependencyNode {
                package: entry.package.clone(),
                dependencies: Vec::new(),
                is_dev: entry.is_dev,
            },
        );
    }

    (all_packages, package_key_by_path)
}

fn wire_dependencies(params: WireDependenciesParams<'_>) {
    let WireDependenciesParams {
        packages,
        package_key_by_path,
        all_packages,
    } = params;

    for (path, metadata) in packages {
        let Some(key) = package_key_by_path.get(path) else {
            continue;
        };

        let deps_obj = metadata
            .get(LOCKFILE_JSON_KEY_DEPENDENCIES)
            .and_then(|d| d.as_object());

        let Some(deps_obj) = deps_obj else {
            continue;
        };

        let mut direct_deps: Vec<String> = deps_obj
            .iter()
            .filter_map(|(dep_name, dep_meta)| {
                resolve_dep_key(ResolveDependencyKeyParams {
                    package_path: path,
                    dep_name,
                    dep_meta,
                    package_key_by_path,
                    all_packages,
                })
            })
            .collect();

        direct_deps.sort();
        direct_deps.dedup();

        if let Some(node) = all_packages.get_mut(key) {
            node.dependencies = direct_deps;
        }
    }
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

    let root_dep_path = format!("{NODE_MODULES_PREFIX}{dep_name}");

    let nested_dep_key = package_key_by_path.get(&nested_dep_path).cloned();
    let root_dep_key = package_key_by_path.get(&root_dep_path).cloned();
    let versioned_dep_key = extract_dep_version(dep_meta).and_then(|version| {
        let key = format!("{dep_name}@{version}");
        all_packages.contains_key(&key).then_some(key)
    });

    match (nested_dep_key, root_dep_key, versioned_dep_key) {
        (Some(key), _, _) => Some(key),
        (None, Some(key), _) => Some(key),
        (None, None, Some(key)) => Some(key),
        (None, None, None) => None,
    }
}

fn extract_dep_version(dep_meta: &serde_json::Value) -> Option<String> {
    match dep_meta {
        serde_json::Value::Object(obj) => obj
            .get(LOCKFILE_JSON_KEY_VERSION)
            .and_then(|v| v.as_str())
            .map(|v| v.to_string()),
        serde_json::Value::String(version_range) => Some(version_range.to_string()),
        _ => None,
    }
}
