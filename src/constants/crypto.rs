pub const INTEGRITY_PREFIX_SHA512: &str = "sha512-";
pub const INTEGRITY_SHORT_LEN: usize = 24;
pub const BYTES_PER_MIB: f64 = 1024.0 * 1024.0;

pub const CRYPTO_ERR_BAD_INTEGRITY_PREFIX_TEMPLATE: &str = "expected '{}' prefix, got '{}'";
pub const CRYPTO_ERR_INVALID_BASE64_TEMPLATE: &str = "invalid base64 in integrity field: {}";
