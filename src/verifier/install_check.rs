use crate::constants::{
    VERIFIER_DETAIL_CLEAN_INSTALL, VERIFIER_DETAIL_COMPROMISED_DOWNLOAD,
    VERIFIER_DETAIL_INVALID_INTEGRITY_FORMAT, VERIFIER_DETAIL_NO_DIST_INTEGRITY,
    VERIFIER_DETAIL_REGISTRY_FETCH_FAILED, VERIFIER_DETAIL_STREAM_ERROR,
    VERIFIER_DETAIL_TARBALL_DOWNLOAD_FAILED, VERIFIER_DETAIL_TARBALL_TOO_LARGE,
    VERIFIER_TARBALL_OPERATION_ERROR_TEMPLATE, render_template,
};
use crate::crypto::{hash_stream, integrity_short, verify_integrity};
use crate::types::{
    ArtifactStore, BuildCleanResultParams, BuildCompromisedResultParams,
    CollectTarballHashesParams, CompromisedSource, ComputeTarballFingerprintParams,
    DownloadTarballParams, DualHash, Evidence, FallbackDecision, FetchRegistryMetadataParams,
    HashStreamParams, HashTarballParams, NpmVersionMeta, PackageAndErrorTemplateArgsParams,
    PackageRef, ResolveDistIntegrityParams, SentinelError, StreamStorageMode,
    TarballOperationErrorParams, ToVerifiedTarballParams, UnverifiableReason,
    UnverifiableTemplateParams, UpdateFingerprintHasherParams, ValidateIntegrityParams, Verdict,
    VerifiedTarball, VerifyBeforeInstallParams, VerifyIntegrityParams, VerifyResult,
};
use sha2::{Digest, Sha256};
use std::io::Read;

use super::{
    Verifier, computed_sha512_integrity, create_unverifiable_from_template, no_tarball_result,
};
use crate::types::VerifyResultWithTarball;

const BYTES_PER_MIB_USIZE: usize = 1024 * 1024;
const MIB_TENTHS_FACTOR: usize = 10;
const TARBALL_PREFIX_DIR: &str = "package/";

type InstallCheckResult<T> = Result<T, String>;

fn format_tarball_size_mib_from_bytes(bytes: usize) -> String {
    let rounded_tenths = bytes
        .saturating_mul(MIB_TENTHS_FACTOR)
        .saturating_add(BYTES_PER_MIB_USIZE / 2)
        / BYTES_PER_MIB_USIZE;

    let whole = rounded_tenths / MIB_TENTHS_FACTOR;
    let decimal = rounded_tenths % MIB_TENTHS_FACTOR;

    format!("{whole}.{decimal}")
}

fn format_tarball_operation_error(params: &TarballOperationErrorParams<'_>) -> String {
    let TarballOperationErrorParams {
        operation_description,
        package_ref,
        error_description,
    } = params;

    let template_args = vec![
        operation_description.to_string(),
        package_ref.to_string(),
        error_description.to_string(),
    ];

    render_template(VERIFIER_TARBALL_OPERATION_ERROR_TEMPLATE, &template_args)
}

fn build_package_and_error_template_args(
    params: &PackageAndErrorTemplateArgsParams<'_>,
) -> Vec<String> {
    let PackageAndErrorTemplateArgsParams {
        package_ref,
        error_description,
    } = params;

    vec![package_ref.to_string(), error_description.to_string()]
}

fn evidence_with_source_url(source_url: String) -> Evidence {
    Evidence {
        source_url: Some(source_url),
        ..Evidence::empty()
    }
}

fn to_no_tarball_unverifiable_result(
    params: UnverifiableTemplateParams<'_>,
) -> VerifyResultWithTarball {
    let unverifiable = create_unverifiable_from_template(params);

    no_tarball_result(unverifiable)
}

async fn fetch_registry_metadata_or_unverifiable(
    params: FetchRegistryMetadataParams<'_>,
) -> Result<NpmVersionMeta, VerifyResultWithTarball> {
    let FetchRegistryMetadataParams {
        verifier,
        package_ref,
    } = params;

    match verifier.registry.fetch_version(package_ref).await {
        Ok(metadata) => Ok(metadata),
        Err(error) => {
            let registry_fetch_unverifiable_template_params = UnverifiableTemplateParams {
                reason: UnverifiableReason::RegistryOffline,
                package: package_ref,
                detail_template: VERIFIER_DETAIL_REGISTRY_FETCH_FAILED,
                template_args: {
                    let error_description = error.to_string();
                    let build_package_and_error_template_args_params =
                        PackageAndErrorTemplateArgsParams {
                            package_ref,
                            error_description: &error_description,
                        };

                    build_package_and_error_template_args(
                        &build_package_and_error_template_args_params,
                    )
                },
                evidence: Evidence::empty(),
            };

            Err(to_no_tarball_unverifiable_result(
                registry_fetch_unverifiable_template_params,
            ))
        }
    }
}

enum DistIntegrityResolution {
    Resolved(String),
    Unverifiable(Box<VerifyResultWithTarball>),
}

fn resolve_dist_integrity_or_unverifiable(
    params: &ResolveDistIntegrityParams<'_>,
) -> DistIntegrityResolution {
    let ResolveDistIntegrityParams {
        package_ref,
        registry_metadata,
    } = params;

    let Some(dist_integrity) = registry_metadata.dist.integrity.clone() else {
        let no_dist_integrity_unverifiable_template_params = UnverifiableTemplateParams {
            reason: UnverifiableReason::NoIntegrityField,
            package: package_ref,
            detail_template: VERIFIER_DETAIL_NO_DIST_INTEGRITY,
            template_args: vec![package_ref.to_string()],
            evidence: evidence_with_source_url(registry_metadata.dist.tarball.clone()),
        };

        return DistIntegrityResolution::Unverifiable(Box::new(to_no_tarball_unverifiable_result(
            no_dist_integrity_unverifiable_template_params,
        )));
    };

    DistIntegrityResolution::Resolved(dist_integrity)
}

async fn download_tarball_or_unverifiable(
    params: DownloadTarballParams<'_>,
) -> Result<reqwest::Response, VerifyResultWithTarball> {
    let DownloadTarballParams {
        verifier,
        package_ref,
        tarball_url,
    } = params;

    match verifier.registry.download_tarball(tarball_url).await {
        Ok(response) => Ok(response),
        Err(error) => {
            let tarball_download_unverifiable_template_params = UnverifiableTemplateParams {
                reason: UnverifiableReason::RegistryOffline,
                package: package_ref,
                detail_template: VERIFIER_DETAIL_TARBALL_DOWNLOAD_FAILED,
                template_args: {
                    let error_description = error.to_string();
                    let build_package_and_error_template_args_params =
                        PackageAndErrorTemplateArgsParams {
                            package_ref,
                            error_description: &error_description,
                        };

                    build_package_and_error_template_args(
                        &build_package_and_error_template_args_params,
                    )
                },
                evidence: evidence_with_source_url(tarball_url.to_string()),
            };

            Err(to_no_tarball_unverifiable_result(
                tarball_download_unverifiable_template_params,
            ))
        }
    }
}

async fn hash_tarball_or_unverifiable(
    params: HashTarballParams<'_>,
) -> Result<DualHash, VerifyResultWithTarball> {
    let HashTarballParams {
        verifier,
        package_ref,
        tarball_response,
        tarball_url,
    } = params;

    let StreamStorageMode {
        capture_buffer,
        spool_to_disk,
        ..
    } = resolve_stream_storage_mode(verifier);

    let hash_stream_params = HashStreamParams {
        stream: tarball_response.bytes_stream(),
        package: &package_ref.name,
        capture_buffer,
        spool_to_disk,
        inflight_counter: Some(verifier.memory_budget.clone_counter()),
    };

    match hash_stream(hash_stream_params).await {
        Ok(hashes) => Ok(hashes),
        Err(SentinelError::TarballTooLarge { bytes, .. }) => {
            let tarball_size_mib_text = format_tarball_size_mib_from_bytes(bytes);
            let tarball_too_large_unverifiable_template_params = UnverifiableTemplateParams {
                reason: UnverifiableReason::RegistryOffline,
                package: package_ref,
                detail_template: VERIFIER_DETAIL_TARBALL_TOO_LARGE,
                template_args: vec![package_ref.to_string(), tarball_size_mib_text],
                evidence: evidence_with_source_url(tarball_url.to_string()),
            };

            Err(to_no_tarball_unverifiable_result(
                tarball_too_large_unverifiable_template_params,
            ))
        }
        Err(error) => {
            let stream_error_unverifiable_template_params = UnverifiableTemplateParams {
                reason: UnverifiableReason::RegistryOffline,
                package: package_ref,
                detail_template: VERIFIER_DETAIL_STREAM_ERROR,
                template_args: {
                    let error_description = error.to_string();
                    let build_package_and_error_template_args_params =
                        PackageAndErrorTemplateArgsParams {
                            package_ref,
                            error_description: &error_description,
                        };

                    build_package_and_error_template_args(
                        &build_package_and_error_template_args_params,
                    )
                },
                evidence: Evidence::empty(),
            };

            Err(to_no_tarball_unverifiable_result(
                stream_error_unverifiable_template_params,
            ))
        }
    }
}

enum IntegrityValidationResolution {
    Validity(bool),
    Unverifiable(Box<VerifyResultWithTarball>),
}

fn validate_integrity_or_unverifiable(
    params: &ValidateIntegrityParams<'_>,
) -> IntegrityValidationResolution {
    let ValidateIntegrityParams {
        package_ref,
        dist_integrity,
        tarball_url,
        tarball_hashes,
    } = params;

    let verify_integrity_params = VerifyIntegrityParams {
        sha512_bytes: &tarball_hashes.sha512_bytes,
        integrity_field: dist_integrity,
    };

    match verify_integrity(verify_integrity_params) {
        Ok(result) => IntegrityValidationResolution::Validity(result),
        Err(error) => {
            cleanup_spool_payload(tarball_hashes.spool_path.as_ref());
            let invalid_integrity_unverifiable_template_params = UnverifiableTemplateParams {
                reason: UnverifiableReason::NoIntegrityField,
                package: package_ref,
                detail_template: VERIFIER_DETAIL_INVALID_INTEGRITY_FORMAT,
                template_args: {
                    let error_description = error.clone();
                    let build_package_and_error_template_args_params =
                        PackageAndErrorTemplateArgsParams {
                            package_ref,
                            error_description: &error_description,
                        };

                    build_package_and_error_template_args(
                        &build_package_and_error_template_args_params,
                    )
                },
                evidence: Evidence {
                    registry_integrity: Some(dist_integrity.to_string()),
                    source_url: Some(tarball_url.to_string()),
                    ..Evidence::empty()
                },
            };

            IntegrityValidationResolution::Unverifiable(Box::new(
                to_no_tarball_unverifiable_result(invalid_integrity_unverifiable_template_params),
            ))
        }
    }
}

fn normalize_tarball_relative_path(path: &str) -> Option<String> {
    let relative_path = path.trim_start_matches('/');
    let invalid_path =
        relative_path.is_empty() || relative_path.split('/').any(|segment| segment == "..");

    if invalid_path {
        return None;
    }

    Some(relative_path.to_string())
}

fn detect_common_tarball_root(raw_paths: &[String]) -> Option<String> {
    let first_path = raw_paths.first()?;
    let mut first_segments = first_path.split('/');
    let first_root = first_segments.next()?;
    let has_first_subpath = first_segments.next().is_some();

    if !has_first_subpath {
        return None;
    }

    let all_share_same_root = raw_paths.iter().all(|path| {
        let mut segments = path.split('/');
        let root = segments.next();
        let has_subpath = segments.next().is_some();

        root == Some(first_root) && has_subpath
    });

    all_share_same_root.then(|| first_root.to_string())
}

fn normalize_tarball_entry_path(path: &str, common_root: Option<&str>) -> Option<String> {
    let common_root_path = common_root.map(|root| format!("{root}/"));

    let relative_path = match common_root_path.as_deref() {
        Some(root_prefix) => path.strip_prefix(root_prefix).unwrap_or(path),
        None => path.strip_prefix(TARBALL_PREFIX_DIR).unwrap_or(path),
    };

    normalize_tarball_relative_path(relative_path)
}

fn collect_tarball_file_hashes(
    params: &CollectTarballHashesParams<'_>,
) -> InstallCheckResult<Vec<(String, Vec<u8>)>> {
    let CollectTarballHashesParams {
        tarball_bytes,
        package_ref,
    } = params;
    let tarball_bytes = *tarball_bytes;
    let package_ref = *package_ref;

    let decoder = flate2::read::GzDecoder::new(tarball_bytes);
    let mut archive = tar::Archive::new(decoder);
    let mut raw_files: Vec<(String, Vec<u8>)> = Vec::new();

    let mut entries = archive.entries().map_err(|error| {
        let error_description = error.to_string();
        let format_tarball_operation_error_params = TarballOperationErrorParams {
            operation_description: "failed to read tarball entries",
            package_ref,
            error_description: &error_description,
        };

        format_tarball_operation_error(&format_tarball_operation_error_params)
    })?;

    for entry_result in &mut entries {
        let mut entry = entry_result.map_err(|error| {
            let error_description = error.to_string();
            let format_tarball_operation_error_params = TarballOperationErrorParams {
                operation_description: "failed to parse tarball entry",
                package_ref,
                error_description: &error_description,
            };

            format_tarball_operation_error(&format_tarball_operation_error_params)
        })?;

        if !entry.header().entry_type().is_file() {
            continue;
        }

        let entry_path = entry.path().map_err(|error| {
            let error_description = error.to_string();
            let format_tarball_operation_error_params = TarballOperationErrorParams {
                operation_description: "failed to read tarball path",
                package_ref,
                error_description: &error_description,
            };

            format_tarball_operation_error(&format_tarball_operation_error_params)
        })?;

        let path_text = entry_path.to_string_lossy().replace('\\', "/");

        let Some(normalized_raw_path) = normalize_tarball_relative_path(&path_text) else {
            continue;
        };

        let mut bytes = Vec::new();

        entry.read_to_end(&mut bytes).map_err(|error| {
            let error_description = error.to_string();
            let format_tarball_operation_error_params = TarballOperationErrorParams {
                operation_description: "failed to read tarball file content",
                package_ref,
                error_description: &error_description,
            };

            format_tarball_operation_error(&format_tarball_operation_error_params)
        })?;

        let entry_hash = Sha256::digest(&bytes).to_vec();

        raw_files.push((normalized_raw_path, entry_hash));
    }

    let raw_paths: Vec<String> = raw_files.iter().map(|(path, _)| path.clone()).collect();
    let common_root = detect_common_tarball_root(&raw_paths);
    let files: Vec<(String, Vec<u8>)> = raw_files
        .into_iter()
        .filter_map(|(path, hash)| {
            let relative_path = normalize_tarball_entry_path(&path, common_root.as_deref())?;
            Some((relative_path, hash))
        })
        .collect();

    Ok(files)
}

fn update_fingerprint_hasher(params: UpdateFingerprintHasherParams<'_>) {
    let UpdateFingerprintHasherParams { hasher, files } = params;

    for (relative_path, content_hash) in files {
        hasher.update(relative_path.as_bytes());
        hasher.update([0]);
        hasher.update(content_hash);
        hasher.update([0]);
    }
}

fn encode_sha256_hex(digest: sha2::digest::Output<Sha256>) -> String {
    let mut encoded = String::with_capacity(digest.len() * 2);

    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut encoded, "{byte:02x}");
    }

    encoded
}

pub(crate) fn compute_tarball_fingerprint_bytes(
    tarball_bytes: &[u8],
    package_ref: &PackageRef,
) -> InstallCheckResult<String> {
    let compute_tarball_fingerprint_params = ComputeTarballFingerprintParams {
        tarball_bytes,
        package_ref,
    };

    compute_tarball_fingerprint(&compute_tarball_fingerprint_params)
}

fn compute_tarball_fingerprint(
    params: &ComputeTarballFingerprintParams<'_>,
) -> InstallCheckResult<String> {
    let ComputeTarballFingerprintParams {
        tarball_bytes,
        package_ref,
    } = params;

    let collect_tarball_file_hashes_params = CollectTarballHashesParams {
        tarball_bytes,
        package_ref,
    };
    let mut files = collect_tarball_file_hashes(&collect_tarball_file_hashes_params)?;

    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut hasher = Sha256::new();

    let update_fingerprint_hasher_params = UpdateFingerprintHasherParams {
        hasher: &mut hasher,
        files,
    };

    update_fingerprint_hasher(update_fingerprint_hasher_params);

    let digest = hasher.finalize();
    let encoded = encode_sha256_hex(digest);

    Ok(encoded)
}

fn resolve_stream_storage_mode(verifier: &Verifier) -> StreamStorageMode {
    let FallbackDecision { effective_mode, .. } = verifier
        .memory_budget
        .should_fallback_to_spool(verifier.artifact_store);

    let capture_buffer = matches!(effective_mode, ArtifactStore::Memory);
    let spool_to_disk = matches!(effective_mode, ArtifactStore::Spool);

    StreamStorageMode {
        effective_mode,
        capture_buffer,
        spool_to_disk,
    }
}

fn to_verified_tarball(params: ToVerifiedTarballParams) -> Option<VerifiedTarball> {
    let ToVerifiedTarballParams { buffer, spool_path } = params;

    match (buffer, spool_path) {
        (Some(bytes), _) => Some(VerifiedTarball::Memory(bytes)),
        (None, Some(path)) => Some(VerifiedTarball::Spool(path)),
        (None, None) => None,
    }
}

fn cleanup_spool_payload(spool_path: Option<&std::path::PathBuf>) {
    let Some(path) = spool_path else {
        return;
    };
    let _ = crate::verifier::artifact_cleanup::cleanup_artifact(path);

    crate::verifier::artifact_cleanup::unregister_artifact(path);
}

fn build_clean_verify_result(params: BuildCleanResultParams<'_>) -> VerifyResult {
    let BuildCleanResultParams {
        package_ref,
        dist_integrity,
        computed_integrity,
        tarball_url,
        tarball_fingerprint,
        tarball_bytes,
    } = params;

    let clean_install_template_args = vec![package_ref.to_string(), tarball_bytes.to_string()];
    let clean_detail = render_template(VERIFIER_DETAIL_CLEAN_INSTALL, &clean_install_template_args);

    VerifyResult {
        package: package_ref.clone(),
        verdict: Verdict::Clean,
        detail: clean_detail,
        evidence: Evidence {
            registry_integrity: Some(dist_integrity),
            computed_sha512: Some(computed_integrity),
            source_url: Some(tarball_url),
            ..Evidence::empty()
        },
        is_direct: false,
        direct_parent: None,
        tarball_fingerprint,
    }
}

fn build_compromised_verify_result(params: BuildCompromisedResultParams<'_>) -> VerifyResult {
    let BuildCompromisedResultParams {
        package_ref,
        dist_integrity,
        computed_integrity,
        tarball_url,
        tarball_fingerprint,
    } = params;

    let compromised_download_template_args = vec![
        package_ref.to_string(),
        integrity_short(&dist_integrity),
        integrity_short(&computed_integrity),
    ];
    let compromised_detail = render_template(
        VERIFIER_DETAIL_COMPROMISED_DOWNLOAD,
        &compromised_download_template_args,
    );

    VerifyResult {
        package: package_ref.clone(),
        verdict: Verdict::Compromised {
            expected: dist_integrity.clone(),
            actual: computed_integrity.clone(),
            source: CompromisedSource::DownloadVsRegistry,
        },
        detail: compromised_detail,
        evidence: Evidence {
            registry_integrity: Some(dist_integrity),
            computed_sha512: Some(computed_integrity),
            source_url: Some(tarball_url),
            ..Evidence::empty()
        },
        is_direct: false,
        direct_parent: None,
        tarball_fingerprint,
    }
}

impl Verifier {
    pub async fn verify_before_install(&self, package_ref: &PackageRef) -> VerifyResultWithTarball {
        let verify_before_install_params = VerifyBeforeInstallParams {
            verifier: self,
            package_ref,
        };

        verify_before_install_impl(verify_before_install_params).await
    }
}

#[allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::implicit_clone
)]
async fn verify_before_install_impl(
    params: VerifyBeforeInstallParams<'_>,
) -> VerifyResultWithTarball {
    let VerifyBeforeInstallParams {
        verifier,
        package_ref,
    } = params;

    let fetch_registry_metadata_or_unverifiable_params = FetchRegistryMetadataParams {
        verifier,
        package_ref,
    };

    let registry_metadata = match fetch_registry_metadata_or_unverifiable(
        fetch_registry_metadata_or_unverifiable_params,
    )
    .await
    {
        Ok(metadata) => metadata,
        Err(result) => return result,
    };

    let resolve_dist_integrity_or_unverifiable_params = ResolveDistIntegrityParams {
        package_ref,
        registry_metadata: &registry_metadata,
    };
    let dist_integrity = match resolve_dist_integrity_or_unverifiable(
        &resolve_dist_integrity_or_unverifiable_params,
    ) {
        DistIntegrityResolution::Resolved(integrity) => integrity,
        DistIntegrityResolution::Unverifiable(result) => return *result,
    };

    let tarball_url = registry_metadata.dist.tarball.clone();

    let download_tarball_or_unverifiable_params = DownloadTarballParams {
        verifier,
        package_ref,
        tarball_url: &tarball_url,
    };
    let tarball_response =
        match download_tarball_or_unverifiable(download_tarball_or_unverifiable_params).await {
            Ok(response) => response,
            Err(result) => return result,
        };

    let hash_tarball_or_unverifiable_params = HashTarballParams {
        verifier,
        package_ref,
        tarball_response,
        tarball_url: &tarball_url,
    };
    let tarball_hashes =
        match hash_tarball_or_unverifiable(hash_tarball_or_unverifiable_params).await {
            Ok(hashes) => hashes,
            Err(result) => return result,
        };

    let computed_integrity = computed_sha512_integrity(&tarball_hashes.sha512_bytes);
    let tarball_fingerprint = tarball_hashes.buffer.as_ref().and_then(|buffer| {
        let compute_tarball_fingerprint_params = ComputeTarballFingerprintParams {
            tarball_bytes: buffer,
            package_ref,
        };

        compute_tarball_fingerprint(&compute_tarball_fingerprint_params).ok()
    });

    let validate_integrity_or_unverifiable_params = ValidateIntegrityParams {
        package_ref,
        dist_integrity: &dist_integrity,
        tarball_url: &tarball_url,
        tarball_hashes: &tarball_hashes,
    };
    let integrity_valid =
        match validate_integrity_or_unverifiable(&validate_integrity_or_unverifiable_params) {
            IntegrityValidationResolution::Validity(is_valid) => is_valid,
            IntegrityValidationResolution::Unverifiable(result) => return *result,
        };

    if !integrity_valid {
        cleanup_spool_payload(tarball_hashes.spool_path.as_ref());

        let build_compromised_verify_result_params = BuildCompromisedResultParams {
            package_ref,
            dist_integrity,
            computed_integrity,
            tarball_url,
            tarball_fingerprint,
        };
        let compromised_result =
            build_compromised_verify_result(build_compromised_verify_result_params);

        verifier.cache.invalidate(package_ref);

        return VerifyResultWithTarball {
            result: compromised_result,
            tarball: None,
        };
    }

    let to_verified_tarball_params = ToVerifiedTarballParams {
        buffer: tarball_hashes.buffer,
        spool_path: tarball_hashes.spool_path,
    };
    let verified_tarball = to_verified_tarball(to_verified_tarball_params);
    let build_clean_verify_result_params = BuildCleanResultParams {
        package_ref,
        dist_integrity,
        computed_integrity,
        tarball_url,
        tarball_fingerprint,
        tarball_bytes: tarball_hashes.bytes,
    };
    let clean_result = build_clean_verify_result(build_clean_verify_result_params);

    verifier.cache.put(&clean_result);

    VerifyResultWithTarball {
        result: clean_result,
        tarball: verified_tarball,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "../../tests/internal/install_check_internal_tests.rs"]
mod tests;
