use std::path::Path;

use sha2::{Digest, Sha256};

use crate::constants::messages::verifier::{RESTORE_FAILED_LOCKFILE, RESTORE_FAILED_PACKAGE_JSON};
use crate::constants::{PACKAGE_JSON_FILE, PACKAGE_LOCK_FILE};
use crate::ecosystem::active_lockfile_path;
use crate::types::{ProjectFilesSnapshot, RestoreFileParams, RestoreProjectFilesSnapshotParams};

fn get_lockfile_name(cwd: &Path) -> String {
    let lockfile_path = active_lockfile_path(cwd);

    lockfile_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(PACKAGE_LOCK_FILE)
        .to_string()
}

pub fn capture_project_files_snapshot(cwd: &Path) -> ProjectFilesSnapshot {
    let lockfile_path = active_lockfile_path(cwd);
    let lockfile_name = get_lockfile_name(cwd);

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

    if let Some(contents) = original_contents {
        return std::fs::write(file_path, contents);
    }

    let remove_result = std::fs::remove_file(file_path);

    match remove_result {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn restore_with_error_context(
    params: RestoreFileParams<'_>,
    error_context: &str,
) -> std::io::Result<()> {
    restore_file(params).map_err(|error| {
        let error_message = format!("{error_context}: {error}");

        std::io::Error::new(error.kind(), error_message)
    })
}

pub fn restore_project_files_snapshot(
    params: RestoreProjectFilesSnapshotParams<'_>,
) -> std::io::Result<()> {
    let RestoreProjectFilesSnapshotParams {
        snapshot,
        current_working_directory,
    } = params;

    let restore_package_json_params = RestoreFileParams {
        current_working_directory,
        file_name: PACKAGE_JSON_FILE,
        original_contents: &snapshot.package_json,
    };

    restore_with_error_context(restore_package_json_params, RESTORE_FAILED_PACKAGE_JSON)?;

    let restore_lockfile_params = RestoreFileParams {
        current_working_directory,
        file_name: &snapshot.lockfile_name,
        original_contents: &snapshot.lockfile_contents,
    };
    let lockfile_error_context = RESTORE_FAILED_LOCKFILE.replace("{}", &snapshot.lockfile_name);

    restore_with_error_context(restore_lockfile_params, &lockfile_error_context)?;

    Ok(())
}

fn digest_to_hex(digest: sha2::digest::Output<Sha256>) -> String {
    let mut hex_digest = String::with_capacity(digest.len() * 2);

    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut hex_digest, "{byte:02x}");
    }

    hex_digest
}

pub fn lockfile_sha256(cwd: &Path) -> Option<String> {
    let lockfile_path = active_lockfile_path(cwd);
    let bytes = std::fs::read(lockfile_path).ok()?;
    let mut hasher = Sha256::new();

    hasher.update(bytes);

    let digest = hasher.finalize();
    let hex_digest = digest_to_hex(digest);

    Some(hex_digest)
}
