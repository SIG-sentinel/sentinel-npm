#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::err_expect,
    clippy::too_many_arguments,
    clippy::needless_raw_string_hashes,
    unused_qualifications
)]

use clap::Parser;
use sentinel::types::{Cli, Commands, OutputFormat};

#[test]
fn test_check_command_defaults() {
    let cli = Cli::try_parse_from(["sentinel", "check"]).expect("check should parse");

    match cli.command {
        Commands::Check(args) => {
            assert!(!args.omit_dev);
            assert!(!args.omit_optional);
            assert!(!args.quiet);
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
        "--quiet",
    ])
    .expect("check with flags should parse");

    match cli.command {
        Commands::Check(args) => {
            assert!(args.omit_dev);
            assert!(args.omit_optional);
            assert!(args.quiet);
            assert_eq!(args.timeout, 9000);
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
