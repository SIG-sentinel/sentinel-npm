#![allow(dead_code)]

use base64::{Engine, engine::general_purpose::STANDARD as B64};
use sha2::{Digest, Sha512};

use super::constants::{SHA256_PREFIX, SHA512_PREFIX};
use super::types::IntegrityFixture;

pub fn integrity_fixture(input: &[u8]) -> IntegrityFixture {
    let bytes = Sha512::digest(input).to_vec();
    let integrity = format!("{SHA512_PREFIX}{}", B64.encode(&bytes));
    IntegrityFixture { bytes, integrity }
}

pub fn wrong_prefix_integrity(input: &[u8]) -> IntegrityFixture {
    let bytes = Sha512::digest(input).to_vec();
    let integrity = format!("{SHA256_PREFIX}{}", B64.encode(&bytes));
    IntegrityFixture { bytes, integrity }
}

pub fn hex_sha512(input: &[u8]) -> String {
    let bytes = Sha512::digest(input).to_vec();
    bytes.iter().map(|value| format!("{value:02x}")).collect()
}
