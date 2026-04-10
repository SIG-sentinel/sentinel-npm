use crate::constants::{PACKAGE_LOCK_FILE, PNPM_LOCK_FILE, YARN_LOCK_FILE};

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