use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::constants::paths::{NODE_MODULES_DIR, PACKAGE_JSON_FILE};
use crate::constants::{
    INSTALL_ERR_FAILED_COMPUTE_RELATIVE_PATH, INSTALL_ERR_FAILED_PARSE_PACKAGE_MANIFEST,
    INSTALL_ERR_FAILED_READ_DIRECTORY, INSTALL_ERR_FAILED_READ_DIRECTORY_ENTRY,
    INSTALL_ERR_FAILED_READ_FILE, INSTALL_ERR_FAILED_READ_FILE_TYPE,
    INSTALL_ERR_FAILED_READ_PACKAGE_MANIFEST, INSTALL_ERR_PACKAGE_NOT_FOUND_NODE_MODULES,
    POST_VERIFY_PACKAGE_PREFIX,
};
use crate::types::{
    DirectoryEntryKind, InstallFingerprintResult, PackageRef, ProcessDirectoryEntryParams,
    ProcessInstalledPackageIndexEntryParams,
};
use crate::utils::{
    format_err_for_path, format_err_in_path, format_err_with_path, format_err_with_subject,
    format_prefixed_package_message,
};

use super::resolve::package_key;

fn normalize_relative_path(path: &Path) -> Option<String> {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let invalid_path = normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.split('/').any(|segment| segment == "..");

    if invalid_path {
        return None;
    }

    Some(normalized)
}

pub(super) fn read_package_ref_from_manifest(
    package_manifest: &Path,
) -> InstallFingerprintResult<Option<PackageRef>> {
    if !package_manifest.exists() {
        return Ok(None);
    }

    let package_manifest_contents = std::fs::read_to_string(package_manifest).map_err(|error| {
        format_err_with_subject(
            INSTALL_ERR_FAILED_READ_PACKAGE_MANIFEST,
            package_manifest,
            &error,
        )
    })?;

    let package_manifest_json =
        serde_json::from_str::<serde_json::Value>(&package_manifest_contents).map_err(|error| {
            format_err_with_subject(
                INSTALL_ERR_FAILED_PARSE_PACKAGE_MANIFEST,
                package_manifest,
                &error,
            )
        })?;

    let Some(package_name) = package_manifest_json
        .get("name")
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(None);
    };

    let Some(package_version) = package_manifest_json
        .get("version")
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(None);
    };

    Ok(Some(PackageRef::new(package_name, package_version)))
}

pub(super) fn process_installed_package_index_entry(
    params: ProcessInstalledPackageIndexEntryParams<'_>,
) -> InstallFingerprintResult<()> {
    let ProcessInstalledPackageIndexEntryParams {
        path,
        entry_result,
        index,
        pending_paths,
    } = params;

    let entry = entry_result.map_err(|error| {
        format_err_in_path(INSTALL_ERR_FAILED_READ_DIRECTORY_ENTRY, path, &error)
    })?;

    let file_type = entry.file_type().map_err(|error| {
        let entry_path = entry.path();
        format_err_for_path(INSTALL_ERR_FAILED_READ_FILE_TYPE, &entry_path, &error)
    })?;

    let is_regular_dir = file_type.is_dir() && !file_type.is_symlink();

    if !is_regular_dir {
        return Ok(());
    }

    let entry_path = entry.path();

    pending_paths.push(entry_path.clone());

    let package_manifest = entry_path.join(PACKAGE_JSON_FILE);

    if let Some(package_ref) = read_package_ref_from_manifest(&package_manifest)? {
        index.insert(package_key(&package_ref), entry_path);
    }

    Ok(())
}

pub(super) fn build_installed_package_index(
    current_working_directory: &Path,
) -> InstallFingerprintResult<HashMap<String, PathBuf>> {
    let node_modules_root = current_working_directory.join(NODE_MODULES_DIR);

    if !node_modules_root.exists() {
        return Ok(HashMap::new());
    }

    let mut index = HashMap::new();
    let mut pending_paths = vec![node_modules_root];

    while let Some(path) = pending_paths.pop() {
        let entries = std::fs::read_dir(&path).map_err(|error| {
            format_err_with_path(INSTALL_ERR_FAILED_READ_DIRECTORY, &path, &error)
        })?;

        for entry_result in entries {
            let process_entry_params = ProcessInstalledPackageIndexEntryParams {
                path: &path,
                entry_result,
                index: &mut index,
                pending_paths: &mut pending_paths,
            };

            process_installed_package_index_entry(process_entry_params)?;
        }
    }

    Ok(index)
}

pub(super) fn process_directory_entry(params: ProcessDirectoryEntryParams<'_>) {
    let ProcessDirectoryEntryParams {
        entry_path,
        file_type,
        entries,
        pending_paths,
    } = params;

    let directory_entry_kind = classify_directory_entry_kind(*file_type);

    match directory_entry_kind {
        DirectoryEntryKind::Directory => {
            pending_paths.push(entry_path);
        }
        DirectoryEntryKind::File => {
            entries.push(entry_path);
        }
        DirectoryEntryKind::SkipSymlink | DirectoryEntryKind::Other => {}
    }
}

fn classify_directory_entry_kind(file_type: std::fs::FileType) -> DirectoryEntryKind {
    let is_symlink = file_type.is_symlink();
    let is_directory = file_type.is_dir();
    let is_file = file_type.is_file();

    match (is_symlink, is_directory, is_file) {
        (true, _, _) => DirectoryEntryKind::SkipSymlink,
        (false, true, _) => DirectoryEntryKind::Directory,
        (false, false, true) => DirectoryEntryKind::File,
        (false, false, false) => DirectoryEntryKind::Other,
    }
}

pub(super) fn collect_directory_entries(root: &Path) -> InstallFingerprintResult<Vec<PathBuf>> {
    let mut entries = Vec::new();
    let mut pending_paths = vec![root.to_path_buf()];

    while let Some(path) = pending_paths.pop() {
        let read_dir = std::fs::read_dir(&path).map_err(|error| {
            format_err_with_path(INSTALL_ERR_FAILED_READ_DIRECTORY, &path, &error)
        })?;

        for entry_result in read_dir {
            let entry = entry_result.map_err(|error| {
                format_err_in_path(INSTALL_ERR_FAILED_READ_DIRECTORY_ENTRY, &path, &error)
            })?;

            let file_type = entry.file_type().map_err(|error| {
                let entry_path = entry.path();

                format_err_for_path(INSTALL_ERR_FAILED_READ_FILE_TYPE, &entry_path, &error)
            })?;

            let entry_path = entry.path();

            let process_directory_entry_params = ProcessDirectoryEntryParams {
                entry_path,
                file_type: &file_type,
                entries: &mut entries,
                pending_paths: &mut pending_paths,
            };

            process_directory_entry(process_directory_entry_params);
        }
    }

    entries.sort();

    Ok(entries)
}

pub(super) fn compute_directory_fingerprint(root: &Path) -> InstallFingerprintResult<String> {
    let entries = collect_directory_entries(root)?;
    let mut hasher = Sha256::new();

    for entry_path in entries {
        let relative_path = entry_path.strip_prefix(root).map_err(|error| {
            format_err_for_path(
                INSTALL_ERR_FAILED_COMPUTE_RELATIVE_PATH,
                &entry_path,
                &error,
            )
        })?;

        let Some(relative_path_text) = normalize_relative_path(relative_path) else {
            continue;
        };

        let file_bytes = std::fs::read(&entry_path).map_err(|error| {
            format_err_with_path(INSTALL_ERR_FAILED_READ_FILE, &entry_path, &error)
        })?;

        let file_digest = Sha256::digest(&file_bytes);

        hasher.update(relative_path_text.as_bytes());
        hasher.update([0]);
        hasher.update(file_digest);
        hasher.update([0]);
    }

    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(digest.len() * 2);

    for byte in digest {
        use std::fmt::Write as _;

        let _ = write!(&mut encoded, "{byte:02x}");
    }

    Ok(encoded)
}

pub(super) fn compute_installed_package_fingerprint(
    installed_package_index: &HashMap<String, PathBuf>,
    package_ref: &PackageRef,
) -> InstallFingerprintResult<String> {
    let installed_root = installed_package_index
        .get(&package_key(package_ref))
        .cloned()
        .ok_or_else(|| {
            format_prefixed_package_message(
                POST_VERIFY_PACKAGE_PREFIX,
                package_ref,
                INSTALL_ERR_PACKAGE_NOT_FOUND_NODE_MODULES,
            )
        })?;

    compute_directory_fingerprint(&installed_root)
}

pub(super) fn collect_installed_file_hashes(
    installed_root: &Path,
) -> InstallFingerprintResult<HashMap<String, Vec<u8>>> {
    let entries = collect_directory_entries(installed_root)?;
    let mut file_hashes = HashMap::new();

    for entry_path in entries {
        let relative_path = entry_path.strip_prefix(installed_root).map_err(|error| {
            format_err_for_path(
                INSTALL_ERR_FAILED_COMPUTE_RELATIVE_PATH,
                &entry_path,
                &error,
            )
        })?;

        let Some(relative_path_text) = normalize_relative_path(relative_path) else {
            continue;
        };

        let file_bytes = std::fs::read(&entry_path).map_err(|error| {
            format_err_with_path(INSTALL_ERR_FAILED_READ_FILE, &entry_path, &error)
        })?;

        let file_digest = Sha256::digest(&file_bytes).to_vec();

        file_hashes.insert(relative_path_text, file_digest);
    }

    Ok(file_hashes)
}
