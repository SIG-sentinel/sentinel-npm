use std::collections::{HashMap, VecDeque};
use std::path::Path;

use crate::constants::{
    PNPM_LOCK_FILE, PNPM_LOCK_KEY_DEPENDENCIES, PNPM_LOCK_KEY_DEV, PNPM_LOCK_KEY_INTEGRITY,
    PNPM_LOCK_KEY_OPTIONAL_DEPENDENCIES, PNPM_LOCK_KEY_PACKAGES, PNPM_LOCK_KEY_RESOLUTION,
    PNPM_LOCK_PEER_SEPARATOR, YARN_LOCK_FILE, YARN_LOCK_KEY_DEPENDENCIES, YARN_LOCK_KEY_INTEGRITY,
    YARN_LOCK_KEY_VERSION, YARN_LOCK_SELECTOR_SEPARATOR,
};
use crate::npm::{build_dependency_tree, read_npm_lockfile, read_package_json_deps};
use crate::types::{
    BuildPnpmDependencyKeyParams, DependencyNode, DependencyTree, FlushYarnEntryParams,
    LockfileEntry, LockfileParser, NpmLockfileParser, PackageRef, PnpmLockfileParser,
    ReadPackageJsonDepsParams, ResolveChildrenForParentAssignmentParams,
    ResolvePnpmDependencyMapParams, SentinelError, YarnLineKind, YarnLockfileParser,
    YarnParseState, YarnParsedEntry,
};

use super::{PackageManager, detect_package_manager};

const ROOT_SCOPED_PACKAGE_AT_INDEX: usize = 0;

impl LockfileParser for NpmLockfileParser {
    fn parse_entries(
        &self,
        project_dir: &Path,
    ) -> Result<HashMap<String, LockfileEntry>, SentinelError> {
        read_npm_lockfile(project_dir)
    }
}

impl LockfileParser for YarnLockfileParser {
    fn parse_entries(
        &self,
        project_dir: &Path,
    ) -> Result<HashMap<String, LockfileEntry>, SentinelError> {
        let lock_path = project_dir.join(YARN_LOCK_FILE);

        if !lock_path.exists() {
            return Err(SentinelError::LockfileNotFound);
        }

        let content = std::fs::read_to_string(lock_path)?;

        Ok(parse_yarn_lock_entries(&content))
    }
}

impl LockfileParser for PnpmLockfileParser {
    fn parse_entries(
        &self,
        project_dir: &Path,
    ) -> Result<HashMap<String, LockfileEntry>, SentinelError> {
        let lock_path = project_dir.join(PNPM_LOCK_FILE);

        if !lock_path.exists() {
            return Err(SentinelError::LockfileNotFound);
        }

        let content = std::fs::read_to_string(lock_path)?;

        parse_pnpm_lock_entries(&content)
    }
}

pub fn read_lockfile_entries(
    project_dir: &Path,
) -> Result<HashMap<String, LockfileEntry>, SentinelError> {
    let manager = detect_package_manager(project_dir).ok_or(SentinelError::LockfileNotFound)?;

    match manager {
        PackageManager::Npm => NpmLockfileParser.parse_entries(project_dir),
        PackageManager::Yarn => YarnLockfileParser.parse_entries(project_dir),
        PackageManager::Pnpm => PnpmLockfileParser.parse_entries(project_dir),
    }
}

pub fn build_dependency_tree_for_manager<S: std::hash::BuildHasher>(
    project_dir: &Path,
    entries: &HashMap<String, LockfileEntry, S>,
) -> Result<DependencyTree, SentinelError> {
    let manager = detect_package_manager(project_dir).ok_or(SentinelError::LockfileNotFound)?;

    let mut tree = match manager {
        PackageManager::Npm => build_dependency_tree(project_dir),
        PackageManager::Yarn | PackageManager::Pnpm => Ok(build_lockfile_dependency_tree(entries)),
    }?;
    let read_package_json_deps_params = ReadPackageJsonDepsParams {
        project_dir,
        include_dev: true,
    };

    if let Ok(direct_deps) = read_package_json_deps(read_package_json_deps_params) {
        mark_direct_dependencies(&mut tree, &direct_deps);
        assign_direct_parents(&mut tree);
    }

    Ok(tree)
}

fn build_lockfile_dependency_tree<S: std::hash::BuildHasher>(
    entries: &HashMap<String, LockfileEntry, S>,
) -> DependencyTree {
    let mut tree = DependencyTree::new();

    for entry in entries.values() {
        let dependency_node = DependencyNode {
            package: entry.package.clone(),
            dependencies: entry.dependencies.clone(),
            is_dev: entry.is_dev,
            is_direct: false,
            direct_parent: None,
        };

        tree.insert(dependency_node);
    }
    tree
}

fn mark_direct_dependencies(tree: &mut DependencyTree, direct_deps: &HashMap<String, String>) {
    for node in tree.nodes.values_mut() {
        node.is_direct = direct_deps.contains_key(&node.package.name);
    }
}

fn assign_direct_parents(tree: &mut DependencyTree) {
    let direct_keys: Vec<String> = tree
        .nodes
        .values()
        .filter(|node| node.is_direct)
        .map(|node| node.package.to_string())
        .collect();

    for direct_key in &direct_keys {
        let Some(direct_node) = tree.nodes.get(direct_key) else {
            continue;
        };

        let mut queue = VecDeque::new();

        queue.extend(direct_node.dependencies.clone());

        while let Some(dep_key) = queue.pop_front() {
            let resolve_children_params = ResolveChildrenForParentAssignmentParams {
                tree,
                dependency_key: &dep_key,
                direct_key,
            };
            let maybe_children = resolve_children_for_parent_assignment(resolve_children_params);
            let Some(children) = maybe_children else {
                continue;
            };

            queue.extend(children);
        }
    }
}

fn resolve_children_for_parent_assignment(
    params: ResolveChildrenForParentAssignmentParams<'_>,
) -> Option<Vec<String>> {
    let ResolveChildrenForParentAssignmentParams {
        tree,
        dependency_key,
        direct_key,
    } = params;

    let dep_node = tree.nodes.get(dependency_key)?;

    let is_direct_dependency = dep_node.is_direct;
    let has_direct_parent_assigned = dep_node.direct_parent.is_some();
    let should_skip_parent_assignment = is_direct_dependency || has_direct_parent_assigned;

    if should_skip_parent_assignment {
        return None;
    }

    let children = dep_node.dependencies.clone();
    let direct_parent = Some(direct_key.to_string());
    let node = tree.nodes.get_mut(dependency_key)?;

    node.direct_parent = direct_parent;

    Some(children)
}

fn parse_yarn_lock_entries(content: &str) -> HashMap<String, LockfileEntry> {
    let mut parsed_entries = Vec::new();
    let mut state = YarnParseState::default();

    for raw_line in content.lines() {
        if let Some((dependency_name, dependency_range)) =
            consume_yarn_dependency_line(raw_line, &mut state)
        {
            state
                .dependency_specs
                .push((dependency_name, dependency_range));

            continue;
        }

        match classify_yarn_line(raw_line) {
            YarnLineKind::Header(line) => {
                let flush_yarn_entry_params = FlushYarnEntryParams {
                    parsed_entries: &mut parsed_entries,
                    state: &mut state,
                };

                flush_yarn_entry(flush_yarn_entry_params);

                state.selectors = parse_yarn_selector_header(line);
            }
            YarnLineKind::Version(value) => {
                state.version = Some(strip_wrapping_quotes(value));
                state.in_dependencies_block = false;
            }
            YarnLineKind::Integrity(value) => {
                state.integrity = Some(strip_wrapping_quotes(value));
                state.in_dependencies_block = false;
            }
            YarnLineKind::Dependencies => {
                state.in_dependencies_block = true;
            }
            YarnLineKind::Ignore => {}
        }
    }

    let flush_yarn_entry_params = FlushYarnEntryParams {
        parsed_entries: &mut parsed_entries,
        state: &mut state,
    };

    flush_yarn_entry(flush_yarn_entry_params);
    build_yarn_lock_entries(parsed_entries)
}

fn classify_yarn_line(raw_line: &str) -> YarnLineKind<'_> {
    let trimmed_line = raw_line.trim_end();
    let trimmed_line_without_indentation = trimmed_line.trim_start();

    let is_header =
        !trimmed_line.is_empty() && !trimmed_line.starts_with(' ') && trimmed_line.ends_with(':');
    let version_value = trimmed_line_without_indentation.strip_prefix(YARN_LOCK_KEY_VERSION);
    let integrity_value = trimmed_line_without_indentation.strip_prefix(YARN_LOCK_KEY_INTEGRITY);
    let is_dependencies = trimmed_line_without_indentation == YARN_LOCK_KEY_DEPENDENCIES;

    match (is_header, version_value, integrity_value, is_dependencies) {
        (true, _, _, _) => YarnLineKind::Header(trimmed_line),
        (false, Some(value), _, _) => YarnLineKind::Version(value),
        (false, None, Some(value), _) => YarnLineKind::Integrity(value),
        (false, None, None, true) => YarnLineKind::Dependencies,
        (false, None, None, false) => YarnLineKind::Ignore,
    }
}

fn consume_yarn_dependency_line(
    raw_line: &str,
    state: &mut YarnParseState,
) -> Option<(String, String)> {
    if !state.in_dependencies_block {
        return None;
    }

    let parsed_dependency = parse_yarn_dependency_line(raw_line);

    if parsed_dependency.is_none() {
        state.in_dependencies_block = false;
    }

    parsed_dependency
}

fn flush_yarn_entry(params: FlushYarnEntryParams<'_>) {
    let FlushYarnEntryParams {
        parsed_entries,
        state,
    } = params;
    let selectors = std::mem::take(&mut state.selectors);
    let version = state.version.take();
    let integrity = state.integrity.take();
    let dependency_specs = std::mem::take(&mut state.dependency_specs);

    state.in_dependencies_block = false;

    let (Some(first_selector), Some(version)) = (selectors.first(), version) else {
        return;
    };

    let Some((name, _)) = parse_name_and_version(first_selector) else {
        return;
    };

    let parsed_entry = YarnParsedEntry {
        package: PackageRef::new(name, version),
        selectors,
        integrity,
        dependency_specs,
    };
    parsed_entries.push(parsed_entry);
}

fn build_yarn_lock_entries(parsed_entries: Vec<YarnParsedEntry>) -> HashMap<String, LockfileEntry> {
    let selector_to_package_key = build_yarn_selector_index(&parsed_entries);
    let mut entries = HashMap::new();

    for parsed_entry in parsed_entries {
        let package_key = parsed_entry.package.to_string();
        let dependencies =
            resolve_yarn_dependencies(&parsed_entry.dependency_specs, &selector_to_package_key);

        entries.insert(
            package_key,
            LockfileEntry {
                package: parsed_entry.package,
                integrity: parsed_entry.integrity,
                is_dev: false,
                dependencies,
            },
        );
    }

    entries
}

fn build_yarn_selector_index(parsed_entries: &[YarnParsedEntry]) -> HashMap<String, String> {
    parsed_entries
        .iter()
        .flat_map(|entry| {
            let package_key = entry.package.to_string();
            entry
                .selectors
                .iter()
                .cloned()
                .map(move |selector| (selector, package_key.clone()))
        })
        .collect()
}

fn resolve_yarn_dependencies(
    dependency_specs: &[(String, String)],
    selector_to_package_key: &HashMap<String, String>,
) -> Vec<String> {
    dependency_specs
        .iter()
        .filter_map(|(name, version_range)| {
            let selector = format!("{name}@{version_range}");
            selector_to_package_key.get(&selector).cloned()
        })
        .collect()
}

fn parse_yarn_selector_header(line: &str) -> Vec<String> {
    let Some(without_colon) = line.strip_suffix(':') else {
        return Vec::new();
    };

    without_colon
        .split(YARN_LOCK_SELECTOR_SEPARATOR)
        .map(strip_wrapping_quotes)
        .filter(|selector| !selector.is_empty())
        .collect()
}

fn parse_yarn_dependency_line(raw_line: &str) -> Option<(String, String)> {
    if !raw_line.starts_with("    ") {
        return None;
    }

    let trimmed = raw_line.trim();
    let (dependency_name, dependency_range) = trimmed.split_once(' ')?;
    let normalized_range = strip_wrapping_quotes(dependency_range);

    (!dependency_name.is_empty() && !normalized_range.is_empty())
        .then_some((dependency_name.to_string(), normalized_range))
}

fn parse_pnpm_lock_entries(content: &str) -> Result<HashMap<String, LockfileEntry>, SentinelError> {
    let yaml_root: serde_yaml::Value = serde_yaml::from_str(content)
        .map_err(|error| SentinelError::LockfileParse(error.to_string()))?;

    let Some(packages) = yaml_root
        .get(PNPM_LOCK_KEY_PACKAGES)
        .and_then(|value| value.as_mapping())
    else {
        return Ok(HashMap::new());
    };

    let mut entries = HashMap::new();

    for (raw_key, raw_meta) in packages {
        let Some(key) = raw_key.as_str() else {
            continue;
        };

        let Some((name, version)) = parse_pnpm_package_name_and_version(key) else {
            continue;
        };

        let integrity = raw_meta
            .get(PNPM_LOCK_KEY_RESOLUTION)
            .and_then(|value| value.get(PNPM_LOCK_KEY_INTEGRITY))
            .and_then(|value| value.as_str())
            .map(ToString::to_string);

        let is_dev = raw_meta
            .get(PNPM_LOCK_KEY_DEV)
            .and_then(serde_yaml::Value::as_bool)
            .unwrap_or(false);

        let package_ref = PackageRef::new(name, version);

        entries.insert(
            package_ref.to_string(),
            LockfileEntry {
                package: package_ref,
                integrity,
                is_dev,
                dependencies: Vec::new(),
            },
        );
    }

    let known_keys: std::collections::HashSet<String> = entries.keys().cloned().collect();

    for (raw_key, raw_meta) in packages {
        let Some(package_spec) = raw_key.as_str() else {
            continue;
        };

        let Some((name, version)) = parse_pnpm_package_name_and_version(package_spec) else {
            continue;
        };

        let package_key = PackageRef::new(name, version).to_string();
        let dependencies = resolve_pnpm_dependencies(raw_meta, &known_keys);

        if let Some(entry) = entries.get_mut(&package_key) {
            entry.dependencies = dependencies;
        }
    }

    Ok(entries)
}

fn parse_pnpm_package_name_and_version(spec: &str) -> Option<(String, String)> {
    let normalized = normalize_pnpm_spec(spec);

    parse_name_and_version(normalized)
}

fn normalize_pnpm_spec(spec: &str) -> &str {
    spec.trim_start_matches('/')
        .split(PNPM_LOCK_PEER_SEPARATOR)
        .next()
        .unwrap_or("")
}

fn resolve_pnpm_dependencies(
    raw_meta: &serde_yaml::Value,
    known_keys: &std::collections::HashSet<String>,
) -> Vec<String> {
    let mut dependencies = Vec::new();

    for dependency_key in [
        PNPM_LOCK_KEY_DEPENDENCIES,
        PNPM_LOCK_KEY_OPTIONAL_DEPENDENCIES,
    ] {
        let Some(dependency_map) = raw_meta
            .get(dependency_key)
            .and_then(|value| value.as_mapping())
        else {
            continue;
        };

        let resolve_pnpm_dependency_map_params = ResolvePnpmDependencyMapParams {
            dependency_map,
            known_keys,
        };

        dependencies.extend(resolve_pnpm_dependency_map(
            &resolve_pnpm_dependency_map_params,
        ));
    }

    dependencies
}

fn resolve_pnpm_dependency_map(params: &ResolvePnpmDependencyMapParams<'_>) -> Vec<String> {
    let ResolvePnpmDependencyMapParams {
        dependency_map,
        known_keys,
    } = *params;

    dependency_map
        .iter()
        .filter_map(|(raw_name, raw_spec)| {
            let dependency_name = raw_name.as_str()?;
            let dependency_spec = raw_spec.as_str()?;
            let build_pnpm_dependency_key_params = BuildPnpmDependencyKeyParams {
                dependency_name,
                dependency_spec,
                known_keys,
            };

            build_pnpm_dependency_key(&build_pnpm_dependency_key_params)
        })
        .collect()
}

fn build_pnpm_dependency_key(params: &BuildPnpmDependencyKeyParams<'_>) -> Option<String> {
    let BuildPnpmDependencyKeyParams {
        dependency_name,
        dependency_spec,
        known_keys,
    } = *params;

    let normalized_spec = normalize_pnpm_spec(dependency_spec);

    if normalized_spec.contains(':') {
        return None;
    }

    let dependency_key = format!("{dependency_name}@{normalized_spec}");

    known_keys
        .contains(&dependency_key)
        .then_some(dependency_key)
}

fn parse_name_and_version(spec: &str) -> Option<(String, String)> {
    let at_index = spec.rfind('@')?;

    if at_index == ROOT_SCOPED_PACKAGE_AT_INDEX {
        return None;
    }

    let name = spec[..at_index].to_string();
    let version = spec[at_index + 1..].to_string();
    let is_empty_name = name.is_empty();
    let is_empty_version = version.is_empty();
    let has_empty_parts = is_empty_name || is_empty_version;

    if has_empty_parts {
        return None;
    }

    Some((name, version))
}

fn strip_wrapping_quotes(input: &str) -> String {
    input.trim().trim_matches('"').to_string()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "../../tests/internal/lockfile_tests.rs"]
mod tests;
