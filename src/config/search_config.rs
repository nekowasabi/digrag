//! Search configuration structures
//!
//! Defines the search modes and configuration options for the search engine.

use serde::{Deserialize, Serialize};

/// Search mode enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    /// BM25 keyword-based search
    Bm25,
    /// Semantic vector search
    Semantic,
    /// Hybrid search combining BM25 and semantic with RRF
    #[default]
    Hybrid,
}

/// Search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Search mode to use
    pub search_mode: SearchMode,
    /// Number of results to return
    pub top_k: usize,
    /// Optional tag filter
    pub tag_filter: Option<String>,
    /// Enable query rewriting
    pub enable_rewrite: bool,
    /// BM25 weight for hybrid search (0.0 to 1.0)
    pub bm25_weight: f32,
    /// Semantic weight for hybrid search (0.0 to 1.0)
    pub semantic_weight: f32,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            search_mode: SearchMode::default(),
            top_k: 10,
            tag_filter: None,
            enable_rewrite: true,
            bm25_weight: 0.5,
            semantic_weight: 0.5,
        }
    }
}

impl SearchConfig {
    /// Create a new search configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the search mode
    pub fn with_mode(mut self, mode: SearchMode) -> Self {
        self.search_mode = mode;
        self
    }

    /// Set the number of results to return
    pub fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = top_k;
        self
    }

    /// Set the tag filter
    pub fn with_tag_filter(mut self, tag: Option<String>) -> Self {
        self.tag_filter = tag;
        self
    }

    /// Set whether to enable query rewriting
    pub fn with_rewrite(mut self, enable: bool) -> Self {
        self.enable_rewrite = enable;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_mode_default() {
        assert_eq!(SearchMode::default(), SearchMode::Hybrid);
    }

    #[test]
    fn test_search_config_default() {
        let config = SearchConfig::default();
        assert_eq!(config.search_mode, SearchMode::Hybrid);
        assert_eq!(config.top_k, 10);
        assert!(config.tag_filter.is_none());
        assert!(config.enable_rewrite);
    }

    #[test]
    fn test_search_config_builder() {
        let config = SearchConfig::new()
            .with_mode(SearchMode::Bm25)
            .with_top_k(5)
            .with_tag_filter(Some("worklog".to_string()))
            .with_rewrite(false);

        assert_eq!(config.search_mode, SearchMode::Bm25);
        assert_eq!(config.top_k, 5);
        assert_eq!(config.tag_filter, Some("worklog".to_string()));
        assert!(!config.enable_rewrite);
    }

    #[test]
    fn test_search_mode_serialization() {
        let mode = SearchMode::Hybrid;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"hybrid\"");

        let deserialized: SearchMode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, SearchMode::Hybrid);
    }

    #[test]
    fn test_search_config_serialization() {
        let config = SearchConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: SearchConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.search_mode, config.search_mode);
        assert_eq!(deserialized.top_k, config.top_k);
    }
}
