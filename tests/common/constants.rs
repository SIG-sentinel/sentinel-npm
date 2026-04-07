#![allow(dead_code)]

pub const EMPTY_INPUT: &[u8] = b"";
pub const SAMPLE_DATA: &[u8] = b"data";
pub const ORIGINAL_TARBALL: &[u8] = b"original tarball";
pub const TAMPERED_TARBALL: &[u8] = b"tampered tarball with extra content";
pub const TEST_TARBALL_CONTENT: &[u8] = b"fake tarball content for testing";

pub const SHA512_PREFIX: &str = "sha512-";
pub const SHA256_PREFIX: &str = "sha256-";
pub const INVALID_BASE64_INTEGRITY: &str = "sha512-!!!not_valid_base64!!!";

pub const SHA256_EMPTY_HEX: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
pub const SHA512_EMPTY_HEX: &str = "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e";
