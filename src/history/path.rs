use std::path::{Path, PathBuf};

use crate::constants::{ENV_SENTINEL_HISTORY_PATH, SENTINEL_HISTORY_FILE, SENTINEL_HOME_DIR};

pub fn resolve_project_root(project_dir: &Path) -> Result<PathBuf, std::io::Error> {
    std::fs::canonicalize(project_dir)
}

pub fn resolve_history_ledger_path(project_root: &Path) -> PathBuf {
    if let Ok(path) = std::env::var(ENV_SENTINEL_HISTORY_PATH)
        && !path.trim().is_empty()
    {
        return PathBuf::from(path);
    }

    project_root
        .join(SENTINEL_HOME_DIR)
        .join(SENTINEL_HISTORY_FILE)
}
