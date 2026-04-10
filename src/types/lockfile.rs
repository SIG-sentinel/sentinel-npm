use std::collections::HashMap;
use std::path::Path;

use super::{LockfileEntry, SentinelError};

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
    Ignore,
}