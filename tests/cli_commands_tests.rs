#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::err_expect,
    clippy::too_many_arguments,
    clippy::needless_raw_string_hashes,
    unused_qualifications
)]

use clap::Parser;
use sentinel::types::{ArtifactStore, Cli, Commands, HistoryOutputFormat, OutputFormat};

#[test]
fn test_check_command_defaults() {
    let cli = Cli::try_parse_from(["sentinel", "check"]).expect("check should parse");

    match cli.command {
        Commands::Check(args) => {
            assert!(!args.omit_dev);
            assert!(!args.omit_optional);
            assert!(!args.quiet);
            assert!(args.registry_max_in_flight.is_none());
            assert_eq!(
                args.timeout,
                sentinel::constants::DEFAULT_REGISTRY_TIMEOUT_MS
            );
            assert_eq!(args.format, OutputFormat::Text);
            assert_eq!(args.cwd.to_string_lossy(), ".");
        }
        _ => panic!("expected check command"),
    }
}

#[test]
fn test_check_command_with_flags() {
    let cli = Cli::try_parse_from([
        "sentinel",
        "check",
        "--omit-dev",
        "--omit-optional",
        "--format",
        "json",
        "--cwd",
        "./tmp",
        "--timeout",
        "9000",
        "--registry-max-in-flight",
        "4",
        "--quiet",
    ])
    .expect("check with flags should parse");

    match cli.command {
        Commands::Check(args) => {
            assert!(args.omit_dev);
            assert!(args.omit_optional);
            assert!(args.quiet);
            assert_eq!(args.timeout, 9000);
            assert_eq!(args.registry_max_in_flight, Some(4));
            assert_eq!(args.format, OutputFormat::Json);
            assert_eq!(args.cwd.to_string_lossy(), "./tmp");
        }
        _ => panic!("expected check command"),
    }
}

#[test]
fn test_ci_command_defaults() {
    let cli = Cli::try_parse_from(["sentinel", "ci"]).expect("ci should parse");

    match cli.command {
        Commands::Ci(args) => {
            assert!(!args.omit_dev);
            assert!(!args.omit_optional);
            assert!(!args.allow_scripts, "scripts should be blocked by default");
            assert!(!args.dry_run);
            assert!(!args.quiet);
            assert!(args.registry_max_in_flight.is_none());
            assert_eq!(args.format, OutputFormat::Text);
            assert_eq!(args.report.to_string_lossy(), "sentinel-report.json");
            assert_eq!(args.timeout, sentinel::constants::CI_REGISTRY_TIMEOUT_MS);
            assert!(!args.post_verify);
        }
        _ => panic!("expected ci command"),
    }
}

#[test]
fn test_ci_command_with_allow_scripts_flag() {
    let cli = Cli::try_parse_from([
        "sentinel",
        "ci",
        "--omit-dev",
        "--omit-optional",
        "--allow-scripts",
        "--dry-run",
        "--format",
        "text",
        "--report",
        "./out/report.json",
        "--cwd",
        "./project",
        "--timeout",
        "12000",
        "--registry-max-in-flight",
        "5",
        "--post-verify",
        "--quiet",
    ])
    .expect("ci with flags should parse");

    match cli.command {
        Commands::Ci(args) => {
            assert!(args.omit_dev);
            assert!(args.omit_optional);
            assert!(args.allow_scripts, "--allow-scripts should enable scripts");
            assert!(args.dry_run);
            assert!(args.quiet);
            assert_eq!(args.format, OutputFormat::Text);
            assert_eq!(args.report.to_string_lossy(), "./out/report.json");
            assert_eq!(args.cwd.to_string_lossy(), "./project");
            assert_eq!(args.timeout, 12000);
            assert_eq!(args.registry_max_in_flight, Some(5));
            assert!(args.post_verify);
        }
        _ => panic!("expected ci command"),
    }
}

#[test]
fn test_install_command_requires_package_with_exact_version() {
    let parsed = Cli::try_parse_from(["sentinel", "install"]);
    assert!(parsed.is_err(), "install must require <package>@<version>");
}

#[test]
fn test_install_command_defaults() {
    let cli = Cli::try_parse_from(["sentinel", "install", "left-pad@1.3.0"])
        .expect("install should parse");

    match cli.command {
        Commands::Install(args) => {
            assert_eq!(args.package, "left-pad@1.3.0");
            assert!(!args.allow_scripts, "scripts should be blocked by default");
            assert!(!args.dry_run);
            assert!(!args.quiet);
            assert!(args.registry_max_in_flight.is_none());
            assert_eq!(
                args.timeout,
                sentinel::constants::DEFAULT_REGISTRY_TIMEOUT_MS
            );
            assert_eq!(args.format, OutputFormat::Text);
            assert_eq!(args.cwd.to_string_lossy(), ".");
            assert!(!args.post_verify);
        }
        _ => panic!("expected install command"),
    }
}

#[test]
fn test_install_command_with_allow_scripts_flag() {
    let cli = Cli::try_parse_from([
        "sentinel",
        "install",
        "@scope/pkg@2.0.1",
        "--allow-scripts",
        "--dry-run",
        "--format",
        "json",
        "--cwd",
        "./project",
        "--timeout",
        "8000",
        "--registry-max-in-flight",
        "3",
        "--post-verify",
        "--quiet",
    ])
    .expect("install with flags should parse");

    match cli.command {
        Commands::Install(args) => {
            assert_eq!(args.package, "@scope/pkg@2.0.1");
            assert!(args.allow_scripts, "--allow-scripts should enable scripts");
            assert!(args.dry_run);
            assert!(args.quiet);
            assert_eq!(args.timeout, 8000);
            assert_eq!(args.registry_max_in_flight, Some(3));
            assert_eq!(args.format, OutputFormat::Json);
            assert_eq!(args.cwd.to_string_lossy(), "./project");
            assert!(args.post_verify);
        }
        _ => panic!("expected install command"),
    }
}

#[test]
fn test_install_command_accepts_range_version_for_candidate_resolution() {
    let cli = Cli::try_parse_from(["sentinel", "install", "left-pad@^1.3.0"])
        .expect("install should accept semver ranges for candidate resolution");

    match cli.command {
        Commands::Install(args) => {
            assert_eq!(args.package, "left-pad@^1.3.0");
        }
        _ => panic!("expected install command"),
    }
}

#[test]
fn test_history_command_defaults() {
    let cli = Cli::try_parse_from([
        "sentinel",
        "history",
        "--from",
        "2026-04-20T00:00:00+00:00",
        "--to",
        "2026-04-23T23:59:59+00:00",
    ])
    .expect("history should parse");

    match cli.command {
        Commands::History(args) => {
            assert!(args.package.is_none());
            assert!(args.version.is_none());
            assert!(args.project.is_none());
            assert!(args.package_manager.is_none());
            assert!(!args.quiet);
            assert_eq!(args.format, HistoryOutputFormat::Text);
            assert_eq!(args.cwd.to_string_lossy(), ".");
        }
        _ => panic!("expected history command"),
    }
}

#[test]
fn test_history_command_with_all_optional_flags() {
    let cli = Cli::try_parse_from([
        "sentinel",
        "history",
        "--from",
        "7 days ago",
        "--to",
        "now",
        "--package",
        "left-pad",
        "--version",
        "1.3.0",
        "--project",
        "/tmp/demo",
        "--package-manager",
        "npm",
        "--format",
        "json",
        "--cwd",
        "./workspace",
        "--quiet",
    ])
    .expect("history with all optional flags should parse");

    match cli.command {
        Commands::History(args) => {
            assert_eq!(args.package.as_deref(), Some("left-pad"));
            assert_eq!(args.version.as_deref(), Some("1.3.0"));
            assert_eq!(
                args.project
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string()),
                Some("/tmp/demo".to_string())
            );
            assert_eq!(args.package_manager.as_deref(), Some("npm"));
            assert!(args.quiet);
            assert_eq!(args.format, HistoryOutputFormat::Json);
            assert_eq!(args.cwd.to_string_lossy(), "./workspace");
        }
        _ => panic!("expected history command"),
    }
}

#[test]
fn test_history_version_requires_package() {
    let cli = Cli::try_parse_from([
        "sentinel",
        "history",
        "--from",
        "2026-04-20T00:00:00+00:00",
        "--to",
        "2026-04-23T23:59:59+00:00",
        "--version",
        "1.3.0",
    ]);

    assert!(
        cli.is_err(),
        "history --version must require --package according to CLI contract"
    );
}

#[test]
fn test_global_artifact_store_values_parse_for_check() {
    let cli_memory = Cli::try_parse_from(["sentinel", "--artifact-store", "memory", "check"])
        .expect("memory artifact store should parse");

    let cli_spool = Cli::try_parse_from(["sentinel", "--artifact-store", "spool", "check"])
        .expect("spool artifact store should parse");

    let cli_auto = Cli::try_parse_from(["sentinel", "--artifact-store", "auto", "check"])
        .expect("auto artifact store should parse");

    assert_eq!(cli_memory.artifact_store, ArtifactStore::Memory);
    assert_eq!(cli_spool.artifact_store, ArtifactStore::Spool);
    assert_eq!(cli_auto.artifact_store, ArtifactStore::Auto);
}

#[test]
fn test_global_artifact_store_rejects_unknown_value() {
    let cli = Cli::try_parse_from(["sentinel", "--artifact-store", "invalid", "check"]);

    assert!(
        cli.is_err(),
        "invalid --artifact-store value should be rejected by clap"
    );
}

#[test]
fn test_registry_max_in_flight_rejects_zero() {
    let check_cli = Cli::try_parse_from(["sentinel", "check", "--registry-max-in-flight", "0"]);

    let install_cli = Cli::try_parse_from([
        "sentinel",
        "install",
        "left-pad@1.3.0",
        "--registry-max-in-flight",
        "0",
    ]);

    let ci_cli = Cli::try_parse_from(["sentinel", "ci", "--registry-max-in-flight", "0"]);

    assert!(check_cli.is_err());
    assert!(install_cli.is_err());
    assert!(ci_cli.is_err());
}
