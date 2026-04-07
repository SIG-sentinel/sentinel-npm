use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, params};

use crate::constants::{
    SENTINEL_CACHE_DB_FILE, SENTINEL_HOME_DIR, SQL_DELETE_CACHE_BY_KEY, SQL_INIT_CACHE_SCHEMA,
    SQL_SELECT_CACHE_BY_KEY, SQL_UPSERT_CACHE, UNVERIFIABLE_CACHE_TTL_SECS,
};
use crate::types::{PackageRef, SentinelError, Verdict, VerifyResult};

pub use crate::types::LocalCache;

impl LocalCache {
    pub fn open(cache_dir: Option<&str>) -> Result<Self, SentinelError> {
        let dir = match cache_dir {
            Some(cache_directory) => PathBuf::from(cache_directory),
            None => dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(SENTINEL_HOME_DIR),
        };

        std::fs::create_dir_all(&dir)?;

        let db_path = dir.join(SENTINEL_CACHE_DB_FILE);
        let conn = Connection::open(&db_path).map_err(std::io::Error::other)?;

        conn.execute_batch(SQL_INIT_CACHE_SCHEMA)
            .map_err(std::io::Error::other)?;

        Ok(Self { db_path })
    }

    fn conn(&self) -> Result<Connection, SentinelError> {
        Connection::open(&self.db_path)
            .map_err(|error| SentinelError::Io(std::io::Error::other(error)))
    }

    fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    pub fn get(&self, package_ref: &PackageRef) -> Option<VerifyResult> {
        let connection = self.conn().ok()?;
        let cache_key = package_ref.to_string();
        let current_time = Self::now();

        let row: Option<(String, i64, Option<i64>)> = connection
            .query_row(SQL_SELECT_CACHE_BY_KEY, params![cache_key], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .ok();

        match row {
            None => None,
            Some((result_json, cached_at, ttl_secs)) => {
                let is_expired = ttl_secs
                    .map(|ttl_secs_value| current_time - cached_at > ttl_secs_value)
                    .unwrap_or(false);

                match is_expired {
                    true => {
                        connection
                            .execute(SQL_DELETE_CACHE_BY_KEY, params![cache_key])
                            .ok();
                        None
                    }
                    false => serde_json::from_str(&result_json).ok(),
                }
            }
        }
    }

    pub fn put(&self, result: &VerifyResult) {
        if !matches!(result.verdict, Verdict::Compromised { .. }) {
            let ttl_secs = match &result.verdict {
                Verdict::Clean => None,
                Verdict::Unverifiable { .. } => Some(UNVERIFIABLE_CACHE_TTL_SECS),
                Verdict::Compromised { .. } => None,
            };

            let connection = self.conn().ok();
            let result_json = serde_json::to_string(result).ok();

            if let (Some(connection), Some(result_json)) = (connection, result_json) {
                let cache_key = result.package.to_string();

                connection
                    .execute(
                        SQL_UPSERT_CACHE,
                        params![cache_key, result_json, Self::now(), ttl_secs],
                    )
                    .ok();
            }
        }
    }

    pub fn invalidate(&self, package_ref: &PackageRef) {
        if let Ok(connection) = self.conn() {
            connection
                .execute(SQL_DELETE_CACHE_BY_KEY, params![package_ref.to_string()])
                .ok();
        }
    }
}
