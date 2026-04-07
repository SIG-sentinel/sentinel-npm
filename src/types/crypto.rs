use bytes::Bytes;
use futures_util::Stream;

pub struct DualHash {
    pub sha512_bytes: Vec<u8>,
    pub bytes: usize,
    pub buffer: Vec<u8>,
}

pub struct HashStreamParams<'a, S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Send,
{
    pub stream: S,
    pub package: &'a str,
}

pub struct VerifyIntegrityParams<'a> {
    pub sha512_bytes: &'a [u8],
    pub integrity_field: &'a str,
}
