//! Cache Integration Tests (Process 200: TDD)
//!
//! Tests for LRU cache integration with ContentSummarizer

use digrag::extract::cache::{CacheStats, CachedSummary, LruCache, SummaryCache};
use digrag::extract::{ContentStats, ExtractedContent};
use std::time::Duration;

// =============================================================================
// LRU Cache Basic Tests
// =============================================================================

#[test]
fn test_lru_cache_new() {
    let cache: LruCache<String> = LruCache::new(100, Duration::from_secs(3600));
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

#[test]
fn test_lru_cache_insert_and_get() {
    let cache: LruCache<String> = LruCache::new(100, Duration::from_secs(3600));
    cache.insert("key1".to_string(), "value1".to_string());

    assert_eq!(cache.get("key1"), Some("value1".to_string()));
    assert_eq!(cache.get("nonexistent"), None);
}

#[test]
fn test_lru_cache_eviction_at_capacity() {
    let cache: LruCache<String> = LruCache::new(2, Duration::from_secs(3600));

    cache.insert("key1".to_string(), "value1".to_string());
    cache.insert("key2".to_string(), "value2".to_string());
    cache.insert("key3".to_string(), "value3".to_string());

    // Should have evicted oldest entry
    assert_eq!(cache.len(), 2);
    assert!(cache.get("key3").is_some());

    let stats = cache.stats();
    assert_eq!(stats.evictions, 1);
}

#[test]
fn test_lru_cache_ttl_expiration() {
    let cache: LruCache<String> = LruCache::new(100, Duration::from_millis(50));

    cache.insert("key1".to_string(), "value1".to_string());
    assert!(cache.get("key1").is_some());

    // Wait for TTL to expire
    std::thread::sleep(Duration::from_millis(100));

    assert!(cache.get("key1").is_none());

    let stats = cache.stats();
    assert!(stats.expirations > 0);
}

#[test]
fn test_lru_cache_stats_hit_rate() {
    let cache: LruCache<String> = LruCache::new(100, Duration::from_secs(3600));

    cache.insert("key1".to_string(), "value1".to_string());

    cache.get("key1"); // hit
    cache.get("key1"); // hit
    cache.get("key2"); // miss
    cache.get("key2"); // miss

    let stats = cache.stats();
    assert_eq!(stats.hits, 2);
    assert_eq!(stats.misses, 2);
    assert!((stats.hit_rate() - 50.0).abs() < 1.0);
}

#[test]
fn test_lru_cache_generate_key_deterministic() {
    let key1 = LruCache::<String>::generate_key("content", "model");
    let key2 = LruCache::<String>::generate_key("content", "model");
    let key3 = LruCache::<String>::generate_key("different", "model");

    assert_eq!(key1, key2);
    assert_ne!(key1, key3);
}

#[test]
fn test_lru_cache_cleanup_expired() {
    let cache: LruCache<String> = LruCache::new(100, Duration::from_millis(50));

    cache.insert("key1".to_string(), "value1".to_string());
    cache.insert("key2".to_string(), "value2".to_string());

    assert_eq!(cache.len(), 2);

    std::thread::sleep(Duration::from_millis(100));
    cache.cleanup_expired();

    assert_eq!(cache.len(), 0);
}

// =============================================================================
// SummaryCache Tests
// =============================================================================

#[test]
fn test_summary_cache_for_summaries() {
    let cache = SummaryCache::for_summaries();
    assert!(cache.is_empty());
}

#[test]
fn test_summary_cache_cache_and_retrieve() {
    let cache = SummaryCache::for_summaries();

    let summary = CachedSummary {
        text: "This is a cached summary".to_string(),
        model: "cerebras/llama-3.3-70b".to_string(),
        tokens_used: Some(150),
    };

    cache.cache_summary(
        "original content",
        "cerebras/llama-3.3-70b",
        summary.clone(),
    );

    let retrieved = cache.get_summary("original content", "cerebras/llama-3.3-70b");
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.text, "This is a cached summary");
    assert_eq!(retrieved.model, "cerebras/llama-3.3-70b");
    assert_eq!(retrieved.tokens_used, Some(150));
}

#[test]
fn test_summary_cache_different_models() {
    let cache = SummaryCache::for_summaries();

    let summary1 = CachedSummary {
        text: "Summary from model 1".to_string(),
        model: "model1".to_string(),
        tokens_used: Some(100),
    };

    let summary2 = CachedSummary {
        text: "Summary from model 2".to_string(),
        model: "model2".to_string(),
        tokens_used: Some(200),
    };

    cache.cache_summary("same content", "model1", summary1);
    cache.cache_summary("same content", "model2", summary2);

    // Different models should have different cache entries
    let retrieved1 = cache.get_summary("same content", "model1");
    let retrieved2 = cache.get_summary("same content", "model2");

    assert!(retrieved1.is_some());
    assert!(retrieved2.is_some());
    assert_eq!(retrieved1.unwrap().text, "Summary from model 1");
    assert_eq!(retrieved2.unwrap().text, "Summary from model 2");
}

#[test]
fn test_summary_cache_miss() {
    let cache = SummaryCache::for_summaries();

    let retrieved = cache.get_summary("uncached content", "any-model");
    assert!(retrieved.is_none());
}

// =============================================================================
// CachedSummary Tests
// =============================================================================

#[test]
fn test_cached_summary_clone() {
    let summary = CachedSummary {
        text: "Test summary".to_string(),
        model: "test-model".to_string(),
        tokens_used: Some(50),
    };

    let cloned = summary.clone();
    assert_eq!(summary.text, cloned.text);
    assert_eq!(summary.model, cloned.model);
    assert_eq!(summary.tokens_used, cloned.tokens_used);
}

// =============================================================================
// CacheStats Tests
// =============================================================================

#[test]
fn test_cache_stats_default() {
    let stats = CacheStats::default();
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
    assert_eq!(stats.evictions, 0);
    assert_eq!(stats.expirations, 0);
}

#[test]
fn test_cache_stats_hit_rate_zero_total() {
    let stats = CacheStats::default();
    assert_eq!(stats.hit_rate(), 0.0);
}

#[test]
fn test_cache_stats_hit_rate_calculation() {
    let stats = CacheStats {
        hits: 75,
        misses: 25,
        evictions: 0,
        expirations: 0,
    };
    assert!((stats.hit_rate() - 75.0).abs() < 0.1);
}

// =============================================================================
// Cache Integration with Summarizer (TDD: Future Integration)
// =============================================================================

// These tests are for verifying cache integration with ContentSummarizer
// They will guide the implementation of cached summarization

fn create_test_content(text: &str) -> ExtractedContent {
    let chars = text.chars().count();
    let lines = text.lines().count();
    ExtractedContent {
        text: text.to_string(),
        truncated: false,
        stats: ContentStats {
            total_chars: chars,
            total_lines: lines,
            extracted_chars: chars,
        },
    }
}

#[tokio::test]
async fn test_cached_summarizer_uses_cache_on_hit() {
    // This test verifies that when the same content is summarized twice,
    // the second call should use the cached result

    // Create a cached summarizer (this will be the integration point)
    let cache = SummaryCache::for_summaries();
    let content = create_test_content("Test content to summarize");

    // First call: cache should be empty
    let key = LruCache::<String>::generate_key(&content.text, "test-model");
    assert!(cache.get(&key).is_none());

    // Simulate caching after first API call
    let cached_summary = CachedSummary {
        text: "Cached summary result".to_string(),
        model: "test-model".to_string(),
        tokens_used: Some(70),
    };
    cache.cache_summary(&content.text, "test-model", cached_summary);

    // Second call: should use cache
    let cached_result = cache.get_summary(&content.text, "test-model");
    assert!(cached_result.is_some());
    assert_eq!(cached_result.unwrap().text, "Cached summary result");

    // Verify cache stats
    let stats = cache.stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1); // Initial check was a miss
}

#[test]
fn test_cache_thread_safety() {
    use std::sync::Arc;
    use std::thread;

    let cache: Arc<LruCache<String>> = Arc::new(LruCache::new(1000, Duration::from_secs(3600)));
    let mut handles = vec![];

    // Spawn multiple threads that read and write to the cache
    for i in 0..10 {
        let cache_clone = Arc::clone(&cache);
        let handle = thread::spawn(move || {
            for j in 0..100 {
                let key = format!("key_{}_{}", i, j);
                let value = format!("value_{}_{}", i, j);
                cache_clone.insert(key.clone(), value);
                cache_clone.get(&key);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Cache should still be functional
    assert!(cache.len() <= 1000);
}
