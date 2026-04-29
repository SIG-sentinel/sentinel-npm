pub mod artifact_cleanup;
mod install_check;
mod lockfile_check;
pub mod memory_budget;

use crate::cache::LocalCache;
use crate::constants::{INTEGRITY_PREFIX_SHA512, render_template};
use crate::npm::NpmRegistry;
use crate::types::{
    CacheMatchParams, CreateUnverifiableParams, NpmRegistryNewParams, SentinelError,
    UnverifiableTemplateParams, Verdict, VerifierNewParams, VerifyResult,
};

pub use crate::types::Verifier;
pub use crate::types::VerifyResultWithTarball;
pub(crate) use install_check::compute_tarball_fingerprint_bytes;

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
        is_direct: false,
        direct_parent: None,
        tarball_fingerprint: None,
    }
}

pub(super) fn create_unverifiable_from_template(
    params: UnverifiableTemplateParams<'_>,
) -> VerifyResult {
    let UnverifiableTemplateParams {
        reason,
        package,
        detail_template,
        template_args,
        evidence,
    } = params;

    let create_unverifiable_params = CreateUnverifiableParams {
        reason,
        package,
        detail_template,
        template_args: &template_args,
        evidence,
    };

    create_unverifiable(create_unverifiable_params)
}

pub(super) fn cache_unverifiable_from_template(
    verifier: &Verifier,
    params: UnverifiableTemplateParams<'_>,
) -> VerifyResult {
    let unverifiable = create_unverifiable_from_template(params);

    verifier.cache_and_return(unverifiable)
}

pub(super) fn cache_matches_lockfile(params: CacheMatchParams<'_>) -> bool {
    let CacheMatchParams {
        entry,
        cached_result,
    } = params;

    entry.integrity == cached_result.evidence.lockfile_integrity
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
            registry_max_in_flight,
            current_working_directory,
            cache_dir,
            artifact_store,
            max_memory_bytes,
        } = params;

        let npm_registry_new_params = NpmRegistryNewParams {
            timeout_ms,
            registry_max_in_flight,
            current_working_directory,
        };

        Ok(Self {
            registry: NpmRegistry::new(npm_registry_new_params)?,
            cache: LocalCache::open(cache_dir)?,
            artifact_store,
            memory_budget: memory_budget::MemoryBudgetTracker::new(max_memory_bytes),
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
#[path = "../../tests/internal/verifier_tests.rs"]
mod tests;
