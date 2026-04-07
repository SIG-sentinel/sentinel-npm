use base64::{Engine, engine::general_purpose::STANDARD as B64};
use bytes::Bytes;
use futures_util::Stream;
use sha2::{Digest, Sha512};
use subtle::ConstantTimeEq;

use crate::constants::{
    BYTES_PER_MIB, CRYPTO_ERR_BAD_INTEGRITY_PREFIX_TEMPLATE, CRYPTO_ERR_INVALID_BASE64_TEMPLATE,
    INTEGRITY_PREFIX_SHA512, INTEGRITY_SHORT_LEN, render_template,
};
use crate::types::{HashStreamParams, SentinelError, VerifyIntegrityParams};

pub const MAX_TARBALL_BYTES: usize = (50.0 * BYTES_PER_MIB) as usize;

pub async fn hash_stream(
    params: HashStreamParams<'_, impl Stream<Item = Result<Bytes, reqwest::Error>> + Send>,
) -> Result<DualHash, SentinelError> {
    let HashStreamParams { stream, package } = params;

    use futures_util::StreamExt;

    let mut sha512 = Sha512::new();
    let mut total = 0usize;
    let mut buf = Vec::new();

    futures_util::pin_mut!(stream);

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| SentinelError::Http(error.to_string()))?;

        total += chunk.len();

        if total > MAX_TARBALL_BYTES {
            Err(SentinelError::TarballTooLarge {
                package: package.to_string(),
                bytes: total,
            })?;
        }

        sha512.update(&chunk);
        buf.extend_from_slice(&chunk);
    }

    Ok(DualHash {
        sha512_bytes: sha512.finalize().to_vec(),
        bytes: total,
        buffer: buf,
    })
}

pub use crate::types::DualHash;

pub fn verify_integrity(params: VerifyIntegrityParams<'_>) -> Result<bool, String> {
    let VerifyIntegrityParams {
        sha512_bytes,
        integrity_field,
    } = params;

    let encoded = integrity_field
        .strip_prefix(INTEGRITY_PREFIX_SHA512)
        .ok_or_else(|| {
            render_template(
                CRYPTO_ERR_BAD_INTEGRITY_PREFIX_TEMPLATE,
                &[
                    INTEGRITY_PREFIX_SHA512.to_string(),
                    integrity_field.to_string(),
                ],
            )
        })?;

    let expected = B64.decode(encoded).map_err(|error| {
        render_template(CRYPTO_ERR_INVALID_BASE64_TEMPLATE, &[error.to_string()])
    })?;

    Ok(expected.ct_eq(sha512_bytes).unwrap_u8() == 1)
}

pub fn integrity_short(integrity: &str) -> String {
    let chars: String = integrity.chars().take(INTEGRITY_SHORT_LEN).collect();

    format!("{chars}…")
}
