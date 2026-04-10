use std::path::{Path, PathBuf};

use crate::constants::{PACKAGE_LOCK_FILE, PNPM_LOCK_FILE, YARN_LOCK_FILE};
use crate::types::PackageManager;

pub fn detect_package_manager(project_dir: &Path) -> Option<PackageManager> {
    let has_package_lock = project_dir.join(PACKAGE_LOCK_FILE).exists();
    let has_yarn_lock = project_dir.join(YARN_LOCK_FILE).exists();
    let has_pnpm_lock = project_dir.join(PNPM_LOCK_FILE).exists();

    match (has_package_lock, has_yarn_lock, has_pnpm_lock) {
        (true, _, _) => Some(PackageManager::Npm),
        (false, true, _) => Some(PackageManager::Yarn),
        (false, false, true) => Some(PackageManager::Pnpm),
        (false, false, false) => None,
    }
}

pub fn active_lockfile_path(project_dir: &Path) -> PathBuf {
    match detect_package_manager(project_dir) {
        Some(manager) => project_dir.join(manager.lockfile_name()),
        None => project_dir.join(PACKAGE_LOCK_FILE),
    }
}

