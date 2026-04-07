use std::collections::HashMap;
use std::path::Path;

use crate::constants::{
    LOCKFILE_JSON_KEY_DEPENDENCIES, LOCKFILE_JSON_KEY_DEV, LOCKFILE_JSON_KEY_INTEGRITY,
    LOCKFILE_JSON_KEY_PACKAGES, LOCKFILE_JSON_KEY_VERSION, NODE_MODULES_PREFIX, PACKAGE_LOCK_FILE,
};
use crate::types::{
    ExtractV1DepsParams, LockfileEntry, PackageRef, ProcessLockfilePackageParams, SentinelError,
};

pub(crate) fn process_lockfile_package(
    params: ProcessLockfilePackageParams<'_>,
) -> Option<(String, LockfileEntry)> {
    let ProcessLockfilePackageParams {
        package_path,
        package_metadata,
    } = params;

    let path_is_empty = package_path.is_empty();
    if path_is_empty {
        return None;
    }

    let package_name = package_path
        .rsplit(NODE_MODULES_PREFIX)
        .next()
        .unwrap_or(package_path)
        .to_string();

    let package_version = package_metadata
        .get(LOCKFILE_JSON_KEY_VERSION)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let version_is_empty = package_version.is_empty();
    if version_is_empty {
        return None;
    }

    let package_integrity = package_metadata
        .get(LOCKFILE_JSON_KEY_INTEGRITY)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let is_dev = package_metadata
        .get(LOCKFILE_JSON_KEY_DEV)
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let package_ref = PackageRef::new(package_name, package_version);
    let key = package_ref.to_string();

    Some((
        key,
        LockfileEntry {
            package: package_ref,
            integrity: package_integrity,
            is_dev,
        },
    ))
}

fn extract_v1_deps(params: ExtractV1DepsParams<'_>) {
    let ExtractV1DepsParams { deps, entries } = params;

    for (package_name, package_metadata) in deps {
        let package_version = package_metadata
            .get(LOCKFILE_JSON_KEY_VERSION)
            .and_then(|version_value| version_value.as_str())
            .unwrap_or("")
            .to_string();

        let version_is_empty = package_version.is_empty();
        if version_is_empty {
            continue;
        }

        let package_integrity = package_metadata
            .get(LOCKFILE_JSON_KEY_INTEGRITY)
            .and_then(|integrity_value| integrity_value.as_str())
            .map(|integrity| integrity.to_string());

        let is_dev_dependency = package_metadata
            .get(LOCKFILE_JSON_KEY_DEV)
            .and_then(|is_dev_value| is_dev_value.as_bool())
            .unwrap_or(false);

        let package_ref = PackageRef::new(package_name, &package_version);
        entries.insert(
            package_ref.to_string(),
            LockfileEntry {
                package: package_ref,
                integrity: package_integrity,
                is_dev: is_dev_dependency,
            },
        );

        if let Some(transitive_dependencies) = package_metadata
            .get(LOCKFILE_JSON_KEY_DEPENDENCIES)
            .and_then(|dependencies| dependencies.as_object())
        {
            extract_v1_deps(ExtractV1DepsParams {
                deps: transitive_dependencies,
                entries,
            });
        }
    }
}

pub fn read_npm_lockfile(
    project_dir: &Path,
) -> Result<HashMap<String, LockfileEntry>, SentinelError> {
    let lock_path = project_dir.join(PACKAGE_LOCK_FILE);
    let lockfile_exists = lock_path.exists();
    if !lockfile_exists {
        return Err(SentinelError::LockfileNotFound);
    }

    let content = std::fs::read_to_string(&lock_path)?;
    let lock: serde_json::Value = serde_json::from_str(&content)
        .map_err(|error| SentinelError::LockfileParse(error.to_string()))?;

    let mut entries = HashMap::new();

    let v2_packages = lock
        .get(LOCKFILE_JSON_KEY_PACKAGES)
        .and_then(|package_map| package_map.as_object());

    let v1_dependencies = lock
        .get(LOCKFILE_JSON_KEY_DEPENDENCIES)
        .and_then(|dependency_map| dependency_map.as_object());

    match (v2_packages, v1_dependencies) {
        (Some(packages), _) => {
            for (path, metadata) in packages {
                if let Some((key, entry)) = process_lockfile_package(ProcessLockfilePackageParams {
                    package_path: path,
                    package_metadata: metadata,
                }) {
                    entries.insert(key, entry);
                }
            }
        }
        (None, Some(dependencies)) => {
            extract_v1_deps(ExtractV1DepsParams {
                deps: dependencies,
                entries: &mut entries,
            });
        }
        (None, None) => {}
    }

    Ok(entries)
}
