use std::path::Path;

use sha2::{Digest, Sha256};

use crate::constants::PACKAGE_JSON_FILE;
use crate::ecosystem::active_lockfile_path;
use crate::types::{ProjectFilesSnapshot, RestoreFileParams, RestoreProjectFilesSnapshotParams};

pub fn capture_project_files_snapshot(cwd: &Path) -> ProjectFilesSnapshot {
    let lockfile_path = active_lockfile_path(cwd);
    let lockfile_name = lockfile_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("package-lock.json")
        .to_string();

    ProjectFilesSnapshot {
        package_json: std::fs::read(cwd.join(PACKAGE_JSON_FILE)).ok(),
        lockfile_name,
        lockfile_contents: std::fs::read(lockfile_path).ok(),
    }
}

fn restore_file(params: RestoreFileParams<'_>) -> std::io::Result<()> {
    let RestoreFileParams {
        current_working_directory,
        file_name,
        original_contents,
    } = params;

    let file_path = current_working_directory.join(file_name);

    match original_contents {
        Some(contents) => {
            std::fs::write(file_path, contents)?;
            Ok(())
        }
        None => match std::fs::remove_file(file_path) {
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        },
    }
}

pub fn restore_project_files_snapshot(
    params: RestoreProjectFilesSnapshotParams<'_>,
) -> std::io::Result<()> {
    let RestoreProjectFilesSnapshotParams {
        snapshot,
        current_working_directory,
    } = params;

    restore_file(RestoreFileParams {
        current_working_directory,
        file_name: PACKAGE_JSON_FILE,
        original_contents: &snapshot.package_json,
    })
    .map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!("failed to restore package.json: {error}"),
        )
    })?;

    restore_file(RestoreFileParams {
        current_working_directory,
        file_name: &snapshot.lockfile_name,
        original_contents: &snapshot.lockfile_contents,
    })
    .map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!("failed to restore {}: {error}", snapshot.lockfile_name),
        )
    })?;

    Ok(())
}

pub fn lockfile_sha256(cwd: &Path) -> Option<String> {
    let lockfile_path = active_lockfile_path(cwd);
    let bytes = std::fs::read(lockfile_path).ok()?;
    let mut hasher = Sha256::new();

    hasher.update(bytes);
    
    let digest = hasher.finalize();
    let hash = digest.iter().map(|byte| format!("{byte:02x}")).collect();

    Some(hash)
}
