use std::path::{Path, PathBuf};

use crate::types::{InstallFromVerifiedSourceParams, PackageRef};

pub(super) fn materialize_verified_tarball(
    package_ref: &PackageRef,
    tarball: crate::types::VerifiedTarball,
) -> std::io::Result<PathBuf> {
    match tarball {
        crate::types::VerifiedTarball::Spool(path) => Ok(path),
        crate::types::VerifiedTarball::Memory(bytes) => {
            let process_id = std::process::id();
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default();

            let file_name = crate::utils::build_prevalidated_tarball_file_name(
                crate::constants::paths::PREVALIDATED_TARBALL_PREFIX,
                process_id,
                nanos,
                &package_ref.name,
            );
            let path = std::env::temp_dir().join(file_name);

            std::fs::write(&path, bytes)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
            }

            crate::verifier::artifact_cleanup::register_artifact(path.clone());

            Ok(path)
        }
    }
}

pub(super) fn cleanup_materialized_tarball(path: &Path) {
    let _ = crate::verifier::artifact_cleanup::cleanup_artifact(path);

    crate::verifier::artifact_cleanup::unregister_artifact(path);
}

pub(super) fn install_from_verified_source(
    params: InstallFromVerifiedSourceParams<'_>,
) -> std::io::Result<std::process::ExitStatus> {
    use crate::utils::install_package;
    use crate::utils::install_package_source;

    let InstallFromVerifiedSourceParams {
        args,
        package_ref,
        ignore_scripts,
        prevalidated_tarball,
    } = params;

    if let Some(tarball) = prevalidated_tarball
        && let Ok(tarball_path) = materialize_verified_tarball(package_ref, tarball)
    {
        let source = tarball_path.to_string_lossy().to_string();
        let install_package_source_params = crate::types::InstallPackageSourceParams {
            current_working_directory: &args.cwd,
            package_reference: package_ref,
            package_source: &source,
            ignore_scripts,
        };
        let status = install_package_source(install_package_source_params);

        cleanup_materialized_tarball(&tarball_path);

        return status;
    }

    let install_package_params = crate::types::InstallPackageParams {
        current_working_directory: &args.cwd,
        package_reference: package_ref,
        ignore_scripts,
    };

    install_package(install_package_params)
}
