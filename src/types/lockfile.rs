use std::collections::HashMap;
use std::path::Path;

use super::{LockfileEntry, SentinelError};
use crate::types::PackageRef;

pub trait LockfileParser {
    fn parse_entries(
        &self,
        project_dir: &Path,
    ) -> Result<HashMap<String, LockfileEntry>, SentinelError>;
}

pub struct NpmLockfileParser;
pub struct YarnLockfileParser;
pub struct PnpmLockfileParser;

pub enum YarnLineKind<'a> {
    Header(&'a str),
    Version(&'a str),
    Integrity(&'a str),
    Dependencies,
    Ignore,
}

pub(crate) struct FlushYarnEntryParams<'a> {
    pub(crate) parsed_entries: &'a mut Vec<YarnParsedEntry>,
    pub(crate) state: &'a mut YarnParseState,
}

#[derive(Default)]
pub(crate) struct YarnParseState {
    pub(crate) selectors: Vec<String>,
    pub(crate) version: Option<String>,
    pub(crate) integrity: Option<String>,
    pub(crate) dependency_specs: Vec<(String, String)>,
    pub(crate) in_dependencies_block: bool,
}

pub(crate) struct YarnParsedEntry {
    pub(crate) package: PackageRef,
    pub(crate) selectors: Vec<String>,
    pub(crate) integrity: Option<String>,
    pub(crate) dependency_specs: Vec<(String, String)>,
}

pub(crate) struct BuildPnpmDependencyKeyParams<'a> {
    pub(crate) dependency_name: &'a str,
    pub(crate) dependency_spec: &'a str,
    pub(crate) known_keys: &'a std::collections::HashSet<String>,
}

pub(crate) struct ResolvePnpmDependencyMapParams<'a> {
    pub(crate) dependency_map: &'a serde_yaml::Mapping,
    pub(crate) known_keys: &'a std::collections::HashSet<String>,
}
