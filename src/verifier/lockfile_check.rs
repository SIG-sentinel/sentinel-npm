use crate::constants::lockfile_check::{LOG_CACHE_HIT, LOG_CACHE_STALE};
use crate::constants::{
    INTEGRITY_PREFIX_SHA1, VERIFIER_DETAIL_CLEAN_LOCKFILE, VERIFIER_DETAIL_COMPROMISED_LOCKFILE,
    VERIFIER_DETAIL_COMPROMISED_TARBALL_VS_LOCKFILE, VERIFIER_DETAIL_LEGACY_SHA1_LOCKFILE,
    VERIFIER_DETAIL_NO_LOCKFILE_INTEGRITY, VERIFIER_DETAIL_NOT_IN_REGISTRY,
    VERIFIER_DETAIL_PREDATES_INTEGRITY, VERIFIER_DETAIL_PROVENANCE_INCONSISTENT,
    VERIFIER_DETAIL_PROVENANCE_MISSING, VERIFIER_DETAIL_REGISTRY_FETCH_ERROR,
    VERIFIER_DETAIL_REGISTRY_TIMEOUT, VERIFIER_DETAIL_REGISTRY_UNREACHABLE,
    VERIFIER_DETAIL_TARBALL_DOWNLOAD_FAILED_DURING_CHECK,
    VERIFIER_DETAIL_TARBALL_INTEGRITY_FORMAT_ERROR_DURING_CHECK,
    VERIFIER_DETAIL_TARBALL_STREAM_ERROR_DURING_CHECK,
    VERIFIER_DETAIL_TARBALL_TOO_LARGE_DURING_CHECK, render_template,
};
use crate::crypto::{hash_stream, integrity_short, normalize_integrity, verify_integrity};
use crate::npm::LockfileEntry;
use crate::types::{
    BuildCleanLockfileResultParams, BuildCompromisedLockfileResultParams,
    BuildCompromisedTarballResultParams, BuildIntegrityEvidenceWithComputedParams,
    BuildIntegrityEvidenceWithoutComputedParams, BuildIntegrityResultParams, CacheMatchParams,
    CacheUnverifiableWithErrorDetailsParams, CheckFromLockfileParams, CompromisedSource, Evidence,
    FinalizeCleanProvenanceResultParams, HandleHashStreamErrorParams,
    HandleRegistryFetchErrorParams, HashStreamParams, IntegrityEvidenceParams,
    LockfilePackageAndErrorTemplateArgsParams, SentinelError, UnverifiableReason,
    UnverifiableTemplateParams, Verdict, VerifyIntegrityParams, VerifyResult,
    VerifyTarballIntegrityParams,
};

use super::{
    Verifier, cache_matches_lockfile, cache_requires_tarball_revalidation,
    cache_unverifiable_from_template, computed_sha512_integrity, create_unverifiable_from_template,
};

const BYTES_PER_MIB_USIZE: usize = 1024 * 1024;
const MIB_TENTHS_FACTOR: usize = 10;

fn format_tarball_size_mib_from_bytes(bytes: usize) -> String {
    let rounded_tenths = bytes
        .saturating_mul(MIB_TENTHS_FACTOR)
        .saturating_add(BYTES_PER_MIB_USIZE / 2)
        / BYTES_PER_MIB_USIZE;

    let whole = rounded_tenths / MIB_TENTHS_FACTOR;
    let decimal = rounded_tenths % MIB_TENTHS_FACTOR;

    format!("{whole}.{decimal}")
}

fn build_package_and_error_template_args(
    params: &LockfilePackageAndErrorTemplateArgsParams<'_>,
) -> Vec<String> {
    let LockfilePackageAndErrorTemplateArgsParams {
        package_ref,
        error_description,
    } = params;

    vec![package_ref.to_string(), error_description.to_string()]
}

fn build_integrity_evidence(params: &IntegrityEvidenceParams<'_>) -> Evidence {
    let IntegrityEvidenceParams {
        lockfile_integrity,
        registry_integrity,
        tarball_url,
        computed_sha512,
    } = params;

    Evidence {
        lockfile_integrity: Some(lockfile_integrity.to_string()),
        registry_integrity: Some(registry_integrity.to_string()),
        computed_sha512: computed_sha512.map(ToString::to_string),
        source_url: Some(tarball_url.to_string()),
        ..Evidence::empty()
    }
}

fn build_integrity_evidence_without_computed(
    params: &BuildIntegrityEvidenceWithoutComputedParams<'_>,
) -> Evidence {
    let BuildIntegrityEvidenceWithoutComputedParams {
        lockfile_integrity,
        registry_integrity,
        tarball_url,
    } = params;

    let build_integrity_evidence_params = IntegrityEvidenceParams {
        lockfile_integrity,
        registry_integrity,
        tarball_url,
        computed_sha512: None,
    };

    build_integrity_evidence(&build_integrity_evidence_params)
}

fn build_integrity_evidence_with_computed(
    params: &BuildIntegrityEvidenceWithComputedParams<'_>,
) -> Evidence {
    let BuildIntegrityEvidenceWithComputedParams {
        lockfile_integrity,
        registry_integrity,
        tarball_url,
        computed_sha512,
    } = params;

    let build_integrity_evidence_params = IntegrityEvidenceParams {
        lockfile_integrity,
        registry_integrity,
        tarball_url,
        computed_sha512: Some(computed_sha512),
    };

    build_integrity_evidence(&build_integrity_evidence_params)
}

impl Verifier {
    pub async fn check_from_lockfile(&self, entry: &LockfileEntry) -> VerifyResult {
        let check_params = CheckFromLockfileParams {
            verifier: self,
            entry,
        };

        check_from_lockfile_impl(check_params).await
    }
}

fn try_use_cached_result(params: &CheckFromLockfileParams<'_, Verifier>) -> Option<VerifyResult> {
    let CheckFromLockfileParams { verifier, entry } = params;
    let package_ref = &entry.package;

    let cached_result = verifier.cache.get(package_ref)?;
    let cache_match_params = CacheMatchParams {
        entry,
        cached_result: &cached_result,
    };
    let cache_valid = cache_matches_lockfile(cache_match_params);

    if !cache_valid {
        tracing::debug!("{package_ref}: {LOG_CACHE_STALE}");
        verifier.cache.invalidate(package_ref);

        return None;
    }

    let needs_revalidation = cache_requires_tarball_revalidation(&cached_result);

    if needs_revalidation {
        tracing::debug!("{package_ref}: {LOG_CACHE_STALE} (missing computed_sha512 evidence)");
        verifier.cache.invalidate(package_ref);

        return None;
    }

    tracing::debug!("{package_ref}: {LOG_CACHE_HIT}");
    Some(cached_result)
}

fn build_compromised_lockfile_result(
    params: &BuildCompromisedLockfileResultParams<'_>,
) -> VerifyResult {
    let BuildCompromisedLockfileResultParams {
        package_ref,
        lockfile_integrity,
        registry_integrity,
        tarball_url,
    } = params;

    let compromised_lockfile_template_args = vec![
        package_ref.to_string(),
        integrity_short(lockfile_integrity),
        integrity_short(registry_integrity),
        package_ref.to_string(),
    ];

    VerifyResult {
        package: (*package_ref).clone(),
        verdict: Verdict::Compromised {
            expected: registry_integrity.to_string(),
            actual: lockfile_integrity.to_string(),
            source: CompromisedSource::LockfileVsRegistry,
        },
        detail: render_template(
            VERIFIER_DETAIL_COMPROMISED_LOCKFILE,
            &compromised_lockfile_template_args,
        ),
        evidence: Evidence {
            registry_integrity: Some(registry_integrity.to_string()),
            lockfile_integrity: Some(lockfile_integrity.to_string()),
            source_url: Some(tarball_url.to_string()),
            ..Evidence::empty()
        },
        is_direct: false,
        direct_parent: None,
        tarball_fingerprint: None,
    }
}

#[allow(clippy::implicit_clone)]
async fn check_from_lockfile_impl(params: CheckFromLockfileParams<'_, Verifier>) -> VerifyResult {
    let CheckFromLockfileParams { verifier, entry } = params;
    let package_ref = &entry.package;

    let try_use_cached_result_params = CheckFromLockfileParams { verifier, entry };

    if let Some(cached_result) = try_use_cached_result(&try_use_cached_result_params) {
        return cached_result;
    }

    let Some(lockfile_integrity) = entry.integrity.clone() else {
        let unverifiable_params = UnverifiableTemplateParams {
            reason: UnverifiableReason::MissingFromLockfile,
            package: package_ref,
            detail_template: VERIFIER_DETAIL_NO_LOCKFILE_INTEGRITY,
            template_args: vec![package_ref.to_string()],
            evidence: Evidence::empty(),
        };

        return cache_unverifiable_from_template(verifier, unverifiable_params);
    };

    let normalized_lockfile_integrity = normalize_integrity(&lockfile_integrity).to_string();
    let lockfile_uses_legacy_sha1 =
        normalized_lockfile_integrity.starts_with(INTEGRITY_PREFIX_SHA1);

    if lockfile_uses_legacy_sha1 {
        let unverifiable_params = UnverifiableTemplateParams {
            reason: UnverifiableReason::LegacySha1Lockfile,
            package: package_ref,
            detail_template: VERIFIER_DETAIL_LEGACY_SHA1_LOCKFILE,
            template_args: vec![
                package_ref.to_string(),
                integrity_short(&normalized_lockfile_integrity),
            ],
            evidence: Evidence {
                lockfile_integrity: Some(normalized_lockfile_integrity),
                ..Evidence::empty()
            },
        };

        return cache_unverifiable_from_template(verifier, unverifiable_params);
    }

    let registry_metadata = match verifier.registry.fetch_version(package_ref).await {
        Ok(metadata) => metadata,
        Err(error) => {
            let registry_fetch_error_params = HandleRegistryFetchErrorParams {
                verifier,
                package_ref,
                error,
                lockfile_integrity: &lockfile_integrity,
            };
            let registry_fetch_error =
                handle_registry_fetch_error_impl(registry_fetch_error_params);

            return registry_fetch_error;
        }
    };

    let Some(registry_integrity) = registry_metadata.dist.integrity.clone() else {
        let unverifiable_params = UnverifiableTemplateParams {
            reason: UnverifiableReason::NoIntegrityField,
            package: package_ref,
            detail_template: VERIFIER_DETAIL_PREDATES_INTEGRITY,
            template_args: vec![
                package_ref.to_string(),
                integrity_short(&lockfile_integrity),
            ],
            evidence: Evidence {
                lockfile_integrity: Some(lockfile_integrity.clone()),
                source_url: Some(registry_metadata.dist.tarball.clone()),
                ..Evidence::empty()
            },
        };

        return cache_unverifiable_from_template(verifier, unverifiable_params);
    };

    let normalized_registry_integrity = normalize_integrity(&registry_integrity).to_string();
    let integrities_match = normalized_lockfile_integrity == normalized_registry_integrity;

    if integrities_match {
        let verify_tarball_integrity_params = VerifyTarballIntegrityParams {
            verifier,
            package_ref,
            lockfile_integrity: &lockfile_integrity,
            registry_integrity: &registry_integrity,
            tarball_url: &registry_metadata.dist.tarball,
            provenance: registry_metadata.dist.provenance.as_ref(),
        };

        return verify_tarball_integrity_impl(verify_tarball_integrity_params).await;
    }

    let build_compromised_lockfile_result_params = BuildCompromisedLockfileResultParams {
        package_ref,
        lockfile_integrity: &normalized_lockfile_integrity,
        registry_integrity: &normalized_registry_integrity,
        tarball_url: &registry_metadata.dist.tarball,
    };
    let compromised = build_compromised_lockfile_result(&build_compromised_lockfile_result_params);

    verifier.cache.invalidate(package_ref);

    compromised
}

#[allow(clippy::implicit_clone)]
fn handle_registry_fetch_error_impl(
    params: HandleRegistryFetchErrorParams<'_, Verifier>,
) -> VerifyResult {
    let HandleRegistryFetchErrorParams {
        verifier,
        package_ref,
        error,
        lockfile_integrity,
    } = params;

    let (reason, detail_template, detail_template_arguments) = match error {
        SentinelError::RegistryUnreachable(unreachable_error) => (
            UnverifiableReason::RegistryOffline,
            VERIFIER_DETAIL_REGISTRY_UNREACHABLE,
            vec![
                package_ref.to_string(),
                unreachable_error.to_string(),
                integrity_short(lockfile_integrity).to_string(),
            ],
        ),
        SentinelError::RegistryTimeout {
            package,
            version,
            ms,
        } => {
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
            {
                let error_description = error.to_string();
                let build_package_and_error_template_args_params =
                    LockfilePackageAndErrorTemplateArgsParams {
                        package_ref,
                        error_description: &error_description,
                    };

                build_package_and_error_template_args(&build_package_and_error_template_args_params)
            },
        ),
    };

    let unverifiable_params = UnverifiableTemplateParams {
        reason,
        package: package_ref,
        detail_template,
        template_args: detail_template_arguments,
        evidence: Evidence {
            lockfile_integrity: Some(lockfile_integrity.to_string()),
            ..Evidence::empty()
        },
    };

    cache_unverifiable_from_template(verifier, unverifiable_params)
}

fn cache_unverifiable_with_error_details(
    params: &CacheUnverifiableWithErrorDetailsParams<'_, Verifier>,
) -> VerifyResult {
    let CacheUnverifiableWithErrorDetailsParams {
        verifier,
        package_ref,
        lockfile_integrity,
        registry_integrity,
        tarball_url,
        reason,
        detail_template,
        error_description,
    } = params;

    let build_package_and_error_template_args_params = LockfilePackageAndErrorTemplateArgsParams {
        package_ref,
        error_description,
    };
    let build_integrity_evidence_without_computed_params =
        BuildIntegrityEvidenceWithoutComputedParams {
            lockfile_integrity,
            registry_integrity,
            tarball_url,
        };

    let unverifiable_params = UnverifiableTemplateParams {
        reason: *reason,
        package: package_ref,
        detail_template,
        template_args: build_package_and_error_template_args(
            &build_package_and_error_template_args_params,
        ),
        evidence: build_integrity_evidence_without_computed(
            &build_integrity_evidence_without_computed_params,
        ),
    };

    cache_unverifiable_from_template(verifier, unverifiable_params)
}

#[allow(clippy::implicit_clone)]
async fn verify_tarball_integrity_impl(
    params: VerifyTarballIntegrityParams<'_, Verifier>,
) -> VerifyResult {
    let VerifyTarballIntegrityParams {
        verifier,
        package_ref,
        lockfile_integrity,
        registry_integrity,
        tarball_url,
        provenance,
    } = params;

    let tarball_response = match verifier.registry.download_tarball(tarball_url).await {
        Ok(response) => response,
        Err(error) => {
            let error_description = error.to_string();

            let cache_unverifiable_with_error_details_params =
                CacheUnverifiableWithErrorDetailsParams {
                    verifier,
                    package_ref,
                    lockfile_integrity,
                    registry_integrity,
                    tarball_url,
                    reason: UnverifiableReason::RegistryOffline,
                    detail_template: VERIFIER_DETAIL_TARBALL_DOWNLOAD_FAILED_DURING_CHECK,
                    error_description: &error_description,
                };

            return cache_unverifiable_with_error_details(
                &cache_unverifiable_with_error_details_params,
            );
        }
    };

    let hash_stream_params = HashStreamParams {
        stream: tarball_response.bytes_stream(),
        package: &package_ref.name,
        capture_buffer: false,
        spool_to_disk: false,
        inflight_counter: None,
    };
    let tarball_hashes = match hash_stream(hash_stream_params).await {
        Ok(hashes) => hashes,
        Err(hash_stream_error) => {
            let hash_stream_error_params = HandleHashStreamErrorParams {
                error: hash_stream_error,
                package_ref,
                lockfile_integrity,
                registry_integrity,
                tarball_url,
            };
            let unverifiable = handle_hash_stream_error(hash_stream_error_params);

            return verifier.cache_and_return(unverifiable);
        }
    };

    let computed_integrity = computed_sha512_integrity(&tarball_hashes.sha512_bytes);

    let verify_integrity_params = VerifyIntegrityParams {
        sha512_bytes: &tarball_hashes.sha512_bytes,
        integrity_field: lockfile_integrity,
    };
    let integrity_valid = match verify_integrity(verify_integrity_params) {
        Ok(result) => result,
        Err(error) => {
            let error_description = error.to_string();

            let cache_unverifiable_with_error_details_params =
                CacheUnverifiableWithErrorDetailsParams {
                    verifier,
                    package_ref,
                    lockfile_integrity,
                    registry_integrity,
                    tarball_url,
                    reason: UnverifiableReason::NoIntegrityField,
                    detail_template: VERIFIER_DETAIL_TARBALL_INTEGRITY_FORMAT_ERROR_DURING_CHECK,
                    error_description: &error_description,
                };

            return cache_unverifiable_with_error_details(
                &cache_unverifiable_with_error_details_params,
            );
        }
    };

    let build_integrity_result_params = BuildIntegrityResultParams {
        integrity_valid,
        verifier,
        package_ref,
        lockfile_integrity,
        registry_integrity,
        tarball_url,
        computed_integrity: &computed_integrity,
        tarball_bytes: tarball_hashes.bytes,
        provenance,
    };

    build_integrity_result(&build_integrity_result_params)
}

fn handle_hash_stream_error(params: HandleHashStreamErrorParams<'_>) -> VerifyResult {
    let HandleHashStreamErrorParams {
        error,
        package_ref,
        lockfile_integrity,
        registry_integrity,
        tarball_url,
    } = params;

    if let SentinelError::TarballTooLarge { bytes, .. } = error {
        let tarball_size_mib_text = format_tarball_size_mib_from_bytes(bytes);

        let tarball_too_large_unverifiable_template_params = UnverifiableTemplateParams {
            reason: UnverifiableReason::TarballTooLarge,
            package: package_ref,
            detail_template: VERIFIER_DETAIL_TARBALL_TOO_LARGE_DURING_CHECK,
            template_args: vec![package_ref.to_string(), tarball_size_mib_text],
            evidence: {
                let build_integrity_evidence_without_computed_params =
                    BuildIntegrityEvidenceWithoutComputedParams {
                        lockfile_integrity,
                        registry_integrity,
                        tarball_url,
                    };

                build_integrity_evidence_without_computed(
                    &build_integrity_evidence_without_computed_params,
                )
            },
        };

        return create_unverifiable_from_template(tarball_too_large_unverifiable_template_params);
    }

    let stream_error_unverifiable_template_params = UnverifiableTemplateParams {
        reason: UnverifiableReason::RegistryOffline,
        package: package_ref,
        detail_template: VERIFIER_DETAIL_TARBALL_STREAM_ERROR_DURING_CHECK,
        template_args: {
            let error_description = error.to_string();
            let build_package_and_error_template_args_params =
                LockfilePackageAndErrorTemplateArgsParams {
                    package_ref,
                    error_description: &error_description,
                };

            build_package_and_error_template_args(&build_package_and_error_template_args_params)
        },
        evidence: {
            let build_integrity_evidence_without_computed_params =
                BuildIntegrityEvidenceWithoutComputedParams {
                    lockfile_integrity,
                    registry_integrity,
                    tarball_url,
                };

            build_integrity_evidence_without_computed(
                &build_integrity_evidence_without_computed_params,
            )
        },
    };

    create_unverifiable_from_template(stream_error_unverifiable_template_params)
}

fn build_clean_lockfile_result(params: &BuildCleanLockfileResultParams<'_>) -> VerifyResult {
    let BuildCleanLockfileResultParams {
        package_ref,
        lockfile_integrity,
        registry_integrity,
        tarball_url,
        computed_integrity,
        tarball_bytes,
    } = params;

    let clean_lockfile_template_args = vec![
        package_ref.to_string(),
        integrity_short(lockfile_integrity),
        tarball_bytes.to_string(),
    ];

    VerifyResult {
        package: (*package_ref).clone(),
        verdict: Verdict::Clean,
        detail: render_template(
            VERIFIER_DETAIL_CLEAN_LOCKFILE,
            &clean_lockfile_template_args,
        ),
        evidence: {
            let build_integrity_evidence_with_computed_params =
                BuildIntegrityEvidenceWithComputedParams {
                    lockfile_integrity,
                    registry_integrity,
                    tarball_url,
                    computed_sha512: computed_integrity,
                };

            build_integrity_evidence_with_computed(&build_integrity_evidence_with_computed_params)
        },
        is_direct: false,
        direct_parent: None,
        tarball_fingerprint: None,
    }
}

fn build_compromised_tarball_result(
    params: &BuildCompromisedTarballResultParams<'_>,
) -> VerifyResult {
    let BuildCompromisedTarballResultParams {
        package_ref,
        lockfile_integrity,
        registry_integrity,
        tarball_url,
        computed_integrity,
    } = params;

    let compromised_tarball_lockfile_template_args = vec![
        package_ref.to_string(),
        integrity_short(lockfile_integrity),
        integrity_short(computed_integrity),
        integrity_short(registry_integrity),
    ];

    VerifyResult {
        package: (*package_ref).clone(),
        verdict: Verdict::Compromised {
            expected: lockfile_integrity.to_string(),
            actual: computed_integrity.to_string(),
            source: CompromisedSource::DownloadVsRegistry,
        },
        detail: render_template(
            VERIFIER_DETAIL_COMPROMISED_TARBALL_VS_LOCKFILE,
            &compromised_tarball_lockfile_template_args,
        ),
        evidence: {
            let build_integrity_evidence_with_computed_params =
                BuildIntegrityEvidenceWithComputedParams {
                    lockfile_integrity,
                    registry_integrity,
                    tarball_url,
                    computed_sha512: computed_integrity,
                };

            build_integrity_evidence_with_computed(&build_integrity_evidence_with_computed_params)
        },
        is_direct: false,
        direct_parent: None,
        tarball_fingerprint: None,
    }
}

fn finalize_clean_provenance_result(
    params: FinalizeCleanProvenanceResultParams<'_, Verifier>,
) -> VerifyResult {
    let FinalizeCleanProvenanceResultParams {
        verifier,
        package_ref,
        lockfile_integrity,
        registry_integrity,
        tarball_url,
        computed_integrity,
        provenance,
        clean_result,
    } = params;

    let Some(provenance_payload) = provenance else {
        let provenance_missing_params = UnverifiableTemplateParams {
            reason: UnverifiableReason::ProvenanceMissing,
            package: package_ref,
            detail_template: VERIFIER_DETAIL_PROVENANCE_MISSING,
            template_args: vec![package_ref.to_string()],
            evidence: {
                let build_integrity_evidence_with_computed_params =
                    BuildIntegrityEvidenceWithComputedParams {
                        lockfile_integrity,
                        registry_integrity,
                        tarball_url,
                        computed_sha512: computed_integrity,
                    };

                build_integrity_evidence_with_computed(
                    &build_integrity_evidence_with_computed_params,
                )
            },
        };
        let provenance_missing = create_unverifiable_from_template(provenance_missing_params);

        return verifier.cache_and_return(provenance_missing);
    };

    let expected_subject_integrity = provenance_payload.subject_integrity.as_deref();
    let normalized_expected_subject =
        expected_subject_integrity.map(|value| normalize_integrity(value).to_string());
    let normalized_actual_integrity = normalize_integrity(computed_integrity).to_string();
    let has_subject_match = normalized_expected_subject
        .as_ref()
        .is_some_and(|expected| expected == &normalized_actual_integrity);

    if !has_subject_match {
        let build_integrity_evidence_with_computed_params =
            BuildIntegrityEvidenceWithComputedParams {
                lockfile_integrity,
                registry_integrity,
                tarball_url,
                computed_sha512: computed_integrity,
            };
        let mut provenance_inconsistent_evidence =
            build_integrity_evidence_with_computed(&build_integrity_evidence_with_computed_params);

        provenance_inconsistent_evidence
            .provenance_subject_digest
            .clone_from(&normalized_expected_subject);
        provenance_inconsistent_evidence
            .provenance_issuer
            .clone_from(&provenance_payload.issuer);
        provenance_inconsistent_evidence
            .provenance_identity
            .clone_from(&provenance_payload.identity);
        provenance_inconsistent_evidence
            .provenance_bundle_source
            .clone_from(&provenance_payload.source);

        let provenance_inconsistent_params = UnverifiableTemplateParams {
            reason: UnverifiableReason::ProvenanceInconsistent,
            package: package_ref,
            detail_template: VERIFIER_DETAIL_PROVENANCE_INCONSISTENT,
            template_args: vec![
                package_ref.to_string(),
                normalized_expected_subject
                    .as_deref()
                    .unwrap_or("<missing>")
                    .to_string(),
                normalized_actual_integrity.clone(),
            ],
            evidence: provenance_inconsistent_evidence,
        };
        let provenance_inconsistent =
            create_unverifiable_from_template(provenance_inconsistent_params);

        verifier.cache.invalidate(package_ref);

        return provenance_inconsistent;
    }

    let mut clean_with_provenance = clean_result;

    clean_with_provenance.evidence.provenance_subject_digest = Some(normalized_actual_integrity);
    clean_with_provenance
        .evidence
        .provenance_issuer
        .clone_from(&provenance_payload.issuer);
    clean_with_provenance
        .evidence
        .provenance_identity
        .clone_from(&provenance_payload.identity);
    clean_with_provenance
        .evidence
        .provenance_bundle_source
        .clone_from(&provenance_payload.source);

    verifier.cache_and_return(clean_with_provenance)
}

#[allow(clippy::implicit_clone)]
fn build_integrity_result(params: &BuildIntegrityResultParams<'_, Verifier>) -> VerifyResult {
    let BuildIntegrityResultParams {
        integrity_valid,
        verifier,
        package_ref,
        lockfile_integrity,
        registry_integrity,
        tarball_url,
        computed_integrity,
        tarball_bytes,
        provenance,
    } = *params;

    if !integrity_valid {
        let build_compromised_tarball_result_params = BuildCompromisedTarballResultParams {
            package_ref,
            lockfile_integrity,
            registry_integrity,
            tarball_url,
            computed_integrity,
        };
        verifier.cache.invalidate(package_ref);

        return build_compromised_tarball_result(&build_compromised_tarball_result_params);
    }

    let build_clean_lockfile_result_params = BuildCleanLockfileResultParams {
        package_ref,
        lockfile_integrity,
        registry_integrity,
        tarball_url,
        computed_integrity,
        tarball_bytes,
    };
    let clean_result = build_clean_lockfile_result(&build_clean_lockfile_result_params);

    let finalize_clean_provenance_result_params = FinalizeCleanProvenanceResultParams {
        verifier,
        package_ref,
        lockfile_integrity,
        registry_integrity,
        tarball_url,
        computed_integrity,
        provenance,
        clean_result,
    };

    finalize_clean_provenance_result(finalize_clean_provenance_result_params)
}
