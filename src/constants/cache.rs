pub const UNVERIFIABLE_CACHE_TTL_SECS: i64 = 300;

pub const SQL_DELETE_CACHE_BY_KEY: &str = "DELETE FROM cache WHERE key = ?1";
pub const SQL_SELECT_CACHE_BY_KEY: &str =
    "SELECT result_json, cached_at, ttl_secs FROM cache WHERE key = ?1";
pub const SQL_UPSERT_CACHE: &str =
    "INSERT OR REPLACE INTO cache (key, result_json, cached_at, ttl_secs) VALUES (?1, ?2, ?3, ?4)";
pub const SQL_INIT_CACHE_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS cache (
    key        TEXT PRIMARY KEY,
    result_json TEXT NOT NULL,
    cached_at  INTEGER NOT NULL,
    ttl_secs   INTEGER
);
CREATE INDEX IF NOT EXISTS idx_cached_at ON cache(cached_at);";
