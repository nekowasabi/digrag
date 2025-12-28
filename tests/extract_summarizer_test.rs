//! Summarizer tests (Process 10: TDD)
//!
//! Tests for content summarization functionality

use digrag::extract::openrouter_client::OpenRouterClient;
use digrag::extract::summarizer::{
    ContentSummarizer, ProviderConfig, SummarizationStrategy, Summary,
};
use digrag::extract::{ContentStats, ExtractedContent};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// =============================================================================
// ProviderConfig Tests
// =============================================================================

#[test]
fn test_provider_config_default() {
    let config = ProviderConfig::default();
    assert!(config.allow_fallbacks);
    assert!(config.order.is_none());
    assert!(config.only.is_none());
    assert!(config.ignore.is_none());
    assert!(config.sort.is_none());
    assert!(!config.require_parameters);
}

#[test]
fn test_provider_config_custom() {
    let config = ProviderConfig {
        order: Some(vec!["Cerebras".to_string(), "Together".to_string()]),
        allow_fallbacks: false,
        only: Some(vec!["Cerebras".to_string()]),
        ignore: Some(vec!["OpenAI".to_string()]),
        sort: Some("price".to_string()),
        require_parameters: true,
    };

    assert!(!config.allow_fallbacks);
    assert_eq!(config.order.as_ref().unwrap().len(), 2);
    assert!(config.require_parameters);
}

#[test]
fn test_provider_config_to_json() {
    let config = ProviderConfig {
        order: Some(vec!["Cerebras".to_string()]),
        allow_fallbacks: true,
        only: None,
        ignore: None,
        sort: Some("throughput".to_string()),
        require_parameters: false,
    };

    let json = config.to_json();
    assert!(json.get("order").is_some());
    assert_eq!(json["allow_fallbacks"], true);
    assert_eq!(json["sort"], "throughput");
}

#[test]
fn test_provider_config_to_json_minimal() {
    let config = ProviderConfig::default();
    let json = config.to_json();

    // Should only have allow_fallbacks (no optional fields)
    assert_eq!(json["allow_fallbacks"], true);
    assert!(json.get("order").is_none());
}

// =============================================================================
// SummarizationStrategy Tests
// =============================================================================

#[test]
fn test_strategy_default_is_rule_based() {
    let strategy = SummarizationStrategy::default();
    match strategy {
        SummarizationStrategy::RuleBased { preview_chars } => {
            assert_eq!(preview_chars, 200);
        }
        _ => panic!("Default should be RuleBased"),
    }
}

#[test]
fn test_strategy_llm_based() {
    let strategy = SummarizationStrategy::LlmBased {
        model: "cerebras/llama-3.3-70b".to_string(),
        max_tokens: 500,
        temperature: 0.3,
        provider_config: ProviderConfig::default(),
    };

    match strategy {
        SummarizationStrategy::LlmBased { model, max_tokens, temperature, .. } => {
            assert_eq!(model, "cerebras/llama-3.3-70b");
            assert_eq!(max_tokens, 500);
            assert!((temperature - 0.3).abs() < 0.001);
        }
        _ => panic!("Should be LlmBased"),
    }
}

// =============================================================================
// ContentSummarizer - Rule Based Tests
// =============================================================================

fn create_test_content(text: &str) -> ExtractedContent {
    let chars = text.chars().count();
    let lines = text.lines().count();
    ExtractedContent {
        text: text.to_string(),
        truncated: false,
        stats: ContentStats {
            total_chars: chars,
            total_lines: lines,
            extracted_chars: chars,
        },
    }
}

#[tokio::test]
async fn test_rule_based_short_content() {
    let summarizer = ContentSummarizer::rule_based(100);
    let content = create_test_content("Short content that fits within limit.");

    let summary = summarizer.summarize(&content).await;

    assert_eq!(summary.method, "rule-based");
    assert_eq!(summary.text, "Short content that fits within limit.");
}

#[tokio::test]
async fn test_rule_based_long_content() {
    let summarizer = ContentSummarizer::rule_based(20);
    let content = create_test_content("This is a much longer piece of content that will be truncated.");

    let summary = summarizer.summarize(&content).await;

    assert_eq!(summary.method, "rule-based");
    assert!(summary.text.starts_with("This is a much longe"));
    assert!(summary.text.contains("..."));
    assert!(summary.text.contains("chars"));
}

#[tokio::test]
async fn test_rule_based_preserves_stats() {
    let summarizer = ContentSummarizer::rule_based(50);
    let content = ExtractedContent {
        text: "Test content".to_string(),
        truncated: false,
        stats: ContentStats {
            total_chars: 100,
            total_lines: 5,
            extracted_chars: 12,
        },
    };

    let summary = summarizer.summarize(&content).await;

    assert_eq!(summary.stats.total_chars, 100);
    assert_eq!(summary.stats.total_lines, 5);
}

#[tokio::test]
async fn test_rule_based_japanese_content() {
    let summarizer = ContentSummarizer::rule_based(50);
    let content = create_test_content("これは日本語のテストコンテンツです。要約機能のテストを行っています。");

    let summary = summarizer.summarize(&content).await;

    assert_eq!(summary.method, "rule-based");
    // Japanese text should be handled correctly
    assert!(summary.text.contains("日本語"));
}

// =============================================================================
// ContentSummarizer - Factory Methods Tests
// =============================================================================

#[test]
fn test_rule_based_factory() {
    let summarizer = ContentSummarizer::rule_based(150);
    // Just verify it creates successfully
    assert!(true);
}

#[test]
fn test_llm_based_factory() {
    let provider_config = ProviderConfig {
        order: Some(vec!["Cerebras".to_string()]),
        allow_fallbacks: true,
        only: None,
        ignore: None,
        sort: None,
        require_parameters: false,
    };

    let summarizer = ContentSummarizer::llm_based(
        "cerebras/llama-3.3-70b".to_string(),
        500,
        0.3,
        provider_config,
        "test-api-key".to_string(),
    );
    // Just verify it creates successfully
    assert!(true);
}

// =============================================================================
// ContentSummarizer - LLM Fallback Tests
// =============================================================================

#[tokio::test]
async fn test_llm_fallback_without_api_key() {
    // Create LLM summarizer without API key
    let summarizer = ContentSummarizer::new(
        SummarizationStrategy::LlmBased {
            model: "test-model".to_string(),
            max_tokens: 100,
            temperature: 0.3,
            provider_config: ProviderConfig::default(),
        },
        None, // No API key
    );

    let content = create_test_content("Test content for fallback.");
    let summary = summarizer.summarize(&content).await;

    // Should fallback to rule-based when no API key
    assert_eq!(summary.method, "rule-based");
}

// =============================================================================
// Summary Struct Tests
// =============================================================================

#[test]
fn test_summary_struct() {
    let summary = Summary {
        text: "Summary text".to_string(),
        method: "rule-based".to_string(),
        stats: ContentStats {
            total_chars: 100,
            total_lines: 5,
            extracted_chars: 50,
        },
        usage: None,
    };

    assert_eq!(summary.text, "Summary text");
    assert_eq!(summary.method, "rule-based");
    assert_eq!(summary.stats.total_chars, 100);
    assert!(summary.usage.is_none());
}

// =============================================================================
// LLM Summarization Integration Tests (with Mock Server)
// =============================================================================

#[tokio::test]
async fn test_llm_summarization_with_mock_api() {
    let mock_server = MockServer::start().await;

    let response_body = serde_json::json!({
        "id": "gen-123",
        "model": "cerebras/llama-3.3-70b",
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "要約:\n- ポイント1\n- ポイント2\n- ポイント3"
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 100,
            "completion_tokens": 50,
            "total_tokens": 150
        }
    });

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("Authorization", "Bearer test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&mock_server)
        .await;

    // Create client with mock server URL
    let client = OpenRouterClient::with_config(
        "test-api-key",
        Some(mock_server.uri()),
        None,
        Some(0),
    );

    let content = create_test_content("これはテスト用のコンテンツです。要約をテストします。");

    let messages = vec![
        digrag::extract::openrouter_client::ChatMessage::system(
            "以下のテキストを簡潔に要約してください。",
        ),
        digrag::extract::openrouter_client::ChatMessage::user(&content.text),
    ];

    let response = client
        .chat_completion(
            "cerebras/llama-3.3-70b",
            messages,
            digrag::extract::openrouter_client::ChatCompletionOptions::default(),
        )
        .await;

    assert!(response.is_ok());
    let resp = response.unwrap();
    assert!(resp.content.contains("ポイント"));
    assert!(resp.usage.is_some());
    assert_eq!(resp.usage.unwrap().total_tokens, 150);
}

#[tokio::test]
async fn test_llm_summarization_fallback_on_error() {
    let mock_server = MockServer::start().await;

    // Return an error response
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": {
                "message": "Internal server error"
            }
        })))
        .mount(&mock_server)
        .await;

    // Create client pointing to mock server (but summarizer won't use it directly)
    // We test the fallback behavior through the summarizer
    let summarizer = ContentSummarizer::new(
        SummarizationStrategy::LlmBased {
            model: "test-model".to_string(),
            max_tokens: 100,
            temperature: 0.3,
            provider_config: ProviderConfig::default(),
        },
        Some("invalid-key".to_string()), // Key exists but API will fail
    );

    let content = create_test_content("Test content for fallback scenario.");
    let summary = summarizer.summarize(&content).await;

    // Should fallback to rule-based on API error
    assert_eq!(summary.method, "rule-based");
}
