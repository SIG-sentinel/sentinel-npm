#![allow(clippy::expect_used, clippy::panic)]

use clap::Parser;

use crate::cli_parsers::parse_rfc3339_timestamp;
use crate::types::{Cli, Commands};

#[test]
fn install_spec_accepts_package_without_version_for_candidate_resolution() {
    let cli = Cli::try_parse_from(["sentinel", "install", "lodash"])
        .expect("install should accept package without explicit version to resolve a candidate");

    match cli.command {
        Commands::Install(args) => {
            assert_eq!(args.packages, vec!["lodash".to_string()]);
        }
        _ => panic!("expected install command"),
    }
}

#[test]
fn install_spec_accepts_latest_tag_for_candidate_resolution() {
    let cli = Cli::try_parse_from(["sentinel", "install", "lodash@latest"])
        .expect("install should accept latest tag to resolve and pin an exact candidate");

    match cli.command {
        Commands::Install(args) => {
            assert_eq!(args.packages, vec!["lodash@latest".to_string()]);
        }
        _ => panic!("expected install command"),
    }
}

#[test]
fn history_timestamp_accepts_relative_ago_input() {
    let result = parse_rfc3339_timestamp("7 days ago");

    assert!(
        result.is_ok(),
        "history timestamp parser should accept relative 'ago' expressions"
    );
}

#[test]
fn history_timestamp_accepts_now_keyword() {
    let result = parse_rfc3339_timestamp("now");

    assert!(
        result.is_ok(),
        "history timestamp parser should accept 'now'"
    );
}

#[test]
fn history_timestamp_accepts_rfc3339_datetime() {
    let result = parse_rfc3339_timestamp("2026-04-23T12:34:56+00:00");

    assert!(
        result.is_ok(),
        "history timestamp parser should keep accepting explicit RFC3339 datetimes"
    );
}
