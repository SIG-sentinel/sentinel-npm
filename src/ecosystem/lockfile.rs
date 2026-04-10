use std::collections::HashMap;
use std::path::Path;

use crate::constants::{
    PNPM_LOCK_FILE, PNPM_LOCK_KEY_DEV, PNPM_LOCK_KEY_INTEGRITY, PNPM_LOCK_KEY_PACKAGES,
    PNPM_LOCK_KEY_RESOLUTION, PNPM_LOCK_PEER_SEPARATOR, YARN_LOCK_FILE, YARN_LOCK_KEY_INTEGRITY,
    YARN_LOCK_KEY_VERSION, YARN_LOCK_SELECTOR_SEPARATOR,
};
use crate::npm::{build_dependency_tree, read_npm_lockfile};
use crate::types::{
    DependencyNode, DependencyTree, FlushYarnEntryParams, LockfileEntry, LockfileParser,
    NpmLockfileParser, PackageRef, PnpmLockfileParser, SentinelError, YarnLineKind,
    YarnLockfileParser,
};

use super::{PackageManager, detect_package_manager};

impl LockfileParser for NpmLockfileParser {
    fn parse_entries(&self, project_dir: &Path) -> Result<HashMap<String, LockfileEntry>, SentinelError> {
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

        parse_yarn_lock_entries(&content)
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

pub fn build_dependency_tree_for_manager(
    project_dir: &Path,
    entries: &HashMap<String, LockfileEntry>,
) -> Result<DependencyTree, SentinelError> {
    let manager = detect_package_manager(project_dir).ok_or(SentinelError::LockfileNotFound)?;

    match manager {
        PackageManager::Npm => build_dependency_tree(project_dir),
        PackageManager::Yarn | PackageManager::Pnpm => Ok(build_flat_dependency_tree(entries)),
    }
}

fn build_flat_dependency_tree(entries: &HashMap<String, LockfileEntry>) -> DependencyTree {
    let mut tree = DependencyTree::new();

    for entry in entries.values() {
        tree.insert(DependencyNode {
            package: entry.package.clone(),
            dependencies: Vec::new(),
            is_dev: entry.is_dev,
        });
    }
    tree
}

fn parse_yarn_lock_entries(content: &str) -> Result<HashMap<String, LockfileEntry>, SentinelError> {
    let mut entries = HashMap::new();
    let mut current_selector: Option<String> = None;
    let mut current_version: Option<String> = None;
    let mut current_integrity: Option<String> = None;

    for raw_line in content.lines() {
        match classify_yarn_line(raw_line) {
            YarnLineKind::Header(line) => {
                flush_yarn_entry(FlushYarnEntryParams {
                    entries: &mut entries,
                    selector: &mut current_selector,
                    version: &mut current_version,
                    integrity: &mut current_integrity,
                });

                current_selector = parse_yarn_selector_header(line);
            }
            YarnLineKind::Version(value) => {
                current_version = Some(strip_wrapping_quotes(value));
            }
            YarnLineKind::Integrity(value) => {
                current_integrity = Some(strip_wrapping_quotes(value));
            }
            YarnLineKind::Ignore => {}
        }
    }

    flush_yarn_entry(FlushYarnEntryParams {
        entries: &mut entries,
        selector: &mut current_selector,
        version: &mut current_version,
        integrity: &mut current_integrity,
    });

    Ok(entries)
}

fn classify_yarn_line(raw_line: &str) -> YarnLineKind<'_> {
    let line = raw_line.trim_end();
    let trimmed = line.trim_start();

    let is_header = !line.is_empty() && !line.starts_with(' ') && line.ends_with(':');
    let version_value = trimmed.strip_prefix(YARN_LOCK_KEY_VERSION);
    let integrity_value = trimmed.strip_prefix(YARN_LOCK_KEY_INTEGRITY);

    match (is_header, version_value, integrity_value) {
        (true, _, _) => YarnLineKind::Header(line),
        (false, Some(value), _) => YarnLineKind::Version(value),
        (false, None, Some(value)) => YarnLineKind::Integrity(value),
        (false, None, None) => YarnLineKind::Ignore,
    }
}

fn flush_yarn_entry(params: FlushYarnEntryParams<'_>) {
    let FlushYarnEntryParams {
        entries,
        selector,
        version,
        integrity,
    } = params;

    let selector_value = selector.take();
    let version_value = version.take();
    let integrity_value = integrity.take();

    if let (Some(selector), Some(version_value)) = (selector_value, version_value)
        && let Some((name, _)) = parse_name_and_version(&selector)
    {
        let package_ref = PackageRef::new(name, version_value);
        entries.insert(
            package_ref.to_string(),
            LockfileEntry {
                package: package_ref,
                integrity: integrity_value,
                is_dev: false,
            },
        );
    }
}

fn parse_yarn_selector_header(line: &str) -> Option<String> {
    let without_colon = line.strip_suffix(':')?;
    let first_selector = without_colon.split(YARN_LOCK_SELECTOR_SEPARATOR).next()?.trim();
    let unquoted = strip_wrapping_quotes(first_selector);
    (!unquoted.is_empty()).then_some(unquoted)
}

fn parse_pnpm_lock_entries(content: &str) -> Result<HashMap<String, LockfileEntry>, SentinelError> {
    let root: serde_yaml::Value = serde_yaml::from_str(content)
        .map_err(|error| SentinelError::LockfileParse(error.to_string()))?;

    let packages = root
        .get(PNPM_LOCK_KEY_PACKAGES)
        .and_then(|value| value.as_mapping())
        .ok_or_else(|| SentinelError::LockfileParse("missing packages map in pnpm-lock.yaml".to_string()))?;

    let mut entries = HashMap::new();

    for (raw_key, raw_meta) in packages {
        let Some(key) = raw_key.as_str() else { continue };

        let normalized = key
            .trim_start_matches('/')
            .split(PNPM_LOCK_PEER_SEPARATOR)
            .next()
            .unwrap_or("");

        let Some((name, version)) = parse_name_and_version(normalized) else { continue };

        let integrity = raw_meta
            .get(PNPM_LOCK_KEY_RESOLUTION)
            .and_then(|value| value.get(PNPM_LOCK_KEY_INTEGRITY))
            .and_then(|value| value.as_str())
            .map(ToString::to_string);

        let is_dev = raw_meta
            .get(PNPM_LOCK_KEY_DEV)
            .and_then(|value| value.as_bool())
            .unwrap_or(false);

        let package_ref = PackageRef::new(name, version);
        entries.insert(
            package_ref.to_string(),
            LockfileEntry {
                package: package_ref,
                integrity,
                is_dev,
            },
        );
    }

    Ok(entries)
}

fn parse_name_and_version(spec: &str) -> Option<(String, String)> {
    let at_index = spec.rfind('@')?;
    
    if at_index == 0 {
        return None;
    }

    let name = spec[..at_index].to_string();
    let version = spec[at_index + 1..].to_string();

    if name.is_empty() || version.is_empty() {
        return None;
    }

    Some((name, version))
}

fn strip_wrapping_quotes(input: &str) -> String {
    input.trim().trim_matches('"').to_string()
}
