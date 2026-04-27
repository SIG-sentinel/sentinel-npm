#![allow(
    clippy::expect_used,
    clippy::needless_raw_string_hashes,
    unused_qualifications
)]

use std::fs;
use std::process::ExitCode;

use sentinel::commands;
use sentinel::types::{CheckArgs, CiArgs, InstallArgs, OutputFormat};

fn write_empty_lockfile(dir: &std::path::Path) {
    let lockfile = r#"
{
  "name": "demo",
  "lockfileVersion": 3,
  "packages": {
    "": {
      "name": "demo",
      "version": "1.0.0"
    }
  }
}
"#;

    std::fs::write(dir.join("package-lock.json"), lockfile).expect("lockfile should be written");
}

fn write_package_json(dir: &std::path::Path) {
    let package_json = r#"
{
  "name": "demo",
  "version": "1.0.0",
  "dependencies": {
    "a": "1.0.0"
  }
}
"#;

    std::fs::write(dir.join("package.json"), package_json).expect("package.json should be written");
}

fn write_cycle_lockfile(dir: &std::path::Path) {
    let lockfile = r#"
{
  "name": "demo",
  "lockfileVersion": 3,
  "packages": {
    "node_modules/a": {
      "version": "1.0.0",
      "dependencies": {
        "b": { "version": "1.0.0" }
      }
    },
    "node_modules/b": {
      "version": "1.0.0",
      "dependencies": {
        "a": { "version": "1.0.0" }
      }
    }
  }
}
"#;

    std::fs::write(dir.join("package-lock.json"), lockfile).expect("lockfile should be written");
}

fn write_yarn_lockfile(dir: &std::path::Path) {
    let lockfile = r#"
"a@1.0.0":
  version "1.0.0"
  integrity "sha512-RUQ9/8WXBcC9FKIkjROXvD+cTFaJwLBJa3gGKWl4E4o12vUUJFyfz5Sr+HkJkGH3FdsMXCKZgmtXBB2a1myhGg=="

"b@2.0.0":
  version "2.0.0"
  integrity "sha512-ocPGGO+DmXfitQobga4I8qZw9M3kvQmh/dePel3LxneYDktbW1uwD1b2FmgEraol+B0MLWgGIP+lwk3mZV+elA=="
"#;

    std::fs::write(dir.join("yarn.lock"), lockfile).expect("yarn.lock should be written");
}

fn write_pnpm_lockfile(dir: &std::path::Path) {
    let lockfile = r#"
lockfileVersion: '9.0'
packages:
  /a@1.0.0:
    resolution:
      integrity: sha512-RUQ9/8WXBcC9FKIkjROXvD+cTFaJwLBJa3gGKWl4E4o12vUUJFyfz5Sr+HkJkGH3FdsMXCKZgmtXBB2a1myhGg==
    dev: false
  /b@2.0.0:
    resolution:
      integrity: sha512-ocPGGO+DmXfitQobga4I8qZw9M3kvQmh/dePel3LxneYDktbW1uwD1b2FmgEraol+B0MLWgGIP+lwk3mZV+elA==
    dev: false
"#;

    std::fs::write(dir.join("pnpm-lock.yaml"), lockfile).expect("pnpm-lock.yaml should be written");
}

#[tokio::test]
async fn test_check_run_succeeds_with_empty_graph() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    write_package_json(temp_dir.path());
    write_empty_lockfile(temp_dir.path());

    let args = CheckArgs {
        omit_dev: false,
        omit_optional: false,
        format: OutputFormat::Json,
        cwd: temp_dir.path().to_path_buf(),
        package_manager: None,
        timeout: 1000,
        quiet: true,
    };

    let exit = commands::check::run(&args).await;
    assert_eq!(exit, ExitCode::SUCCESS);
}

#[tokio::test]
async fn test_ci_run_succeeds_with_empty_graph() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    std::fs::write(
        temp_dir.path().join("package.json"),
        r#"{"name":"demo","version":"1.0.0","dependencies":{}}"#,
    )
    .expect("package.json should be written");
    write_empty_lockfile(temp_dir.path());

    let report_path = temp_dir.path().join("report.json");
    let args = CiArgs {
        omit_dev: false,
        omit_optional: false,
        allow_scripts: false,
        dry_run: true,
        post_verify: false,
        init_lockfile: false,
        format: OutputFormat::Json,
        report: report_path.clone(),
        cwd: temp_dir.path().to_path_buf(),
        package_manager: None,
        timeout: 1000,
        quiet: true,
    };

    let exit = commands::install::run_ci(&args).await;
    assert_eq!(exit, ExitCode::SUCCESS);
    assert!(
        !report_path.exists(),
        "CI exits early when graph is empty and should not write report"
    );
}

#[tokio::test]
async fn test_check_run_fails_when_cycle_exists() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    write_package_json(temp_dir.path());
    write_cycle_lockfile(temp_dir.path());

    let args = CheckArgs {
        omit_dev: false,
        omit_optional: false,
        format: OutputFormat::Json,
        cwd: temp_dir.path().to_path_buf(),
        package_manager: None,
        timeout: 1000,
        quiet: true,
    };

    let exit = commands::check::run(&args).await;
    assert_eq!(exit, ExitCode::FAILURE);
}

#[tokio::test]
async fn test_ci_run_fails_when_cycle_exists() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    write_package_json(temp_dir.path());
    write_cycle_lockfile(temp_dir.path());

    let args = CiArgs {
        omit_dev: false,
        omit_optional: false,
        allow_scripts: false,
        dry_run: true,
        post_verify: false,
        init_lockfile: false,
        format: OutputFormat::Json,
        report: temp_dir.path().join("report.json"),
        cwd: temp_dir.path().to_path_buf(),
        package_manager: None,
        timeout: 1000,
        quiet: true,
    };

    let exit = commands::install::run_ci(&args).await;
    assert_eq!(exit, ExitCode::FAILURE);
}

#[tokio::test]
async fn test_check_run_succeeds_with_yarn_lockfile() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    write_package_json(temp_dir.path());
    write_yarn_lockfile(temp_dir.path());

    let args = CheckArgs {
        omit_dev: false,
        omit_optional: false,
        format: OutputFormat::Json,
        cwd: temp_dir.path().to_path_buf(),
        package_manager: None,
        timeout: 1000,
        quiet: true,
    };

    let exit = commands::check::run(&args).await;
    assert_eq!(exit, ExitCode::SUCCESS);
}

#[tokio::test]
async fn test_check_fails_when_no_lockfile_exists() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    write_package_json(temp_dir.path());

    let args = CheckArgs {
        omit_dev: false,
        omit_optional: false,
        format: OutputFormat::Json,
        cwd: temp_dir.path().to_path_buf(),
        package_manager: Some("npm".to_string()),
        timeout: 1000,
        quiet: true,
    };

    let exit = commands::check::run(&args).await;
    assert_eq!(
        exit,
        ExitCode::FAILURE,
        "check must fail when no lockfile exists"
    );
}

#[tokio::test]
async fn test_check_does_not_generate_lockfile_when_missing() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    write_package_json(temp_dir.path());

    let args = CheckArgs {
        omit_dev: false,
        omit_optional: false,
        format: OutputFormat::Json,
        cwd: temp_dir.path().to_path_buf(),
        package_manager: Some("npm".to_string()),
        timeout: 1000,
        quiet: true,
    };

    let _ = commands::check::run(&args).await;

    assert!(
        !temp_dir.path().join("package-lock.json").exists(),
        "check must not generate package-lock.json"
    );
    assert!(
        !temp_dir.path().join("yarn.lock").exists(),
        "check must not generate yarn.lock"
    );
    assert!(
        !temp_dir.path().join("pnpm-lock.yaml").exists(),
        "check must not generate pnpm-lock.yaml"
    );
}

#[tokio::test]
async fn test_ci_preserves_existing_lockfile_contents() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    std::fs::write(
        temp_dir.path().join("package.json"),
        r#"{"name":"demo","version":"1.0.0","dependencies":{}}"#,
    )
    .expect("package.json should be written");
    write_empty_lockfile(temp_dir.path());

    let lockfile_path = temp_dir.path().join("package-lock.json");
    let contents_before = fs::read_to_string(&lockfile_path).expect("lockfile should be readable");

    let args = CiArgs {
        omit_dev: false,
        omit_optional: false,
        allow_scripts: false,
        dry_run: true,
        post_verify: false,
        init_lockfile: false,
        format: OutputFormat::Json,
        report: temp_dir.path().join("report.json"),
        cwd: temp_dir.path().to_path_buf(),
        package_manager: None,
        timeout: 1000,
        quiet: true,
    };

    let _ = commands::install::run_ci(&args).await;

    let contents_after = fs::read_to_string(&lockfile_path).expect("lockfile should still exist");
    assert_eq!(
        contents_before, contents_after,
        "ci must not modify an existing lockfile"
    );
}

#[tokio::test]
async fn test_ci_fails_when_no_lockfile_exists_without_init_flag() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    write_package_json(temp_dir.path());

    let args = CiArgs {
        omit_dev: false,
        omit_optional: false,
        allow_scripts: false,
        dry_run: true,
        post_verify: false,
        init_lockfile: false,
        format: OutputFormat::Json,
        report: temp_dir.path().join("report.json"),
        cwd: temp_dir.path().to_path_buf(),
        package_manager: Some("npm".to_string()),
        timeout: 1000,
        quiet: true,
    };

    let exit = commands::install::run_ci(&args).await;

    assert_eq!(
        exit,
        ExitCode::FAILURE,
        "ci must fail when lockfile is missing and --init is not enabled"
    );
    assert!(
        !temp_dir.path().join("package-lock.json").exists(),
        "ci must not generate lockfile without --init"
    );
}

#[tokio::test]
async fn test_ci_initializes_lockfile_when_init_flag_is_enabled() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    std::fs::write(
        temp_dir.path().join("package.json"),
        r#"{"name":"demo","version":"1.0.0","dependencies":{}}"#,
    )
    .expect("package.json should be written");

    let args = CiArgs {
        omit_dev: false,
        omit_optional: false,
        allow_scripts: false,
        dry_run: true,
        post_verify: false,
        init_lockfile: true,
        format: OutputFormat::Json,
        report: temp_dir.path().join("report.json"),
        cwd: temp_dir.path().to_path_buf(),
        package_manager: Some("npm".to_string()),
        timeout: 1000,
        quiet: true,
    };

    let exit = commands::install::run_ci(&args).await;

    assert_eq!(
        exit,
        ExitCode::SUCCESS,
        "ci should proceed when lockfile is missing and --init is enabled"
    );
    assert!(
        temp_dir.path().join("package-lock.json").exists(),
        "ci --init should create package-lock.json"
    );
}

#[tokio::test]
async fn test_ci_init_autodetects_manager_from_package_json_and_generates_lockfile() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    std::fs::write(
        temp_dir.path().join("package.json"),
        r#"{"name":"demo","version":"1.0.0","packageManager":"npm@10.9.0","dependencies":{}}"#,
    )
    .expect("package.json should be written");

    let args = CiArgs {
        omit_dev: false,
        omit_optional: false,
        allow_scripts: false,
        dry_run: true,
        post_verify: false,
        init_lockfile: true,
        format: OutputFormat::Text,
        report: temp_dir.path().join("report.json"),
        cwd: temp_dir.path().to_path_buf(),
        package_manager: None,
        timeout: 1000,
        quiet: false,
    };

    let exit = commands::install::run_ci(&args).await;

    assert_eq!(
        exit,
        ExitCode::SUCCESS,
        "ci --init should succeed when manager is auto-detected from package.json"
    );
    assert!(
        temp_dir.path().join("package-lock.json").exists(),
        "ci --init should create package-lock.json after auto-detection"
    );
}

#[tokio::test]
async fn test_ci_init_does_not_regenerate_existing_lockfile() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    std::fs::write(
        temp_dir.path().join("package.json"),
        r#"{"name":"demo","version":"1.0.0","dependencies":{}}"#,
    )
    .expect("package.json should be written");
    write_empty_lockfile(temp_dir.path());

    let lockfile_path = temp_dir.path().join("package-lock.json");
    let contents_before = fs::read_to_string(&lockfile_path).expect("lockfile should be readable");

    let args = CiArgs {
        omit_dev: false,
        omit_optional: false,
        allow_scripts: false,
        dry_run: true,
        post_verify: false,
        init_lockfile: true,
        format: OutputFormat::Json,
        report: temp_dir.path().join("report.json"),
        cwd: temp_dir.path().to_path_buf(),
        package_manager: Some("npm".to_string()),
        timeout: 1000,
        quiet: true,
    };

    let exit = commands::install::run_ci(&args).await;

    assert_eq!(
        exit,
        ExitCode::SUCCESS,
        "ci should remain successful with existing lockfile"
    );

    let contents_after = fs::read_to_string(&lockfile_path).expect("lockfile should still exist");
    assert_eq!(
        contents_before, contents_after,
        "ci --init must not mutate an already existing lockfile"
    );
}

#[tokio::test]
async fn test_install_fails_when_no_lockfile_exists() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    write_package_json(temp_dir.path());

    let args = InstallArgs {
        package: "lodash@4.17.21".to_string(),
        allow_scripts: false,
        dry_run: true,
        post_verify: false,
        format: OutputFormat::Json,
        cwd: temp_dir.path().to_path_buf(),
        package_manager: Some("npm".to_string()),
        timeout: 1000,
        quiet: true,
    };

    let exit = commands::install::run_install(&args).await;
    assert_eq!(
        exit,
        ExitCode::FAILURE,
        "install must fail when no lockfile exists"
    );
}

#[tokio::test]
async fn test_check_run_succeeds_with_pnpm_lockfile() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    write_package_json(temp_dir.path());
    write_pnpm_lockfile(temp_dir.path());

    let args = CheckArgs {
        omit_dev: false,
        omit_optional: false,
        format: OutputFormat::Json,
        cwd: temp_dir.path().to_path_buf(),
        package_manager: None,
        timeout: 1000,
        quiet: true,
    };

    let exit = commands::check::run(&args).await;
    assert_eq!(exit, ExitCode::SUCCESS);
}
