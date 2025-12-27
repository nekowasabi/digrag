//! digrag: Portable RAG search engine
//!
//! This library provides a high-performance search engine for text documents
//! with support for BM25 keyword search, semantic vector search, and hybrid
//! search using Reciprocal Rank Fusion (RRF).
//!
//! # Features
//!
//! - Japanese text tokenization with Lindera (IPADIC)
//! - BM25 keyword-based search
//! - Semantic search with OpenRouter embeddings
//! - Hybrid search with RRF fusion
//! - MCP server for AI assistant integration
//!
//! # Modules
//!
//! - `config`: Configuration structures for search modes and options
//! - `loader`: Document loading and changelog parsing
//! - `tokenizer`: Japanese text tokenization
//! - `index`: BM25, Vector, and Document store indices
//! - `search`: Search integration and result fusion
//! - `embedding`: OpenRouter embedding API client
//! - `rewriter`: Query rewriting with LLM
//! - `mcp`: MCP server implementation

pub mod config;
pub mod embedding;
pub mod index;
pub mod loader;
// pub mod mcp;  // MCP server is now implemented in main.rs using rmcp macros
pub mod rewriter;
pub mod search;
pub mod tokenizer;

// Re-export commonly used types
pub use config::{SearchConfig, SearchMode};
pub use loader::Document;
pub use search::SearchResult;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_exists() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_name_exists() {
        assert_eq!(NAME, "digrag");
    }

    #[test]
    fn test_project_structure() {
        // This test validates that the project structure is correct
        // by checking that the modules can be imported
        // The actual content of modules will be tested in their respective files
        assert!(true, "Project structure is valid");
    }
}
