use std::collections::HashMap;

use crate::constants::{
    PACKAGE_JSON_DEPENDENCIES_KEY, PACKAGE_JSON_DEV_DEPENDENCIES_KEY, PACKAGE_JSON_FILE,
    PACKAGE_JSON_PEER_DEPENDENCIES_KEY, PACKAGE_VERSION_DEFAULT_RANGE,
};
use crate::types::{ReadPackageJsonDepsParams, SentinelError};

pub fn read_package_json_deps(
    params: ReadPackageJsonDepsParams<'_>,
) -> Result<HashMap<String, String>, SentinelError> {
    let ReadPackageJsonDepsParams {
        project_dir,
        include_dev,
    } = params;

    let path = project_dir.join(PACKAGE_JSON_FILE);
    let path_exists = path.exists();
    if !path_exists {
        return Err(SentinelError::PackageJsonNotFound {
            path: project_dir.display().to_string(),
        });
    }

    let content = std::fs::read_to_string(&path)?;
    let package_json: serde_json::Value = serde_json::from_str(&content)?;

    let mut dependency_keys = vec![
        PACKAGE_JSON_DEPENDENCIES_KEY,
        PACKAGE_JSON_PEER_DEPENDENCIES_KEY,
    ];

    if include_dev {
        dependency_keys.push(PACKAGE_JSON_DEV_DEPENDENCIES_KEY);
    }

    let mut dependencies_by_name = HashMap::new();

    for dependency_key in dependency_keys {
        if let Some(dependency_map) = package_json
            .get(dependency_key)
            .and_then(|value| value.as_object())
        {
            dependencies_by_name.extend(dependency_map.iter().map(|(name, version)| {
                (
                    name.clone(),
                    version
                        .as_str()
                        .unwrap_or(PACKAGE_VERSION_DEFAULT_RANGE)
                        .to_string(),
                )
            }));
        }
    }

    Ok(dependencies_by_name)
}
