use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use sha2::Sha256;

use super::verification::{Evidence, UnverifiableReason};
use super::{ArtifactStore, DualHash, LockfileEntry, NpmVersionMeta, PackageRef, VerifyResult};

#[derive(Clone, Copy)]
pub struct CacheMatchParams<'a> {
    pub entry: &'a LockfileEntry,
    pub cached_result: &'a VerifyResult,
}

#[derive(Clone, Copy)]
pub struct VerifierNewParams<'a> {
    pub timeout_ms: u64,
    pub current_working_directory: &'a Path,
    pub cache_dir: Option<&'a str>,
    pub artifact_store: ArtifactStore,
    pub max_memory_bytes: usize,
}

pub struct VerifyBeforeInstallParams<'a> {
    pub verifier: &'a crate::verifier::Verifier,
    pub package_ref: &'a PackageRef,
}

pub struct TarballOperationErrorParams<'a> {
    pub operation_description: &'a str,
    pub package_ref: &'a PackageRef,
    pub error_description: &'a str,
}

pub struct PackageAndErrorTemplateArgsParams<'a> {
    pub package_ref: &'a PackageRef,
    pub error_description: &'a str,
}

pub struct FetchRegistryMetadataParams<'a> {
    pub verifier: &'a crate::verifier::Verifier,
    pub package_ref: &'a PackageRef,
}

pub struct ResolveDistIntegrityParams<'a> {
    pub package_ref: &'a PackageRef,
    pub registry_metadata: &'a NpmVersionMeta,
}

pub struct DownloadTarballParams<'a> {
    pub verifier: &'a crate::verifier::Verifier,
    pub package_ref: &'a PackageRef,
    pub tarball_url: &'a str,
}

pub struct HashTarballParams<'a> {
    pub verifier: &'a crate::verifier::Verifier,
    pub package_ref: &'a PackageRef,
    pub tarball_response: reqwest::Response,
    pub tarball_url: &'a str,
}

pub struct ValidateIntegrityParams<'a> {
    pub package_ref: &'a PackageRef,
    pub dist_integrity: &'a str,
    pub tarball_url: &'a str,
    pub tarball_hashes: &'a DualHash,
}

pub struct CollectTarballHashesParams<'a> {
    pub tarball_bytes: &'a [u8],
    pub package_ref: &'a PackageRef,
}

pub struct UpdateFingerprintHasherParams<'a> {
    pub hasher: &'a mut Sha256,
    pub files: Vec<(String, Vec<u8>)>,
}

pub struct ToVerifiedTarballParams {
    pub buffer: Option<Vec<u8>>,
    pub spool_path: Option<PathBuf>,
}

pub struct ComputeTarballFingerprintParams<'a> {
    pub tarball_bytes: &'a [u8],
    pub package_ref: &'a PackageRef,
}

pub struct BuildCleanResultParams<'a> {
    pub package_ref: &'a PackageRef,
    pub dist_integrity: String,
    pub computed_integrity: String,
    pub tarball_url: String,
    pub tarball_fingerprint: Option<String>,
    pub tarball_bytes: usize,
}

pub struct BuildCompromisedResultParams<'a> {
    pub package_ref: &'a PackageRef,
    pub dist_integrity: String,
    pub computed_integrity: String,
    pub tarball_url: String,
    pub tarball_fingerprint: Option<String>,
}

pub struct ArtifactRegistry {
    pub(crate) temp_artifacts: Vec<PathBuf>,
}

pub struct MemoryBudgetTracker {
    pub(crate) inflight_bytes: Arc<AtomicUsize>,
    pub(crate) max_budget_bytes: usize,
}

pub struct FallbackDecision {
    pub effective_mode: ArtifactStore,
    pub fell_back: bool,
}

pub struct StreamStorageMode {
    pub effective_mode: ArtifactStore,
    pub capture_buffer: bool,
    pub spool_to_disk: bool,
}

pub struct UnverifiableTemplateParams<'a> {
    pub reason: UnverifiableReason,
    pub package: &'a PackageRef,
    pub detail_template: &'a str,
    pub template_args: Vec<String>,
    pub evidence: Evidence,
}
