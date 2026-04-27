use bytes::Bytes;
use futures_util::Stream;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

pub struct DualHash {
    pub sha512_bytes: Vec<u8>,
    pub bytes: usize,
    pub buffer: Option<Vec<u8>>,
    pub spool_path: Option<PathBuf>,
}

pub struct HashStreamParams<'a, S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Send,
{
    pub stream: S,
    pub package: &'a str,
    pub capture_buffer: bool,
    pub spool_to_disk: bool,
    pub inflight_counter: Option<Arc<AtomicUsize>>,
}

#[derive(Clone, Copy)]
pub struct VerifyIntegrityParams<'a> {
    pub sha512_bytes: &'a [u8],
    pub integrity_field: &'a str,
}
