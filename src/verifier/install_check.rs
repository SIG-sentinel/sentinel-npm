use crate::constants::{
    BYTES_PER_MIB, VERIFIER_DETAIL_CLEAN_INSTALL, VERIFIER_DETAIL_COMPROMISED_DOWNLOAD,
    VERIFIER_DETAIL_INVALID_INTEGRITY_FORMAT, VERIFIER_DETAIL_NO_DIST_INTEGRITY,
    VERIFIER_DETAIL_REGISTRY_FETCH_FAILED, VERIFIER_DETAIL_STREAM_ERROR,
    VERIFIER_DETAIL_TARBALL_DOWNLOAD_FAILED, VERIFIER_DETAIL_TARBALL_TOO_LARGE, render_template,
};
use crate::crypto::{hash_stream, integrity_short, verify_integrity};
use crate::types::{
    CompromisedSource, CreateUnverifiableParams, Evidence, HashStreamParams, PackageRef,
    SentinelError, UnverifiableReason, Verdict, VerifyBeforeInstallParams, VerifyIntegrityParams,
    VerifyResult,
};

use super::{Verifier, computed_sha512_integrity, create_unverifiable, no_tarball_result};
use crate::types::VerifyResultWithTarball;

impl Verifier {
    pub async fn verify_before_install(&self, package_ref: &PackageRef) -> VerifyResultWithTarball {
        verify_before_install_impl(VerifyBeforeInstallParams {
            verifier: self,
            package_ref,
        })
        .await
    }
}

async fn verify_before_install_impl(
    params: VerifyBeforeInstallParams<'_>,
) -> VerifyResultWithTarball {
    let VerifyBeforeInstallParams { verifier, package_ref } = params;

    let registry_metadata = match verifier.registry.fetch_version(package_ref).await {
        Ok(metadata) => metadata,
        Err(error) => {
            let unverifiable = create_unverifiable(CreateUnverifiableParams {
                reason: UnverifiableReason::RegistryOffline,
                package: package_ref,
                detail_template: VERIFIER_DETAIL_REGISTRY_FETCH_FAILED,
                template_args: &[package_ref.to_string(), error.to_string()],
                evidence: Evidence::empty(),
            });
            
            return no_tarball_result(unverifiable);
        }
    };

    let Some(dist_integrity) = registry_metadata.dist.integrity.clone() else {
        let unverifiable = create_unverifiable(CreateUnverifiableParams {
            reason: UnverifiableReason::NoIntegrityField,
            package: package_ref,
            detail_template: VERIFIER_DETAIL_NO_DIST_INTEGRITY,
            template_args: &[package_ref.to_string()],
            evidence: Evidence {
                source_url: Some(registry_metadata.dist.tarball.clone()),
                ..Evidence::empty()
            },
        });
        
        return no_tarball_result(unverifiable);
    };

    let tarball_response = match verifier
        .registry
        .download_tarball(&registry_metadata.dist.tarball)
        .await
    {
        Ok(response) => response,
        Err(error) => {
            let unverifiable = create_unverifiable(CreateUnverifiableParams {
                reason: UnverifiableReason::RegistryOffline,
                package: package_ref,
                detail_template: VERIFIER_DETAIL_TARBALL_DOWNLOAD_FAILED,
                template_args: &[package_ref.to_string(), error.to_string()],
                evidence: Evidence {
                    source_url: Some(registry_metadata.dist.tarball),
                    ..Evidence::empty()
                },
            });
        
            return no_tarball_result(unverifiable);
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
            let tarball_size_mib = bytes as f64 / BYTES_PER_MIB;
            let unverifiable = create_unverifiable(CreateUnverifiableParams {
                reason: UnverifiableReason::RegistryOffline,
                package: package_ref,
                detail_template: VERIFIER_DETAIL_TARBALL_TOO_LARGE,
                template_args: &[
                    package_ref.to_string(),
                    format!("{:.1}", tarball_size_mib),
                ],
                evidence: Evidence {
                    source_url: Some(registry_metadata.dist.tarball),
                    ..Evidence::empty()
                },
            });
        
            return no_tarball_result(unverifiable);
        }
        Err(error) => {
            let unverifiable = create_unverifiable(CreateUnverifiableParams {
                reason: UnverifiableReason::RegistryOffline,
                package: package_ref,
                detail_template: VERIFIER_DETAIL_STREAM_ERROR,
                template_args: &[package_ref.to_string(), error.to_string()],
                evidence: Evidence::empty(),
            });
        
            return no_tarball_result(unverifiable);
        }
    };

    let computed_integrity = computed_sha512_integrity(&tarball_hashes.sha512_bytes);

    let integrity_valid = match verify_integrity(VerifyIntegrityParams {
        sha512_bytes: &tarball_hashes.sha512_bytes,
        integrity_field: &dist_integrity,
    }) {
        Ok(result) => result,
        Err(error) => {
            let unverifiable = create_unverifiable(CreateUnverifiableParams {
                reason: UnverifiableReason::NoIntegrityField,
                package: package_ref,
                detail_template: VERIFIER_DETAIL_INVALID_INTEGRITY_FORMAT,
                template_args: &[package_ref.to_string(), error.to_string()],
                evidence: Evidence {
                    registry_integrity: Some(dist_integrity),
                    source_url: Some(registry_metadata.dist.tarball),
                    ..Evidence::empty()
                },
            });
        
            return no_tarball_result(unverifiable);
        }
    };

    match integrity_valid {
        true => {
            let clean_result = VerifyResult {
                package: package_ref.clone(),
                verdict: Verdict::Clean,
                detail: render_template(
                    VERIFIER_DETAIL_CLEAN_INSTALL,
                    &[package_ref.to_string(), tarball_hashes.bytes.to_string()],
                ),
                evidence: Evidence {
                    registry_integrity: Some(dist_integrity),
                    computed_sha512: Some(computed_integrity),
                    source_url: Some(registry_metadata.dist.tarball),
                    ..Evidence::empty()
                },
            };

            verifier.cache.put(&clean_result);

            VerifyResultWithTarball {
                result: clean_result,
                tarball: Some(tarball_hashes.buffer),
            }
        }

        false => {
            let compromised_result = VerifyResult {
                package: package_ref.clone(),
                verdict: Verdict::Compromised {
                    expected: dist_integrity.clone(),
                    actual: computed_integrity.clone(),
                    source: CompromisedSource::DownloadVsRegistry,
                },
                detail: render_template(
                    VERIFIER_DETAIL_COMPROMISED_DOWNLOAD,
                    &[
                        package_ref.to_string(),
                        integrity_short(&dist_integrity),
                        integrity_short(&computed_integrity),
                    ],
                ),
                evidence: Evidence {
                    registry_integrity: Some(dist_integrity),
                    computed_sha512: Some(computed_integrity),
                    source_url: Some(registry_metadata.dist.tarball),
                    ..Evidence::empty()
                },
            };

            verifier.cache.invalidate(package_ref);

            VerifyResultWithTarball {
                result: compromised_result,
                tarball: None,
            }
        }
    }
}
