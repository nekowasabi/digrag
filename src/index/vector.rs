//! Vector Index implementation
//!
//! Provides semantic search using vector embeddings.

use crate::search::SearchResult;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Vector search index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorIndex {
    /// Document IDs in index order
    doc_ids: Vec<String>,
    /// Embedding vectors (each vector is a Vec<f32>)
    vectors: Vec<Vec<f32>>,
    /// Embedding dimension
    dimension: usize,
}

impl Default for VectorIndex {
    fn default() -> Self {
        Self::new(0)
    }
}

impl VectorIndex {
    /// Create a new empty vector index with specified dimension
    pub fn new(dimension: usize) -> Self {
        Self {
            doc_ids: Vec::new(),
            vectors: Vec::new(),
            dimension,
        }
    }

    /// Add a document with its embedding
    pub fn add(&mut self, doc_id: String, vector: Vec<f32>) -> Result<()> {
        if self.dimension == 0 {
            self.dimension = vector.len();
        }
        self.doc_ids.push(doc_id);
        self.vectors.push(vector);
        Ok(())
    }

    /// Search for similar documents using cosine similarity
    pub fn search(&self, query_vec: &[f32], top_k: usize) -> Result<Vec<SearchResult>> {
        if self.vectors.is_empty() || query_vec.is_empty() {
            return Ok(Vec::new());
        }

        // Calculate similarity scores for all documents
        let mut scores: Vec<(usize, f32)> = self
            .vectors
            .iter()
            .enumerate()
            .map(|(idx, doc_vec)| {
                let similarity = Self::cosine_similarity(query_vec, doc_vec);
                (idx, similarity)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        // Sort by score descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top_k results
        let results: Vec<SearchResult> = scores
            .into_iter()
            .take(top_k)
            .map(|(idx, score)| SearchResult::new(self.doc_ids[idx].clone(), score))
            .collect();

        Ok(results)
    }

    /// Calculate cosine similarity between two vectors
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }

    /// Save index to file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load index from file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let index = serde_json::from_str(&content)?;
        Ok(index)
    }

    /// Get document count
    pub fn len(&self) -> usize {
        self.doc_ids.len()
    }

    /// Check if index is empty
    pub fn is_empty(&self) -> bool {
        self.doc_ids.is_empty()
    }

    /// Get embedding dimension
    pub fn dimension(&self) -> usize {
        self.dimension
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_index_creation() {
        let index = VectorIndex::new(384);
        assert!(index.is_empty());
        assert_eq!(index.dimension(), 384);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((VectorIndex::cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![0.0, 1.0, 0.0];
        assert!((VectorIndex::cosine_similarity(&a, &c) - 0.0).abs() < 1e-6);

        let d = vec![-1.0, 0.0, 0.0];
        assert!((VectorIndex::cosine_similarity(&a, &d) - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        assert_eq!(VectorIndex::cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_add_vector() {
        let mut index = VectorIndex::new(0);
        index.add("doc1".to_string(), vec![0.1, 0.2, 0.3]).unwrap();
        assert_eq!(index.len(), 1);
        assert_eq!(index.dimension(), 3);
    }

    #[test]
    fn test_vector_index_serialization() {
        let mut index = VectorIndex::new(3);
        index.add("doc1".to_string(), vec![0.1, 0.2, 0.3]).unwrap();

        let json = serde_json::to_string(&index).unwrap();
        let deserialized: VectorIndex = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized.dimension(), 3);
    }

    #[test]
    fn test_vector_search() {
        let mut index = VectorIndex::new(3);
        index.add("doc1".to_string(), vec![1.0, 0.0, 0.0]).unwrap();
        index.add("doc2".to_string(), vec![0.0, 1.0, 0.0]).unwrap();
        index.add("doc3".to_string(), vec![0.7, 0.7, 0.0]).unwrap();

        // Query similar to doc1
        let results = index.search(&[1.0, 0.0, 0.0], 2).unwrap();

        assert_eq!(results.len(), 2);
        // doc1 should be most similar (exact match)
        assert_eq!(results[0].doc_id, "doc1");
        assert!((results[0].score - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_vector_search_empty() {
        let index = VectorIndex::new(3);
        let results = index.search(&[1.0, 0.0, 0.0], 2).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_vector_search_ranking() {
        let mut index = VectorIndex::new(3);
        index.add("doc1".to_string(), vec![1.0, 0.0, 0.0]).unwrap();
        index.add("doc2".to_string(), vec![0.5, 0.5, 0.0]).unwrap();
        index.add("doc3".to_string(), vec![0.0, 1.0, 0.0]).unwrap();

        // Query that's between doc1 and doc2
        let results = index.search(&[0.8, 0.2, 0.0], 3).unwrap();

        // Results should be ordered by similarity
        assert!(results[0].score >= results[1].score);
        if results.len() > 2 {
            assert!(results[1].score >= results[2].score);
        }
    }
}
