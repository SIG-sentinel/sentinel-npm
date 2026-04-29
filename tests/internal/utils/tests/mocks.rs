use crate::types::PackageRef;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub(crate) fn env_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("env test mutex should lock")
}

pub(crate) fn write_npm_ci_post_verify_stub(
    current_working_directory: &Path,
    package_ref: &PackageRef,
) -> PathBuf {
    let bin_dir = current_working_directory.join("bin");
    std::fs::create_dir_all(&bin_dir).expect("bin dir should be created");

    let npm_stub = bin_dir.join("npm");
    std::fs::write(
        &npm_stub,
        format!(
            "#!/usr/bin/env sh\nset -eu\nmkdir -p \"$PWD/node_modules/{name}\"\ncp \"$PWD/fixture-package/package.json\" \"$PWD/node_modules/{name}/package.json\"\ncp \"$PWD/fixture-package/index.js\" \"$PWD/node_modules/{name}/index.js\"\nexit 0\n",
            name = package_ref.name
        ),
    )
    .expect("npm stub should be written");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&npm_stub)
            .expect("stub metadata should exist")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&npm_stub, permissions).expect("stub should be executable");
    }

    bin_dir
}
