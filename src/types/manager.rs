use crate::constants::{PACKAGE_LOCK_FILE, PNPM_LOCK_FILE, YARN_LOCK_FILE};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Npm,
    Yarn,
    Pnpm,
}

impl PackageManager {
    pub fn command(self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Yarn => "yarn",
            Self::Pnpm => "pnpm",
        }
    }

    pub fn lockfile_name(self) -> &'static str {
        match self {
            Self::Npm => PACKAGE_LOCK_FILE,
            Self::Yarn => YARN_LOCK_FILE,
            Self::Pnpm => PNPM_LOCK_FILE,
        }
    }
}

#[derive(Clone, Copy)]
pub struct BuildResolveErrorMessageParams<'a> {
    pub project_dir: &'a Path,
    pub command_hint: &'a str,
}

#[derive(Clone, Copy)]
pub struct StartsWithManagerPrefixParams<'a> {
    pub value: &'a str,
    pub prefix_at: &'a str,
    pub prefix_space: &'a str,
}

pub struct ResolvePackageManagerParams<'a> {
    pub project_dir: &'a Path,
    pub explicit_pm: Option<&'a str>,
    pub command_hint: &'a str,
}
