#![allow(
    clippy::expect_used,
    clippy::await_holding_lock,
    clippy::needless_borrows_for_generic_args
)]

use super::{
    collect_install_packages_to_verify, compute_directory_fingerprint, ensure_lockfile_exists,
    finalize_ci_run, finalize_install_run, find_missing_post_verify_packages,
    parse_install_package_request, parse_package_ref, print_blocking_install_results,
    print_ci_blocking_results, resolve_install_candidate_package, resolve_install_policy,
    should_print_report,
};
use crate::cache::LocalCache;
use crate::types::{
    CiArgs, CollectInstallPackagesParams, DependencyNode, DependencyTree,
    EnsureLockfileExistsForInstallParams, Evidence, FinalizeCiRunParams, FinalizeInstallRunParams,
    InstallArgs, InstallPolicyDecision, OutputFormat, PackageRef, PrintCiBlockingResultsParams,
    Report, ResolveInstallPolicyParams, RestoreProjectFilesSnapshotParams, RunMode,
    ShouldPrintReportParams, UnverifiableReason, Verdict, VerifiedTarball, VerifyResult,
};
use crate::utils::{
    capture_project_files_snapshot, lockfile_sha256, restore_project_files_snapshot,
};
use crate::verifier::compute_tarball_fingerprint_bytes;
use std::process::ExitCode;
use std::sync::{Mutex, OnceLock};

fn env_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("env test mutex should lock")
}

#[test]
fn test_ensure_lockfile_exists_returns_true_when_file_exists() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let package_json = temp_dir.path().join("package.json");
    let lockfile = temp_dir.path().join("package-lock.json");
    std::fs::write(&package_json, "{\"name\":\"demo\",\"version\":\"1.0.0\"}")
        .expect("package.json should be written");
    std::fs::write(&lockfile, "{}").expect("lockfile should be written");

    let ok = ensure_lockfile_exists(EnsureLockfileExistsForInstallParams {
        current_working_directory: temp_dir.path(),
    });
    assert!(ok);
}

#[test]
fn test_parse_package_ref_plain_package() {
    let parsed = parse_package_ref("lodash@4.17.21").expect("package should parse");
    assert_eq!(parsed.name, "lodash");
    assert_eq!(parsed.version, "4.17.21");
}

#[test]
fn test_parse_package_ref_scoped_package() {
    let parsed = parse_package_ref("@scope/pkg@1.2.3").expect("scoped package should parse");
    assert_eq!(parsed.name, "@scope/pkg");
    assert_eq!(parsed.version, "1.2.3");
}

#[test]
fn test_parse_package_ref_missing_version() {
    let parsed = parse_package_ref("left-pad");
    assert!(parsed.is_none());
}

#[test]
fn test_parse_install_package_request_accepts_package_without_version() {
    let parsed = parse_install_package_request("left-pad")
        .expect("package without version should parse for candidate resolution");

    assert_eq!(parsed.package_name, "left-pad");
    assert!(parsed.version_spec.is_none());
}

#[test]
fn test_parse_install_package_request_accepts_latest_tag() {
    let parsed = parse_install_package_request("left-pad@latest")
        .expect("latest tag should parse for candidate resolution");

    assert_eq!(parsed.package_name, "left-pad");
    assert_eq!(parsed.version_spec.as_deref(), Some("latest"));
}

#[test]
fn test_resolve_install_candidate_package_prefers_direct_dependency() {
    let mut dependency_tree = DependencyTree::new();

    dependency_tree.insert(DependencyNode {
        package: PackageRef::new("left-pad", "1.3.0"),
        dependencies: vec![],
        is_dev: false,
        is_direct: false,
        direct_parent: Some("app@1.0.0".to_string()),
    });
    dependency_tree.insert(DependencyNode {
        package: PackageRef::new("left-pad", "1.1.3"),
        dependencies: vec![],
        is_dev: false,
        is_direct: true,
        direct_parent: None,
    });

    let install_request =
        parse_install_package_request("left-pad").expect("install request should parse");
    let candidate = resolve_install_candidate_package(&dependency_tree, &install_request)
        .expect("candidate should be resolved");

    assert_eq!(candidate, PackageRef::new("left-pad", "1.1.3"));
}

#[test]
fn test_resolve_install_candidate_package_uses_highest_semver_for_latest_tag() {
    let mut dependency_tree = DependencyTree::new();

    dependency_tree.insert(DependencyNode {
        package: PackageRef::new("left-pad", "1.0.0"),
        dependencies: vec![],
        is_dev: false,
        is_direct: true,
        direct_parent: None,
    });
    dependency_tree.insert(DependencyNode {
        package: PackageRef::new("left-pad", "1.3.0"),
        dependencies: vec![],
        is_dev: false,
        is_direct: true,
        direct_parent: None,
    });

    let install_request =
        parse_install_package_request("left-pad@latest").expect("install request should parse");
    let candidate = resolve_install_candidate_package(&dependency_tree, &install_request)
        .expect("candidate should be resolved");

    assert_eq!(candidate, PackageRef::new("left-pad", "1.3.0"));
}

#[test]
fn test_resolve_install_candidate_package_uses_highest_matching_semver_range() {
    let mut dependency_tree = DependencyTree::new();

    dependency_tree.insert(DependencyNode {
        package: PackageRef::new("left-pad", "1.1.0"),
        dependencies: vec![],
        is_dev: false,
        is_direct: true,
        direct_parent: None,
    });
    dependency_tree.insert(DependencyNode {
        package: PackageRef::new("left-pad", "1.2.5"),
        dependencies: vec![],
        is_dev: false,
        is_direct: true,
        direct_parent: None,
    });
    dependency_tree.insert(DependencyNode {
        package: PackageRef::new("left-pad", "2.0.0"),
        dependencies: vec![],
        is_dev: false,
        is_direct: true,
        direct_parent: None,
    });

    let install_request =
        parse_install_package_request("left-pad@^1.0.0").expect("install request should parse");
    let candidate = resolve_install_candidate_package(&dependency_tree, &install_request)
        .expect("candidate should be resolved");

    assert_eq!(candidate, PackageRef::new("left-pad", "1.2.5"));
}

#[test]
fn test_collect_install_packages_includes_target_and_transitives() {
    let mut dependency_tree = DependencyTree::new();

    dependency_tree.insert(DependencyNode {
        package: PackageRef::new("a", "1.0.0"),
        dependencies: vec!["b@1.0.0".to_string(), "c@1.0.0".to_string()],
        is_dev: false,
        is_direct: true,
        direct_parent: None,
    });
    dependency_tree.insert(DependencyNode {
        package: PackageRef::new("b", "1.0.0"),
        dependencies: vec!["d@1.0.0".to_string()],
        is_dev: false,
        is_direct: false,
        direct_parent: Some("a@1.0.0".to_string()),
    });
    dependency_tree.insert(DependencyNode {
        package: PackageRef::new("c", "1.0.0"),
        dependencies: vec![],
        is_dev: false,
        is_direct: false,
        direct_parent: Some("a@1.0.0".to_string()),
    });
    dependency_tree.insert(DependencyNode {
        package: PackageRef::new("d", "1.0.0"),
        dependencies: vec![],
        is_dev: false,
        is_direct: false,
        direct_parent: Some("a@1.0.0".to_string()),
    });

    let packages_to_verify = collect_install_packages_to_verify(CollectInstallPackagesParams {
        dependency_tree: &dependency_tree,
        package_reference: &PackageRef::new("a", "1.0.0"),
    })
    .expect("target package should be present");

    let packages: Vec<String> = packages_to_verify
        .into_iter()
        .map(|node| node.package.to_string())
        .collect();

    assert_eq!(packages, vec!["a@1.0.0", "b@1.0.0", "c@1.0.0", "d@1.0.0"]);
}

#[test]
fn test_find_missing_post_verify_packages_returns_empty_when_target_exists() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let package_dir = temp_dir.path().join("node_modules").join("lodash");
    std::fs::create_dir_all(&package_dir).expect("node_modules/lodash should be created");
    std::fs::write(
        package_dir.join("package.json"),
        r#"{"name":"lodash","version":"4.17.21"}"#,
    )
    .expect("package manifest should be written");

    let packages = vec![PackageRef::new("lodash", "4.17.21")];
    let missing = find_missing_post_verify_packages(temp_dir.path(), &packages);

    assert!(
        missing.is_empty(),
        "target package should be found in node_modules"
    );
}

#[test]
fn test_find_missing_post_verify_packages_returns_missing_when_not_installed() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    std::fs::create_dir_all(temp_dir.path().join("node_modules"))
        .expect("node_modules should be created");

    let packages = vec![PackageRef::new("left-pad", "1.3.0")];
    let missing = find_missing_post_verify_packages(temp_dir.path(), &packages);

    assert_eq!(missing, vec![PackageRef::new("left-pad", "1.3.0")]);
}

#[test]
fn test_compute_directory_fingerprint_changes_when_file_content_changes() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let package_root = temp_dir.path().join("node_modules").join("left-pad");
    std::fs::create_dir_all(&package_root).expect("package root should be created");
    std::fs::write(
        package_root.join("package.json"),
        r#"{"name":"left-pad","version":"1.3.0"}"#,
    )
    .expect("package.json should be written");
    std::fs::write(package_root.join("index.js"), "module.exports = 'a';\n")
        .expect("index.js should be written");

    let fingerprint_before =
        compute_directory_fingerprint(&package_root).expect("fingerprint should be computed");

    std::fs::write(package_root.join("index.js"), "module.exports = 'b';\n")
        .expect("index.js should be updated");
    let fingerprint_after =
        compute_directory_fingerprint(&package_root).expect("fingerprint should be computed");

    assert_ne!(
        fingerprint_before, fingerprint_after,
        "fingerprint must change when file content changes"
    );
}

#[test]
fn test_compute_tarball_fingerprint_matches_installed_directory_fingerprint() {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use tar::Builder;

    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let package_root = temp_dir.path().join("installed");
    std::fs::create_dir_all(package_root.join("lib")).expect("package dirs should be created");
    std::fs::write(
        package_root.join("package.json"),
        r#"{"name":"left-pad","version":"1.3.0"}"#,
    )
    .expect("package.json should be written");
    std::fs::write(
        package_root.join("lib").join("index.js"),
        "module.exports = 42;\n",
    )
    .expect("lib/index.js should be written");

    let expected_fingerprint =
        compute_directory_fingerprint(&package_root).expect("fingerprint should be computed");

    let tarball_path = temp_dir.path().join("left-pad-1.3.0.tgz");
    let tarball_file = std::fs::File::create(&tarball_path).expect("tarball should be created");
    let encoder = GzEncoder::new(tarball_file, Compression::default());
    let mut builder = Builder::new(encoder);
    builder
        .append_path_with_name(package_root.join("package.json"), "package/package.json")
        .expect("package.json should be added to tarball");
    builder
        .append_path_with_name(
            package_root.join("lib").join("index.js"),
            "package/lib/index.js",
        )
        .expect("index.js should be added to tarball");
    builder.finish().expect("tar entries should be finalized");
    let encoder = builder
        .into_inner()
        .expect("tar builder should return encoder");
    encoder.finish().expect("gzip stream should be finalized");

    let tarball_bytes = std::fs::read(&tarball_path).expect("tarball bytes should be readable");
    let package_ref = PackageRef::new("left-pad", "1.3.0");
    let tarball_fingerprint = compute_tarball_fingerprint_bytes(&tarball_bytes, &package_ref)
        .expect("tarball fingerprint should be computed");

    assert_eq!(
        expected_fingerprint, tarball_fingerprint,
        "tarball and installed package fingerprints must match"
    );
}

#[test]
fn test_should_print_report_suppresses_quiet_text_output() {
    let should_print = should_print_report(ShouldPrintReportParams {
        output_format: &OutputFormat::Text,
        quiet: true,
    });

    assert!(!should_print);
}

#[test]
fn test_should_print_report_keeps_quiet_json_output() {
    let should_print = should_print_report(ShouldPrintReportParams {
        output_format: &OutputFormat::Json,
        quiet: true,
    });

    assert!(should_print);
}

#[test]
fn test_finalize_ci_run_aborts_when_lockfile_hash_changes_before_install() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    std::fs::write(temp_dir.path().join("package-lock.json"), "{}")
        .expect("lockfile should be written");

    let args = CiArgs {
        omit_dev: false,
        omit_optional: false,
        allow_scripts: false,
        dry_run: false,
        post_verify: false,
        init_lockfile: false,
        format: OutputFormat::Json,
        report: temp_dir.path().join("report.json"),
        cwd: temp_dir.path().to_path_buf(),
        package_manager: None,
        timeout: 1000,
        quiet: true,
    };

    let report = Report::from_results(RunMode::Ci, vec![], vec![]);
    let previous_hash = Some("different-hash-before-verify".to_string());

    let exit = finalize_ci_run(FinalizeCiRunParams {
        args: &args,
        report: &report,
        lock_hash_before_verify: &previous_hash,
    });

    assert_eq!(exit, ExitCode::FAILURE);
}

#[test]
fn test_finalize_install_run_aborts_when_lockfile_hash_changes_before_install() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    std::fs::write(temp_dir.path().join("package-lock.json"), "{}")
        .expect("lockfile should be written");

    let args = InstallArgs {
        package: "lodash@4.17.21".to_string(),
        allow_scripts: false,
        dry_run: false,
        post_verify: false,
        format: OutputFormat::Json,
        cwd: temp_dir.path().to_path_buf(),
        package_manager: None,
        timeout: 1000,
        quiet: true,
    };

    let report = Report::from_results(RunMode::Install, vec![], vec![]);
    let package_ref = PackageRef::new("lodash", "4.17.21");
    let previous_hash = Some("different-hash-before-verify".to_string());

    let outcome = finalize_install_run(FinalizeInstallRunParams {
        args: &args,
        package_ref: &package_ref,
        report: &report,
        lock_hash_before_verify: &previous_hash,
        prevalidated_tarball: None,
    });

    assert_eq!(outcome.exit_code, ExitCode::FAILURE);
    assert!(outcome.should_restore_snapshot);
}

#[test]
fn test_lockfile_sha256_changes_after_mutation() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let lockfile = temp_dir.path().join("package-lock.json");
    std::fs::write(&lockfile, "{\"name\":\"demo\"}").expect("lockfile should be written");
    let before = lockfile_sha256(temp_dir.path()).expect("hash before should exist");

    std::fs::write(&lockfile, "{\"name\":\"demo\",\"version\":\"1.0.0\"}")
        .expect("lockfile should be mutated");
    let after = lockfile_sha256(temp_dir.path()).expect("hash after should exist");

    assert_ne!(before, after);
}

#[test]
fn test_lockfile_sha256_uses_active_yarn_lockfile() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let lockfile = temp_dir.path().join("yarn.lock");
    std::fs::write(&lockfile, "pkg@^1.0.0:\n  version \"1.0.0\"\n")
        .expect("yarn.lock should be written");
    let before = lockfile_sha256(temp_dir.path()).expect("hash before should exist");

    std::fs::write(
        &lockfile,
        "pkg@^1.0.0:\n  version \"1.0.1\"\n  integrity \"sha512-test\"\n",
    )
    .expect("yarn.lock should be mutated");
    let after = lockfile_sha256(temp_dir.path()).expect("hash after should exist");

    assert_ne!(before, after);
}

#[test]
fn test_rollback_restores_package_json_when_install_blocked() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let package_json = temp_dir.path().join("package.json");
    let lockfile = temp_dir.path().join("package-lock.json");

    let original_package_json = b"{\"name\":\"demo\",\"version\":\"1.0.0\"}";
    let original_lockfile = b"{\"name\":\"demo\",\"lockfileVersion\":3}";

    std::fs::write(&package_json, original_package_json).expect("package.json should be written");
    std::fs::write(&lockfile, original_lockfile).expect("lockfile should be written");

    let snapshot = capture_project_files_snapshot(temp_dir.path());

    std::fs::write(
        &package_json,
        b"{\"name\":\"demo\",\"dependencies\":{\"bad\":\"1.0.0\"}}",
    )
    .expect("mutated package.json should be written");
    std::fs::write(&lockfile, b"{\"packages\":{\"node_modules/bad\":{}}}")
        .expect("mutated lockfile should be written");

    restore_project_files_snapshot(RestoreProjectFilesSnapshotParams {
        snapshot: &snapshot,
        current_working_directory: temp_dir.path(),
    })
    .expect("snapshot restore should succeed");

    let restored_package_json = std::fs::read(&package_json).expect("package.json should exist");
    let restored_lockfile = std::fs::read(&lockfile).expect("lockfile should exist");

    assert_eq!(restored_package_json, original_package_json);
    assert_eq!(restored_lockfile, original_lockfile);
}

#[test]
fn test_dry_run_blocked_path_keeps_no_mutation_after_rollback() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let package_json = temp_dir.path().join("package.json");
    let lockfile = temp_dir.path().join("package-lock.json");

    let original_package_json = b"{\"name\":\"demo\",\"version\":\"1.0.0\"}";
    std::fs::write(&package_json, original_package_json).expect("package.json should be written");
    assert!(!lockfile.exists(), "lockfile must not exist at test start");

    let snapshot = capture_project_files_snapshot(temp_dir.path());

    std::fs::write(
        &package_json,
        b"{\"name\":\"demo\",\"dependencies\":{\"risky\":\"9.9.9\"}}",
    )
    .expect("mutated package.json should be written");
    std::fs::write(&lockfile, b"{\"packages\":{\"node_modules/risky\":{}}}")
        .expect("generated lockfile should be written");

    restore_project_files_snapshot(RestoreProjectFilesSnapshotParams {
        snapshot: &snapshot,
        current_working_directory: temp_dir.path(),
    })
    .expect("snapshot restore should succeed");

    let restored_package_json = std::fs::read(&package_json).expect("package.json should exist");
    assert_eq!(restored_package_json, original_package_json);
    assert!(
        !lockfile.exists(),
        "package-lock.json should be removed after rollback"
    );
}

#[test]
fn test_snapshot_restore_supports_yarn_lockfile() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let package_json = temp_dir.path().join("package.json");
    let lockfile = temp_dir.path().join("yarn.lock");

    let original_package_json = b"{\"name\":\"demo\",\"version\":\"1.0.0\"}";
    let original_lockfile = b"pkg@^1.0.0:\n  version \"1.0.0\"\n";

    std::fs::write(&package_json, original_package_json).expect("package.json should be written");
    std::fs::write(&lockfile, original_lockfile).expect("yarn.lock should be written");

    let snapshot = capture_project_files_snapshot(temp_dir.path());

    std::fs::write(&package_json, b"{\"name\":\"demo\",\"private\":true}")
        .expect("mutated package.json should be written");
    std::fs::write(&lockfile, b"pkg@^1.0.0:\n  version \"9.9.9\"\n")
        .expect("mutated yarn.lock should be written");

    restore_project_files_snapshot(RestoreProjectFilesSnapshotParams {
        snapshot: &snapshot,
        current_working_directory: temp_dir.path(),
    })
    .expect("snapshot restore should succeed");

    let restored_package_json = std::fs::read(&package_json).expect("package.json should exist");
    let restored_lockfile = std::fs::read(&lockfile).expect("yarn.lock should exist");

    assert_eq!(restored_package_json, original_package_json);
    assert_eq!(restored_lockfile, original_lockfile);
}

#[tokio::test]
async fn test_run_install_rolls_back_when_npm_install_command_fails() {
    let _guard = env_test_lock();
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let cwd = temp_dir.path();

    let package_json = cwd.join("package.json");
    let lockfile = cwd.join("package-lock.json");

    let original_package_json = r#"{
  "name": "demo",
  "version": "1.0.0"
}"#;
    std::fs::write(&package_json, original_package_json).expect("package.json should be written");
    std::fs::write(&lockfile, "{}")
        .expect("lockfile should be seeded to skip lockfile generation path");

    let bin_dir = cwd.join("bin");
    std::fs::create_dir_all(&bin_dir).expect("bin dir should be created");
    let npm_stub = bin_dir.join("npm");
    std::fs::write(
        &npm_stub,
        r#"#!/usr/bin/env sh
args="$*"
case "$args" in
        printf '{"name":"demo","lockfileVersion":3,"packages":{"":{"name":"demo","version":"1.0.0"},"node_modules/lodash":{"version":"4.17.21","integrity":"sha512-xyz","dev":false}}}' > package-lock.json
    printf '{"name":"demo","version":"1.0.0","dependencies":{"lodash":"4.17.21"}}' > package.json
    exit 0
    ;;
    exit 55
    ;;
esac
"#,
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

    let previous_path = std::env::var("PATH").unwrap_or_default();
    let patched_path = format!("{}:{}", bin_dir.display(), previous_path);
    let home_dir = cwd.join("home");
    std::fs::create_dir_all(&home_dir).expect("home dir should be created");

    let exit = temp_env::async_with_vars(
        [
            ("PATH", Some(std::ffi::OsString::from(&patched_path))),
            ("HOME", Some(home_dir.as_os_str().to_os_string())),
        ],
        async {
            let cache = LocalCache::open(None).expect("cache should open");
            let cached_clean = VerifyResult {
                package: PackageRef::new("lodash", "4.17.21"),
                verdict: Verdict::Clean,
                detail: "cached clean for test".to_string(),
                evidence: Evidence {
                    lockfile_integrity: Some("sha512-xyz".to_string()),
                    ..Evidence::empty()
                },
                is_direct: false,
                direct_parent: None,
                tarball_fingerprint: None,
            };
            cache.put(&cached_clean);

            let args = InstallArgs {
                package: "lodash@4.17.21".to_string(),
                allow_scripts: false,
                dry_run: false,
                post_verify: false,
                format: OutputFormat::Json,
                cwd: cwd.to_path_buf(),
                package_manager: None,
                timeout: 1,
                quiet: true,
            };

            super::run_install(&args).await
        },
    )
    .await;

    assert_eq!(exit, ExitCode::FAILURE);

    let package_after = std::fs::read_to_string(&package_json).expect("package.json should exist");
    assert_eq!(package_after, original_package_json);

    let lock_after = std::fs::read_to_string(&lockfile).expect("lockfile should exist");
    assert_eq!(lock_after, "{}");
}

#[test]
fn test_finalize_install_run_uses_spool_tarball_and_cleans_up_temp_file() {
    let _guard = env_test_lock();
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let cwd = temp_dir.path();

    std::fs::write(cwd.join("package-lock.json"), "{}").expect("lockfile should be written");

    let package_json = cwd.join("package.json");
    std::fs::write(&package_json, r#"{"name":"demo","version":"1.0.0"}"#)
        .expect("package.json should be written");

    let spool_path = cwd.join("verified-package.tgz");
    std::fs::write(&spool_path, b"fake-tarball-bytes").expect("spool file should be created");

    let bin_dir = cwd.join("bin");
    std::fs::create_dir_all(&bin_dir).expect("bin dir should be created");
    let npm_stub = bin_dir.join("npm");
    let args_log = cwd.join("install-args.log");

    std::fs::write(
        &npm_stub,
        format!(
            "#!/usr/bin/env sh\nprintf '%s\n' \"$*\" > \"{}\"\nexit 0\n",
            args_log.display()
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

    let previous_path = std::env::var("PATH").unwrap_or_default();
    let patched_path = format!("{}:{}", bin_dir.display(), previous_path);

    let outcome = temp_env::with_var("PATH", Some(patched_path.as_str()), || {
        let args = InstallArgs {
            package: "lodash@4.17.21".to_string(),
            allow_scripts: false,
            dry_run: false,
            post_verify: false,
            format: OutputFormat::Json,
            cwd: cwd.to_path_buf(),
            package_manager: None,
            timeout: 1000,
            quiet: true,
        };

        let report = Report::from_results(RunMode::Install, vec![], vec![]);
        let package_ref = PackageRef::new("lodash", "4.17.21");
        let lock_hash_before_verify = lockfile_sha256(cwd);

        finalize_install_run(FinalizeInstallRunParams {
            args: &args,
            package_ref: &package_ref,
            report: &report,
            lock_hash_before_verify: &lock_hash_before_verify,
            prevalidated_tarball: Some(VerifiedTarball::Spool(spool_path.clone())),
        })
    });

    assert_eq!(outcome.exit_code, ExitCode::SUCCESS);
    assert!(!outcome.should_restore_snapshot);
    assert!(
        !spool_path.exists(),
        "spool file should be removed after install"
    );

    let args_used = std::fs::read_to_string(&args_log).expect("args log should exist");
    assert!(
        args_used.contains(&spool_path.to_string_lossy().to_string()),
        "npm install should use prevalidated tarball path"
    );
}

#[test]
fn test_finalize_install_run_uses_yarn_cache_prewarm_without_rewriting_dependency_source() {
    let _guard = env_test_lock();
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let cwd = temp_dir.path();

    std::fs::write(cwd.join("yarn.lock"), "pkg@^1.0.0:\n  version \"1.0.0\"\n")
        .expect("lockfile should be written");
    std::fs::write(
        &cwd.join("package.json"),
        r#"{"name":"demo","version":"1.0.0"}"#,
    )
    .expect("package.json should be written");

    let spool_path = cwd.join("verified-package.tgz");
    std::fs::write(&spool_path, b"fake-tarball-bytes").expect("spool file should be created");

    let bin_dir = cwd.join("bin");
    std::fs::create_dir_all(&bin_dir).expect("bin dir should be created");
    let yarn_stub = bin_dir.join("yarn");
    let args_log = cwd.join("yarn-install-args.log");

    std::fs::write(
        &yarn_stub,
        format!(
            "#!/usr/bin/env sh\nprintf '%s\\n' \"$*\" >> \"{}\"\nexit 0\n",
            args_log.display()
        ),
    )
    .expect("yarn stub should be written");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&yarn_stub)
            .expect("stub metadata should exist")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&yarn_stub, permissions).expect("stub should be executable");
    }

    let previous_path = std::env::var("PATH").unwrap_or_default();
    let patched_path = format!("{}:{}", bin_dir.display(), previous_path);

    let outcome = temp_env::with_var("PATH", Some(patched_path.as_str()), || {
        let args = InstallArgs {
            package: "lodash@4.17.21".to_string(),
            allow_scripts: false,
            dry_run: false,
            post_verify: false,
            format: OutputFormat::Json,
            cwd: cwd.to_path_buf(),
            package_manager: Some("yarn".to_string()),
            timeout: 1000,
            quiet: true,
        };

        let report = Report::from_results(RunMode::Install, vec![], vec![]);
        let package_ref = PackageRef::new("lodash", "4.17.21");
        let lock_hash_before_verify = lockfile_sha256(cwd);

        finalize_install_run(FinalizeInstallRunParams {
            args: &args,
            package_ref: &package_ref,
            report: &report,
            lock_hash_before_verify: &lock_hash_before_verify,
            prevalidated_tarball: Some(VerifiedTarball::Spool(spool_path.clone())),
        })
    });

    assert_eq!(outcome.exit_code, ExitCode::SUCCESS);
    assert!(
        !spool_path.exists(),
        "spool file should be removed after install"
    );

    let commands: Vec<_> = std::fs::read_to_string(&args_log)
        .expect("args log should exist")
        .lines()
        .map(str::to_string)
        .collect();

    assert_eq!(
        commands.len(),
        2,
        "yarn should preload cache and then install by package ref"
    );
    assert!(commands[0].contains(&spool_path.to_string_lossy().to_string()));
    assert!(commands[0].contains("--no-lockfile"));
    assert!(commands[0].contains("--cache-folder"));
    assert!(commands[1].contains("lodash@4.17.21"));
    assert!(commands[1].contains("--exact"));
    assert!(commands[1].contains("--prefer-offline"));
    assert!(commands[1].contains("--cache-folder"));
    assert!(!commands[1].contains(&spool_path.to_string_lossy().to_string()));
}

#[test]
fn test_finalize_install_run_uses_pnpm_store_prewarm_without_rewriting_dependency_source() {
    let _guard = env_test_lock();
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let cwd = temp_dir.path();

    std::fs::write(
        cwd.join("pnpm-lock.yaml"),
        "lockfileVersion: '9.0'\npackages: {}\n",
    )
    .expect("lockfile should be written");
    std::fs::write(
        &cwd.join("package.json"),
        r#"{"name":"demo","version":"1.0.0"}"#,
    )
    .expect("package.json should be written");

    let spool_path = cwd.join("verified-package.tgz");
    std::fs::write(&spool_path, b"fake-tarball-bytes").expect("spool file should be created");

    let bin_dir = cwd.join("bin");
    std::fs::create_dir_all(&bin_dir).expect("bin dir should be created");
    let pnpm_stub = bin_dir.join("pnpm");
    let args_log = cwd.join("pnpm-install-args.log");

    std::fs::write(
        &pnpm_stub,
        format!(
            "#!/usr/bin/env sh\nprintf '%s\\n' \"$*\" >> \"{}\"\nexit 0\n",
            args_log.display()
        ),
    )
    .expect("pnpm stub should be written");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&pnpm_stub)
            .expect("stub metadata should exist")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&pnpm_stub, permissions).expect("stub should be executable");
    }

    let previous_path = std::env::var("PATH").unwrap_or_default();
    let patched_path = format!("{}:{}", bin_dir.display(), previous_path);

    let outcome = temp_env::with_var("PATH", Some(patched_path.as_str()), || {
        let args = InstallArgs {
            package: "lodash@4.17.21".to_string(),
            allow_scripts: false,
            dry_run: false,
            post_verify: false,
            format: OutputFormat::Json,
            cwd: cwd.to_path_buf(),
            package_manager: Some("pnpm".to_string()),
            timeout: 1000,
            quiet: true,
        };

        let report = Report::from_results(RunMode::Install, vec![], vec![]);
        let package_ref = PackageRef::new("lodash", "4.17.21");
        let lock_hash_before_verify = lockfile_sha256(cwd);

        finalize_install_run(FinalizeInstallRunParams {
            args: &args,
            package_ref: &package_ref,
            report: &report,
            lock_hash_before_verify: &lock_hash_before_verify,
            prevalidated_tarball: Some(VerifiedTarball::Spool(spool_path.clone())),
        })
    });

    assert_eq!(outcome.exit_code, ExitCode::SUCCESS);
    assert!(
        !spool_path.exists(),
        "spool file should be removed after install"
    );

    let commands: Vec<_> = std::fs::read_to_string(&args_log)
        .expect("args log should exist")
        .lines()
        .map(str::to_string)
        .collect();

    assert_eq!(
        commands.len(),
        2,
        "pnpm should preload store and then install by package ref"
    );
    assert!(commands[0].contains("store"));
    assert!(commands[0].contains("add"));
    assert!(commands[0].contains(&spool_path.to_string_lossy().to_string()));
    assert!(commands[1].contains("lodash@4.17.21"));
    assert!(commands[1].contains("--save-exact"));
    assert!(commands[1].contains("--prefer-offline"));
    assert!(!commands[1].contains(&spool_path.to_string_lossy().to_string()));
}

#[test]
fn test_compute_tarball_fingerprint_ignores_symlinks() {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use tar::Builder;

    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let package_root = temp_dir.path().join("pkg");
    std::fs::create_dir_all(&package_root).expect("package dir should be created");
    std::fs::write(package_root.join("index.js"), "module.exports = 1;\n")
        .expect("index.js should be written");

    let package_ref = PackageRef::new("my-pkg", "1.0.0");

    let tarball_plain = {
        let path = temp_dir.path().join("plain.tgz");
        let file = std::fs::File::create(&path).expect("tarball file should be created");
        let enc = GzEncoder::new(file, Compression::default());
        let mut builder = Builder::new(enc);
        builder
            .append_path_with_name(package_root.join("index.js"), "package/index.js")
            .expect("index.js should be added");
        builder.finish().expect("tar should be finalized");
        builder
            .into_inner()
            .expect("encoder")
            .finish()
            .expect("gz should be finalized");
        std::fs::read(&path).expect("tarball bytes should be readable")
    };

    let tarball_with_symlink = {
        let path = temp_dir.path().join("with_symlink.tgz");
        let file = std::fs::File::create(&path).expect("tarball file should be created");
        let enc = GzEncoder::new(file, Compression::default());
        let mut builder = Builder::new(enc);
        builder
            .append_path_with_name(package_root.join("index.js"), "package/index.js")
            .expect("index.js should be added");
        let mut symlink_header = tar::Header::new_gnu();
        symlink_header.set_entry_type(tar::EntryType::Symlink);
        symlink_header.set_size(0);
        symlink_header
            .set_path("package/alias.js")
            .expect("symlink path should be set");
        symlink_header
            .set_link_name("index.js")
            .expect("symlink target should be set");
        symlink_header.set_mode(0o777);
        symlink_header.set_cksum();
        builder
            .append(&symlink_header, std::io::empty())
            .expect("symlink entry should be appended");
        builder.finish().expect("tar should be finalized");
        builder
            .into_inner()
            .expect("encoder")
            .finish()
            .expect("gz should be finalized");
        std::fs::read(&path).expect("tarball bytes should be readable")
    };

    let fp_plain = compute_tarball_fingerprint_bytes(&tarball_plain, &package_ref)
        .expect("plain fingerprint should be computed");
    let fp_with_symlink = compute_tarball_fingerprint_bytes(&tarball_with_symlink, &package_ref)
        .expect("symlink fingerprint should be computed");

    assert_eq!(
        fp_plain, fp_with_symlink,
        "symlinks must not affect tarball fingerprint"
    );
}

#[test]
#[cfg(unix)]
fn test_compute_directory_fingerprint_ignores_symlinks() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let package_root = temp_dir.path().join("pkg");
    std::fs::create_dir_all(&package_root).expect("package dir should be created");
    std::fs::write(package_root.join("index.js"), "module.exports = 1;\n")
        .expect("index.js should be written");

    let fp_without_symlink =
        compute_directory_fingerprint(&package_root).expect("fingerprint should be computed");

    symlink(package_root.join("index.js"), package_root.join("alias.js"))
        .expect("symlink should be created");

    let fp_with_symlink =
        compute_directory_fingerprint(&package_root).expect("fingerprint should be computed");

    assert_eq!(
        fp_without_symlink, fp_with_symlink,
        "symlinks must not affect directory fingerprint"
    );
}

#[test]
#[cfg(unix)]
fn test_tarball_and_directory_fingerprints_consistent_when_symlink_present() {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::os::unix::fs::symlink;
    use tar::Builder;

    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let package_root = temp_dir.path().join("installed");
    std::fs::create_dir_all(&package_root).expect("installed dir should be created");
    std::fs::write(package_root.join("index.js"), "module.exports = 42;\n")
        .expect("index.js should be written");
    symlink(package_root.join("index.js"), package_root.join("alias.js"))
        .expect("symlink should be created");

    let dir_fp =
        compute_directory_fingerprint(&package_root).expect("directory fingerprint should work");

    let tarball_path = temp_dir.path().join("pkg.tgz");
    let tarball_file =
        std::fs::File::create(&tarball_path).expect("tarball file should be created");
    let enc = GzEncoder::new(tarball_file, Compression::default());
    let mut builder = Builder::new(enc);
    builder
        .append_path_with_name(package_root.join("index.js"), "package/index.js")
        .expect("index.js should be added");
    let mut symlink_header = tar::Header::new_gnu();
    symlink_header.set_entry_type(tar::EntryType::Symlink);
    symlink_header.set_size(0);
    symlink_header
        .set_path("package/alias.js")
        .expect("symlink path should be set");
    symlink_header
        .set_link_name("index.js")
        .expect("symlink target should be set");
    symlink_header.set_mode(0o777);
    symlink_header.set_cksum();
    builder
        .append(&symlink_header, std::io::empty())
        .expect("symlink entry should be appended");
    builder.finish().expect("tar should be finalized");
    builder
        .into_inner()
        .expect("encoder")
        .finish()
        .expect("gz should be finalized");

    let tarball_bytes = std::fs::read(&tarball_path).expect("tarball bytes should be readable");
    let package_ref = PackageRef::new("my-pkg", "1.0.0");
    let tarball_fp = compute_tarball_fingerprint_bytes(&tarball_bytes, &package_ref)
        .expect("tarball fingerprint should be computed");

    assert_eq!(
        dir_fp, tarball_fp,
        "installed directory and tarball fingerprints must match even when symlinks are present"
    );
}

#[test]
fn test_resolve_install_policy_forces_ignore_scripts_when_post_verify_active() {
    let decision: InstallPolicyDecision = resolve_install_policy(ResolveInstallPolicyParams {
        compromised_count: 0,
        unverifiable_count: 0,
        allow_scripts: false,
        post_verify: true,
    });

    assert!(
        decision.ignore_scripts,
        "post_verify without allow_scripts must force ignore_scripts"
    );
    assert!(decision.block_reason.is_none());
}

#[test]
fn test_resolve_install_policy_allow_scripts_overrides_post_verify() {
    let decision: InstallPolicyDecision = resolve_install_policy(ResolveInstallPolicyParams {
        compromised_count: 0,
        unverifiable_count: 0,
        allow_scripts: true,
        post_verify: true,
    });

    assert!(
        !decision.ignore_scripts,
        "explicit allow_scripts must override post_verify ignore_scripts"
    );
}

#[test]
fn test_resolve_install_policy_blocks_scripts_by_default_without_post_verify() {
    let decision: InstallPolicyDecision = resolve_install_policy(ResolveInstallPolicyParams {
        compromised_count: 0,
        unverifiable_count: 0,
        allow_scripts: false,
        post_verify: false,
    });

    assert!(
        decision.ignore_scripts,
        "scripts should be blocked by default when --allow-scripts is not set"
    );
}

#[test]
fn test_verify_result_tarball_fingerprint_defaults_to_none_when_absent_from_json() {
    let json = r#"{
            "package": {"name": "left-pad", "version": "1.3.0"},
            "verdict": "CLEAN",
            "detail": "ok",
            "evidence": {}
        }"#;
    let result: VerifyResult =
        serde_json::from_str(json).expect("legacy result should deserialize");
    assert!(
        result.tarball_fingerprint.is_none(),
        "tarball_fingerprint must default to None for legacy cache entries"
    );
}

#[test]
fn test_verify_result_tarball_fingerprint_omitted_from_json_when_none() {
    let result = VerifyResult {
        package: PackageRef::new("left-pad", "1.3.0"),
        verdict: Verdict::Clean,
        detail: "ok".to_string(),
        evidence: Evidence::empty(),
        is_direct: false,
        direct_parent: None,
        tarball_fingerprint: None,
    };
    let json = serde_json::to_string(&result).expect("result should serialize");
    assert!(
        !json.contains("tarball_fingerprint"),
        "None tarball_fingerprint must be omitted from JSON output"
    );
}

#[test]
fn test_verify_result_tarball_fingerprint_round_trips_when_present() {
    let result = VerifyResult {
        package: PackageRef::new("left-pad", "1.3.0"),
        verdict: Verdict::Clean,
        detail: "ok".to_string(),
        evidence: Evidence::empty(),
        is_direct: false,
        direct_parent: None,
        tarball_fingerprint: Some("abc123".to_string()),
    };
    let json = serde_json::to_string(&result).expect("result should serialize");
    let restored: VerifyResult =
        serde_json::from_str(&json).expect("serialized result should deserialize");
    assert_eq!(restored.tarball_fingerprint.as_deref(), Some("abc123"));
}

#[test]
fn test_report_summary_does_not_block_when_only_provenance_missing_exists() {
    let report = Report::from_results(
        RunMode::Check,
        vec![VerifyResult {
            package: PackageRef::new("left-pad", "1.3.0"),
            verdict: Verdict::Unverifiable {
                reason: UnverifiableReason::ProvenanceMissing,
            },
            detail: "provenance not available".to_string(),
            evidence: Evidence::empty(),
            is_direct: true,
            direct_parent: None,
            tarball_fingerprint: None,
        }],
        vec![],
    );

    assert_eq!(
        report.summary.exit_code, 0,
        "missing provenance must not block"
    );
}

#[test]
fn test_report_summary_blocks_when_provenance_is_inconsistent() {
    let report = Report::from_results(
        RunMode::Check,
        vec![VerifyResult {
            package: PackageRef::new("left-pad", "1.3.0"),
            verdict: Verdict::Unverifiable {
                reason: UnverifiableReason::ProvenanceInconsistent,
            },
            detail: "provenance mismatch".to_string(),
            evidence: Evidence::empty(),
            is_direct: true,
            direct_parent: None,
            tarball_fingerprint: None,
        }],
        vec![],
    );

    assert_eq!(
        report.summary.exit_code, 1,
        "inconsistent provenance must block"
    );
}

#[test]
fn test_install_blocking_ignores_provenance_missing_only_results() {
    let results = vec![VerifyResult {
        package: PackageRef::new("left-pad", "1.3.0"),
        verdict: Verdict::Unverifiable {
            reason: UnverifiableReason::ProvenanceMissing,
        },
        detail: "provenance not available".to_string(),
        evidence: Evidence::empty(),
        is_direct: true,
        direct_parent: None,
        tarball_fingerprint: None,
    }];

    let blocked = print_blocking_install_results(&results);
    assert!(!blocked, "install must not block for missing provenance");
}

#[test]
fn test_ci_blocking_ignores_provenance_missing_only_results() {
    let args = CiArgs {
        omit_dev: false,
        omit_optional: false,
        allow_scripts: false,
        dry_run: true,
        post_verify: false,
        init_lockfile: false,
        format: OutputFormat::Json,
        report: std::path::PathBuf::from("sentinel-report.json"),
        cwd: std::path::PathBuf::from("."),
        package_manager: None,
        timeout: crate::constants::CI_REGISTRY_TIMEOUT_MS,
        quiet: true,
    };

    let results = vec![VerifyResult {
        package: PackageRef::new("left-pad", "1.3.0"),
        verdict: Verdict::Unverifiable {
            reason: UnverifiableReason::ProvenanceMissing,
        },
        detail: "provenance not available".to_string(),
        evidence: Evidence::empty(),
        is_direct: true,
        direct_parent: None,
        tarball_fingerprint: None,
    }];

    let blocked = print_ci_blocking_results(PrintCiBlockingResultsParams {
        results: &results,
        args: &args,
    });
    assert!(!blocked, "ci must not block for missing provenance");
}

#[test]
fn test_ci_blocking_blocks_on_provenance_inconsistent() {
    let args = CiArgs {
        omit_dev: false,
        omit_optional: false,
        allow_scripts: false,
        dry_run: true,
        post_verify: false,
        init_lockfile: false,
        format: OutputFormat::Json,
        report: std::path::PathBuf::from("sentinel-report.json"),
        cwd: std::path::PathBuf::from("."),
        package_manager: None,
        timeout: crate::constants::CI_REGISTRY_TIMEOUT_MS,
        quiet: true,
    };

    let results = vec![VerifyResult {
        package: PackageRef::new("left-pad", "1.3.0"),
        verdict: Verdict::Unverifiable {
            reason: UnverifiableReason::ProvenanceInconsistent,
        },
        detail: "provenance mismatch".to_string(),
        evidence: Evidence::empty(),
        is_direct: true,
        direct_parent: None,
        tarball_fingerprint: None,
    }];

    let blocked = print_ci_blocking_results(PrintCiBlockingResultsParams {
        results: &results,
        args: &args,
    });
    assert!(blocked, "ci must block for inconsistent provenance");
}

#[test]
fn test_report_provenance_summary_metrics_are_computed() {
    let report = Report::from_results(
        RunMode::Check,
        vec![
            VerifyResult {
                package: PackageRef::new("trusted", "1.0.0"),
                verdict: Verdict::Clean,
                detail: "ok".to_string(),
                evidence: Evidence {
                    provenance_subject_digest: Some("sha512-trusted".to_string()),
                    ..Evidence::empty()
                },
                is_direct: true,
                direct_parent: None,
                tarball_fingerprint: None,
            },
            VerifyResult {
                package: PackageRef::new("missing", "1.0.0"),
                verdict: Verdict::Unverifiable {
                    reason: UnverifiableReason::ProvenanceMissing,
                },
                detail: "missing provenance".to_string(),
                evidence: Evidence::empty(),
                is_direct: true,
                direct_parent: None,
                tarball_fingerprint: None,
            },
            VerifyResult {
                package: PackageRef::new("inconsistent", "1.0.0"),
                verdict: Verdict::Unverifiable {
                    reason: UnverifiableReason::ProvenanceInconsistent,
                },
                detail: "inconsistent provenance".to_string(),
                evidence: Evidence::empty(),
                is_direct: true,
                direct_parent: None,
                tarball_fingerprint: None,
            },
        ],
        vec![],
    );

    let provenance_summary = report.summary.provenance_summary;
    assert_eq!(provenance_summary.trusted_count, 1);
    assert_eq!(provenance_summary.warning_count, 1);
    assert_eq!(provenance_summary.inconsistent_count, 1);
    assert_eq!(provenance_summary.provenance_missing_count, 1);
    assert_eq!(provenance_summary.provenance_missing_shown, 1);
    assert_eq!(provenance_summary.provenance_missing_suppressed, 0);
    assert!((provenance_summary.trust_coverage - 0.5).abs() < f64::EPSILON);
    assert!((provenance_summary.provenance_availability - (2.0 / 3.0)).abs() < f64::EPSILON);
}

#[test]
fn test_report_provenance_summary_defaults_for_no_results() {
    let report = Report::from_results(RunMode::Check, vec![], vec![]);

    let provenance_summary = report.summary.provenance_summary;
    assert_eq!(provenance_summary.trusted_count, 0);
    assert_eq!(provenance_summary.warning_count, 0);
    assert_eq!(provenance_summary.inconsistent_count, 0);
    assert_eq!(provenance_summary.provenance_missing_count, 0);
    assert!(provenance_summary.trust_coverage.abs() < f64::EPSILON);
    assert!(provenance_summary.provenance_availability.abs() < f64::EPSILON);
}
