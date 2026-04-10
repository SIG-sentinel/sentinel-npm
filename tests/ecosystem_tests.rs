use std::fs;

use sentinel::ecosystem::{
    InstallExecutor, LockfileParser, NpmLockfileParser, PackageManager,
    PackageManagerExecutor, PnpmLockfileParser, YarnLockfileParser, compare_integrity,
    detect_package_manager, read_lockfile_entries, active_lockfile_path,
};
use sentinel::types::CleanInstallPlanParams;
use sentinel::types::ComparisonVerdict;
use sentinel::types::SentinelError;
use tempfile::tempdir;

#[test]
fn detects_package_manager_from_lockfile_presence() {
    let temp = tempdir().expect("temp dir");
    fs::write(temp.path().join("yarn.lock"), "").expect("write lockfile");

    let manager = detect_package_manager(temp.path());
    assert_eq!(manager, Some(PackageManager::Yarn));
}

#[test]
fn active_lockfile_path_defaults_to_package_lock_when_no_lockfile_exists() {
    let temp = tempdir().expect("temp dir");

    let path = active_lockfile_path(temp.path());
    assert_eq!(path, temp.path().join("package-lock.json"));
}

#[test]
fn parses_yarn_lock_entry() {
    let temp = tempdir().expect("temp dir");

    let yarn_lock = r#""lodash@^4.17.21":
  version "4.17.21"
  resolved "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz"
  integrity "sha512-v2kDEe57lecTulaDIuNTPy3Ry4g"
"#;

    fs::write(temp.path().join("yarn.lock"), yarn_lock).expect("write yarn lock");

    let entries = read_lockfile_entries(temp.path()).expect("parse entries");
    let entry = entries.get("lodash@4.17.21").expect("lodash entry");

    assert_eq!(entry.package.name, "lodash");
    assert_eq!(entry.package.version, "4.17.21");
    assert_eq!(
        entry.integrity.as_deref(),
        Some("sha512-v2kDEe57lecTulaDIuNTPy3Ry4g")
    );
}

#[test]
fn parses_pnpm_lock_entry() {
    let temp = tempdir().expect("temp dir");

    let pnpm_lock = r#"lockfileVersion: '9.0'
packages:
  /lodash@4.17.21:
    resolution:
      integrity: sha512-v2kDEe57lecTulaDIuNTPy3Ry4g
    dev: false
"#;

    fs::write(temp.path().join("pnpm-lock.yaml"), pnpm_lock).expect("write pnpm lock");

    let entries = read_lockfile_entries(temp.path()).expect("parse entries");
    let entry = entries.get("lodash@4.17.21").expect("lodash entry");

    assert_eq!(entry.package.name, "lodash");
    assert_eq!(entry.package.version, "4.17.21");
    assert_eq!(
        entry.integrity.as_deref(),
        Some("sha512-v2kDEe57lecTulaDIuNTPy3Ry4g")
    );
}

#[test]
fn compare_integrity_reports_clean_and_compromised() {
    assert_eq!(
        compare_integrity("sha512-a", "sha512-a"),
        ComparisonVerdict::Clean
    );
    assert_eq!(
        compare_integrity("sha512-a", "sha512-b"),
        ComparisonVerdict::Compromised
    );
}

#[test]
fn npm_executor_uses_deterministic_clean_install_command() {
    let executor = PackageManagerExecutor::new(PackageManager::Npm);
    let plan = executor.clean_install_plan(CleanInstallPlanParams {
        ignore_scripts: true,
        omit_dev: true,
        omit_optional: true,
        silent_output: true,
    });

    assert_eq!(plan.program, "npm");
    assert!(plan.args.contains(&"ci".to_string()));
    assert!(plan.args.contains(&"--ignore-scripts".to_string()));
    assert!(plan.args.contains(&"--omit=dev".to_string()));
    assert!(plan.args.contains(&"--omit=optional".to_string()));
}

#[test]
fn yarn_executor_uses_frozen_lockfile() {
    let executor = PackageManagerExecutor::new(PackageManager::Yarn);
    let plan = executor.clean_install_plan(CleanInstallPlanParams {
        ignore_scripts: false,
        omit_dev: false,
        omit_optional: false,
        silent_output: false,
    });

    assert_eq!(plan.program, "yarn");
    assert!(plan.args.contains(&"install".to_string()));
    assert!(plan.args.contains(&"--frozen-lockfile".to_string()));
}

#[test]
fn pnpm_executor_uses_frozen_lockfile() {
    let executor = PackageManagerExecutor::new(PackageManager::Pnpm);
    let plan = executor.clean_install_plan(CleanInstallPlanParams {
        ignore_scripts: false,
        omit_dev: false,
        omit_optional: false,
        silent_output: false,
    });

    assert_eq!(plan.program, "pnpm");
    assert!(plan.args.contains(&"install".to_string()));
    assert!(plan.args.contains(&"--frozen-lockfile".to_string()));
}

#[test]
fn detects_package_manager_prefers_lockfile_over_package_manager_field() {
    let temp = tempdir().expect("temp dir");
    fs::write(
        temp.path().join("package.json"),
        r#"{"name":"demo","version":"1.0.0","packageManager":"yarn@1.22.22"}"#,
    )
    .expect("write package.json");
    fs::write(temp.path().join("package-lock.json"), "{}")
        .expect("write package-lock.json");

    let manager = detect_package_manager(temp.path());
    assert_eq!(manager, Some(PackageManager::Npm));
}

#[test]
fn npm_lockfile_parser_errors_when_lockfile_missing() {
    let temp = tempdir().expect("temp dir");
    let parser = NpmLockfileParser;

    let result = parser.parse_entries(temp.path());
    assert!(matches!(result, Err(SentinelError::LockfileNotFound)));
}

#[test]
fn yarn_lockfile_parser_errors_when_lockfile_missing() {
    let temp = tempdir().expect("temp dir");
    let parser = YarnLockfileParser;

    let result = parser.parse_entries(temp.path());
    assert!(matches!(result, Err(SentinelError::LockfileNotFound)));
}

#[test]
fn pnpm_lockfile_parser_errors_when_lockfile_missing() {
    let temp = tempdir().expect("temp dir");
    let parser = PnpmLockfileParser;

    let result = parser.parse_entries(temp.path());
    assert!(matches!(result, Err(SentinelError::LockfileNotFound)));
}

#[test]
fn npm_lockfile_parser_errors_on_corrupted_json() {
    let temp = tempdir().expect("temp dir");
    fs::write(temp.path().join("package-lock.json"), "{ invalid json")
        .expect("write package-lock.json");
    let parser = NpmLockfileParser;

    let result = parser.parse_entries(temp.path());
    assert!(matches!(result, Err(SentinelError::LockfileParse(_))));
}

#[test]
fn yarn_lockfile_parser_keeps_entry_without_integrity_as_none() {
    let temp = tempdir().expect("temp dir");
    let yarn_lock = r#""lodash@^4.17.21":
  version "4.17.21"
"#;

    fs::write(temp.path().join("yarn.lock"), yarn_lock).expect("write yarn.lock");

    let entries = read_lockfile_entries(temp.path()).expect("parse entries");
    let entry = entries.get("lodash@4.17.21").expect("lodash entry should exist");

    assert_eq!(entry.integrity, None);
}

#[test]
fn pnpm_lockfile_parser_errors_on_corrupted_yaml() {
    let temp = tempdir().expect("temp dir");
    fs::write(temp.path().join("pnpm-lock.yaml"), "lockfileVersion: [")
        .expect("write pnpm-lock.yaml");
    let parser = PnpmLockfileParser;

    let result = parser.parse_entries(temp.path());
    assert!(matches!(result, Err(SentinelError::LockfileParse(_))));
}
