use crate::constants::{
    VERIFIER_DETAIL_CLEAN_LOCKFILE, VERIFIER_DETAIL_COMPROMISED_LOCKFILE,
    VERIFIER_DETAIL_NO_LOCKFILE_INTEGRITY, VERIFIER_DETAIL_NOT_IN_REGISTRY,
    VERIFIER_DETAIL_PREDATES_INTEGRITY, VERIFIER_DETAIL_REGISTRY_FETCH_ERROR,
    VERIFIER_DETAIL_REGISTRY_TIMEOUT, VERIFIER_DETAIL_REGISTRY_UNREACHABLE, render_template,
};
use crate::crypto::integrity_short;
use crate::npm::LockfileEntry;
use crate::types::{
    CacheMatchParams, CompromisedSource, CreateUnverifiableParams, Evidence, SentinelError,
    UnverifiableReason, Verdict, VerifyResult,
};

use super::{Verifier, cache_matches_lockfile, create_unverifiable};

impl Verifier {
    pub async fn check_from_lockfile(&self, entry: &LockfileEntry) -> VerifyResult {
        let package_ref = &entry.package;

        if let Some(cached_result) = self.cache.get(package_ref) {
            if cache_matches_lockfile(CacheMatchParams {
                entry,
                cached_result: &cached_result,
            }) {
                tracing::debug!("{package_ref}: cache hit");

                return cached_result;
            }

            tracing::debug!("{package_ref}: cache stale due to lockfile drift; invalidating");
            self.cache.invalidate(package_ref);
        }

        let lockfile_integrity = match &entry.integrity {
            None => {
                let unverifiable_result = create_unverifiable(CreateUnverifiableParams {
                    reason: UnverifiableReason::MissingFromLockfile,
                    package: package_ref,
                    detail_template: VERIFIER_DETAIL_NO_LOCKFILE_INTEGRITY,
                    template_args: &[package_ref.to_string()],
                    evidence: Evidence::empty(),
                });

                return self.cache_and_return(unverifiable_result);
            }

            Some(integrity) => integrity.clone(),
        };

        let registry_metadata = match self.registry.fetch_version(package_ref).await {
            Ok(metadata) => metadata,

            Err(SentinelError::RegistryUnreachable(error)) => {
                let unverifiable_result = create_unverifiable(CreateUnverifiableParams {
                    reason: UnverifiableReason::RegistryOffline,
                    package: package_ref,
                    detail_template: VERIFIER_DETAIL_REGISTRY_UNREACHABLE,
                    template_args: &[
                        package_ref.to_string(),
                        error.to_string(),
                        integrity_short(&lockfile_integrity),
                    ],
                    evidence: Evidence {
                        lockfile_integrity: Some(lockfile_integrity.clone()),
                        ..Evidence::empty()
                    },
                });

                return self.cache_and_return(unverifiable_result);
            }

            Err(SentinelError::RegistryTimeout {
                package,
                version,
                ms,
            }) => {
                let unverifiable_result = create_unverifiable(CreateUnverifiableParams {
                    reason: UnverifiableReason::RegistryTimeout,
                    package: package_ref,
                    detail_template: VERIFIER_DETAIL_REGISTRY_TIMEOUT,
                    template_args: &[
                        package_ref.to_string(),
                        ms.to_string(),
                        integrity_short(&lockfile_integrity),
                    ],
                    evidence: Evidence {
                        lockfile_integrity: Some(lockfile_integrity.clone()),
                        ..Evidence::empty()
                    },
                });

                let _ = (package, version);

                return self.cache_and_return(unverifiable_result);
            }

            Err(SentinelError::NoIntegrity { .. }) => {
                let unverifiable_result = create_unverifiable(CreateUnverifiableParams {
                    reason: UnverifiableReason::NoIntegrityField,
                    package: package_ref,
                    detail_template: VERIFIER_DETAIL_NOT_IN_REGISTRY,
                    template_args: &[
                        package_ref.to_string(),
                        integrity_short(&lockfile_integrity),
                    ],
                    evidence: Evidence {
                        lockfile_integrity: Some(lockfile_integrity.clone()),
                        ..Evidence::empty()
                    },
                });

                return self.cache_and_return(unverifiable_result);
            }

            Err(error) => {
                let unverifiable_result = create_unverifiable(CreateUnverifiableParams {
                    reason: UnverifiableReason::RegistryOffline,
                    package: package_ref,
                    detail_template: VERIFIER_DETAIL_REGISTRY_FETCH_ERROR,
                    template_args: &[package_ref.to_string(), error.to_string()],
                    evidence: Evidence {
                        lockfile_integrity: Some(lockfile_integrity.clone()),
                        ..Evidence::empty()
                    },
                });

                return self.cache_and_return(unverifiable_result);
            }
        };

        let registry_integrity = match &registry_metadata.dist.integrity {
            Some(integrity) => integrity.clone(),
            None => {
                let unverifiable_result = create_unverifiable(CreateUnverifiableParams {
                    reason: UnverifiableReason::NoIntegrityField,
                    package: package_ref,
                    detail_template: VERIFIER_DETAIL_PREDATES_INTEGRITY,
                    template_args: &[
                        package_ref.to_string(),
                        integrity_short(&lockfile_integrity),
                    ],
                    evidence: Evidence {
                        lockfile_integrity: Some(lockfile_integrity.clone()),
                        source_url: Some(registry_metadata.dist.tarball.clone()),
                        ..Evidence::empty()
                    },
                });

                return self.cache_and_return(unverifiable_result);
            }
        };

        let integrities_match = lockfile_integrity == registry_integrity;

        match integrities_match {
            true => {
                let clean_result = VerifyResult {
                    package: package_ref.clone(),
                    verdict: Verdict::Clean,
                    detail: render_template(
                        VERIFIER_DETAIL_CLEAN_LOCKFILE,
                        &[
                            package_ref.to_string(),
                            integrity_short(&lockfile_integrity),
                        ],
                    ),
                    evidence: Evidence {
                        registry_integrity: Some(registry_integrity),
                        lockfile_integrity: Some(lockfile_integrity),
                        source_url: Some(registry_metadata.dist.tarball),
                        ..Evidence::empty()
                    },
                };
                self.cache_and_return(clean_result)
            }
            false => {
                let compromised_result = VerifyResult {
                    package: package_ref.clone(),
                    verdict: Verdict::Compromised {
                        expected: registry_integrity.clone(),
                        actual: lockfile_integrity.clone(),
                        source: CompromisedSource::LockfileVsRegistry,
                    },
                    detail: render_template(
                        VERIFIER_DETAIL_COMPROMISED_LOCKFILE,
                        &[
                            package_ref.to_string(),
                            integrity_short(&lockfile_integrity),
                            integrity_short(&registry_integrity),
                            package_ref.to_string(),
                        ],
                    ),
                    evidence: Evidence {
                        registry_integrity: Some(registry_integrity),
                        lockfile_integrity: Some(lockfile_integrity),
                        source_url: Some(registry_metadata.dist.tarball),
                        ..Evidence::empty()
                    },
                };

                self.cache.invalidate(package_ref);

                compromised_result
            }
        }
    }
}
