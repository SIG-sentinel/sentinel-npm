use super::{LockfileEntry, VerifyResult};

pub struct CacheMatchParams<'a> {
    pub entry: &'a LockfileEntry,
    pub cached_result: &'a VerifyResult,
}

pub struct VerifierNewParams<'a> {
    pub timeout_ms: u64,
    pub cache_dir: Option<&'a str>,
}
