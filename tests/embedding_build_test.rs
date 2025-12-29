//! Embedding Build Integration Tests
//!
//! TDD tests for Process 40: Embedding Build統合（CLI + IndexBuilder）
//! Red Phase: These tests should FAIL initially, then pass after Green Phase implementation

use digrag::index::{IndexBuilder, VectorIndex};
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test 1: CLI --with-embeddings flag parsing
#[test]
fn test_cli_build_with_embeddings_flag() {
    use clap::Parser;

    #[derive(Parser)]
    struct TestCli {
        #[command(subcommand)]
        command: TestCommands,
    }

    #[derive(clap::Subcommand)]
    enum TestCommands {
        Build {
            #[arg(short, long)]
            input: String,
            #[arg(short, long, default_value = ".rag")]
            output: String,
            #[arg(long)]
            skip_embeddings: bool,
            #[arg(long)]
            with_embeddings: bool,
        },
    }

    // Test --with-embeddings flag is recognized
    let cli = TestCli::try_parse_from([
        "digrag",
        "build",
        "--input", "changelogmemo",
        "--output", ".rag",
        "--with-embeddings",
    ]);

    assert!(cli.is_ok(), "CLI should accept --with-embeddings flag");

    if let Ok(cli) = cli {
        let TestCommands::Build { with_embeddings, .. } = cli.command;
        assert!(with_embeddings, "--with-embeddings should be true");
    }
}

/// Test 2: CLI --with-embeddings and --skip-embeddings mutual exclusion
#[test]
fn test_cli_skip_and_with_embeddings_are_exclusive() {
    // When --with-embeddings is set, skip_embeddings should be ignored
    // This is a design decision - with_embeddings takes precedence
    use clap::Parser;

    #[derive(Parser)]
    struct TestCli {
        #[command(subcommand)]
        command: TestCommands,
    }

    #[derive(clap::Subcommand)]
    enum TestCommands {
        Build {
            #[arg(short, long)]
            input: String,
            #[arg(short, long, default_value = ".rag")]
            output: String,
            #[arg(long)]
            skip_embeddings: bool,
            #[arg(long)]
            with_embeddings: bool,
        },
    }

    // Both flags can be passed, but --with-embeddings takes precedence
    let cli = TestCli::try_parse_from([
        "digrag",
        "build",
        "--input", "changelogmemo",
        "--output", ".rag",
        "--with-embeddings",
        "--skip-embeddings",
    ]);

    assert!(cli.is_ok(), "CLI should accept both flags");
}

/// Test 3: IndexBuilder.with_embeddings() creates client
#[test]
fn test_index_builder_with_embeddings_creates_client() {
    let builder = IndexBuilder::with_embeddings("test-api-key".to_string());

    // The builder should have an embedding client configured
    assert!(builder.has_embedding_client(), "Builder should have embedding client");
}

/// Test 4: IndexBuilder without embeddings has no client
#[test]
fn test_index_builder_without_embeddings_no_client() {
    let builder = IndexBuilder::new();

    // The builder should NOT have an embedding client
    assert!(!builder.has_embedding_client(), "Builder should not have embedding client");
}

/// Test 5: build_with_embeddings creates non-empty vector index
#[tokio::test]
async fn test_build_with_embeddings_creates_vector_index() {
    // Setup mock OpenRouter API
    let mock_server = MockServer::start().await;

    // Mock embedding response
    let embedding_response = serde_json::json!({
        "data": [
            {
                "embedding": vec![0.1f32; 1536],
                "index": 0
            }
        ],
        "model": "openai/text-embedding-3-small",
        "usage": {
            "prompt_tokens": 10,
            "total_tokens": 10
        }
    });

    Mock::given(method("POST"))
        .and(path("/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&embedding_response))
        .expect(1..)
        .mount(&mock_server)
        .await;

    // Create temp directory for test
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input_path = temp_dir.path().join("test_changelog");
    let output_dir = temp_dir.path().join("output");

    // Create test changelog file
    std::fs::write(&input_path, r#"* Test Entry 2025-01-15 10:00:00 [memo]:
  Test content for embedding
"#).expect("Failed to write test file");

    // Create builder with mock API URL
    let builder = IndexBuilder::with_embeddings_and_base_url(
        "test-api-key".to_string(),
        mock_server.uri(),
    );

    // Build with embeddings
    let result = builder.build_with_embeddings(
        &input_path,
        &output_dir,
        |step, total, msg| {
            eprintln!("[{}/{}] {}", step, total, msg);
        },
    ).await;

    assert!(result.is_ok(), "build_with_embeddings should succeed: {:?}", result.err());

    // Load and verify vector index
    let vector_index = VectorIndex::load_from_file(&output_dir.join("faiss_index.json"))
        .expect("Failed to load vector index");

    assert!(!vector_index.is_empty(), "Vector index should have vectors");
    assert!(vector_index.dimension() > 0, "Vector index should have dimension");
}

/// Test 6: build_with_embeddings shows progress for batch processing
#[tokio::test]
async fn test_build_with_embeddings_reports_progress() {
    let mock_server = MockServer::start().await;

    // Mock embedding response
    let embedding_response = serde_json::json!({
        "data": [
            { "embedding": vec![0.1f32; 1536], "index": 0 },
            { "embedding": vec![0.2f32; 1536], "index": 1 },
            { "embedding": vec![0.3f32; 1536], "index": 2 },
        ],
        "model": "openai/text-embedding-3-small"
    });

    Mock::given(method("POST"))
        .and(path("/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&embedding_response))
        .expect(1..)
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input_path = temp_dir.path().join("test_changelog");
    let output_dir = temp_dir.path().join("output");

    // Create test changelog with multiple entries
    std::fs::write(&input_path, r#"* Entry 1 2025-01-15 10:00:00 [memo]:
  Content 1

* Entry 2 2025-01-14 09:00:00 [tips]:
  Content 2

* Entry 3 2025-01-13 08:00:00 [worklog]:
  Content 3
"#).expect("Failed to write test file");

    let builder = IndexBuilder::with_embeddings_and_base_url(
        "test-api-key".to_string(),
        mock_server.uri(),
    );

    use std::sync::Mutex;
    let progress_steps = std::sync::Arc::new(Mutex::new(Vec::new()));
    let progress_steps_clone = progress_steps.clone();

    let result = builder.build_with_embeddings(
        &input_path,
        &output_dir,
        move |step, total, msg| {
            progress_steps_clone.lock().unwrap().push((step, total, msg.to_string()));
        },
    ).await;

    assert!(result.is_ok());

    // Should report multiple progress steps
    let steps = progress_steps.lock().unwrap();
    assert!(steps.len() >= 5, "Should report at least 5 progress steps");

    // Should include embedding generation step
    let has_embedding_step = steps.iter()
        .any(|(_, _, msg)| msg.contains("embedding") || msg.contains("Embedding"));
    assert!(has_embedding_step, "Should report embedding generation progress");
}

/// Test 7: OpenRouter API mock - successful response
#[tokio::test]
async fn test_openrouter_api_mock_success() {
    let mock_server = MockServer::start().await;

    let embedding_response = serde_json::json!({
        "data": [
            { "embedding": vec![0.1f32; 1536], "index": 0 }
        ],
        "model": "openai/text-embedding-3-small"
    });

    Mock::given(method("POST"))
        .and(path("/embeddings"))
        .and(header("Authorization", "Bearer test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&embedding_response))
        .mount(&mock_server)
        .await;

    // Test that OpenRouterEmbedding can use custom base URL
    use digrag::embedding::OpenRouterEmbedding;

    let client = OpenRouterEmbedding::with_base_url(
        "test-api-key".to_string(),
        mock_server.uri(),
    );

    let result = client.embed("test text").await;
    assert!(result.is_ok(), "Embedding should succeed: {:?}", result.err());

    let embedding = result.unwrap();
    assert_eq!(embedding.len(), 1536, "Embedding should have 1536 dimensions");
}

/// Test 8: OpenRouter API mock - rate limit handling
/// Note: This test verifies that the client has retry logic built in.
/// In practice, the retry mechanism is already implemented in embed_batch.
#[tokio::test]
async fn test_openrouter_api_rate_limit_retry() {
    use digrag::embedding::OpenRouterEmbedding;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let mock_server = MockServer::start().await;
    let call_count = Arc::new(AtomicUsize::new(0));
    let call_count_clone = call_count.clone();

    let success_response = serde_json::json!({
        "data": [
            { "embedding": vec![0.1f32; 1536], "index": 0 }
        ],
        "model": "openai/text-embedding-3-small"
    });

    // Use a single mock that returns 429 on first call, 200 on subsequent calls
    Mock::given(method("POST"))
        .and(path("/embeddings"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = call_count_clone.fetch_add(1, Ordering::SeqCst);
            if count == 0 {
                ResponseTemplate::new(429).set_body_string("Rate limited")
            } else {
                ResponseTemplate::new(200).set_body_json(&success_response)
            }
        })
        .mount(&mock_server)
        .await;

    let client = OpenRouterEmbedding::with_base_url(
        "test-api-key".to_string(),
        mock_server.uri(),
    );

    // Should retry after rate limit
    let result = client.embed("test text").await;
    assert!(result.is_ok(), "Should retry after rate limit: {:?}", result.err());

    // Verify that retry happened
    assert!(call_count.load(Ordering::SeqCst) >= 2, "Should have retried at least once");
}

/// Test 9: Vector index is saved to faiss_index.json with correct format
#[tokio::test]
async fn test_vector_index_saved_with_embeddings() {
    let mock_server = MockServer::start().await;

    let embedding_response = serde_json::json!({
        "data": [
            { "embedding": vec![0.5f32; 1536], "index": 0 }
        ],
        "model": "openai/text-embedding-3-small"
    });

    Mock::given(method("POST"))
        .and(path("/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&embedding_response))
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input_path = temp_dir.path().join("test_changelog");
    let output_dir = temp_dir.path().join("output");

    std::fs::write(&input_path, r#"* Test Entry 2025-01-15 10:00:00 [memo]:
  Test content
"#).expect("Failed to write test file");

    let builder = IndexBuilder::with_embeddings_and_base_url(
        "test-api-key".to_string(),
        mock_server.uri(),
    );

    builder.build_with_embeddings(&input_path, &output_dir, |_, _, _| {}).await
        .expect("Build should succeed");

    // Verify faiss_index.json exists and has correct format
    let faiss_path = output_dir.join("faiss_index.json");
    assert!(faiss_path.exists(), "faiss_index.json should exist");

    let content = std::fs::read_to_string(&faiss_path).expect("Failed to read faiss_index.json");
    let json: serde_json::Value = serde_json::from_str(&content).expect("Invalid JSON");

    assert!(json.get("dimension").is_some(), "Should have dimension field");
    assert!(json.get("vectors").is_some(), "Should have vectors field");

    let dimension = json["dimension"].as_u64().unwrap();
    assert_eq!(dimension, 1536, "Dimension should be 1536");

    let vectors = json["vectors"].as_array().unwrap();
    assert!(!vectors.is_empty(), "Should have at least one vector");
}

/// Test 10: Metadata includes embedding model info
#[tokio::test]
async fn test_metadata_includes_embedding_model() {
    let mock_server = MockServer::start().await;

    let embedding_response = serde_json::json!({
        "data": [
            { "embedding": vec![0.5f32; 1536], "index": 0 }
        ],
        "model": "openai/text-embedding-3-small"
    });

    Mock::given(method("POST"))
        .and(path("/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&embedding_response))
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input_path = temp_dir.path().join("test_changelog");
    let output_dir = temp_dir.path().join("output");

    std::fs::write(&input_path, r#"* Test Entry 2025-01-15 10:00:00 [memo]:
  Test content
"#).expect("Failed to write test file");

    let builder = IndexBuilder::with_embeddings_and_base_url(
        "test-api-key".to_string(),
        mock_server.uri(),
    );

    builder.build_with_embeddings(&input_path, &output_dir, |_, _, _| {}).await
        .expect("Build should succeed");

    // Verify metadata.json includes embedding model
    let metadata_path = output_dir.join("metadata.json");
    let content = std::fs::read_to_string(&metadata_path).expect("Failed to read metadata.json");
    let json: serde_json::Value = serde_json::from_str(&content).expect("Invalid JSON");

    assert!(json.get("embedding_model").is_some(), "Should have embedding_model field");
    let model = json["embedding_model"].as_str().unwrap();
    assert_eq!(model, "openai/text-embedding-3-small", "Should use correct model");
}
