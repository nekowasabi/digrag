//! Semantic Search Tests (Process 41)
//!
//! TDD tests for semantic search implementation.

use chrono::{TimeZone, Utc};
use digrag::config::{SearchConfig, SearchMode};
use digrag::embedding::OpenRouterEmbedding;
use digrag::index::{Bm25Index, Docstore, VectorIndex};
use digrag::loader::Document;
use digrag::search::Searcher;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Create test documents with known content
fn create_test_documents() -> Vec<Document> {
    vec![
        Document::with_id(
            "doc1".to_string(),
            "意志力と自制心について".to_string(),
            Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap(),
            vec!["tips".to_string()],
            "意志力は筋肉のように鍛えられる。自制心を高めるためには、小さな目標から始めることが重要。".to_string(),
        ),
        Document::with_id(
            "doc2".to_string(),
            "プログラミングのコツ".to_string(),
            Utc.with_ymd_and_hms(2025, 1, 14, 10, 0, 0).unwrap(),
            vec!["tips".to_string()],
            "コードを書く前に設計を考える。テストファーストで開発することで品質が向上する。".to_string(),
        ),
        Document::with_id(
            "doc3".to_string(),
            "健康的な生活習慣".to_string(),
            Utc.with_ymd_and_hms(2025, 1, 13, 10, 0, 0).unwrap(),
            vec!["memo".to_string()],
            "毎日の運動と十分な睡眠が重要。意志力を保つためには体調管理が欠かせない。".to_string(),
        ),
    ]
}

/// Create a vector index with test embeddings (3-dimensional for testing)
fn create_test_vector_index() -> VectorIndex {
    let mut index = VectorIndex::new(3);

    // doc1: 意志力関連 - 高い意志力ベクトル
    index.add("doc1".to_string(), vec![0.9, 0.1, 0.1]).unwrap();
    // doc2: プログラミング関連 - 低い意志力ベクトル
    index.add("doc2".to_string(), vec![0.1, 0.9, 0.1]).unwrap();
    // doc3: 健康関連 - 中程度の意志力ベクトル
    index.add("doc3".to_string(), vec![0.5, 0.3, 0.5]).unwrap();

    index
}

/// Helper function to setup test indices
fn setup_test_indices(index_path: &std::path::Path) -> Docstore {
    let docs = create_test_documents();

    // Build BM25 index from documents
    let bm25 = Bm25Index::build(&docs).unwrap();
    bm25.save_to_file(&index_path.join("bm25_index.json"))
        .unwrap();

    // Build vector index
    let vector_index = create_test_vector_index();
    vector_index
        .save_to_file(&index_path.join("faiss_index.json"))
        .unwrap();

    // Build docstore
    let mut docstore = Docstore::new();
    for doc in docs {
        docstore.add(doc);
    }
    docstore
        .save_to_file(&index_path.join("docstore.json"))
        .unwrap();

    docstore
}

#[test]
fn test_vector_index_search_returns_results() {
    let index = create_test_vector_index();

    // Query vector similar to doc1 (意志力関連)
    let query_vec = vec![0.9, 0.1, 0.1];
    let results = index.search(&query_vec, 3).unwrap();

    assert!(!results.is_empty(), "Vector search should return results");
    assert_eq!(
        results[0].doc_id, "doc1",
        "Most similar document should be doc1"
    );
    assert!(
        results[0].score > 0.9,
        "Similarity score should be high for exact match"
    );
}

#[test]
fn test_vector_index_search_ranking() {
    let index = create_test_vector_index();

    // Query for "意志力" concept (similar to doc1)
    let query_vec = vec![0.85, 0.15, 0.1];
    let results = index.search(&query_vec, 3).unwrap();

    // doc1 should rank highest (most similar to 意志力 concept)
    assert_eq!(results[0].doc_id, "doc1");
}

#[tokio::test]
async fn test_searcher_semantic_mode_with_vector_index() {
    let temp_dir = tempdir().unwrap();
    let index_path = temp_dir.path();

    setup_test_indices(index_path);

    let searcher = Searcher::new(index_path).unwrap();
    assert!(
        searcher.has_vector_index(),
        "Vector index should be available"
    );
}

/// Test semantic search using pre-computed vector (bypasses embedding API)
#[tokio::test]
async fn test_semantic_search_with_precomputed_vector() {
    let temp_dir = tempdir().unwrap();
    let index_path = temp_dir.path();

    setup_test_indices(index_path);

    let searcher = Searcher::new(index_path).unwrap();
    assert!(searcher.has_vector_index());

    // Use pre-computed vector similar to "意志力" embedding
    let query_vec = vec![0.88, 0.12, 0.08];
    let results = searcher.search_semantic_with_vector(&query_vec, 5).unwrap();

    assert!(!results.is_empty(), "Semantic search should return results");
    assert_eq!(
        results[0].doc_id, "doc1",
        "Most relevant doc should be doc1"
    );
}

/// Test semantic search with mock embedding API
#[tokio::test(flavor = "multi_thread")]
async fn test_semantic_search_with_embedding_client() {
    let mock_server = MockServer::start().await;

    // Mock embedding response - returns vector similar to doc1 (意志力)
    let embedding_response = serde_json::json!({
        "data": [
            { "embedding": vec![0.88f32, 0.12, 0.08], "index": 0 }
        ],
        "model": "openai/text-embedding-3-small"
    });

    Mock::given(method("POST"))
        .and(path("/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&embedding_response))
        .expect(1..)
        .mount(&mock_server)
        .await;

    let temp_dir = tempdir().unwrap();
    let index_path = temp_dir.path();

    setup_test_indices(index_path);

    // Create searcher with embedding client
    let embedding_client =
        OpenRouterEmbedding::with_base_url("test-api-key".to_string(), mock_server.uri());

    let searcher = Searcher::with_embedding_client(index_path, embedding_client).unwrap();

    let config = SearchConfig::new()
        .with_mode(SearchMode::Semantic)
        .with_top_k(5);
    let results = searcher.search("意志力", &config).unwrap();

    assert!(
        !results.is_empty(),
        "Semantic search should return results with embedding client"
    );
    assert_eq!(
        results[0].doc_id, "doc1",
        "Most relevant doc should be doc1"
    );
}

/// Test hybrid search combines BM25 and semantic results
#[tokio::test(flavor = "multi_thread")]
async fn test_hybrid_search_with_embedding_client() {
    let mock_server = MockServer::start().await;

    let embedding_response = serde_json::json!({
        "data": [
            { "embedding": vec![0.88f32, 0.12, 0.08], "index": 0 }
        ],
        "model": "openai/text-embedding-3-small"
    });

    Mock::given(method("POST"))
        .and(path("/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&embedding_response))
        .expect(1..)
        .mount(&mock_server)
        .await;

    let temp_dir = tempdir().unwrap();
    let index_path = temp_dir.path();

    setup_test_indices(index_path);

    let embedding_client =
        OpenRouterEmbedding::with_base_url("test-api-key".to_string(), mock_server.uri());

    let searcher = Searcher::with_embedding_client(index_path, embedding_client).unwrap();

    let config = SearchConfig::new()
        .with_mode(SearchMode::Hybrid)
        .with_top_k(5);
    let results = searcher.search("意志力", &config).unwrap();

    assert!(!results.is_empty(), "Hybrid search should return results");

    // doc1 should be in results (high BM25 and semantic relevance)
    let has_doc1 = results.iter().any(|r| r.doc_id == "doc1");
    assert!(has_doc1, "Hybrid results should include doc1");
}

#[cfg(test)]
mod vector_search_unit_tests {
    use super::*;

    #[test]
    fn test_empty_query_vector() {
        let index = create_test_vector_index();
        let results = index.search(&[], 3).unwrap();
        assert!(results.is_empty(), "Empty query should return no results");
    }

    #[test]
    fn test_top_k_limit() {
        let index = create_test_vector_index();
        let query_vec = vec![0.5, 0.5, 0.5];

        let results_1 = index.search(&query_vec, 1).unwrap();
        assert_eq!(
            results_1.len(),
            1,
            "Should return only 1 result when top_k=1"
        );

        let results_2 = index.search(&query_vec, 2).unwrap();
        assert_eq!(
            results_2.len(),
            2,
            "Should return only 2 results when top_k=2"
        );
    }

    #[test]
    fn test_similarity_scores_are_within_bounds() {
        let index = create_test_vector_index();
        let query_vec = vec![0.9, 0.1, 0.1];
        let results = index.search(&query_vec, 3).unwrap();

        for result in &results {
            assert!(
                result.score >= -1.0 - 1e-5 && result.score <= 1.0 + 1e-5,
                "Cosine similarity should be approximately in [-1, 1] range, got {}",
                result.score
            );
        }
    }
}
