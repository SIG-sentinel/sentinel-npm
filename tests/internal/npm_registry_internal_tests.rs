use std::collections::HashMap;

use super::{
    encode_package_name, is_retryable_status, parse_npmrc_content, resolve_auth_token_for_url,
    resolve_max_in_flight_requests, resolve_registry_base_for_package, resolve_registry_settings,
};
use crate::constants::{REGISTRY_MAX_IN_FLIGHT_REQUESTS, REGISTRY_MAX_IN_FLIGHT_REQUESTS_ENV};
use crate::types::{ParsedNpmrc, ResolveAuthTokenParams, ResolveRegistryBaseParams};

const SCOPED_PACKAGE_NAME: &str = "@scope/pkg";
const ENCODED_SCOPED_PACKAGE_NAME: &str = "@scope%2Fpkg";
const UNSCOPED_PACKAGE_NAME: &str = "lodash";
const PUBLIC_REGISTRY_URL: &str = "https://registry.npmjs.org";
const PUBLIC_REGISTRY_URL_WITH_SLASH: &str = "https://registry.npmjs.org/";
const PRIVATE_REGISTRY_URL: &str = "https://npm.pkg.github.com";
const PRIVATE_REGISTRY_HTTP_URL: &str = "http://npm.pkg.github.com";
const TEST_SCOPE: &str = "acme";
const TEST_TOKEN: &str = "ghp_test_token";
const PROJECT_TOKEN: &str = "ghp_project_token";
const HOME_ENV_VAR: &str = "HOME";
const SCOPED_PACKAGE_REQUEST_PATH: &str =
    "https://registry.npmjs.org/@acme/design-system/-/pkg-1.0.0.tgz";
const CLI_OVERRIDE_MAX_IN_FLIGHT: usize = 3;
const ENV_MAX_IN_FLIGHT_HIGH: &str = "9";
const ENV_MAX_IN_FLIGHT_MEDIUM: &str = "7";
const INVALID_MAX_IN_FLIGHT: &str = "invalid";
const ZERO_MAX_IN_FLIGHT: &str = "0";
const HTTP_STATUS_OK: u16 = 200;
const HTTP_STATUS_NOT_FOUND_TEST: u16 = 404;
const HTTP_STATUS_TOO_MANY_REQUESTS_TEST: u16 = 429;
const HTTP_STATUS_SERVER_ERROR_TEST: u16 = 500;
const HTTP_STATUS_SERVICE_UNAVAILABLE: u16 = 503;
const HTTP_STATUS_CLIENT_CLOSED_REQUEST: u16 = 499;
const PROJECT_NPMRC_CONTENT: &str = "registry=https://npm.pkg.github.com/\n@acme:registry=https://npm.pkg.github.com/\n//npm.pkg.github.com/:_authToken=ghp_project_token\n";
const HOME_NPMRC_CONTENT: &str = "registry=https://registry.npmjs.org/\n";
const PARSE_NPMRC_FIXTURE: &str = r"
            registry=https://registry.npmjs.org/
            @acme:registry=https://npm.pkg.github.com/
            //npm.pkg.github.com/:_authToken=ghp_test_token
        ";

#[test]
fn encode_package_name_encodes_scoped_package() {
    let encoded = encode_package_name(SCOPED_PACKAGE_NAME);
    assert_eq!(encoded, ENCODED_SCOPED_PACKAGE_NAME);
}

#[test]
fn encode_package_name_keeps_unscoped_package() {
    let encoded = encode_package_name(UNSCOPED_PACKAGE_NAME);
    assert_eq!(encoded, UNSCOPED_PACKAGE_NAME);
}

#[test]
fn retryable_status_matches_policy() {
    assert!(is_retryable_status(HTTP_STATUS_TOO_MANY_REQUESTS_TEST));
    assert!(is_retryable_status(HTTP_STATUS_SERVER_ERROR_TEST));
    assert!(is_retryable_status(HTTP_STATUS_SERVICE_UNAVAILABLE));

    assert!(!is_retryable_status(HTTP_STATUS_OK));
    assert!(!is_retryable_status(HTTP_STATUS_NOT_FOUND_TEST));
    assert!(!is_retryable_status(HTTP_STATUS_CLIENT_CLOSED_REQUEST));
}

#[test]
fn parse_npmrc_content_extracts_default_scoped_and_auth_data() {
    let npmrc = PARSE_NPMRC_FIXTURE;

    let ParsedNpmrc {
        default_registry,
        scoped_registries: scoped,
        auth_token_prefixes: auth_tokens,
    } = parse_npmrc_content(npmrc);

    assert_eq!(
        default_registry.as_deref(),
        Some(PUBLIC_REGISTRY_URL_WITH_SLASH)
    );
    assert_eq!(
        scoped.get(TEST_SCOPE).map(String::as_str),
        Some(PRIVATE_REGISTRY_URL)
    );
    assert_eq!(auth_tokens.len(), 2);
    assert_eq!(auth_tokens[0].0, PRIVATE_REGISTRY_URL);
    assert_eq!(auth_tokens[1].0, PRIVATE_REGISTRY_HTTP_URL);
    assert_eq!(auth_tokens[0].1, TEST_TOKEN);
    assert_eq!(auth_tokens[1].1, TEST_TOKEN);
}

#[test]
fn resolve_registry_base_prefers_scoped_when_available() {
    let mut scoped = HashMap::new();
    scoped.insert(TEST_SCOPE.to_string(), PRIVATE_REGISTRY_URL.to_string());

    let resolve_registry_base_params = ResolveRegistryBaseParams {
        package_name: "@acme/design-system",
        default_registry_base: PUBLIC_REGISTRY_URL,
        scoped_registry_bases: &scoped,
    };
    let scoped_base = resolve_registry_base_for_package(resolve_registry_base_params);
    let resolve_registry_base_params = ResolveRegistryBaseParams {
        package_name: UNSCOPED_PACKAGE_NAME,
        default_registry_base: PUBLIC_REGISTRY_URL,
        scoped_registry_bases: &scoped,
    };
    let default_base = resolve_registry_base_for_package(resolve_registry_base_params);

    assert_eq!(scoped_base, PRIVATE_REGISTRY_URL);
    assert_eq!(default_base, PUBLIC_REGISTRY_URL);
}

#[test]
fn resolve_auth_token_uses_longest_matching_prefix() {
    let auth_tokens = vec![
        (PUBLIC_REGISTRY_URL.to_string(), "public-token".to_string()),
        (
            "https://registry.npmjs.org/@acme".to_string(),
            "scoped-token".to_string(),
        ),
    ];

    let resolve_auth_token_params = ResolveAuthTokenParams {
        url: SCOPED_PACKAGE_REQUEST_PATH,
        auth_token_prefixes: &auth_tokens,
    };
    let token = resolve_auth_token_for_url(resolve_auth_token_params);
    assert_eq!(token, Some("scoped-token"));
}

#[test]
fn resolve_registry_settings_prefers_project_npmrc_over_home_npmrc() {
    let project_dir = tempfile::tempdir().expect("project tempdir should be created");
    let home_dir = tempfile::tempdir().expect("home tempdir should be created");

    std::fs::write(home_dir.path().join(".npmrc"), HOME_NPMRC_CONTENT)
        .expect("home .npmrc should be written");

    std::fs::write(project_dir.path().join(".npmrc"), PROJECT_NPMRC_CONTENT)
        .expect("project .npmrc should be written");

    temp_env::with_var(
        HOME_ENV_VAR,
        Some(home_dir.path().to_string_lossy().as_ref()),
        || {
            let settings = resolve_registry_settings(project_dir.path());

            assert_eq!(settings.default_registry_base, PRIVATE_REGISTRY_URL);
            assert_eq!(
                settings
                    .scoped_registry_bases
                    .get(TEST_SCOPE)
                    .map(String::as_str),
                Some(PRIVATE_REGISTRY_URL)
            );

            let token_matches = settings
                .auth_token_prefixes
                .iter()
                .filter(|(prefix, token)| prefix == PRIVATE_REGISTRY_URL && token == PROJECT_TOKEN)
                .count();

            assert!(
                token_matches >= 1,
                "expected project token for npm.pkg.github.com"
            );
        },
    );
}

#[test]
fn resolve_max_in_flight_prefers_cli_override_over_env() {
    temp_env::with_var(
        REGISTRY_MAX_IN_FLIGHT_REQUESTS_ENV,
        Some(ENV_MAX_IN_FLIGHT_HIGH),
        || {
            let resolved = resolve_max_in_flight_requests(Some(CLI_OVERRIDE_MAX_IN_FLIGHT));

            assert_eq!(resolved, CLI_OVERRIDE_MAX_IN_FLIGHT);
        },
    );
}

#[test]
fn resolve_max_in_flight_reads_env_when_cli_is_absent() {
    temp_env::with_var(
        REGISTRY_MAX_IN_FLIGHT_REQUESTS_ENV,
        Some(ENV_MAX_IN_FLIGHT_MEDIUM),
        || {
            let resolved = resolve_max_in_flight_requests(None);

            assert_eq!(resolved, 7);
        },
    );
}

#[test]
fn resolve_max_in_flight_uses_default_for_invalid_or_zero_env() {
    temp_env::with_var(
        REGISTRY_MAX_IN_FLIGHT_REQUESTS_ENV,
        Some(INVALID_MAX_IN_FLIGHT),
        || {
            let resolved = resolve_max_in_flight_requests(None);

            assert_eq!(resolved, REGISTRY_MAX_IN_FLIGHT_REQUESTS);
        },
    );

    temp_env::with_var(
        REGISTRY_MAX_IN_FLIGHT_REQUESTS_ENV,
        Some(ZERO_MAX_IN_FLIGHT),
        || {
            let resolved = resolve_max_in_flight_requests(None);

            assert_eq!(resolved, REGISTRY_MAX_IN_FLIGHT_REQUESTS);
        },
    );
}
