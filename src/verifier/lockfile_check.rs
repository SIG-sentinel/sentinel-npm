use crate::constants::{
    BYTES_PER_MIB, VERIFIER_DETAIL_CLEAN_LOCKFILE, VERIFIER_DETAIL_COMPROMISED_LOCKFILE,
    VERIFIER_DETAIL_COMPROMISED_TARBALL_VS_LOCKFILE,
    VERIFIER_DETAIL_NO_LOCKFILE_INTEGRITY, VERIFIER_DETAIL_NOT_IN_REGISTRY,
    VERIFIER_DETAIL_PREDATES_INTEGRITY, VERIFIER_DETAIL_REGISTRY_FETCH_ERROR,
    VERIFIER_DETAIL_REGISTRY_TIMEOUT, VERIFIER_DETAIL_REGISTRY_UNREACHABLE,
    VERIFIER_DETAIL_TARBALL_DOWNLOAD_FAILED_DURING_CHECK,
    VERIFIER_DETAIL_TARBALL_INTEGRITY_FORMAT_ERROR_DURING_CHECK,
    VERIFIER_DETAIL_TARBALL_STREAM_ERROR_DURING_CHECK,
    VERIFIER_DETAIL_TARBALL_TOO_LARGE_DURING_CHECK, render_template,
};
use crate::crypto::{hash_stream, integrity_short, verify_integrity};
use crate::npm::LockfileEntry;
use crate::types::{
    CacheMatchParams, CompromisedSource, CreateUnverifiableParams, Evidence, HashStreamParams,
    PackageRef, SentinelError, UnverifiableReason, Verdict, VerifyIntegrityParams, VerifyResult,
};

use super::{Verifier, cache_matches_lockfile, computed_sha512_integrity, create_unverifiable};

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
                // Source 1 ✓ (lockfile) == Source 2 ✓ (registry)
                // Now verify Source 3: download tarball and compute hash
                self.verify_tarball_integrity(
                    package_ref,
                    &lockfile_integrity,
                    &registry_integrity,
                    &registry_metadata.dist.tarball,
                )
                .await
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

    async fn verify_tarball_integrity(
        &self,
        package_ref: &PackageRef,
        lockfile_integrity: &str,
        registry_integrity: &str,
        tarball_url: &str,
    ) -> VerifyResult {
        let tarball_response = match self.registry.download_tarball(tarball_url).await {
            Ok(response) => response,
            Err(error) => {
                return self.cache_and_return(create_unverifiable(CreateUnverifiableParams {
                    reason: UnverifiableReason::RegistryOffline,
                    package: package_ref,
                    detail_template: VERIFIER_DETAIL_TARBALL_DOWNLOAD_FAILED_DURING_CHECK,
                    template_args: &[package_ref.to_string(), error.to_string()],
                    evidence: Evidence {
                        lockfile_integrity: Some(lockfile_integrity.to_string()),
                        registry_integrity: Some(registry_integrity.to_string()),
                        source_url: Some(tarball_url.to_string()),
                        ..Evidence::empty()
                    },
                }));
            }
        };

        let tarball_hashes = match hash_stream(HashStreamParams {
            stream: tarball_response.bytes_stream(),
            package: &package_ref.name,
        })
        .await
        {
            Ok(hashes) => hashes,
            Err(SentinelError::TarballTooLarge { bytes, .. }) => {
                return self.cache_and_return(create_unverifiable(CreateUnverifiableParams {
                    reason: UnverifiableReason::TarballTooLarge,
                    package: package_ref,
                    detail_template: VERIFIER_DETAIL_TARBALL_TOO_LARGE_DURING_CHECK,
                    template_args: &[
                        package_ref.to_string(),
                        format!("{:.1}", bytes as f64 / BYTES_PER_MIB),
                    ],
                    evidence: Evidence {
                        lockfile_integrity: Some(lockfile_integrity.to_string()),
                        registry_integrity: Some(registry_integrity.to_string()),
                        source_url: Some(tarball_url.to_string()),
                        ..Evidence::empty()
                    },
                }));
            }
            Err(error) => {
                return self.cache_and_return(create_unverifiable(CreateUnverifiableParams {
                    reason: UnverifiableReason::RegistryOffline,
                    package: package_ref,
                    detail_template: VERIFIER_DETAIL_TARBALL_STREAM_ERROR_DURING_CHECK,
                    template_args: &[package_ref.to_string(), error.to_string()],
                    evidence: Evidence {
                        lockfile_integrity: Some(lockfile_integrity.to_string()),
                        registry_integrity: Some(registry_integrity.to_string()),
                        source_url: Some(tarball_url.to_string()),
                        ..Evidence::empty()
                    },
                }));
            }
        };

        let computed_integrity = computed_sha512_integrity(&tarball_hashes.sha512_bytes);

        match verify_integrity(VerifyIntegrityParams {
            sha512_bytes: &tarball_hashes.sha512_bytes,
            integrity_field: lockfile_integrity,
        }) {
            Ok(true) => {
                // All three sources agree: lockfile == registry == computed tarball hash
                let clean_result = VerifyResult {
                    package: package_ref.clone(),
                    verdict: Verdict::Clean,
                    detail: render_template(
                        VERIFIER_DETAIL_CLEAN_LOCKFILE,
                        &[
                            package_ref.to_string(),
                            integrity_short(lockfile_integrity),
                            tarball_hashes.bytes.to_string(),
                        ],
                    ),
                    evidence: Evidence {
                        registry_integrity: Some(registry_integrity.to_string()),
                        lockfile_integrity: Some(lockfile_integrity.to_string()),
                        computed_sha512: Some(computed_integrity),
                        source_url: Some(tarball_url.to_string()),
                    },
                };
                self.cache_and_return(clean_result)
            }

            Ok(false) => {
                // Lockfile and registry agree, but tarball diverges — CDN/registry compromise
                let compromised_result = VerifyResult {
                    package: package_ref.clone(),
                    verdict: Verdict::Compromised {
                        expected: lockfile_integrity.to_string(),
                        actual: computed_integrity.clone(),
                        source: CompromisedSource::DownloadVsRegistry,
                    },
                    detail: render_template(
                        VERIFIER_DETAIL_COMPROMISED_TARBALL_VS_LOCKFILE,
                        &[
                            package_ref.to_string(),
                            integrity_short(lockfile_integrity),
                            integrity_short(&computed_integrity),
                            integrity_short(registry_integrity),
                        ],
                    ),
                    evidence: Evidence {
                        registry_integrity: Some(registry_integrity.to_string()),
                        lockfile_integrity: Some(lockfile_integrity.to_string()),
                        computed_sha512: Some(computed_integrity),
                        source_url: Some(tarball_url.to_string()),
                    },
                };

                self.cache.invalidate(package_ref);

                compromised_result
            }

            Err(error) => {
                self.cache_and_return(create_unverifiable(CreateUnverifiableParams {
                    reason: UnverifiableReason::NoIntegrityField,
                    package: package_ref,
                    detail_template: VERIFIER_DETAIL_TARBALL_INTEGRITY_FORMAT_ERROR_DURING_CHECK,
                    template_args: &[package_ref.to_string(), error.to_string()],
                    evidence: Evidence {
                        registry_integrity: Some(registry_integrity.to_string()),
                        lockfile_integrity: Some(lockfile_integrity.to_string()),
                        source_url: Some(tarball_url.to_string()),
                        ..Evidence::empty()
                    },
                }))
            }
        }
    }
}
