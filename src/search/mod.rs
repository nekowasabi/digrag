//! Search module
//!
//! This module provides the main search functionality and result types.

mod fusion;
mod searcher;

pub use fusion::ReciprocalRankFusion;
pub use searcher::Searcher;

use serde::{Deserialize, Serialize};

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Document ID
    pub doc_id: String,
    /// Relevance score
    pub score: f32,
    /// Document title (optional, for display)
    pub title: Option<String>,
    /// Document snippet (optional, for display)
    pub snippet: Option<String>,
}

impl SearchResult {
    /// Create a new search result
    pub fn new(doc_id: String, score: f32) -> Self {
        Self {
            doc_id,
            score,
            title: None,
            snippet: None,
        }
    }

    /// Create a search result with title and snippet
    pub fn with_details(doc_id: String, score: f32, title: String, snippet: String) -> Self {
        Self {
            doc_id,
            score,
            title: Some(title),
            snippet: Some(snippet),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_result_creation() {
        let result = SearchResult::new("doc1".to_string(), 0.85);
        assert_eq!(result.doc_id, "doc1");
        assert!((result.score - 0.85).abs() < 1e-6);
        assert!(result.title.is_none());
        assert!(result.snippet.is_none());
    }

    #[test]
    fn test_search_result_with_details() {
        let result = SearchResult::with_details(
            "doc1".to_string(),
            0.85,
            "Test Title".to_string(),
            "Test snippet...".to_string(),
        );
        assert_eq!(result.doc_id, "doc1");
        assert_eq!(result.title.as_ref().unwrap(), "Test Title");
        assert_eq!(result.snippet.as_ref().unwrap(), "Test snippet...");
    }

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult::new("doc1".to_string(), 0.85);
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: SearchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.doc_id, result.doc_id);
    }
}
