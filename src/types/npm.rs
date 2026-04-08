use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use super::dependency_tree::DependencyNode;
use super::package::PackageRef;

#[derive(Debug, Deserialize)]
pub struct NpmVersionMeta {
    pub dist: NpmDist,
}

#[derive(Debug, Deserialize)]
pub struct NpmDist {
    pub integrity: Option<String>,
    pub tarball: String,
}

#[derive(Debug, Clone)]
pub struct LockfileEntry {
    pub package: PackageRef,
    pub integrity: Option<String>,
    pub is_dev: bool,
}

pub struct ReadPackageJsonDepsParams<'a> {
    pub project_dir: &'a Path,
    pub include_dev: bool,
}

pub struct ProcessLockfilePackageParams<'a> {
    pub package_path: &'a str,
    pub package_metadata: &'a serde_json::Value,
}

pub struct ExtractV1DepsParams<'a> {
    pub deps: &'a serde_json::Map<String, serde_json::Value>,
    pub entries: &'a mut HashMap<String, LockfileEntry>,
}

pub struct FlushYarnEntryParams<'a> {
    pub entries: &'a mut HashMap<String, LockfileEntry>,
    pub selector: &'a mut Option<String>,
    pub version: &'a mut Option<String>,
    pub integrity: &'a mut Option<String>,
}

pub struct ResolveDependencyKeyParams<'a> {
    pub package_path: &'a str,
    pub dep_name: &'a str,
    pub dep_meta: &'a serde_json::Value,
    pub package_key_by_path: &'a HashMap<String, String>,
    pub all_packages: &'a HashMap<String, DependencyNode>,
}
