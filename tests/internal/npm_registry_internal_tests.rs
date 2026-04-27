use std::collections::HashMap;

use super::{
    encode_package_name, is_retryable_status, parse_npmrc_content, resolve_auth_token_for_url,
    resolve_registry_base_for_package, resolve_registry_settings,
};
use crate::types::{ParsedNpmrc, ResolveAuthTokenParams, ResolveRegistryBaseParams};

#[test]
fn encode_package_name_encodes_scoped_package() {
    let encoded = encode_package_name("@scope/pkg");
    assert_eq!(encoded, "@scope%2Fpkg");
}

#[test]
fn encode_package_name_keeps_unscoped_package() {
    let encoded = encode_package_name("lodash");
    assert_eq!(encoded, "lodash");
}

#[test]
fn retryable_status_matches_policy() {
    assert!(is_retryable_status(429));
    assert!(is_retryable_status(500));
    assert!(is_retryable_status(503));

    assert!(!is_retryable_status(200));
    assert!(!is_retryable_status(404));
    assert!(!is_retryable_status(499));
}

#[test]
fn parse_npmrc_content_extracts_default_scoped_and_auth_data() {
    let npmrc = r"
            registry=https://registry.npmjs.org/
            @acme:registry=https://npm.pkg.github.com/
            //npm.pkg.github.com/:_authToken=ghp_test_token
        ";

    let ParsedNpmrc {
        default_registry,
        scoped_registries: scoped,
        auth_token_prefixes: auth_tokens,
    } = parse_npmrc_content(npmrc);

    assert_eq!(
        default_registry.as_deref(),
        Some("https://registry.npmjs.org/")
    );
    assert_eq!(
        scoped.get("acme").map(String::as_str),
        Some("https://npm.pkg.github.com")
    );
    assert_eq!(auth_tokens.len(), 2);
    assert_eq!(auth_tokens[0].0, "https://npm.pkg.github.com");
    assert_eq!(auth_tokens[1].0, "http://npm.pkg.github.com");
    assert_eq!(auth_tokens[0].1, "ghp_test_token");
    assert_eq!(auth_tokens[1].1, "ghp_test_token");
}

#[test]
fn resolve_registry_base_prefers_scoped_when_available() {
    let mut scoped = HashMap::new();
    scoped.insert("acme".to_string(), "https://npm.pkg.github.com".to_string());

    let scoped_base = resolve_registry_base_for_package(ResolveRegistryBaseParams {
        package_name: "@acme/design-system",
        default_registry_base: "https://registry.npmjs.org",
        scoped_registry_bases: &scoped,
    });
    let default_base = resolve_registry_base_for_package(ResolveRegistryBaseParams {
        package_name: "lodash",
        default_registry_base: "https://registry.npmjs.org",
        scoped_registry_bases: &scoped,
    });

    assert_eq!(scoped_base, "https://npm.pkg.github.com");
    assert_eq!(default_base, "https://registry.npmjs.org");
}

#[test]
fn resolve_auth_token_uses_longest_matching_prefix() {
    let auth_tokens = vec![
        (
            "https://registry.npmjs.org".to_string(),
            "public-token".to_string(),
        ),
        (
            "https://registry.npmjs.org/@acme".to_string(),
            "scoped-token".to_string(),
        ),
    ];

    let token = resolve_auth_token_for_url(ResolveAuthTokenParams {
        url: "https://registry.npmjs.org/@acme/design-system/-/pkg-1.0.0.tgz",
        auth_token_prefixes: &auth_tokens,
    });
    assert_eq!(token, Some("scoped-token"));
}

#[test]
fn resolve_registry_settings_prefers_project_npmrc_over_home_npmrc() {
    let project_dir = tempfile::tempdir().expect("project tempdir should be created");
    let home_dir = tempfile::tempdir().expect("home tempdir should be created");

    std::fs::write(
        home_dir.path().join(".npmrc"),
        "registry=https://registry.npmjs.org/\n",
    )
    .expect("home .npmrc should be written");

    std::fs::write(
        project_dir.path().join(".npmrc"),
        "registry=https://npm.pkg.github.com/\n@acme:registry=https://npm.pkg.github.com/\n//npm.pkg.github.com/:_authToken=ghp_project_token\n",
    )
    .expect("project .npmrc should be written");

    temp_env::with_var(
        "HOME",
        Some(home_dir.path().to_string_lossy().as_ref()),
        || {
            let settings = resolve_registry_settings(project_dir.path());

            assert_eq!(settings.default_registry_base, "https://npm.pkg.github.com");
            assert_eq!(
                settings
                    .scoped_registry_bases
                    .get("acme")
                    .map(String::as_str),
                Some("https://npm.pkg.github.com")
            );

            let token_matches = settings
                .auth_token_prefixes
                .iter()
                .filter(|(prefix, token)| {
                    prefix == "https://npm.pkg.github.com" && token == "ghp_project_token"
                })
                .count();

            assert!(
                token_matches >= 1,
                "expected project token for npm.pkg.github.com"
            );
        },
    );
}
