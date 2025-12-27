//! Index Builder implementation
//!
//! Provides the pipeline for building all indices from changelog files.

use super::{Bm25Index, Docstore, VectorIndex};
use crate::embedding::OpenRouterEmbedding;
use crate::loader::ChangelogLoader;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Index metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexMetadata {
    /// Number of documents
    pub doc_count: usize,
    /// Index creation timestamp
    pub created_at: String,
    /// Model used for embeddings
    pub embedding_model: Option<String>,
}

/// Index builder for creating all search indices
pub struct IndexBuilder {
    /// Optional embedding client for vector index
    embedding_client: Option<OpenRouterEmbedding>,
}

impl Default for IndexBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl IndexBuilder {
    /// Create a new index builder
    pub fn new() -> Self {
        Self {
            embedding_client: None,
        }
    }

    /// Create with embedding client for vector search
    pub fn with_embeddings(api_key: String) -> Self {
        Self {
            embedding_client: Some(OpenRouterEmbedding::new(api_key)),
        }
    }

    /// Create with embedding client using custom base URL (for testing)
    pub fn with_embeddings_and_base_url(api_key: String, base_url: String) -> Self {
        Self {
            embedding_client: Some(OpenRouterEmbedding::with_base_url(api_key, base_url)),
        }
    }

    /// Check if this builder has an embedding client configured
    pub fn has_embedding_client(&self) -> bool {
        self.embedding_client.is_some()
    }

    /// Build all indices from a changelog file (sync version, no embeddings)
    pub fn build(&self, input: &Path, output_dir: &Path) -> Result<()> {
        self.build_with_progress(input, output_dir, |_, _, _| {})
    }

    /// Build indices with progress reporting
    pub fn build_with_progress<F>(&self, input: &Path, output_dir: &Path, progress: F) -> Result<()>
    where
        F: Fn(usize, usize, &str),
    {
        // Step 1: Parse changelog
        progress(1, 5, "Parsing changelog...");
        let loader = ChangelogLoader::new();
        let documents = loader.load_from_file(input)?;
        let doc_count = documents.len();

        // Step 2: Build BM25 index
        progress(2, 5, "Building BM25 index...");
        let bm25_index = Bm25Index::build(&documents)?;

        // Step 3: Build docstore
        progress(3, 5, "Building document store...");
        let mut docstore = Docstore::new();
        for doc in &documents {
            docstore.add(doc.clone());
        }

        // Step 4: Save indices
        progress(4, 5, "Saving indices...");
        std::fs::create_dir_all(output_dir)?;

        bm25_index.save_to_file(&output_dir.join("bm25_index.json"))?;
        docstore.save_to_file(&output_dir.join("docstore.json"))?;

        // Save empty vector index placeholder
        let vector_index = VectorIndex::new(0);
        vector_index.save_to_file(&output_dir.join("faiss_index.json"))?;

        // Save metadata
        let metadata = IndexMetadata {
            doc_count,
            created_at: chrono::Utc::now().to_rfc3339(),
            embedding_model: self
                .embedding_client
                .as_ref()
                .map(|c| c.model().to_string()),
        };
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(output_dir.join("metadata.json"), metadata_json)?;

        progress(5, 5, "Done!");

        Ok(())
    }

    /// Build indices with embeddings (async)
    pub async fn build_with_embeddings<F>(
        &self,
        input: &Path,
        output_dir: &Path,
        progress: F,
    ) -> Result<()>
    where
        F: Fn(usize, usize, &str),
    {
        // OpenRouter has a limit on total tokens per request
        // Testing showed that batch size 10 works reliably with Japanese text
        const BATCH_SIZE: usize = 10;

        // Step 1: Parse changelog
        progress(1, 6, "Parsing changelog...");
        let loader = ChangelogLoader::new();
        let documents = loader.load_from_file(input)?;
        let doc_count = documents.len();

        // Step 2: Build BM25 index
        progress(2, 6, "Building BM25 index...");
        let bm25_index = Bm25Index::build(&documents)?;

        // Step 3: Build docstore
        progress(3, 6, "Building document store...");
        let mut docstore = Docstore::new();
        for doc in &documents {
            docstore.add(doc.clone());
        }

        // Step 4: Build vector index (if embedding client available)
        let vector_index = if let Some(client) = &self.embedding_client {
            let total_batches = doc_count.div_ceil(BATCH_SIZE);
            progress(4, 6, &format!("Generating embeddings ({} documents in {} batches)...", doc_count, total_batches));

            let mut index = VectorIndex::new(1536); // OpenAI embedding dimension
            let texts: Vec<String> = documents.iter().map(|d| d.text.clone()).collect();

            // Batch embed in chunks with rate limiting
            for (batch_idx, chunk) in texts.chunks(BATCH_SIZE).enumerate() {
                // Add delay between batches to avoid rate limiting (except first batch)
                if batch_idx > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }

                let batch_progress = format!(
                    "Embedding batch {}/{} ({} documents)...",
                    batch_idx + 1,
                    total_batches,
                    chunk.len()
                );
                progress(4, 6, &batch_progress);

                let embeddings = client.embed_batch(chunk).await?;

                // Calculate actual document indices
                let start_idx = batch_idx * BATCH_SIZE;
                for (i, embedding) in embeddings.into_iter().enumerate() {
                    let doc_idx = start_idx + i;
                    if doc_idx < documents.len() {
                        index.add(documents[doc_idx].id.clone(), embedding)?;
                    }
                }
            }
            index
        } else {
            progress(4, 6, "Skipping embeddings (no client configured)...");
            VectorIndex::new(0)
        };

        // Step 5: Save indices
        progress(5, 6, "Saving indices...");
        std::fs::create_dir_all(output_dir)?;

        bm25_index.save_to_file(&output_dir.join("bm25_index.json"))?;
        docstore.save_to_file(&output_dir.join("docstore.json"))?;
        vector_index.save_to_file(&output_dir.join("faiss_index.json"))?;

        // Save metadata
        let metadata = IndexMetadata {
            doc_count,
            created_at: chrono::Utc::now().to_rfc3339(),
            embedding_model: self
                .embedding_client
                .as_ref()
                .map(|c| c.model().to_string()),
        };
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(output_dir.join("metadata.json"), metadata_json)?;

        progress(6, 6, "Done!");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_builder_creation() {
        let _builder = IndexBuilder::new();
    }

    // TODO: Add more tests in Process 12
}
