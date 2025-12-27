//! Query rewrite cache
//!
//! SQLite-based cache for query rewrites with TTL.

use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Default TTL for cache entries (24 hours)
const DEFAULT_TTL_SECS: u64 = 24 * 60 * 60;

/// Query rewrite cache
pub struct RewriteCache {
    conn: Connection,
    ttl: Duration,
}

impl RewriteCache {
    /// Create a new cache with the given path
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS rewrites (
                query TEXT PRIMARY KEY,
                rewritten TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(Self {
            conn,
            ttl: Duration::from_secs(DEFAULT_TTL_SECS),
        })
    }

    /// Create an in-memory cache (for testing)
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS rewrites (
                query TEXT PRIMARY KEY,
                rewritten TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(Self {
            conn,
            ttl: Duration::from_secs(DEFAULT_TTL_SECS),
        })
    }

    /// Set custom TTL
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Get a cached rewrite
    pub fn get(&self, query: &str) -> Result<Option<String>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let min_time = now - self.ttl.as_secs() as i64;

        let result: Option<String> = self
            .conn
            .query_row(
                "SELECT rewritten FROM rewrites WHERE query = ? AND created_at > ?",
                params![query, min_time],
                |row| row.get(0),
            )
            .ok();

        Ok(result)
    }

    /// Set a cached rewrite
    pub fn set(&self, query: &str, rewritten: &str) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        self.conn.execute(
            "INSERT OR REPLACE INTO rewrites (query, rewritten, created_at) VALUES (?, ?, ?)",
            params![query, rewritten, now],
        )?;

        Ok(())
    }

    /// Remove expired entries
    pub fn cleanup(&self) -> Result<usize> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let min_time = now - self.ttl.as_secs() as i64;

        let deleted = self.conn.execute(
            "DELETE FROM rewrites WHERE created_at <= ?",
            params![min_time],
        )?;

        Ok(deleted)
    }

    /// Get cache size
    pub fn size(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM rewrites", [], |row| row.get(0))?;

        Ok(count as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_creation() {
        let cache = RewriteCache::in_memory();
        assert!(cache.is_ok());
    }

    #[test]
    fn test_cache_set_and_get() {
        let cache = RewriteCache::in_memory().unwrap();

        cache.set("test query", "rewritten query").unwrap();
        let result = cache.get("test query").unwrap();

        assert_eq!(result, Some("rewritten query".to_string()));
    }

    #[test]
    fn test_cache_miss() {
        let cache = RewriteCache::in_memory().unwrap();
        let result = cache.get("nonexistent").unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_cache_ttl() {
        let cache = RewriteCache::in_memory()
            .unwrap()
            .with_ttl(Duration::from_secs(0)); // Immediate expiration

        cache.set("test query", "rewritten").unwrap();

        // Should be expired
        std::thread::sleep(Duration::from_millis(10));
        let result = cache.get("test query").unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_cache_size() {
        let cache = RewriteCache::in_memory().unwrap();

        assert_eq!(cache.size().unwrap(), 0);

        cache.set("query1", "rewritten1").unwrap();
        cache.set("query2", "rewritten2").unwrap();

        assert_eq!(cache.size().unwrap(), 2);
    }

    #[test]
    fn test_cache_cleanup() {
        let cache = RewriteCache::in_memory()
            .unwrap()
            .with_ttl(Duration::from_secs(0));

        cache.set("query", "rewritten").unwrap();

        std::thread::sleep(Duration::from_millis(10));
        let deleted = cache.cleanup().unwrap();

        assert_eq!(deleted, 1);
        assert_eq!(cache.size().unwrap(), 0);
    }

    // TODO: Add more tests in Process 11
}
