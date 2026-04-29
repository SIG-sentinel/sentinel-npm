#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::err_expect,
    clippy::too_many_arguments,
    clippy::needless_raw_string_hashes,
    unused_qualifications,
    clippy::await_holding_lock,
    clippy::items_after_statements
)]

use std::process::ExitCode;
use std::sync::{Mutex, OnceLock};

use sentinel::commands;
use sentinel::ecosystem::PackageManager;
use sentinel::history::ledger::{AppendHistoryEventsParams, append_history_events};
use sentinel::history::path::{resolve_history_ledger_path, resolve_project_root};
use sentinel::history::types::{HistoryOutputFormat, HistoryPackageMetadata};
use sentinel::types::HistoryArgs;

const EXIT_SUCCESS: ExitCode = ExitCode::SUCCESS;
const EXIT_FAILURE: ExitCode = ExitCode::FAILURE;

fn exit_invalid_input() -> ExitCode {
    ExitCode::from(2u8)
}

const FROM_PAST: &str = "2026-01-01T00:00:00Z";
const TO_FUTURE: &str = "2099-12-31T23:59:59Z";
const FROM_FUTURE: &str = "2099-01-01T00:00:00Z";
const TO_FAR_FUTURE: &str = "2099-12-31T23:59:59Z";

fn env_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("history env test mutex should lock")
}

fn history_args(dir: &std::path::Path, from: &str, to: &str) -> HistoryArgs {
    HistoryArgs {
        from: from.to_string(),
        to: to.to_string(),
        package: None,
        version: None,
        project: None,
        package_manager: None,
        format: HistoryOutputFormat::Text,
        cwd: dir.to_path_buf(),
        quiet: false,
    }
}

fn package(name: &str, version: &str) -> HistoryPackageMetadata {
    HistoryPackageMetadata {
        name: name.to_string(),
        version: version.to_string(),
        direct: true,
    }
}

fn seed_ledger(dir: &std::path::Path, pkgs: &[HistoryPackageMetadata]) {
    let lock_hash = Some("sha256-abc".to_string());
    let append_history_events_params = AppendHistoryEventsParams {
        current_working_directory: dir,
        package_manager: PackageManager::Npm,
        command: "install",
        lockfile_path: "package-lock.json",
        lock_hash_before: &lock_hash,
        lock_hash_after: &lock_hash,
        packages: pkgs,
    };

    append_history_events(append_history_events_params).expect("seeding ledger should succeed");
}

#[test]
fn history_command_defaults_parse() {
    let _guard = env_test_lock();
    use clap::Parser;
    use sentinel::types::{Cli, Commands};

    let cli = Cli::try_parse_from([
        "sentinel", "history", "--from", FROM_PAST, "--to", TO_FUTURE,
    ])
    .expect("history with required flags should parse");

    match cli.command {
        Commands::History(args) => {
            assert!(
                args.from.starts_with("2026-01-01T00:00:00"),
                "from timestamp preserved"
            );
            assert!(
                args.to.starts_with("2099-12-31T23:59:59"),
                "to timestamp preserved"
            );
            assert!(args.package.is_none());
            assert!(args.version.is_none());
            assert!(args.project.is_none());
            assert!(args.package_manager.is_none());
            assert_eq!(args.format, HistoryOutputFormat::Text);
            assert_eq!(args.cwd.to_string_lossy(), ".");
            assert!(!args.quiet);
        }
        _ => panic!("expected history command"),
    }
}

#[test]
fn history_command_all_flags_parse() {
    let _guard = env_test_lock();
    use clap::Parser;
    use sentinel::types::{Cli, Commands};

    let cli = Cli::try_parse_from([
        "sentinel",
        "history",
        "--from",
        FROM_PAST,
        "--to",
        TO_FUTURE,
        "--package",
        "lodash",
        "--version",
        "4.17.21",
        "--package-manager",
        "npm",
        "--format",
        "json",
        "--cwd",
        "/tmp",
        "--quiet",
    ])
    .expect("history with all flags should parse");

    match cli.command {
        Commands::History(args) => {
            assert_eq!(args.package.as_deref(), Some("lodash"));
            assert_eq!(args.version.as_deref(), Some("4.17.21"));
            assert_eq!(args.package_manager.as_deref(), Some("npm"));
            assert_eq!(args.format, HistoryOutputFormat::Json);
            assert_eq!(args.cwd.to_string_lossy(), "/tmp");
            assert!(args.quiet);
        }
        _ => panic!("expected history command"),
    }
}

#[test]
fn history_command_accepts_relative_time_expressions() {
    let _guard = env_test_lock();
    use clap::Parser;
    use sentinel::types::{Cli, Commands};

    let cli = Cli::try_parse_from(["sentinel", "history", "--from", "7 days ago", "--to", "now"])
        .expect("history should parse relative timestamps");

    match cli.command {
        Commands::History(args) => {
            assert!(
                args.from.contains('T') && args.from.contains('+'),
                "--from should be normalized into RFC3339"
            );
            assert!(
                args.to.contains('T') && args.to.contains('+'),
                "--to should be normalized into RFC3339"
            );
        }
        _ => panic!("expected history command"),
    }
}

#[test]
fn history_rejects_version_without_package() {
    let _guard = env_test_lock();
    use clap::Parser;
    use sentinel::types::Cli;

    let result = Cli::try_parse_from([
        "sentinel",
        "history",
        "--from",
        FROM_PAST,
        "--to",
        TO_FUTURE,
        "--version",
        "4.17.21",
    ]);

    assert!(
        result.is_err(),
        "--version without --package must be rejected"
    );
}

#[tokio::test]
async fn history_range_mode_returns_failure_when_ledger_absent() {
    let _guard = env_test_lock();
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let args = history_args(temp_dir.path(), FROM_PAST, TO_FUTURE);

    let exit = commands::history::run(&args).await;

    assert_eq!(exit, EXIT_FAILURE, "missing ledger should exit 1");
}

#[tokio::test]
async fn history_range_mode_returns_success_with_empty_ledger() {
    let _guard = env_test_lock();
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");

    let project_root = resolve_project_root(temp_dir.path()).expect("project root should resolve");
    let ledger_path = resolve_history_ledger_path(&project_root);
    std::fs::create_dir_all(ledger_path.parent().expect("parent should exist"))
        .expect("sentinel dir should be created");
    std::fs::File::create(&ledger_path).expect("empty ledger should be created");

    let args = history_args(temp_dir.path(), FROM_PAST, TO_FUTURE);
    let exit = commands::history::run(&args).await;

    assert_eq!(exit, EXIT_SUCCESS, "empty ledger should exit 0");
}

#[tokio::test]
async fn history_range_mode_returns_success_with_seeded_events() {
    let _guard = env_test_lock();
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");

    seed_ledger(
        temp_dir.path(),
        &[package("lodash", "4.17.21"), package("react", "18.3.1")],
    );

    let args = history_args(temp_dir.path(), FROM_PAST, TO_FUTURE);
    let exit = commands::history::run(&args).await;

    assert_eq!(exit, EXIT_SUCCESS, "range query with events should exit 0");
}

#[tokio::test]
async fn history_range_mode_returns_success_with_no_matches_in_range() {
    let _guard = env_test_lock();
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");

    seed_ledger(temp_dir.path(), &[package("lodash", "4.17.21")]);

    let args = history_args(temp_dir.path(), FROM_FUTURE, TO_FAR_FUTURE);
    let exit = commands::history::run(&args).await;

    assert_eq!(
        exit, EXIT_SUCCESS,
        "range with no matches should still exit 0"
    );
}

#[tokio::test]
async fn history_package_mode_returns_success_when_package_found() {
    let _guard = env_test_lock();
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");

    seed_ledger(temp_dir.path(), &[package("lodash", "4.17.21")]);

    let args = HistoryArgs {
        package: Some("lodash".to_string()),
        ..history_args(temp_dir.path(), FROM_PAST, TO_FUTURE)
    };
    let exit = commands::history::run(&args).await;

    assert_eq!(exit, EXIT_SUCCESS, "package query found should exit 0");
}

#[tokio::test]
async fn history_package_mode_returns_success_when_package_not_found() {
    let _guard = env_test_lock();
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");

    seed_ledger(temp_dir.path(), &[package("lodash", "4.17.21")]);

    let args = HistoryArgs {
        package: Some("express".to_string()),
        ..history_args(temp_dir.path(), FROM_PAST, TO_FUTURE)
    };
    let exit = commands::history::run(&args).await;

    assert_eq!(
        exit, EXIT_SUCCESS,
        "package not found is valid result, exit 0"
    );
}

#[tokio::test]
async fn history_package_mode_version_filter_returns_success() {
    let _guard = env_test_lock();
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");

    seed_ledger(
        temp_dir.path(),
        &[package("lodash", "4.17.21"), package("lodash", "4.17.20")],
    );

    let args = HistoryArgs {
        package: Some("lodash".to_string()),
        version: Some("4.17.21".to_string()),
        ..history_args(temp_dir.path(), FROM_PAST, TO_FUTURE)
    };
    let exit = commands::history::run(&args).await;

    assert_eq!(
        exit, EXIT_SUCCESS,
        "version-filtered package query should exit 0"
    );
}

#[tokio::test]
async fn history_package_manager_filter_returns_success() {
    let _guard = env_test_lock();
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");

    seed_ledger(temp_dir.path(), &[package("lodash", "4.17.21")]);

    let args = HistoryArgs {
        package_manager: Some("npm".to_string()),
        ..history_args(temp_dir.path(), FROM_PAST, TO_FUTURE)
    };
    let exit = commands::history::run(&args).await;

    assert_eq!(exit, EXIT_SUCCESS, "package-manager filter should exit 0");
}

#[tokio::test]
async fn history_json_format_returns_success() {
    let _guard = env_test_lock();
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");

    seed_ledger(temp_dir.path(), &[package("lodash", "4.17.21")]);

    let args = HistoryArgs {
        format: HistoryOutputFormat::Json,
        ..history_args(temp_dir.path(), FROM_PAST, TO_FUTURE)
    };
    let exit = commands::history::run(&args).await;

    assert_eq!(exit, EXIT_SUCCESS, "json format should exit 0");
}

#[tokio::test]
async fn history_quiet_mode_returns_success() {
    let _guard = env_test_lock();
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");

    seed_ledger(temp_dir.path(), &[package("lodash", "4.17.21")]);

    let args = HistoryArgs {
        quiet: true,
        ..history_args(temp_dir.path(), FROM_PAST, TO_FUTURE)
    };
    let exit = commands::history::run(&args).await;

    assert_eq!(exit, EXIT_SUCCESS, "quiet mode should exit 0");
}

#[tokio::test]
async fn history_returns_exit_2_when_from_is_after_to() {
    let _guard = env_test_lock();
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let args = history_args(temp_dir.path(), TO_FUTURE, FROM_PAST);

    let exit = commands::history::run(&args).await;

    assert_eq!(
        exit,
        exit_invalid_input(),
        "from > to should exit 2 (invalid input)"
    );
}

#[tokio::test]
async fn history_uses_sentinel_history_path_env_override() {
    let _guard = env_test_lock();
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let custom_ledger = temp_dir.path().join("custom-history.ndjson");

    let lock_hash = Some("sha256-abc".to_string());
    let pkgs = vec![package("axios", "1.7.9")];

    let exit = temp_env::async_with_vars(
        [(
            "SENTINEL_HISTORY_PATH",
            Some(custom_ledger.as_os_str().to_os_string()),
        )],
        async {
            let append_history_events_params = AppendHistoryEventsParams {
                current_working_directory: temp_dir.path(),
                package_manager: PackageManager::Npm,
                command: "ci",
                lockfile_path: "package-lock.json",
                lock_hash_before: &lock_hash,
                lock_hash_after: &lock_hash,
                packages: &pkgs,
            };

            append_history_events(append_history_events_params)
                .expect("seeding via env override should succeed");

            let args = HistoryArgs {
                package: Some("axios".to_string()),
                ..history_args(temp_dir.path(), FROM_PAST, TO_FUTURE)
            };

            commands::history::run(&args).await
        },
    )
    .await;

    assert_eq!(
        exit, EXIT_SUCCESS,
        "env-overridden ledger query should exit 0"
    );
}
