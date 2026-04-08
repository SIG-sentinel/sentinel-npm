use std::process::ExitCode;

use sentinel::commands;
use sentinel::types::{CheckArgs, CiArgs, OutputFormat};

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
        timeout: 1000,
        quiet: true,
    };

    let exit = commands::check::run(&args).await;
    assert_eq!(exit, ExitCode::SUCCESS);
}

#[tokio::test]
async fn test_ci_run_succeeds_with_empty_graph() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    // No external dependencies → packages_to_verify is empty → early exit, no report written
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
        no_scripts: false,
        dry_run: true,
        format: OutputFormat::Json,
        report: report_path.clone(),
        cwd: temp_dir.path().to_path_buf(),
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
        no_scripts: false,
        dry_run: true,
        format: OutputFormat::Json,
        report: temp_dir.path().join("report.json"),
        cwd: temp_dir.path().to_path_buf(),
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
    timeout: 1000,
    quiet: true,
  };

  let exit = commands::check::run(&args).await;
  assert_eq!(exit, ExitCode::SUCCESS);
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
    timeout: 1000,
    quiet: true,
  };

  let exit = commands::check::run(&args).await;
  assert_eq!(exit, ExitCode::SUCCESS);
}
