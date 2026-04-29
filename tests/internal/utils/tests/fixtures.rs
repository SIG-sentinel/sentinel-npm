use crate::types::{Evidence, PackageRef, Verdict, VerifyResult};
use std::path::Path;

pub(crate) fn build_clean_verify_result(
    package_ref: &PackageRef,
    tarball_fingerprint: String,
) -> VerifyResult {
    VerifyResult {
        package: package_ref.clone(),
        verdict: Verdict::Clean,
        detail: "clean package for post-verify test".to_string(),
        evidence: Evidence::empty(),
        is_direct: true,
        direct_parent: None,
        tarball_fingerprint: Some(tarball_fingerprint),
    }
}

pub(crate) fn create_installed_package_fixture<F>(
    current_working_directory: &Path,
    package_ref: &PackageRef,
    compute_directory_fingerprint: F,
) -> String
where
    F: Fn(&Path) -> Result<String, String>,
{
    let fixture_root = current_working_directory.join("fixture-package");
    std::fs::create_dir_all(&fixture_root).expect("fixture root should be created");
    std::fs::write(
        fixture_root.join("package.json"),
        format!(
            "{{\"name\":\"{}\",\"version\":\"{}\"}}",
            package_ref.name, package_ref.version
        ),
    )
    .expect("fixture package.json should be written");
    std::fs::write(
        fixture_root.join("index.js"),
        "module.exports = 'sentinel';\n",
    )
    .expect("fixture index.js should be written");

    compute_directory_fingerprint(&fixture_root).expect("fixture fingerprint should be computed")
}
