use crate::npm::LockfileEntry;

use super::PackageRef;
use super::SentinelError;

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
}

pub struct HandleHashStreamErrorParams<'a> {
    pub error: SentinelError,
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
}
