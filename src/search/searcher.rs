//! Searcher implementation
//!
//! Provides the main search interface that combines all search methods.

use super::{ReciprocalRankFusion, SearchResult};
use crate::config::{SearchConfig, SearchMode};
use crate::embedding::OpenRouterEmbedding;
use crate::index::{Bm25Index, Docstore, VectorIndex};
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Main searcher that combines all search methods
pub struct Searcher {
    /// BM25 index
    bm25_index: Bm25Index,
    /// Vector index
    vector_index: VectorIndex,
    /// Document store
    docstore: Docstore,
    /// RRF fusion
    rrf: ReciprocalRankFusion,
    /// Optional embedding client for semantic search
    embedding_client: Option<Arc<Mutex<OpenRouterEmbedding>>>,
}

impl Searcher {
    /// Create a new searcher from an index directory
    pub fn new<P: AsRef<Path>>(index_dir: P) -> Result<Self> {
        let index_dir = index_dir.as_ref();

        // Load indices
        let bm25_path = index_dir.join("bm25_index.json");
        let vector_path = index_dir.join("faiss_index.json");
        let docstore_path = index_dir.join("docstore.json");

        let bm25_index = if bm25_path.exists() {
            Bm25Index::load_from_file(&bm25_path)?
        } else {
            Bm25Index::new()
        };

        let vector_index = if vector_path.exists() {
            VectorIndex::load_from_file(&vector_path)?
        } else {
            VectorIndex::new(0)
        };

        let docstore = if docstore_path.exists() {
            Docstore::load_from_file(&docstore_path)?
        } else {
            Docstore::new()
        };

        Ok(Self {
            bm25_index,
            vector_index,
            docstore,
            rrf: ReciprocalRankFusion::new(),
            embedding_client: None,
        })
    }

    /// Create a new searcher with an embedding client for semantic search
    pub fn with_embedding_client<P: AsRef<Path>>(
        index_dir: P,
        embedding_client: OpenRouterEmbedding,
    ) -> Result<Self> {
        let mut searcher = Self::new(index_dir)?;
        searcher.embedding_client = Some(Arc::new(Mutex::new(embedding_client)));
        Ok(searcher)
    }

    /// Set embedding client after creation
    pub fn set_embedding_client(&mut self, client: OpenRouterEmbedding) {
        self.embedding_client = Some(Arc::new(Mutex::new(client)));
    }

    /// Search with the given configuration
    pub fn search(&self, query: &str, config: &SearchConfig) -> Result<Vec<SearchResult>> {
        // Apply tag filter
        let results = match config.search_mode {
            SearchMode::Bm25 => self.search_bm25(query, config.top_k)?,
            SearchMode::Semantic => self.search_semantic(query, config.top_k)?,
            SearchMode::Hybrid => self.search_hybrid(query, config.top_k)?,
        };

        // Filter by tag if specified
        if let Some(tag) = &config.tag_filter {
            Ok(results
                .into_iter()
                .filter(|r| {
                    self.docstore
                        .get(&r.doc_id)
                        .map(|doc| doc.has_tag(tag))
                        .unwrap_or(false)
                })
                .take(config.top_k)
                .collect())
        } else {
            Ok(results)
        }
    }

    /// BM25 keyword search
    fn search_bm25(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        self.bm25_index.search(query, top_k)
    }

    /// Semantic vector search
    fn search_semantic(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        // Check if vector index is available
        if self.vector_index.is_empty() {
            tracing::warn!("Vector index is empty. Semantic search requires embeddings.");
            return Ok(Vec::new());
        }

        tracing::info!("Semantic search for '{}' with top_k={}", query, top_k);

        // Use embedding client if available
        if let Some(ref client) = self.embedding_client {
            // Get embedding for query using blocking runtime
            let query_embedding = {
                let client = client.clone();
                let query = query.to_string();

                // Use tokio runtime to run async code in sync context
                let rt = tokio::runtime::Handle::try_current();
                match rt {
                    Ok(handle) => {
                        // We're inside an async context, use block_in_place
                        tokio::task::block_in_place(|| {
                            handle.block_on(async {
                                let client = client.lock().await;
                                client.embed(&query).await
                            })
                        })
                    }
                    Err(_) => {
                        // No runtime, create a new one
                        let rt = tokio::runtime::Runtime::new()?;
                        rt.block_on(async {
                            let client = client.lock().await;
                            client.embed(&query).await
                        })
                    }
                }
            };

            match query_embedding {
                Ok(embedding) => {
                    return self.vector_index.search(&embedding, top_k);
                }
                Err(e) => {
                    tracing::error!("Failed to generate query embedding: {}", e);
                    return Ok(Vec::new());
                }
            }
        }

        // Fallback: no embedding client available
        tracing::warn!("No embedding client available for semantic search");
        Ok(Vec::new())
    }

    /// Semantic search with pre-computed query vector (for testing or cached queries)
    pub fn search_semantic_with_vector(
        &self,
        query_vec: &[f32],
        top_k: usize,
    ) -> Result<Vec<SearchResult>> {
        if self.vector_index.is_empty() {
            return Ok(Vec::new());
        }
        self.vector_index.search(query_vec, top_k)
    }

    /// Check if vector index is available
    pub fn has_vector_index(&self) -> bool {
        !self.vector_index.is_empty()
    }

    /// Hybrid search using RRF
    fn search_hybrid(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        let bm25_results = self.search_bm25(query, top_k * 2)?;
        let vector_results = self.search_semantic(query, top_k * 2)?;

        let fused = self.rrf.fuse(&bm25_results, &vector_results);

        Ok(fused.into_iter().take(top_k).collect())
    }

    /// Get document store reference
    pub fn docstore(&self) -> &Docstore {
        &self.docstore
    }

    /// List all tags
    pub fn list_tags(&self) -> Vec<String> {
        self.docstore.get_all_tags()
    }

    /// Get recent memos
    pub fn get_recent_memos(&self, limit: usize) -> Vec<&crate::loader::Document> {
        self.docstore.get_recent(limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_searcher_with_empty_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let searcher = Searcher::new(temp_dir.path());
        assert!(searcher.is_ok());
    }

    #[test]
    fn test_searcher_list_tags_empty() {
        let temp_dir = tempfile::tempdir().unwrap();
        let searcher = Searcher::new(temp_dir.path()).unwrap();
        let tags = searcher.list_tags();
        assert!(tags.is_empty());
    }

    // TODO: Add more tests in Process 9
}
