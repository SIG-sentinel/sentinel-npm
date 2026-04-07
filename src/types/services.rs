use std::path::PathBuf;
use std::time::Duration;

pub struct Verifier {
    pub(crate) registry: NpmRegistry,
    pub(crate) cache: LocalCache,
}

pub struct NpmRegistry {
    pub(crate) client: reqwest::Client,
    pub(crate) timeout: Duration,
}

pub struct LocalCache {
    pub(crate) db_path: PathBuf,
}

pub struct ProgressBarConfig {
    pub length: usize,
    pub message: &'static str,
    pub template: &'static str,
}
