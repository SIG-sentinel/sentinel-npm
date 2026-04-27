use crate::npm::LockfileEntry;

use super::NpmProvenance;
use super::PackageRef;
use super::SentinelError;
use super::UnverifiableReason;
use super::VerifyResult;

pub struct CheckFromLockfileParams<'a, V> {
    pub verifier: &'a V,
    pub entry: &'a LockfileEntry,
}

pub struct HandleRegistryFetchErrorParams<'a, V> {
    pub verifier: &'a V,
    pub package_ref: &'a PackageRef,
    pub error: SentinelError,
    pub lockfile_integrity: &'a str,
}

pub struct VerifyTarballIntegrityParams<'a, V> {
    pub verifier: &'a V,
    pub package_ref: &'a PackageRef,
    pub lockfile_integrity: &'a str,
    pub registry_integrity: &'a str,
    pub tarball_url: &'a str,
    pub provenance: Option<&'a NpmProvenance>,
}

pub struct HandleHashStreamErrorParams<'a> {
    pub error: SentinelError,
    pub package_ref: &'a PackageRef,
    pub lockfile_integrity: &'a str,
    pub registry_integrity: &'a str,
    pub tarball_url: &'a str,
}

pub struct LockfilePackageAndErrorTemplateArgsParams<'a> {
    pub package_ref: &'a PackageRef,
    pub error_description: &'a str,
}

pub struct IntegrityEvidenceParams<'a> {
    pub lockfile_integrity: &'a str,
    pub registry_integrity: &'a str,
    pub tarball_url: &'a str,
    pub computed_sha512: Option<&'a str>,
}

pub struct BuildCompromisedLockfileResultParams<'a> {
    pub package_ref: &'a PackageRef,
    pub lockfile_integrity: &'a str,
    pub registry_integrity: &'a str,
    pub tarball_url: &'a str,
}

pub struct BuildIntegrityResultParams<'a, V> {
    pub integrity_valid: bool,
    pub verifier: &'a V,
    pub package_ref: &'a PackageRef,
    pub lockfile_integrity: &'a str,
    pub registry_integrity: &'a str,
    pub tarball_url: &'a str,
    pub computed_integrity: &'a str,
    pub tarball_bytes: usize,
    pub provenance: Option<&'a NpmProvenance>,
}

pub struct BuildCleanLockfileResultParams<'a> {
    pub package_ref: &'a PackageRef,
    pub lockfile_integrity: &'a str,
    pub registry_integrity: &'a str,
    pub tarball_url: &'a str,
    pub computed_integrity: &'a str,
    pub tarball_bytes: usize,
}

pub struct BuildCompromisedTarballResultParams<'a> {
    pub package_ref: &'a PackageRef,
    pub lockfile_integrity: &'a str,
    pub registry_integrity: &'a str,
    pub tarball_url: &'a str,
    pub computed_integrity: &'a str,
}

pub struct BuildIntegrityEvidenceWithoutComputedParams<'a> {
    pub lockfile_integrity: &'a str,
    pub registry_integrity: &'a str,
    pub tarball_url: &'a str,
}

pub struct BuildIntegrityEvidenceWithComputedParams<'a> {
    pub lockfile_integrity: &'a str,
    pub registry_integrity: &'a str,
    pub tarball_url: &'a str,
    pub computed_sha512: &'a str,
}

pub struct FinalizeCleanProvenanceResultParams<'a, V> {
    pub verifier: &'a V,
    pub package_ref: &'a PackageRef,
    pub lockfile_integrity: &'a str,
    pub registry_integrity: &'a str,
    pub tarball_url: &'a str,
    pub computed_integrity: &'a str,
    pub provenance: Option<&'a NpmProvenance>,
    pub clean_result: VerifyResult,
}

pub struct CacheUnverifiableWithErrorDetailsParams<'a, V> {
    pub verifier: &'a V,
    pub package_ref: &'a PackageRef,
    pub lockfile_integrity: &'a str,
    pub registry_integrity: &'a str,
    pub tarball_url: &'a str,
    pub reason: UnverifiableReason,
    pub detail_template: &'a str,
    pub error_description: &'a str,
}
