use crate::constants::lockfile_check::{LOG_CACHE_HIT, LOG_CACHE_STALE};
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
    BuildIntegrityResultParams, CacheMatchParams, CheckFromLockfileParams, CompromisedSource,
    CreateUnverifiableParams, Evidence, HandleHashStreamErrorParams,
    HandleRegistryFetchErrorParams, HashStreamParams, SentinelError,
    UnverifiableReason, VerifyIntegrityParams, VerifyResult, VerifyTarballIntegrityParams, Verdict,
};

use super::{Verifier, cache_matches_lockfile, computed_sha512_integrity, create_unverifiable};

impl Verifier {
    pub async fn check_from_lockfile(&self, entry: &LockfileEntry) -> VerifyResult {
        check_from_lockfile_impl(CheckFromLockfileParams {
            verifier: self,
            entry,
        })
        .await
    }
}

async fn check_from_lockfile_impl(params: CheckFromLockfileParams<'_, Verifier>) -> VerifyResult {
    let CheckFromLockfileParams { verifier, entry } = params;
    let package_ref = &entry.package;

    if let Some(cached_result) = verifier.cache.get(package_ref) {
        let cache_valid = cache_matches_lockfile(CacheMatchParams {
            entry,
            cached_result: &cached_result,
        });

        if cache_valid {
            tracing::debug!("{package_ref}: {LOG_CACHE_HIT}");
            return cached_result;
        }

        tracing::debug!("{package_ref}: {LOG_CACHE_STALE}");
        verifier.cache.invalidate(package_ref);
    }

    let Some(lockfile_integrity) = entry.integrity.clone() else {
        let unverifiable = create_unverifiable(CreateUnverifiableParams {
            reason: UnverifiableReason::MissingFromLockfile,
            package: package_ref,
            detail_template: VERIFIER_DETAIL_NO_LOCKFILE_INTEGRITY,
            template_args: &[package_ref.to_string()],
            evidence: Evidence::empty(),
        });
        return verifier.cache_and_return(unverifiable);
    };

    let registry_metadata = match verifier.registry.fetch_version(package_ref).await {
        Ok(metadata) => metadata,
        Err(error) => {
            return handle_registry_fetch_error_impl(HandleRegistryFetchErrorParams {
                verifier,
                package_ref,
                error,
                lockfile_integrity: &lockfile_integrity,
            })
        }
    };

    let Some(registry_integrity) = registry_metadata.dist.integrity.clone() else {
        let unverifiable = create_unverifiable(CreateUnverifiableParams {
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
        return verifier.cache_and_return(unverifiable);
    };

    let integrities_match = lockfile_integrity == registry_integrity;

    match (integrities_match, &lockfile_integrity, &registry_integrity) {
        (true, _, _) => {
            verify_tarball_integrity_impl(VerifyTarballIntegrityParams {
                verifier,
                package_ref,
                lockfile_integrity: &lockfile_integrity,
                registry_integrity: &registry_integrity,
                tarball_url: &registry_metadata.dist.tarball,
            })
            .await
        }
        (false, lock_int, reg_int) => {
            let compromised = VerifyResult {
                package: package_ref.clone(),
                verdict: Verdict::Compromised {
                    expected: reg_int.to_string(),
                    actual: lock_int.to_string(),
                    source: CompromisedSource::LockfileVsRegistry,
                },
                detail: render_template(
                    VERIFIER_DETAIL_COMPROMISED_LOCKFILE,
                    &[
                        package_ref.to_string(),
                        integrity_short(lock_int),
                        integrity_short(reg_int),
                        package_ref.to_string(),
                    ],
                ),
                evidence: Evidence {
                    registry_integrity: Some(reg_int.to_string()),
                    lockfile_integrity: Some(lock_int.to_string()),
                    source_url: Some(registry_metadata.dist.tarball),
                    ..Evidence::empty()
                },
            };
            verifier.cache.invalidate(package_ref);
            compromised
        }
    }
}

fn handle_registry_fetch_error_impl(params: HandleRegistryFetchErrorParams<'_, Verifier>) -> VerifyResult {
    let HandleRegistryFetchErrorParams {
        verifier,
        package_ref,
        error,
        lockfile_integrity,
    } = params;

    let (reason, detail_template, args) = match error {
        SentinelError::RegistryUnreachable(err) => (
            UnverifiableReason::RegistryOffline,
            VERIFIER_DETAIL_REGISTRY_UNREACHABLE,
            vec![
                package_ref.to_string(),
                err.to_string(),
                integrity_short(lockfile_integrity).to_string(),
            ],
        ),
        SentinelError::RegistryTimeout { package, version, ms } => {
            let _ = (package, version);
            (
                UnverifiableReason::RegistryTimeout,
                VERIFIER_DETAIL_REGISTRY_TIMEOUT,
                vec![
                    package_ref.to_string(),
                    ms.to_string(),
                    integrity_short(lockfile_integrity).to_string(),
                ],
            )
        }
        SentinelError::NoIntegrity { .. } => (
            UnverifiableReason::NoIntegrityField,
            VERIFIER_DETAIL_NOT_IN_REGISTRY,
            vec![
                package_ref.to_string(),
                integrity_short(lockfile_integrity).to_string(),
            ],
        ),
        _ => (
            UnverifiableReason::RegistryOffline,
            VERIFIER_DETAIL_REGISTRY_FETCH_ERROR,
            vec![package_ref.to_string(), error.to_string()],
        ),
    };

    let unverifiable = create_unverifiable(CreateUnverifiableParams {
        reason,
        package: package_ref,
        detail_template,
        template_args: &args,
        evidence: Evidence {
            lockfile_integrity: Some(lockfile_integrity.to_string()),
            ..Evidence::empty()
        },
    });

    verifier.cache_and_return(unverifiable)
}

async fn verify_tarball_integrity_impl(params: VerifyTarballIntegrityParams<'_, Verifier>) -> VerifyResult {
    let VerifyTarballIntegrityParams {
        verifier,
        package_ref,
        lockfile_integrity,
        registry_integrity,
        tarball_url,
    } = params;

    let tarball_response = match verifier.registry.download_tarball(tarball_url).await {
        Ok(response) => response,
        Err(error) => {
            let unverifiable = create_unverifiable(CreateUnverifiableParams {
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
            });
            
            return verifier.cache_and_return(unverifiable);
        }
    };

    let tarball_hashes = match hash_stream(HashStreamParams {
        stream: tarball_response.bytes_stream(),
        package: &package_ref.name,
    })
    .await
    {
        Ok(hashes) => hashes,
        Err(err) => {
            let unverifiable = handle_hash_stream_error(HandleHashStreamErrorParams {
                error: err,
                package_ref,
                lockfile_integrity,
                registry_integrity,
                tarball_url,
            });
            
            return verifier.cache_and_return(unverifiable);
        }
    };

    let computed_integrity = computed_sha512_integrity(&tarball_hashes.sha512_bytes);

    let integrity_valid = match verify_integrity(VerifyIntegrityParams {
        sha512_bytes: &tarball_hashes.sha512_bytes,
        integrity_field: lockfile_integrity,
    }) {
        Ok(result) => result,
        Err(error) => {
            let unverifiable = create_unverifiable(CreateUnverifiableParams {
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
            });
            
            return verifier.cache_and_return(unverifiable);
        }
    };

    build_integrity_result(BuildIntegrityResultParams {
        integrity_valid,
        verifier,
        package_ref,
        lockfile_integrity,
        registry_integrity,
        tarball_url,
        computed_integrity: &computed_integrity,
        tarball_bytes: tarball_hashes.bytes,
    })
}

fn handle_hash_stream_error(params: HandleHashStreamErrorParams<'_>) -> VerifyResult {
    let HandleHashStreamErrorParams {
        error,
        package_ref,
        lockfile_integrity,
        registry_integrity,
        tarball_url,
    } = params;

    match error {
        SentinelError::TarballTooLarge { bytes, .. } => {
            let tarball_size_mib = bytes as f64 / BYTES_PER_MIB;
            create_unverifiable(CreateUnverifiableParams {
                reason: UnverifiableReason::TarballTooLarge,
                package: package_ref,
                detail_template: VERIFIER_DETAIL_TARBALL_TOO_LARGE_DURING_CHECK,
                template_args: &[
                    package_ref.to_string(),
                    format!("{:.1}", tarball_size_mib),
                ],
                evidence: Evidence {
                    lockfile_integrity: Some(lockfile_integrity.to_string()),
                    registry_integrity: Some(registry_integrity.to_string()),
                    source_url: Some(tarball_url.to_string()),
                    ..Evidence::empty()
                },
            })
        }
        _ => create_unverifiable(CreateUnverifiableParams {
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
        }),
    }
}

fn build_integrity_result(params: BuildIntegrityResultParams<'_, Verifier>) -> VerifyResult {
    let BuildIntegrityResultParams {
        integrity_valid,
        verifier,
        package_ref,
        lockfile_integrity,
        registry_integrity,
        tarball_url,
        computed_integrity,
        tarball_bytes,
    } = params;

    match integrity_valid {
        true => VerifyResult {
            package: package_ref.clone(),
            verdict: Verdict::Clean,
            detail: render_template(
                VERIFIER_DETAIL_CLEAN_LOCKFILE,
                &[
                    package_ref.to_string(),
                    integrity_short(lockfile_integrity),
                    tarball_bytes.to_string(),
                ],
            ),
            evidence: Evidence {
                registry_integrity: Some(registry_integrity.to_string()),
                lockfile_integrity: Some(lockfile_integrity.to_string()),
                computed_sha512: Some(computed_integrity.to_string()),
                source_url: Some(tarball_url.to_string()),
            },
        },

        false => {
            verifier.cache.invalidate(package_ref);
            VerifyResult {
                package: package_ref.clone(),
                verdict: Verdict::Compromised {
                    expected: lockfile_integrity.to_string(),
                    actual: computed_integrity.to_string(),
                    source: CompromisedSource::DownloadVsRegistry,
                },
                detail: render_template(
                    VERIFIER_DETAIL_COMPROMISED_TARBALL_VS_LOCKFILE,
                    &[
                        package_ref.to_string(),
                        integrity_short(lockfile_integrity),
                        integrity_short(computed_integrity),
                        integrity_short(registry_integrity),
                    ],
                ),
                evidence: Evidence {
                    registry_integrity: Some(registry_integrity.to_string()),
                    lockfile_integrity: Some(lockfile_integrity.to_string()),
                    computed_sha512: Some(computed_integrity.to_string()),
                    source_url: Some(tarball_url.to_string()),
                },
            }
        }
    }
}
