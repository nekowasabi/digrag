//! OpenRouter Client tests (Process 8: TDD)
//!
//! Tests for OpenRouter API HTTP client functionality

use digrag::extract::openrouter_client::{
    ChatCompletionOptions, ChatMessage, OpenRouterClient, OpenRouterError,
    UsageStats,
};
use digrag::extract::summarizer::ProviderConfig;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// =============================================================================
// Client Initialization Tests
// =============================================================================

#[test]
fn test_client_new_sets_api_key() {
    let client = OpenRouterClient::new("sk-test-key-12345");
    assert_eq!(client.api_key(), "sk-test-key-12345");
}

#[test]
fn test_client_new_sets_default_base_url() {
    let client = OpenRouterClient::new("test-key");
    assert_eq!(client.base_url(), "https://openrouter.ai/api/v1");
}

#[test]
fn test_client_with_custom_base_url() {
    let client = OpenRouterClient::with_config(
        "test-key",
        Some("http://localhost:3000".to_string()),
        None,
        None,
    );
    assert_eq!(client.base_url(), "http://localhost:3000");
}

// =============================================================================
// Request Body Building Tests
// =============================================================================

#[test]
fn test_build_request_body_minimal() {
    let client = OpenRouterClient::new("key");
    let messages = vec![ChatMessage::user("Hello, world!")];
    let options = ChatCompletionOptions::default();

    let body = client.build_request_body("gpt-4", &messages, &options);

    assert_eq!(body["model"], "gpt-4");
    assert!(body["messages"].is_array());
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["messages"][0]["content"], "Hello, world!");
}

#[test]
fn test_build_request_body_with_all_options() {
    let client = OpenRouterClient::new("key");
    let messages = vec![
        ChatMessage::system("You are a helpful assistant."),
        ChatMessage::user("Summarize this."),
    ];
    let options = ChatCompletionOptions {
        max_tokens: Some(500),
        temperature: Some(0.3),
        top_p: Some(0.95),
        provider_config: Some(ProviderConfig {
            order: Some(vec!["Cerebras".to_string(), "Together".to_string()]),
            allow_fallbacks: true,
            only: None,
            ignore: Some(vec!["OpenAI".to_string()]),
            sort: Some("price".to_string()),
            require_parameters: false,
        }),
    };

    let body = client.build_request_body("cerebras/llama-3.3-70b", &messages, &options);

    assert_eq!(body["model"], "cerebras/llama-3.3-70b");
    assert_eq!(body["max_tokens"], 500);
    // Use approximate comparison for floats
    let temp = body["temperature"].as_f64().unwrap();
    assert!((temp - 0.3).abs() < 0.01);
    let top_p = body["top_p"].as_f64().unwrap();
    assert!((top_p - 0.95).abs() < 0.01);
    assert!(body["provider"]["order"].is_array());
    assert_eq!(body["provider"]["sort"], "price");
}

#[test]
fn test_build_request_body_with_provider_only() {
    let client = OpenRouterClient::new("key");
    let messages = vec![ChatMessage::user("Test")];
    let options = ChatCompletionOptions {
        max_tokens: None,
        temperature: None,
        top_p: None,
        provider_config: Some(ProviderConfig {
            order: None,
            allow_fallbacks: false,
            only: Some(vec!["Cerebras".to_string()]),
            ignore: None,
            sort: None,
            require_parameters: true,
        }),
    };

    let body = client.build_request_body("test-model", &messages, &options);

    assert_eq!(body["provider"]["allow_fallbacks"], false);
    assert!(body["provider"]["only"].is_array());
    assert_eq!(body["provider"]["require_parameters"], true);
}

// =============================================================================
// ChatMessage Tests
// =============================================================================

#[test]
fn test_chat_message_system() {
    let msg = ChatMessage::system("You are helpful");
    assert_eq!(msg.role, "system");
    assert_eq!(msg.content, "You are helpful");
}

#[test]
fn test_chat_message_user() {
    let msg = ChatMessage::user("Hello!");
    assert_eq!(msg.role, "user");
    assert_eq!(msg.content, "Hello!");
}

#[test]
fn test_chat_message_assistant() {
    let msg = ChatMessage::assistant("Hi there!");
    assert_eq!(msg.role, "assistant");
    assert_eq!(msg.content, "Hi there!");
}

// =============================================================================
// API Response Parsing Tests (with Mock Server)
// =============================================================================

#[tokio::test]
async fn test_chat_completion_success() {
    let mock_server = MockServer::start().await;

    let response_body = serde_json::json!({
        "id": "gen-123",
        "model": "cerebras/llama-3.3-70b",
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "This is a summary of the content."
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 50,
            "completion_tokens": 20,
            "total_tokens": 70
        }
    });

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("Authorization", "Bearer test-api-key"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&mock_server)
        .await;

    let client = OpenRouterClient::with_config(
        "test-api-key",
        Some(mock_server.uri()),
        None,
        Some(0), // No retries for test
    );

    let result = client
        .chat_completion(
            "cerebras/llama-3.3-70b",
            vec![ChatMessage::user("Summarize this.")],
            ChatCompletionOptions::default(),
        )
        .await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.content, "This is a summary of the content.");
    assert_eq!(response.model, "cerebras/llama-3.3-70b");
    assert!(response.usage.is_some());
    let usage = response.usage.unwrap();
    assert_eq!(usage.prompt_tokens, 50);
    assert_eq!(usage.completion_tokens, 20);
    assert_eq!(usage.total_tokens, 70);
}

#[tokio::test]
async fn test_chat_completion_unauthorized() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": {
                "message": "Invalid API key",
                "code": "invalid_api_key"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = OpenRouterClient::with_config(
        "invalid-key",
        Some(mock_server.uri()),
        None,
        Some(0),
    );

    let result = client
        .chat_completion(
            "test-model",
            vec![ChatMessage::user("Test")],
            ChatCompletionOptions::default(),
        )
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        OpenRouterError::Unauthorized => {}
        e => panic!("Expected Unauthorized error, got: {:?}", e),
    }
}

#[tokio::test]
async fn test_chat_completion_rate_limit() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "30")
                .set_body_json(serde_json::json!({
                    "error": {
                        "message": "Rate limit exceeded"
                    }
                })),
        )
        .mount(&mock_server)
        .await;

    let client = OpenRouterClient::with_config(
        "test-key",
        Some(mock_server.uri()),
        None,
        Some(0), // No retries to test immediate error
    );

    let result = client
        .chat_completion(
            "test-model",
            vec![ChatMessage::user("Test")],
            ChatCompletionOptions::default(),
        )
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        OpenRouterError::RateLimit { retry_after_secs } => {
            assert_eq!(retry_after_secs, 30);
        }
        e => panic!("Expected RateLimit error, got: {:?}", e),
    }
}

#[tokio::test]
async fn test_chat_completion_model_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": {
                "message": "Model 'nonexistent/model' not found",
                "code": "model_not_found"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = OpenRouterClient::with_config(
        "test-key",
        Some(mock_server.uri()),
        None,
        Some(0),
    );

    let result = client
        .chat_completion(
            "nonexistent/model",
            vec![ChatMessage::user("Test")],
            ChatCompletionOptions::default(),
        )
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        OpenRouterError::ModelNotFound(msg) => {
            assert!(msg.contains("not found"));
        }
        e => panic!("Expected ModelNotFound error, got: {:?}", e),
    }
}

#[tokio::test]
async fn test_chat_completion_api_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": {
                "message": "Internal server error"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = OpenRouterClient::with_config(
        "test-key",
        Some(mock_server.uri()),
        None,
        Some(0),
    );

    let result = client
        .chat_completion(
            "test-model",
            vec![ChatMessage::user("Test")],
            ChatCompletionOptions::default(),
        )
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        OpenRouterError::Api { status, message } => {
            assert_eq!(status, 500);
            assert!(message.contains("Internal server error"));
        }
        e => panic!("Expected Api error, got: {:?}", e),
    }
}

#[tokio::test]
async fn test_chat_completion_invalid_json_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&mock_server)
        .await;

    let client = OpenRouterClient::with_config(
        "test-key",
        Some(mock_server.uri()),
        None,
        Some(0),
    );

    let result = client
        .chat_completion(
            "test-model",
            vec![ChatMessage::user("Test")],
            ChatCompletionOptions::default(),
        )
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        OpenRouterError::Parse(_) => {}
        e => panic!("Expected Parse error, got: {:?}", e),
    }
}

#[tokio::test]
async fn test_chat_completion_empty_choices() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-123",
            "choices": []
        })))
        .mount(&mock_server)
        .await;

    let client = OpenRouterClient::with_config(
        "test-key",
        Some(mock_server.uri()),
        None,
        Some(0),
    );

    let result = client
        .chat_completion(
            "test-model",
            vec![ChatMessage::user("Test")],
            ChatCompletionOptions::default(),
        )
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        OpenRouterError::Parse(msg) => {
            assert!(msg.contains("No content"));
        }
        e => panic!("Expected Parse error, got: {:?}", e),
    }
}

// =============================================================================
// Error Type Tests
// =============================================================================

#[test]
fn test_error_display_network() {
    let err = OpenRouterError::Network("Connection refused".to_string());
    assert!(err.to_string().contains("Network error"));
    assert!(err.to_string().contains("Connection refused"));
}

#[test]
fn test_error_display_api() {
    let err = OpenRouterError::Api {
        status: 400,
        message: "Bad request".to_string(),
    };
    assert!(err.to_string().contains("API error"));
    assert!(err.to_string().contains("400"));
}

#[test]
fn test_error_display_rate_limit() {
    let err = OpenRouterError::RateLimit { retry_after_secs: 60 };
    assert!(err.to_string().contains("Rate limit"));
    assert!(err.to_string().contains("60"));
}

#[test]
fn test_error_display_unauthorized() {
    let err = OpenRouterError::Unauthorized;
    assert!(err.to_string().contains("Invalid API key"));
}

#[test]
fn test_error_display_model_not_found() {
    let err = OpenRouterError::ModelNotFound("test-model".to_string());
    assert!(err.to_string().contains("Model not found"));
    assert!(err.to_string().contains("test-model"));
}

// =============================================================================
// Usage Stats Tests
// =============================================================================

#[test]
fn test_usage_stats_creation() {
    let usage = UsageStats {
        prompt_tokens: 100,
        completion_tokens: 50,
        total_tokens: 150,
    };
    assert_eq!(usage.prompt_tokens, 100);
    assert_eq!(usage.completion_tokens, 50);
    assert_eq!(usage.total_tokens, 150);
}

// =============================================================================
// ChatCompletionOptions Tests
// =============================================================================

#[test]
fn test_chat_completion_options_default() {
    let options = ChatCompletionOptions::default();
    assert!(options.max_tokens.is_none());
    assert!(options.temperature.is_none());
    assert!(options.top_p.is_none());
    assert!(options.provider_config.is_none());
}
