use thiserror::Error;

#[derive(Debug, Error)]
pub enum SentinelError {
    #[error(
        "{package}@{version} has no integrity field in npm registry (package predates integrity)"
    )]
    NoIntegrity { package: String, version: String },

    #[error("npm registry unreachable: {0}")]
    RegistryUnreachable(String),

    #[error("timeout fetching {package}@{version} from npm registry ({ms}ms)")]
    RegistryTimeout {
        package: String,
        version: String,
        ms: u64,
    },

    #[error("package.json not found in {path}")]
    PackageJsonNotFound { path: String },

    #[error("lockfile not found — run npm install first, then sentinel check")]
    LockfileNotFound,

    #[error("lockfile parse error: {0}")]
    LockfileParse(String),

    #[error("tarball too large for {package}: {bytes} bytes (limit: 50MB)")]
    TarballTooLarge { package: String, bytes: usize },

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("http: {0}")]
    Http(String),
}

impl From<reqwest::Error> for SentinelError {
    fn from(error: reqwest::Error) -> Self {
        match (error.is_timeout(), error.is_connect()) {
            (true, _) => Self::RegistryUnreachable(format!("timeout: {error}")),
            (_, true) => Self::RegistryUnreachable(format!("connection refused: {error}")),
            _ => Self::Http(error.to_string()),
        }
    }
}
