mod install_check;
mod lockfile_check;

use crate::cache::LocalCache;
use crate::constants::{INTEGRITY_PREFIX_SHA512, render_template};
use crate::npm::NpmRegistry;
use crate::types::{
    CacheMatchParams, CreateUnverifiableParams, SentinelError, Verdict, VerifierNewParams,
    VerifyResult,
};

pub use crate::types::Verifier;
pub use crate::types::VerifyResultWithTarball;

pub(super) fn create_unverifiable(params: CreateUnverifiableParams<'_>) -> VerifyResult {
    let CreateUnverifiableParams {
        reason,
        package,
        detail_template,
        template_args,
        evidence,
    } = params;

    VerifyResult {
        package: package.clone(),
        verdict: Verdict::Unverifiable { reason },
        detail: render_template(detail_template, template_args),
        evidence,
    }
}

pub(super) fn cache_matches_lockfile(params: CacheMatchParams<'_>) -> bool {
    let CacheMatchParams {
        entry,
        cached_result,
    } = params;

    match (&entry.integrity, &cached_result.evidence.lockfile_integrity) {
        (Some(current), Some(cached)) => current == cached,
        (None, None) => true,
        _ => false,
    }
}

pub(super) fn cache_requires_tarball_revalidation(cached_result: &VerifyResult) -> bool {
    let is_clean = matches!(cached_result.verdict, Verdict::Clean);
    let missing_computed_sha512 = cached_result.evidence.computed_sha512.is_none();

    is_clean && missing_computed_sha512
}

pub(super) fn no_tarball_result(result: VerifyResult) -> VerifyResultWithTarball {
    VerifyResultWithTarball {
        result,
        tarball: None,
    }
}

pub(super) fn computed_sha512_integrity(sha512_bytes: &[u8]) -> String {
    use base64::{Engine, engine::general_purpose::STANDARD as B64};

    format!("{INTEGRITY_PREFIX_SHA512}{}", B64.encode(sha512_bytes))
}

impl Verifier {
    pub(super) fn cache_and_return(&self, result: VerifyResult) -> VerifyResult {
        self.cache.put(&result);
        result
    }

    pub fn new(params: VerifierNewParams<'_>) -> Result<Self, SentinelError> {
        let VerifierNewParams {
            timeout_ms,
            cache_dir,
        } = params;

        Ok(Self {
            registry: NpmRegistry::new(timeout_ms)?,
            cache: LocalCache::open(cache_dir)?,
        })
    }
}

#[cfg(test)]
#[path = "../../tests/internal/verifier_tests.rs"]
mod tests;
