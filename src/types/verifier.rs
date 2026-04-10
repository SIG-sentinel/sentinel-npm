use super::{LockfileEntry, PackageRef, VerifyResult};

pub struct CacheMatchParams<'a> {
    pub entry: &'a LockfileEntry,
    pub cached_result: &'a VerifyResult,
}

pub struct VerifierNewParams<'a> {
    pub timeout_ms: u64,
    pub cache_dir: Option<&'a str>,
}

pub struct VerifyBeforeInstallParams<'a> {
    pub verifier: &'a crate::verifier::Verifier,
    pub package_ref: &'a PackageRef,
}
