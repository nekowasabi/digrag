//! Reciprocal Rank Fusion implementation
//!
//! Combines results from multiple search methods using RRF.

use super::SearchResult;
use std::collections::HashMap;

/// RRF constant (typically 60)
const RRF_K: f32 = 60.0;

/// Reciprocal Rank Fusion for combining search results
pub struct ReciprocalRankFusion {
    /// RRF constant k
    k: f32,
}

impl Default for ReciprocalRankFusion {
    fn default() -> Self {
        Self::new()
    }
}

impl ReciprocalRankFusion {
    /// Create a new RRF instance with default k=60
    pub fn new() -> Self {
        Self { k: RRF_K }
    }

    /// Create a new RRF instance with custom k
    pub fn with_k(k: f32) -> Self {
        Self { k }
    }

    /// Fuse BM25 and vector search results
    ///
    /// RRF score = sum(1 / (k + rank_i)) for each result list
    pub fn fuse(
        &self,
        bm25_results: &[SearchResult],
        vector_results: &[SearchResult],
    ) -> Vec<SearchResult> {
        let mut scores: HashMap<String, f32> = HashMap::new();
        let mut titles: HashMap<String, String> = HashMap::new();
        let mut snippets: HashMap<String, String> = HashMap::new();

        // Calculate RRF scores from BM25 results
        for (rank, result) in bm25_results.iter().enumerate() {
            let rrf_score = 1.0 / (self.k + (rank + 1) as f32);
            *scores.entry(result.doc_id.clone()).or_insert(0.0) += rrf_score;

            if let Some(title) = &result.title {
                titles
                    .entry(result.doc_id.clone())
                    .or_insert_with(|| title.clone());
            }
            if let Some(snippet) = &result.snippet {
                snippets
                    .entry(result.doc_id.clone())
                    .or_insert_with(|| snippet.clone());
            }
        }

        // Add RRF scores from vector results
        for (rank, result) in vector_results.iter().enumerate() {
            let rrf_score = 1.0 / (self.k + (rank + 1) as f32);
            *scores.entry(result.doc_id.clone()).or_insert(0.0) += rrf_score;

            if let Some(title) = &result.title {
                titles
                    .entry(result.doc_id.clone())
                    .or_insert_with(|| title.clone());
            }
            if let Some(snippet) = &result.snippet {
                snippets
                    .entry(result.doc_id.clone())
                    .or_insert_with(|| snippet.clone());
            }
        }

        // Convert to sorted results
        let mut results: Vec<SearchResult> = scores
            .into_iter()
            .map(|(doc_id, score)| {
                let mut result = SearchResult::new(doc_id.clone(), score);
                result.title = titles.get(&doc_id).cloned();
                result.snippet = snippets.get(&doc_id).cloned();
                result
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrf_creation() {
        let rrf = ReciprocalRankFusion::new();
        assert!((rrf.k - 60.0).abs() < 1e-6);
    }

    #[test]
    fn test_rrf_custom_k() {
        let rrf = ReciprocalRankFusion::with_k(30.0);
        assert!((rrf.k - 30.0).abs() < 1e-6);
    }

    #[test]
    fn test_rrf_fusion_basic() {
        let rrf = ReciprocalRankFusion::new();

        let bm25_results = vec![
            SearchResult::new("doc1".to_string(), 0.9),
            SearchResult::new("doc2".to_string(), 0.8),
            SearchResult::new("doc3".to_string(), 0.7),
        ];

        let vector_results = vec![
            SearchResult::new("doc2".to_string(), 0.95),
            SearchResult::new("doc1".to_string(), 0.85),
            SearchResult::new("doc4".to_string(), 0.75),
        ];

        let fused = rrf.fuse(&bm25_results, &vector_results);

        // doc1 and doc2 should have higher scores (appear in both)
        assert!(!fused.is_empty());

        // Find doc1 and doc2
        let doc1_score = fused.iter().find(|r| r.doc_id == "doc1").map(|r| r.score);
        let doc2_score = fused.iter().find(|r| r.doc_id == "doc2").map(|r| r.score);
        let doc3_score = fused.iter().find(|r| r.doc_id == "doc3").map(|r| r.score);
        let doc4_score = fused.iter().find(|r| r.doc_id == "doc4").map(|r| r.score);

        // doc1 and doc2 should have higher scores than doc3 and doc4
        assert!(doc1_score.unwrap() > doc3_score.unwrap());
        assert!(doc2_score.unwrap() > doc4_score.unwrap());
    }

    #[test]
    fn test_rrf_fusion_empty() {
        let rrf = ReciprocalRankFusion::new();
        let fused = rrf.fuse(&[], &[]);
        assert!(fused.is_empty());
    }

    #[test]
    fn test_rrf_score_calculation() {
        let rrf = ReciprocalRankFusion::with_k(60.0);

        // Expected RRF score for rank 1: 1 / (60 + 1) = 0.01639...
        let expected_rank1 = 1.0 / 61.0;

        let bm25_results = vec![SearchResult::new("doc1".to_string(), 0.9)];
        let fused = rrf.fuse(&bm25_results, &[]);

        assert!((fused[0].score - expected_rank1).abs() < 1e-5);
    }

    // TODO: Add more tests in Process 8
}
