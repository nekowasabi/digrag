//! LRU Cache for API response caching
//!
//! Provides:
//! - In-memory LRU cache for summarization results
//! - Content hash-based cache keys
//! - TTL-based expiration
//! - Thread-safe access

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Cache entry with value and metadata
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    created_at: Instant,
    #[allow(dead_code)]
    access_count: u64,
}

/// LRU Cache with TTL support
pub struct LruCache<V: Clone> {
    entries: Arc<RwLock<HashMap<String, CacheEntry<V>>>>,
    max_size: usize,
    ttl: Duration,
    stats: Arc<RwLock<CacheStats>>,
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub expirations: u64,
}

impl CacheStats {
    /// Get hit rate as percentage
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }
}

impl<V: Clone> LruCache<V> {
    /// Create a new LRU cache
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            ttl,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Create with default settings (100 entries, 1 hour TTL)
    pub fn default_settings() -> Self {
        Self::new(100, Duration::from_secs(3600))
    }

    /// Generate cache key from content
    pub fn generate_key(content: &str, model: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hasher.update(model.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Get value from cache
    pub fn get(&self, key: &str) -> Option<V> {
        let now = Instant::now();

        // Try read lock first
        {
            let entries = self.entries.read().unwrap();
            if let Some(entry) = entries.get(key) {
                // Check TTL
                if now.duration_since(entry.created_at) > self.ttl {
                    // Entry expired, need to remove (will do with write lock)
                    drop(entries);
                    self.remove(key);
                    let mut stats = self.stats.write().unwrap();
                    stats.expirations += 1;
                    stats.misses += 1;
                    return None;
                }

                let mut stats = self.stats.write().unwrap();
                stats.hits += 1;
                return Some(entry.value.clone());
            }
        }

        let mut stats = self.stats.write().unwrap();
        stats.misses += 1;
        None
    }

    /// Insert value into cache
    pub fn insert(&self, key: String, value: V) {
        let mut entries = self.entries.write().unwrap();

        // Evict if at capacity
        if entries.len() >= self.max_size && !entries.contains_key(&key) {
            self.evict_oldest(&mut entries);
        }

        entries.insert(
            key,
            CacheEntry {
                value,
                created_at: Instant::now(),
                access_count: 0,
            },
        );
    }

    /// Remove entry from cache
    pub fn remove(&self, key: &str) -> Option<V> {
        let mut entries = self.entries.write().unwrap();
        entries.remove(key).map(|e| e.value)
    }

    /// Clear all entries
    pub fn clear(&self) {
        let mut entries = self.entries.write().unwrap();
        entries.clear();
    }

    /// Get current cache size
    pub fn len(&self) -> usize {
        let entries = self.entries.read().unwrap();
        entries.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let stats = self.stats.read().unwrap();
        stats.clone()
    }

    /// Clean expired entries
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        let mut entries = self.entries.write().unwrap();
        let mut stats = self.stats.write().unwrap();

        let expired_keys: Vec<String> = entries
            .iter()
            .filter(|(_, entry)| now.duration_since(entry.created_at) > self.ttl)
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            entries.remove(&key);
            stats.expirations += 1;
        }
    }

    fn evict_oldest(&self, entries: &mut HashMap<String, CacheEntry<V>>) {
        // Find oldest entry
        if let Some(oldest_key) = entries
            .iter()
            .min_by_key(|(_, entry)| entry.created_at)
            .map(|(key, _)| key.clone())
        {
            entries.remove(&oldest_key);
            let mut stats = self.stats.write().unwrap();
            stats.evictions += 1;
        }
    }
}

/// Cached summarization result
#[derive(Debug, Clone)]
pub struct CachedSummary {
    pub text: String,
    pub model: String,
    pub tokens_used: Option<usize>,
}

/// Summarization cache
pub type SummaryCache = LruCache<CachedSummary>;

impl SummaryCache {
    /// Create a summary cache with recommended settings
    pub fn for_summaries() -> Self {
        // 200 entries, 2 hour TTL
        Self::new(200, Duration::from_secs(7200))
    }

    /// Get cached summary for content
    pub fn get_summary(&self, content: &str, model: &str) -> Option<CachedSummary> {
        let key = Self::generate_key(content, model);
        self.get(&key)
    }

    /// Cache a summary
    pub fn cache_summary(&self, content: &str, model: &str, summary: CachedSummary) {
        let key = Self::generate_key(content, model);
        self.insert(key, summary);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic_operations() {
        let cache: LruCache<String> = LruCache::new(10, Duration::from_secs(60));

        cache.insert("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
        assert_eq!(cache.get("key2"), None);
    }

    #[test]
    fn test_cache_eviction() {
        let cache: LruCache<String> = LruCache::new(2, Duration::from_secs(60));

        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());
        cache.insert("key3".to_string(), "value3".to_string());

        // One entry should have been evicted
        assert_eq!(cache.len(), 2);

        // key3 should still exist
        assert!(cache.get("key3").is_some());
    }

    #[test]
    fn test_cache_ttl() {
        let cache: LruCache<String> = LruCache::new(10, Duration::from_millis(50));

        cache.insert("key1".to_string(), "value1".to_string());
        assert!(cache.get("key1").is_some());

        // Wait for TTL to expire
        std::thread::sleep(Duration::from_millis(100));

        assert!(cache.get("key1").is_none());
    }

    #[test]
    fn test_cache_stats() {
        let cache: LruCache<String> = LruCache::new(10, Duration::from_secs(60));

        cache.insert("key1".to_string(), "value1".to_string());
        cache.get("key1"); // hit
        cache.get("key1"); // hit
        cache.get("key2"); // miss

        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate() - 66.67).abs() < 1.0);
    }

    #[test]
    fn test_generate_key() {
        let key1 = LruCache::<String>::generate_key("content", "model");
        let key2 = LruCache::<String>::generate_key("content", "model");
        let key3 = LruCache::<String>::generate_key("other", "model");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_summary_cache() {
        let cache = SummaryCache::for_summaries();

        let summary = CachedSummary {
            text: "This is a summary".to_string(),
            model: "test-model".to_string(),
            tokens_used: Some(100),
        };

        cache.cache_summary("test content", "test-model", summary.clone());

        let retrieved = cache.get_summary("test content", "test-model");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().text, "This is a summary");
    }

    #[test]
    fn test_cache_clear() {
        let cache: LruCache<String> = LruCache::new(10, Duration::from_secs(60));

        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());

        assert_eq!(cache.len(), 2);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cleanup_expired() {
        let cache: LruCache<String> = LruCache::new(10, Duration::from_millis(50));

        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());

        std::thread::sleep(Duration::from_millis(100));

        cache.cleanup_expired();

        assert_eq!(cache.len(), 0);
    }
}
