use super::{
    ensure_lockfile_exists, finalize_ci_run, finalize_install_run, parse_package_ref,
    should_print_report,
};
use crate::cache::LocalCache;
use crate::types::{
    CiArgs, EnsureLockfileExistsParams, Evidence, FinalizeCiRunParams, FinalizeInstallRunParams,
    InstallArgs, OutputFormat, PackageRef, Report, RunMode,
    RestoreProjectFilesSnapshotParams, VerifyResult,
};
use crate::utils::{
    capture_project_files_snapshot, lockfile_sha256, restore_project_files_snapshot,
};
use std::process::ExitCode;

fn set_test_env_var(key: &str, value: impl AsRef<std::ffi::OsStr>) {
    unsafe {
        std::env::set_var(key, value);
    }
}

fn remove_test_env_var(key: &str) {
    unsafe {
        std::env::remove_var(key);
    }
}

#[tokio::test]
async fn test_ensure_lockfile_exists_returns_true_when_file_exists() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let package_json = temp_dir.path().join("package.json");
    let lockfile = temp_dir.path().join("package-lock.json");
    std::fs::write(&package_json, "{\"name\":\"demo\",\"version\":\"1.0.0\"}")
        .expect("package.json should be written");
    std::fs::write(&lockfile, "{}").expect("lockfile should be written");

    let ok = ensure_lockfile_exists(EnsureLockfileExistsParams {
        current_working_directory: temp_dir.path(),
        quiet: true,
    })
    .await;
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
fn test_should_print_report_suppresses_quiet_text_output() {
    let should_print = should_print_report(&OutputFormat::Text, true);

    assert!(!should_print);
}

#[test]
fn test_should_print_report_keeps_quiet_json_output() {
    let should_print = should_print_report(&OutputFormat::Json, true);

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
        no_scripts: false,
        dry_run: false,
        format: OutputFormat::Json,
        report: temp_dir.path().join("report.json"),
        cwd: temp_dir.path().to_path_buf(),
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
        no_scripts: true,
        dry_run: false,
        format: OutputFormat::Json,
        cwd: temp_dir.path().to_path_buf(),
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
    let previous_home = std::env::var("HOME").ok();
    let home_dir = cwd.join("home");
    std::fs::create_dir_all(&home_dir).expect("home dir should be created");

    set_test_env_var("PATH", &patched_path);
    set_test_env_var("HOME", &home_dir);

    let cache = LocalCache::open(None).expect("cache should open");
    let cached_clean = VerifyResult {
        package: PackageRef::new("lodash", "4.17.21"),
        verdict: crate::types::Verdict::Clean,
        detail: "cached clean for test".to_string(),
        evidence: Evidence {
            lockfile_integrity: Some("sha512-xyz".to_string()),
            ..Evidence::empty()
        },
    };
    cache.put(&cached_clean);

    let args = InstallArgs {
        package: "lodash@4.17.21".to_string(),
        allow_scripts: false,
        no_scripts: true,
        dry_run: false,
        format: OutputFormat::Json,
        cwd: cwd.to_path_buf(),
        timeout: 1,
        quiet: true,
    };

    let exit = super::run_install(&args).await;
    set_test_env_var("PATH", previous_path);
    if let Some(home) = previous_home {
        set_test_env_var("HOME", home);
    } else {
        remove_test_env_var("HOME");
    }

    assert_eq!(exit, ExitCode::FAILURE);

    let package_after = std::fs::read_to_string(&package_json).expect("package.json should exist");
    assert_eq!(package_after, original_package_json);

    let lock_after = std::fs::read_to_string(&lockfile).expect("lockfile should exist");
    assert_eq!(lock_after, "{}");
}
