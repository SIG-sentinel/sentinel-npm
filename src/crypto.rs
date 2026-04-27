use base64::{Engine, engine::general_purpose::STANDARD as B64};
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use sha2::{Digest, Sha512};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use subtle::ConstantTimeEq;

use crate::constants::{
    CRYPTO_ERR_BAD_INTEGRITY_PREFIX_TEMPLATE, CRYPTO_ERR_INVALID_BASE64_TEMPLATE,
    INTEGRITY_PREFIX_SHA512, INTEGRITY_SHORT_LEN, render_template,
};
use crate::types::{HashStreamParams, SentinelError, VerifyIntegrityParams};
use crate::verifier::artifact_cleanup::{cleanup_artifact, register_artifact, unregister_artifact};

const MAX_TARBALL_MIB: usize = 50;
pub const MAX_TARBALL_BYTES: usize = MAX_TARBALL_MIB * 1024 * 1024;
const EMPTY_TRACKED_BUFFER_BYTES: usize = 0;
const CT_EQUAL_TRUE: u8 = 1;

fn release_inflight_bytes(
    inflight_counter: Option<&std::sync::Arc<std::sync::atomic::AtomicUsize>>,
    tracked_buffer_bytes: usize,
) {
    let has_tracked_bytes = tracked_buffer_bytes > EMPTY_TRACKED_BUFFER_BYTES;

    if has_tracked_bytes && let Some(counter) = inflight_counter {
        counter.fetch_sub(tracked_buffer_bytes, Ordering::SeqCst);
    }
}

pub async fn hash_stream(
    params: HashStreamParams<'_, impl Stream<Item = Result<Bytes, reqwest::Error>> + Send>,
) -> Result<DualHash, SentinelError> {
    let HashStreamParams {
        stream,
        package,
        capture_buffer,
        spool_to_disk,
        inflight_counter,
    } = params;

    let mut sha512 = Sha512::new();
    let mut total = 0usize;
    let mut captured_buffer = capture_buffer.then(Vec::new);
    let mut tracked_buffer_bytes = 0usize;

    let mut spool_file = spool_to_disk
        .then(|| create_spool_file(package))
        .transpose()?;

    futures_util::pin_mut!(stream);

    macro_rules! bail {
        ($err:expr) => {{
            release_inflight_bytes(inflight_counter.as_ref(), tracked_buffer_bytes);
            cleanup_spool_on_error(spool_file.as_ref());

            return Err($err);
        }};
    }

    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(bytes) => bytes,
            Err(error) => bail!(SentinelError::Http(error.to_string())),
        };

        let chunk_len = chunk.len();

        total += chunk_len;

        let exceeds_size_limit = total > MAX_TARBALL_BYTES;

        if exceeds_size_limit {
            bail!(SentinelError::TarballTooLarge {
                package: package.to_string(),
                bytes: total,
            });
        }

        sha512.update(&chunk);

        if let Some((_, file)) = &mut spool_file
            && let Err(error) = file.write_all(&chunk)
        {
            bail!(error.into());
        }

        if let Some(buffer) = &mut captured_buffer {
            buffer.extend_from_slice(&chunk);
            tracked_buffer_bytes += chunk_len;

            inflight_counter.as_ref().inspect(|counter| {
                counter.fetch_add(chunk_len, Ordering::SeqCst);
            });
        }
    }

    let spool_path = finalize_spool_file(spool_file)?;

    release_inflight_bytes(inflight_counter.as_ref(), tracked_buffer_bytes);

    let dual_hash = DualHash {
        sha512_bytes: sha512.finalize().to_vec(),
        bytes: total,
        buffer: captured_buffer,
        spool_path,
    };

    Ok(dual_hash)
}

fn cleanup_spool_on_error(spool_file: Option<&(PathBuf, std::fs::File)>) {
    if let Some((spool_path, _)) = spool_file {
        let _ = cleanup_artifact(spool_path);

        unregister_artifact(spool_path);
    }
}

fn finalize_spool_file(
    spool_file: Option<(PathBuf, std::fs::File)>,
) -> Result<Option<PathBuf>, SentinelError> {
    let Some((spool_path, mut file)) = spool_file else {
        return Ok(None);
    };

    if let Err(error) = file.flush() {
        let _ = cleanup_artifact(&spool_path);

        unregister_artifact(&spool_path);

        return Err(error.into());
    }

    if let Err(error) = file.sync_all() {
        let _ = cleanup_artifact(&spool_path);

        unregister_artifact(&spool_path);

        return Err(error.into());
    }

    Ok(Some(spool_path))
}

fn create_spool_file(package: &str) -> Result<(PathBuf, std::fs::File), SentinelError> {
    let process_id = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    let file_name = format!(
        "sentinel-spool-{process_id}-{nanos}-{}.tmp",
        package.replace(['/', '@'], "_")
    );
    let path = std::env::temp_dir().join(file_name);

    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    register_artifact(path.clone());

    Ok((path, file))
}

#[cfg(test)]
#[path = "../tests/internal/crypto_internal_tests.rs"]
mod tests;

pub use crate::types::DualHash;

pub fn normalize_integrity(integrity_field: &str) -> &str {
    let mut tokens = integrity_field.split_whitespace().peekable();
    let first_token = tokens.peek().copied();

    tokens
        .find(|token| token.starts_with(INTEGRITY_PREFIX_SHA512))
        .or(first_token)
        .unwrap_or_else(|| integrity_field.trim())
}

pub fn verify_integrity(params: VerifyIntegrityParams<'_>) -> Result<bool, String> {
    let VerifyIntegrityParams {
        sha512_bytes,
        integrity_field,
    } = params;

    let normalized_integrity = normalize_integrity(integrity_field);

    let encoded = normalized_integrity
        .strip_prefix(INTEGRITY_PREFIX_SHA512)
        .ok_or_else(|| {
            let bad_prefix_template_args = vec![
                INTEGRITY_PREFIX_SHA512.to_string(),
                normalized_integrity.to_string(),
            ];

            render_template(
                CRYPTO_ERR_BAD_INTEGRITY_PREFIX_TEMPLATE,
                &bad_prefix_template_args,
            )
        })?;

    let expected = B64.decode(encoded).map_err(|error| {
        let invalid_base64_template_args = vec![error.to_string()];

        render_template(
            CRYPTO_ERR_INVALID_BASE64_TEMPLATE,
            &invalid_base64_template_args,
        )
    })?;

    Ok(expected.ct_eq(sha512_bytes).unwrap_u8() == CT_EQUAL_TRUE)
}

pub fn integrity_short(integrity: &str) -> String {
    let chars: String = integrity.chars().take(INTEGRITY_SHORT_LEN).collect();

    format!("{chars}…")
}
