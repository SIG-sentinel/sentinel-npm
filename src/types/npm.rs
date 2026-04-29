use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use super::dependency_tree::DependencyNode;
use super::error::SentinelError;
use super::package::PackageRef;

#[derive(Debug, Deserialize)]
pub struct NpmVersionMeta {
    pub dist: NpmDist,
}

#[derive(Debug, Deserialize)]
pub struct NpmDistAttestations {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct NpmDist {
    pub integrity: Option<String>,
    pub tarball: String,
    #[serde(default)]
    pub provenance: Option<NpmProvenance>,
    #[serde(default)]
    pub attestations: Option<NpmDistAttestations>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NpmProvenance {
    #[serde(default)]
    pub subject_integrity: Option<String>,
    #[serde(default)]
    pub issuer: Option<String>,
    #[serde(default)]
    pub identity: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AttestationsResponse {
    pub attestations: Vec<AttestationEntry>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AttestationEntry {
    #[serde(rename = "predicateType")]
    pub predicate_type: String,
    pub bundle: AttestationBundle,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AttestationBundle {
    #[serde(rename = "dsseEnvelope")]
    pub dsse_envelope: DsseEnvelope,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DsseEnvelope {
    pub payload: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SlsaV1Payload {
    pub subject: Vec<SlsaSubject>,
    pub predicate: Option<SlsaV1Predicate>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SlsaSubject {
    pub digest: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SlsaV1Predicate {
    #[serde(rename = "buildDefinition")]
    pub build_definition: Option<SlsaBuildDefinition>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SlsaBuildDefinition {
    #[serde(rename = "externalParameters")]
    pub external_parameters: Option<SlsaExternalParameters>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SlsaExternalParameters {
    pub workflow: Option<SlsaWorkflow>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SlsaWorkflow {
    #[serde(rename = "ref")]
    pub ref_: Option<String>,
    pub repository: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LockfileEntry {
    pub package: PackageRef,
    pub integrity: Option<String>,
    pub is_dev: bool,
    pub dependencies: Vec<String>,
}

#[derive(Clone, Copy)]
pub struct DiagnoseLockfileFailureParams<'a> {
    pub stderr: &'a str,
    pub manager: super::manager::PackageManager,
}

pub struct ExtractV1DepsParams<'a> {
    pub deps: &'a serde_json::Map<String, serde_json::Value>,
    pub entries: &'a mut HashMap<String, LockfileEntry>,
}

#[derive(Clone, Copy)]
pub struct ProcessLockfilePackageParams<'a> {
    pub package_path: &'a str,
    pub package_metadata: &'a serde_json::Value,
}

#[derive(Clone, Copy)]
pub struct ReadPackageJsonDepsParams<'a> {
    pub project_dir: &'a Path,
    pub include_dev: bool,
}

#[derive(Clone, Copy)]
pub struct ResolveDependencyKeyParams<'a> {
    pub package_path: &'a str,
    pub dep_name: &'a str,
    pub dep_meta: &'a serde_json::Value,
    pub package_key_by_path: &'a HashMap<String, String>,
    pub all_packages: &'a HashMap<String, DependencyNode>,
}

#[derive(Clone, Copy)]
pub struct ResolveAuthTokenParams<'a> {
    pub url: &'a str,
    pub auth_token_prefixes: &'a [(String, String)],
}

pub struct WireDependenciesParams<'a> {
    pub packages: &'a serde_json::Map<String, serde_json::Value>,
    pub package_key_by_path: &'a HashMap<String, String>,
    pub all_packages: &'a mut HashMap<String, DependencyNode>,
}

pub struct RegistrySettings {
    pub default_registry_base: String,
    pub scoped_registry_bases: HashMap<String, String>,
    pub auth_token_prefixes: Vec<(String, String)>,
}

pub struct ParsedNpmrc {
    pub default_registry: Option<String>,
    pub scoped_registries: HashMap<String, String>,
    pub auth_token_prefixes: Vec<(String, String)>,
}

pub struct AuthTokenPrefixPair {
    pub https_prefix: String,
    pub http_prefix: String,
}

pub struct ResolveTimedResponseParams {
    pub response_result:
        Result<Result<reqwest::Response, reqwest::Error>, tokio::time::error::Elapsed>,
    pub attempt: usize,
    pub timeout_error: SentinelError,
}

#[derive(Clone, Copy)]
pub struct NpmRegistryNewParams<'a> {
    pub timeout_ms: u64,
    pub registry_max_in_flight: Option<usize>,
    pub current_working_directory: &'a Path,
}

pub(crate) enum NpmrcEntryKind {
    DefaultRegistry,
    ScopedRegistry,
    AuthToken,
    Ignore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegistryResponseClassification {
    Retry,
    NotFound,
    Success,
    Failure,
}

#[derive(Clone, Copy)]
pub struct BuildRegistryRequestParams<'a> {
    pub client: &'a reqwest::Client,
    pub url: &'a str,
    pub auth_token_prefixes: &'a [(String, String)],
}

#[derive(Clone, Copy)]
pub struct ResolveRegistryBaseParams<'a> {
    pub package_name: &'a str,
    pub default_registry_base: &'a str,
    pub scoped_registry_bases: &'a HashMap<String, String>,
}

#[derive(Clone, Copy)]
pub struct RunPackageSourcePreloadParams<'a> {
    pub work_dir: &'a Path,
    pub package_source: &'a str,
    pub cache_or_store_dir: &'a Path,
}

pub struct RunPackageSourceInstallParams<'a> {
    pub work_dir: &'a Path,
    pub package_reference: &'a PackageRef,
    pub cache_or_store_dir: &'a Path,
    pub ignore_scripts: bool,
}
