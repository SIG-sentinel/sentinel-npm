use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, params};

use crate::constants::{
    CLEAN_CACHE_TTL_SECS, SENTINEL_CACHE_DB_FILE, SENTINEL_HOME_DIR, SQL_DELETE_CACHE_BY_KEY,
    SQL_INIT_CACHE_SCHEMA, SQL_SELECT_CACHE_BY_KEY, SQL_UPSERT_CACHE, UNVERIFIABLE_CACHE_TTL_SECS,
};
use crate::types::{PackageRef, SentinelError, Verdict, VerifyResult};

pub use crate::types::LocalCache;

impl LocalCache {
    pub fn open(cache_dir: Option<&str>) -> Result<Self, SentinelError> {
        let cache_directory_path = match cache_dir {
            Some(cache_directory) => PathBuf::from(cache_directory),
            None => dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(SENTINEL_HOME_DIR),
        };

        std::fs::create_dir_all(&cache_directory_path)?;

        let db_path = cache_directory_path.join(SENTINEL_CACHE_DB_FILE);
        let database_connection = Connection::open(&db_path).map_err(std::io::Error::other)?;

        database_connection
            .execute_batch(SQL_INIT_CACHE_SCHEMA)
            .map_err(std::io::Error::other)?;

        Ok(Self { db_path })
    }

    fn open_connection(&self) -> Result<Connection, SentinelError> {
        Connection::open(&self.db_path)
            .map_err(|error| SentinelError::Io(std::io::Error::other(error)))
    }

    fn now() -> i64 {
        let seconds_since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        i64::try_from(seconds_since_epoch).unwrap_or(i64::MAX)
    }

    pub fn get(&self, package_ref: &PackageRef) -> Option<VerifyResult> {
        let connection = self.open_connection().ok()?;
        let cache_key = package_ref.to_string();
        let current_time = Self::now();

        let (result_json, cached_at, ttl_secs): (String, i64, Option<i64>) = connection
            .query_row(SQL_SELECT_CACHE_BY_KEY, params![cache_key], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .ok()?;

        let is_expired = ttl_secs.is_some_and(|ttl| current_time - cached_at > ttl);

        if is_expired {
            connection
                .execute(SQL_DELETE_CACHE_BY_KEY, params![cache_key])
                .ok();

            return None;
        }

        serde_json::from_str(&result_json).ok()
    }

    pub fn put(&self, result: &VerifyResult) {
        let ttl_secs = match &result.verdict {
            Verdict::Clean => Some(CLEAN_CACHE_TTL_SECS),
            Verdict::Unverifiable { .. } => Some(UNVERIFIABLE_CACHE_TTL_SECS),
            Verdict::Compromised { .. } => return,
        };

        let Some(connection) = self.open_connection().ok() else {
            return;
        };
        let Some(result_json) = serde_json::to_string(result).ok() else {
            return;
        };

        let cache_key = result.package.to_string();

        connection
            .execute(
                SQL_UPSERT_CACHE,
                params![cache_key, result_json, Self::now(), ttl_secs],
            )
            .ok();
    }

    pub fn invalidate(&self, package_ref: &PackageRef) {
        if let Ok(connection) = self.open_connection() {
            connection
                .execute(SQL_DELETE_CACHE_BY_KEY, params![package_ref.to_string()])
                .ok();
        }
    }
}
