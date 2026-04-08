use std::path::{Path, PathBuf};

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

pub fn detect_package_manager(project_dir: &Path) -> Option<PackageManager> {
    let has_package_lock = project_dir.join(PACKAGE_LOCK_FILE).exists();
    
    if has_package_lock {
        return Some(PackageManager::Npm);
    }

    let has_yarn_lock = project_dir.join(YARN_LOCK_FILE).exists();
    
    if has_yarn_lock {
        return Some(PackageManager::Yarn);
    }

    let has_pnpm_lock = project_dir.join(PNPM_LOCK_FILE).exists();
    
    if has_pnpm_lock {
        return Some(PackageManager::Pnpm);
    }

    None
}

pub fn active_lockfile_path(project_dir: &Path) -> PathBuf {
    match detect_package_manager(project_dir) {
        Some(manager) => project_dir.join(manager.lockfile_name()),
        None => project_dir.join(PACKAGE_LOCK_FILE),
    }
}

