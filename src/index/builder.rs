//! Index Builder implementation
//!
//! Provides the pipeline for building all indices from changelog files.

use super::{Bm25Index, Docstore, VectorIndex};
use crate::embedding::OpenRouterEmbedding;
use crate::loader::{ChangelogLoader, Document};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Create embedding input text from a document
///
/// Combines title, tags, and text content into a structured format
/// optimized for semantic embedding generation.
///
/// # Format
/// - With tags: `# {title}\nタグ: {tag1}, {tag2}\n\n{text}`
/// - Without tags: `# {title}\n\n{text}`
///
/// # Arguments
/// * `doc` - The document to create embedding text for
///
/// # Returns
/// A formatted string suitable for embedding generation
fn create_embedding_text(doc: &Document) -> String {
    let tags = doc.tags().join(", ");
    let title = doc.title();

    if tags.is_empty() {
        format!("# {}\n\n{}", title, doc.text)
    } else {
        format!("# {}\nタグ: {}\n\n{}", title, tags, doc.text)
    }
}

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

        self.build_from_documents_with_progress(documents, output_dir, progress, 2)
    }

    /// Build indices from pre-loaded documents with progress reporting
    ///
    /// This method allows building indices from documents that have already been loaded,
    /// useful for stdin input or custom document sources.
    pub fn build_from_documents_with_progress<F>(
        &self,
        documents: Vec<Document>,
        output_dir: &Path,
        progress: F,
        start_step: usize,
    ) -> Result<()>
    where
        F: Fn(usize, usize, &str),
    {
        let doc_count = documents.len();
        let total_steps = start_step + 3; // BM25, docstore, save, done

        // Build BM25 index
        progress(start_step, total_steps, "Building BM25 index...");
        let bm25_index = Bm25Index::build(&documents)?;

        // Build docstore
        progress(start_step + 1, total_steps, "Building document store...");
        let mut docstore = Docstore::new();
        for doc in &documents {
            docstore.add(doc.clone());
        }

        // Save indices
        progress(start_step + 2, total_steps, "Saving indices...");
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

        progress(total_steps, total_steps, "Done!");

        Ok(())
    }

    /// Build indices from pre-loaded documents (convenience method)
    pub fn build_from_documents(&self, documents: Vec<Document>, output_dir: &Path) -> Result<()> {
        self.build_from_documents_with_progress(documents, output_dir, |_, _, _| {}, 1)
    }

    /// Build indices from pre-loaded documents with embeddings (async)
    pub async fn build_from_documents_with_embeddings<F>(
        &self,
        documents: Vec<Document>,
        output_dir: &Path,
        progress: F,
    ) -> Result<()>
    where
        F: Fn(usize, usize, &str),
    {
        const BATCH_SIZE: usize = 10;
        let doc_count = documents.len();

        // Step 1: Build BM25 index
        progress(1, 5, "Building BM25 index...");
        let bm25_index = Bm25Index::build(&documents)?;

        // Step 2: Build docstore
        progress(2, 5, "Building document store...");
        let mut docstore = Docstore::new();
        for doc in &documents {
            docstore.add(doc.clone());
        }

        // Step 3: Build vector index (if embedding client available)
        let vector_index = if let Some(client) = &self.embedding_client {
            let total_batches = doc_count.div_ceil(BATCH_SIZE);
            progress(3, 5, &format!("Generating embeddings ({} documents in {} batches)...", doc_count, total_batches));

            let mut index = VectorIndex::new(1536);
            let texts: Vec<String> = documents.iter().map(create_embedding_text).collect();

            for (batch_idx, chunk) in texts.chunks(BATCH_SIZE).enumerate() {
                if batch_idx > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }

                let batch_progress = format!(
                    "Embedding batch {}/{} ({} documents)...",
                    batch_idx + 1,
                    total_batches,
                    chunk.len()
                );
                progress(3, 5, &batch_progress);

                let embeddings = client.embed_batch(chunk).await?;

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
            progress(3, 5, "Skipping embeddings (no client configured)...");
            VectorIndex::new(0)
        };

        // Step 4: Save indices
        progress(4, 5, "Saving indices...");
        std::fs::create_dir_all(output_dir)?;

        bm25_index.save_to_file(&output_dir.join("bm25_index.json"))?;
        docstore.save_to_file(&output_dir.join("docstore.json"))?;
        vector_index.save_to_file(&output_dir.join("faiss_index.json"))?;

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
            let texts: Vec<String> = documents.iter().map(create_embedding_text).collect();

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
    use crate::loader::Document;
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_index_builder_creation() {
        let _builder = IndexBuilder::new();
    }

    // Process 1: TDD Tests for create_embedding_text

    #[test]
    fn test_create_embedding_text_with_title_and_text() {
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let doc = Document::new(
            "VimConf 2024 参加レポート".to_string(),
            date,
            vec![],
            "Vimカンファレンスに参加しました。".to_string(),
        );

        let result = create_embedding_text(&doc);

        assert!(result.contains("# VimConf 2024 参加レポート"));
        assert!(result.contains("Vimカンファレンスに参加しました。"));
    }

    #[test]
    fn test_create_embedding_text_with_tags() {
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let doc = Document::new(
            "Claude Code / hookタイミング".to_string(),
            date,
            vec!["memo".to_string(), "tips".to_string()],
            "hookの実行タイミングについて。".to_string(),
        );

        let result = create_embedding_text(&doc);

        assert!(result.contains("# Claude Code / hookタイミング"));
        assert!(result.contains("タグ: memo, tips"));
        assert!(result.contains("hookの実行タイミングについて。"));
    }

    #[test]
    fn test_create_embedding_text_without_tags() {
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let doc = Document::new(
            "シンプルなメモ".to_string(),
            date,
            vec![],
            "本文のみ".to_string(),
        );

        let result = create_embedding_text(&doc);

        // タグセクションがないことを確認
        assert!(!result.contains("タグ:"));
        assert!(result.contains("# シンプルなメモ"));
        assert!(result.contains("本文のみ"));
    }

    #[test]
    fn test_create_embedding_text_empty_title() {
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let doc = Document::new(
            "".to_string(),
            date,
            vec!["worklog".to_string()],
            "タイトルなしの本文".to_string(),
        );

        let result = create_embedding_text(&doc);

        // 空タイトルでも動作することを確認
        assert!(result.contains("タイトルなしの本文"));
        assert!(result.contains("タグ: worklog"));
    }

    // TODO: Add more tests in Process 12
}
