//! MCP server implementation types
//!
//! This module provides type definitions for the MCP server.
//! The actual MCP server implementation using rmcp macros is in main.rs.

use serde::{Deserialize, Serialize};

/// Query memos response
#[derive(Debug, Serialize, Clone)]
pub struct QueryMemosResponse {
    /// Search results
    pub results: Vec<MemoResult>,
    /// Total count
    pub total: usize,
}

/// Individual memo result
#[derive(Debug, Serialize, Clone)]
pub struct MemoResult {
    /// Document ID
    pub id: String,
    /// Title
    pub title: String,
    /// Date
    pub date: String,
    /// Tags
    pub tags: Vec<String>,
    /// Content snippet
    pub snippet: String,
    /// Relevance score
    pub score: f32,
}

/// List tags response
#[derive(Debug, Serialize, Clone)]
pub struct ListTagsResponse {
    /// All tags
    pub tags: Vec<TagInfo>,
}

/// Tag information
#[derive(Debug, Serialize, Clone)]
pub struct TagInfo {
    /// Tag name
    pub name: String,
    /// Document count
    pub count: usize,
}

/// Get recent memos response
#[derive(Debug, Serialize, Clone)]
pub struct GetRecentMemosResponse {
    /// Recent memos
    pub memos: Vec<MemoResult>,
}

/// Query memos request (for external API use)
#[derive(Debug, Deserialize, Clone)]
pub struct QueryMemosRequest {
    /// Search query
    pub query: String,
    /// Number of results (default: 10)
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    /// Optional tag filter
    pub tag_filter: Option<String>,
    /// Search mode: bm25, semantic, or hybrid (default: bm25)
    #[serde(default)]
    pub search_mode: String,
    /// Enable query rewriting (default: true)
    #[serde(default = "default_true")]
    pub enable_rewrite: bool,
}

fn default_top_k() -> usize {
    10
}

fn default_true() -> bool {
    true
}

/// Get recent memos request (for external API use)
#[derive(Debug, Deserialize, Clone)]
pub struct GetRecentMemosRequest {
    /// Number of memos to return (default: 10)
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_memos_request_defaults() {
        let json = r#"{"query": "test"}"#;
        let request: QueryMemosRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.query, "test");
        assert_eq!(request.top_k, 10);
        assert!(request.tag_filter.is_none());
        assert!(request.enable_rewrite);
    }

    #[test]
    fn test_memo_result_serialization() {
        let result = MemoResult {
            id: "test-id".to_string(),
            title: "Test Title".to_string(),
            date: "2025-01-15 10:00:00".to_string(),
            tags: vec!["memo".to_string()],
            snippet: "Test content...".to_string(),
            score: 0.85,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Test Title"));
    }

    #[test]
    fn test_get_recent_memos_request_defaults() {
        let json = r#"{}"#;
        let request: GetRecentMemosRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.limit, 10);
    }
}
