use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;

use crate::constants::{
    DOWNLOAD_TARBALL_TIMEOUT_SECS, NPM_ERR_PARSE_RESPONSE_TEMPLATE,
    NPM_ERR_REGISTRY_RESPONSE_TEMPLATE, NPM_ERR_TIMEOUT_DOWNLOAD_TEMPLATE, NPM_REGISTRY_BASE_URL,
    NPM_SCOPED_SEPARATOR, NPM_USER_AGENT_PREFIX, REGISTRY_MAX_IN_FLIGHT_REQUESTS,
    REGISTRY_MAX_IN_FLIGHT_REQUESTS_ENV, REGISTRY_MAX_RETRIES, REGISTRY_RETRY_BASE_DELAY_MS,
    REGISTRY_RETRY_MAX_JITTER_MS,
    messages::{
        HTTP_PREFIXED_REGISTRY_TEMPLATE, HTTPS_PREFIXED_REGISTRY_TEMPLATE,
        NPM_IDENTITY_PATH_TEMPLATE, REGISTRY_VERSION_URL_TEMPLATE,
    },
    render_template_from_iter,
    template::render_template,
};
use crate::types::{
    AttestationsResponse, AuthTokenPrefixPair, BuildRegistryRequestParams, NpmProvenance,
    NpmRegistryNewParams, NpmVersionMeta, NpmrcEntryKind, PackageRef, ParsedNpmrc,
    RegistryResponseClassification, RegistrySettings, ResolveAuthTokenParams,
    ResolveRegistryBaseParams, ResolveTimedResponseParams, SentinelError, SlsaV1Payload,
    templated_http_error,
};

pub(super) use crate::types::NpmRegistry;

const SLSA_PREDICATE_TYPE_V1: &str = "https://slsa.dev/provenance/v1";
const SUBJECT_DIGEST_ALGO: &str = "sha512";
const INTEGRITY_SHA512_PREFIX: &str = "sha512-";
const HTTP_STATUS_NOT_FOUND: u16 = 404;
const HTTP_STATUS_TOO_MANY_REQUESTS: u16 = 429;
const HTTP_STATUS_SERVER_ERROR_MIN: u16 = 500;
const NPMRC_FILE_NAME: &str = ".npmrc";
const NPMRC_DEFAULT_REGISTRY_KEY: &str = "registry";
const NPMRC_SCOPED_REGISTRY_SUFFIX: &str = ":registry";
const NPMRC_AUTH_TOKEN_SUFFIX: &str = ":_authToken";
const NPMRC_AUTH_TOKEN_PREFIX: &str = "//";
const REGISTRY_REQUEST_GATE_CLOSED_MSG: &str = "registry request gate closed";
const HEX_BYTE_PAIR_LEN: usize = 2;
const HEX_RADIX: u32 = 16;
const HEX_HIGH_NIBBLE_SHIFT_BITS: u32 = 4;
const SCOPED_PACKAGE_PREFIX: char = '@';
const PACKAGE_SCOPE_SEPARATOR: char = '/';
const RETRY_BACKOFF_MULTIPLIER_BASE: u64 = 2;
const JITTER_INCLUSIVE_UPPER_BOUND_OFFSET: u64 = 1;
const POSITIVE_USIZE_MIN: usize = 0;
const LINE_SEPARATOR: char = '\n';
const NPMRC_HASH_COMMENT_PREFIX: char = '#';
const NPMRC_SEMICOLON_COMMENT_PREFIX: char = ';';
const NPMRC_ENTRY_SEPARATOR: char = '=';
const DOUBLE_QUOTE_CHAR: char = '"';
const SINGLE_QUOTE_CHAR: char = '\'';

fn hex_to_bytes(hex: &str) -> Option<Vec<u8>> {
    let hex_bytes = hex.as_bytes();
    let is_even_length = hex_bytes.len() % HEX_BYTE_PAIR_LEN == 0;

    if !is_even_length {
        return None;
    }

    hex_bytes
        .chunks_exact(HEX_BYTE_PAIR_LEN)
        .map(|pair| {
            let high_nibble = char::from(pair[0]).to_digit(HEX_RADIX)?;
            let low_nibble = char::from(pair[1]).to_digit(HEX_RADIX)?;
            let combined_byte = (high_nibble << HEX_HIGH_NIBBLE_SHIFT_BITS) | low_nibble;

            u8::try_from(combined_byte).ok()
        })
        .collect::<Option<Vec<u8>>>()
}

fn classify_npmrc_entry_key(parsed_key: &str) -> NpmrcEntryKind {
    let is_scoped_registry_key = parsed_key.starts_with(SCOPED_PACKAGE_PREFIX);
    let has_scoped_registry_suffix = parsed_key.ends_with(NPMRC_SCOPED_REGISTRY_SUFFIX);
    let is_scoped_registry_config = is_scoped_registry_key && has_scoped_registry_suffix;
    let has_auth_token_prefix = parsed_key.starts_with(NPMRC_AUTH_TOKEN_PREFIX);
    let has_auth_token_suffix = parsed_key.ends_with(NPMRC_AUTH_TOKEN_SUFFIX);
    let is_auth_token_config = has_auth_token_prefix && has_auth_token_suffix;

    match (
        parsed_key == NPMRC_DEFAULT_REGISTRY_KEY,
        is_scoped_registry_config,
        is_auth_token_config,
    ) {
        (true, _, _) => NpmrcEntryKind::DefaultRegistry,
        (false, true, _) => NpmrcEntryKind::ScopedRegistry,
        (false, false, true) => NpmrcEntryKind::AuthToken,
        _ => NpmrcEntryKind::Ignore,
    }
}

fn classify_registry_response(
    status: reqwest::StatusCode,
    attempt: usize,
) -> RegistryResponseClassification {
    let status_code = status.as_u16();
    let is_retryable_response = is_retryable_status(status_code);
    let has_remaining_attempts = attempt < REGISTRY_MAX_RETRIES;
    let should_retry_response = is_retryable_response && has_remaining_attempts;
    let is_not_found = status_code == HTTP_STATUS_NOT_FOUND;
    let is_success_status = status.is_success();

    match (should_retry_response, is_not_found, is_success_status) {
        (true, _, _) => RegistryResponseClassification::Retry,
        (false, true, _) => RegistryResponseClassification::NotFound,
        (false, false, true) => RegistryResponseClassification::Success,
        (false, false, false) => RegistryResponseClassification::Failure,
    }
}

async fn should_retry_with_backoff(attempt: usize, should_retry: bool) -> bool {
    if !should_retry {
        return false;
    }

    sleep_before_retry(attempt).await;

    true
}

async fn resolve_timed_response(
    params: ResolveTimedResponseParams,
) -> Result<Option<reqwest::Response>, SentinelError> {
    let ResolveTimedResponseParams {
        response_result,
        attempt,
        timeout_error,
    } = params;

    match response_result {
        Ok(Ok(response)) => Ok(Some(response)),
        Ok(Err(error)) => {
            let has_retryable_network_error = should_retry_network_error(&error);
            let has_remaining_attempts = attempt < REGISTRY_MAX_RETRIES;
            let should_retry_request = has_retryable_network_error && has_remaining_attempts;

            if should_retry_with_backoff(attempt, should_retry_request).await {
                return Ok(None);
            }

            Err(SentinelError::from(error))
        }
        Err(_) => {
            let has_remaining_attempts = attempt < REGISTRY_MAX_RETRIES;

            if should_retry_with_backoff(attempt, has_remaining_attempts).await {
                return Ok(None);
            }

            Err(timeout_error)
        }
    }
}

fn parse_scope_from_registry_key(parsed_key: &str) -> String {
    parsed_key
        .trim_start_matches(SCOPED_PACKAGE_PREFIX)
        .trim_end_matches(NPMRC_SCOPED_REGISTRY_SUFFIX)
        .to_string()
}

fn expand_auth_token_prefixes(parsed_key: &str) -> AuthTokenPrefixPair {
    let prefix_without_token = parsed_key.trim_end_matches(NPMRC_AUTH_TOKEN_SUFFIX);
    let secure_registry_prefix = render_template(
        HTTPS_PREFIXED_REGISTRY_TEMPLATE,
        &[prefix_without_token.to_string()],
    );
    let insecure_registry_prefix = render_template(
        HTTP_PREFIXED_REGISTRY_TEMPLATE,
        &[prefix_without_token.to_string()],
    );

    AuthTokenPrefixPair {
        https_prefix: normalize_registry_base(&secure_registry_prefix),
        http_prefix: normalize_registry_base(&insecure_registry_prefix),
    }
}

fn resolve_provenance_identity(slsa_payload: &SlsaV1Payload) -> Option<String> {
    let predicate = slsa_payload.predicate.as_ref()?;
    let build_definition = predicate.build_definition.as_ref()?;
    let external_parameters = build_definition.external_parameters.as_ref()?;
    let workflow = external_parameters.workflow.as_ref()?;
    let repo = workflow.repository.as_deref()?;
    let path = workflow.path.as_deref()?;
    let ref_ = workflow.ref_.as_deref()?;
    let identity_path = render_template(
        NPM_IDENTITY_PATH_TEMPLATE,
        &[repo.to_string(), path.to_string(), ref_.to_string()],
    );

    Some(identity_path)
}

impl NpmRegistry {
    pub fn new(params: NpmRegistryNewParams<'_>) -> Result<Self, SentinelError> {
        let NpmRegistryNewParams {
            timeout_ms,
            registry_max_in_flight,
            current_working_directory,
        } = params;

        let client = build_registry_client(timeout_ms)?;
        let RegistrySettings {
            default_registry_base,
            scoped_registry_bases,
            auth_token_prefixes,
        } = resolve_registry_settings(current_working_directory);
        let max_in_flight_requests = resolve_max_in_flight_requests(registry_max_in_flight);

        let registry = Self {
            client,
            timeout: Duration::from_millis(timeout_ms),
            request_gate: Arc::new(tokio::sync::Semaphore::new(max_in_flight_requests)),
            default_registry_base,
            scoped_registry_bases,
            auth_token_prefixes,
        };

        Ok(registry)
    }
    pub async fn fetch_version(
        &self,
        package_ref: &PackageRef,
    ) -> Result<NpmVersionMeta, SentinelError> {
        let resolve_registry_base_params = ResolveRegistryBaseParams {
            package_name: &package_ref.name,
            default_registry_base: &self.default_registry_base,
            scoped_registry_bases: &self.scoped_registry_bases,
        };
        let registry_base = resolve_registry_base_for_package(resolve_registry_base_params);
        let url = render_template(
            REGISTRY_VERSION_URL_TEMPLATE,
            &[
                registry_base,
                encode_package_name(&package_ref.name),
                package_ref.version.clone(),
            ],
        );

        for attempt in 0..=REGISTRY_MAX_RETRIES {
            let build_registry_request_params = BuildRegistryRequestParams {
                client: &self.client,
                url: &url,
                auth_token_prefixes: &self.auth_token_prefixes,
            };
            let request_builder = build_registry_request(build_registry_request_params);
            let permit =
                self.request_gate.acquire().await.map_err(|_| {
                    SentinelError::Http(REGISTRY_REQUEST_GATE_CLOSED_MSG.to_string())
                })?;

            let response_result = tokio::time::timeout(self.timeout, request_builder.send()).await;

            drop(permit);

            let timeout_error = SentinelError::RegistryTimeout {
                package: package_ref.name.clone(),
                version: package_ref.version.clone(),
                ms: u64::try_from(self.timeout.as_millis()).unwrap_or(u64::MAX),
            };
            let resolve_timed_response_params = ResolveTimedResponseParams {
                response_result,
                attempt,
                timeout_error,
            };
            let resolved_response = resolve_timed_response(resolve_timed_response_params).await;

            let response = match resolved_response {
                Ok(Some(response)) => response,
                Ok(None) => continue,
                Err(error) => return Err(error),
            };

            let status = response.status();
            let response_classification = classify_registry_response(status, attempt);

            match response_classification {
                RegistryResponseClassification::Retry => {
                    sleep_before_retry(attempt).await;

                    continue;
                }
                RegistryResponseClassification::NotFound => {
                    return Err(SentinelError::NoIntegrity {
                        package: package_ref.name.clone(),
                        version: package_ref.version.clone(),
                    });
                }
                RegistryResponseClassification::Failure => {
                    let registry_response_error = templated_http_error(
                        NPM_ERR_REGISTRY_RESPONSE_TEMPLATE,
                        &[status.to_string(), package_ref.to_string()],
                    );

                    return Err(registry_response_error);
                }
                RegistryResponseClassification::Success => {}
            }

            let mut meta = response.json::<NpmVersionMeta>().await.map_err(|error| {
                templated_http_error(
                    NPM_ERR_PARSE_RESPONSE_TEMPLATE,
                    &[package_ref.to_string(), error.to_string()],
                )
            })?;

            self.populate_provenance_from_attestations_if_needed(&mut meta)
                .await;

            return Ok(meta);
        }

        Err(SentinelError::RegistryTimeout {
            package: package_ref.name.clone(),
            version: package_ref.version.clone(),
            ms: u64::try_from(self.timeout.as_millis()).unwrap_or(u64::MAX),
        })
    }

    async fn populate_provenance_from_attestations_if_needed(&self, meta: &mut NpmVersionMeta) {
        if meta.dist.provenance.is_some() {
            return;
        }

        let Some(attestations_url) = meta.dist.attestations.as_ref().map(|a| a.url.as_str()) else {
            return;
        };

        meta.dist.provenance = self
            .fetch_provenance_from_attestations(attestations_url)
            .await;
    }

    async fn fetch_provenance_from_attestations(&self, url: &str) -> Option<NpmProvenance> {
        let request_builder = self.client.get(url);
        let permit = self.request_gate.acquire().await.ok()?;
        let response_result = tokio::time::timeout(self.timeout, request_builder.send()).await;

        drop(permit);

        let response = response_result.ok()?.ok()?;
        let is_success = response.status().is_success();

        if !is_success {
            return None;
        }

        let body: AttestationsResponse = response.json().await.ok()?;
        let slsa_entry = body
            .attestations
            .into_iter()
            .find(|a| a.predicate_type == SLSA_PREDICATE_TYPE_V1)?;

        let encoded_payload = slsa_entry.bundle.dsse_envelope.payload;
        let payload_bytes = base64::engine::general_purpose::STANDARD
            .decode(&encoded_payload)
            .ok()?;
        let slsa_payload: SlsaV1Payload = serde_json::from_slice(&payload_bytes).ok()?;

        let subject = slsa_payload.subject.first()?;
        let sha512_hex = subject.digest.get(SUBJECT_DIGEST_ALGO)?;
        let sha512_bytes = hex_to_bytes(sha512_hex)?;
        let sha512_b64 = base64::engine::general_purpose::STANDARD.encode(&sha512_bytes);
        let subject_integrity = format!("{INTEGRITY_SHA512_PREFIX}{sha512_b64}");

        let identity = resolve_provenance_identity(&slsa_payload);

        let npm_provenance = NpmProvenance {
            subject_integrity: Some(subject_integrity),
            issuer: None,
            identity,
            source: Some(url.to_string()),
        };

        Some(npm_provenance)
    }

    pub async fn download_tarball(&self, url: &str) -> Result<reqwest::Response, SentinelError> {
        for attempt in 0..=REGISTRY_MAX_RETRIES {
            let build_registry_request_params = BuildRegistryRequestParams {
                client: &self.client,
                url,
                auth_token_prefixes: &self.auth_token_prefixes,
            };
            let request_builder = build_registry_request(build_registry_request_params);
            let permit =
                self.request_gate.acquire().await.map_err(|_| {
                    SentinelError::Http(REGISTRY_REQUEST_GATE_CLOSED_MSG.to_string())
                })?;

            let response_result = tokio::time::timeout(
                Duration::from_secs(DOWNLOAD_TARBALL_TIMEOUT_SECS),
                request_builder.send(),
            )
            .await;

            drop(permit);

            let timeout_error = timeout_download_error(url);
            let resolve_timed_response_params = ResolveTimedResponseParams {
                response_result,
                attempt,
                timeout_error,
            };
            let resolved_response = resolve_timed_response(resolve_timed_response_params).await;

            let response = match resolved_response {
                Ok(Some(response)) => response,
                Ok(None) => continue,
                Err(error) => return Err(error),
            };

            let status_code = response.status().as_u16();
            let is_retryable_response = is_retryable_status(status_code);
            let has_remaining_attempts = attempt < REGISTRY_MAX_RETRIES;
            let should_retry_response = is_retryable_response && has_remaining_attempts;

            if should_retry_with_backoff(attempt, should_retry_response).await {
                continue;
            }

            return Ok(response);
        }

        Err(timeout_download_error(url))
    }
}

fn build_registry_client(timeout_ms: u64) -> Result<reqwest::Client, SentinelError> {
    let user_agent = format!("{}{}", NPM_USER_AGENT_PREFIX, env!("CARGO_PKG_VERSION"));

    reqwest::Client::builder()
        .user_agent(user_agent)
        .timeout(Duration::from_millis(timeout_ms))
        .https_only(true)
        .use_rustls_tls()
        .build()
        .map_err(|error| SentinelError::Http(error.to_string()))
}

fn should_retry_network_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect()
}

fn is_retryable_status(status_code: u16) -> bool {
    status_code == HTTP_STATUS_TOO_MANY_REQUESTS || status_code >= HTTP_STATUS_SERVER_ERROR_MIN
}

async fn sleep_before_retry(attempt: usize) {
    let retry_attempt = u32::try_from(attempt).unwrap_or(u32::MAX);
    let multiplier = RETRY_BACKOFF_MULTIPLIER_BASE.saturating_pow(retry_attempt);
    let base_delay_ms = REGISTRY_RETRY_BASE_DELAY_MS.saturating_mul(multiplier);
    let jitter_ms = jitter_ms();
    let delay_ms = base_delay_ms.saturating_add(jitter_ms);

    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
}

fn jitter_ms() -> u64 {
    let now = SystemTime::now().duration_since(UNIX_EPOCH);
    let nanos = now.map_or(0, |duration| u64::from(duration.subsec_nanos()));

    nanos % (REGISTRY_RETRY_MAX_JITTER_MS.saturating_add(JITTER_INCLUSIVE_UPPER_BOUND_OFFSET))
}

fn resolve_max_in_flight_requests(registry_max_in_flight: Option<usize>) -> usize {
    if let Some(registry_max_in_flight) = registry_max_in_flight {
        return registry_max_in_flight;
    }

    let parsed_value = std::env::var(REGISTRY_MAX_IN_FLIGHT_REQUESTS_ENV)
        .ok()
        .and_then(|raw_value| raw_value.parse::<usize>().ok())
        .filter(|value| *value > POSITIVE_USIZE_MIN);

    parsed_value.unwrap_or(REGISTRY_MAX_IN_FLIGHT_REQUESTS)
}

fn encode_package_name(name: &str) -> String {
    let is_scoped_package = name.starts_with(SCOPED_PACKAGE_PREFIX);
    let mut encoded_package_name = name.to_string();

    if is_scoped_package {
        encoded_package_name = name.replacen(PACKAGE_SCOPE_SEPARATOR, NPM_SCOPED_SEPARATOR, 1);
    }

    encoded_package_name
}

fn build_registry_request(params: BuildRegistryRequestParams<'_>) -> reqwest::RequestBuilder {
    let BuildRegistryRequestParams {
        client,
        url,
        auth_token_prefixes,
    } = params;
    let resolve_auth_token_params = ResolveAuthTokenParams {
        url,
        auth_token_prefixes,
    };

    match resolve_auth_token_for_url(resolve_auth_token_params) {
        Some(token) => client.get(url).bearer_auth(token),
        None => client.get(url),
    }
}

fn resolve_registry_settings(current_working_directory: &Path) -> RegistrySettings {
    let npmrc = read_combined_npmrc(current_working_directory);
    let ParsedNpmrc {
        default_registry,
        scoped_registries,
        auth_token_prefixes,
    } = parse_npmrc_content(&npmrc);
    let default_registry_base = normalize_registry_base(
        &default_registry.unwrap_or_else(|| NPM_REGISTRY_BASE_URL.to_string()),
    );

    RegistrySettings {
        default_registry_base,
        scoped_registry_bases: scoped_registries,
        auth_token_prefixes,
    }
}

fn timeout_download_error(url: &str) -> SentinelError {
    let timeout_download_message =
        render_template_from_iter(NPM_ERR_TIMEOUT_DOWNLOAD_TEMPLATE, [url]);

    SentinelError::Http(timeout_download_message)
}

fn read_combined_npmrc(current_working_directory: &Path) -> String {
    let mut files = Vec::new();

    if let Some(home_dir) = dirs::home_dir() {
        files.push(home_dir.join(NPMRC_FILE_NAME));
    }

    files.push(current_working_directory.join(NPMRC_FILE_NAME));

    let mut combined = String::new();

    for path in files {
        if let Ok(content) = fs::read_to_string(path) {
            combined.push_str(&content);
            combined.push(LINE_SEPARATOR);
        }
    }

    combined
}

fn parse_npmrc_content(content: &str) -> ParsedNpmrc {
    let mut default_registry = None;
    let mut scoped_registries = HashMap::new();
    let mut auth_token_prefixes = Vec::new();

    for raw_line in content.lines() {
        let trimmed_line = raw_line.trim();
        let is_empty_line = trimmed_line.is_empty();
        let is_hash_comment = trimmed_line.starts_with(NPMRC_HASH_COMMENT_PREFIX);
        let is_semicolon_comment = trimmed_line.starts_with(NPMRC_SEMICOLON_COMMENT_PREFIX);
        let should_skip_line = is_empty_line || is_hash_comment || is_semicolon_comment;

        if should_skip_line {
            continue;
        }

        let Some((key, value)) = trimmed_line.split_once(NPMRC_ENTRY_SEPARATOR) else {
            continue;
        };

        let parsed_key = key.trim();
        let parsed_value = value
            .trim()
            .trim_matches(DOUBLE_QUOTE_CHAR)
            .trim_matches(SINGLE_QUOTE_CHAR);

        match classify_npmrc_entry_key(parsed_key) {
            NpmrcEntryKind::DefaultRegistry => {
                default_registry = Some(parsed_value.to_string());
            }
            NpmrcEntryKind::ScopedRegistry => {
                let scope = parse_scope_from_registry_key(parsed_key);

                scoped_registries.insert(scope, normalize_registry_base(parsed_value));
            }
            NpmrcEntryKind::AuthToken => {
                let AuthTokenPrefixPair {
                    https_prefix,
                    http_prefix,
                } = expand_auth_token_prefixes(parsed_key);

                auth_token_prefixes.push((https_prefix, parsed_value.to_string()));
                auth_token_prefixes.push((http_prefix, parsed_value.to_string()));
            }
            NpmrcEntryKind::Ignore => {}
        }
    }

    ParsedNpmrc {
        default_registry,
        scoped_registries,
        auth_token_prefixes,
    }
}

fn normalize_registry_base(value: &str) -> String {
    value.trim_end_matches(PACKAGE_SCOPE_SEPARATOR).to_string()
}

fn resolve_registry_base_for_package(params: ResolveRegistryBaseParams<'_>) -> String {
    let ResolveRegistryBaseParams {
        package_name,
        default_registry_base,
        scoped_registry_bases,
    } = params;
    let maybe_scope = package_name
        .strip_prefix(SCOPED_PACKAGE_PREFIX)
        .and_then(|name| name.split(PACKAGE_SCOPE_SEPARATOR).next());

    let Some(scope) = maybe_scope else {
        return default_registry_base.to_string();
    };

    let maybe_scoped_base = scoped_registry_bases.get(scope);
    let Some(scoped_base) = maybe_scoped_base else {
        return default_registry_base.to_string();
    };

    scoped_base.clone()
}

fn resolve_auth_token_for_url(params: ResolveAuthTokenParams<'_>) -> Option<&str> {
    let ResolveAuthTokenParams {
        url,
        auth_token_prefixes,
    } = params;

    auth_token_prefixes
        .iter()
        .filter(|(prefix, _)| url.starts_with(prefix))
        .max_by_key(|(prefix, _)| prefix.len())
        .map(|(_, token)| token.as_str())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::await_holding_lock,
    clippy::uninlined_format_args,
    clippy::double_ended_iterator_last
)]
#[path = "../../tests/internal/npm_registry_internal_tests.rs"]
mod tests;
