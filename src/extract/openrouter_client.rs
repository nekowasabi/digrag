//! OpenRouter API HTTP client
//!
//! Provides a structured HTTP client for OpenRouter API with:
//! - Bearer token authentication
//! - Request/response serialization
//! - Error handling with network vs API error distinction
//! - Retry logic with exponential backoff

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use thiserror::Error;

use super::summarizer::ProviderConfig;

// =============================================================================
// Error Types
// =============================================================================

/// OpenRouter API errors
#[derive(Debug, Error)]
pub enum OpenRouterError {
    /// Network error (connection failed, timeout, etc.)
    #[error("Network error: {0}")]
    Network(String),

    /// API error (4xx/5xx responses)
    #[error("API error (status {status}): {message}")]
    Api { status: u16, message: String },

    /// Response parsing error
    #[error("Failed to parse response: {0}")]
    Parse(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded, retry after {retry_after_secs} seconds")]
    RateLimit { retry_after_secs: u64 },

    /// Invalid API key
    #[error("Invalid API key")]
    Unauthorized,

    /// Model not found
    #[error("Model not found: {0}")]
    ModelNotFound(String),
}

impl From<reqwest::Error> for OpenRouterError {
    fn from(err: reqwest::Error) -> Self {
        OpenRouterError::Network(err.to_string())
    }
}

// =============================================================================
// API Types
// =============================================================================

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// Chat completion options
#[derive(Debug, Clone, Default)]
pub struct ChatCompletionOptions {
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub provider_config: Option<ProviderConfig>,
}

/// Chat completion response
#[derive(Debug, Clone)]
pub struct ChatCompletionResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<UsageStats>,
    pub finish_reason: Option<String>,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

// =============================================================================
// Internal API Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
struct ApiResponse {
    choices: Option<Vec<Choice>>,
    model: Option<String>,
    usage: Option<ApiUsage>,
    error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Option<ChoiceMessage>,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiUsage {
    prompt_tokens: Option<usize>,
    completion_tokens: Option<usize>,
    total_tokens: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    message: String,
    code: Option<String>,
}

// =============================================================================
// OpenRouter Client
// =============================================================================

/// OpenRouter API client
pub struct OpenRouterClient {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    timeout: Duration,
    max_retries: u32,
}

impl OpenRouterClient {
    /// OpenRouter API base URL
    pub const DEFAULT_BASE_URL: &'static str = "https://openrouter.ai/api/v1";

    /// Create a new OpenRouter client
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            base_url: Self::DEFAULT_BASE_URL.to_string(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
        }
    }

    /// Create client with custom configuration
    pub fn with_config(
        api_key: impl Into<String>,
        base_url: Option<String>,
        timeout: Option<Duration>,
        max_retries: Option<u32>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            base_url: base_url.unwrap_or_else(|| Self::DEFAULT_BASE_URL.to_string()),
            timeout: timeout.unwrap_or(Duration::from_secs(30)),
            max_retries: max_retries.unwrap_or(3),
        }
    }

    /// Build request body for chat completion
    pub fn build_request_body(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: &ChatCompletionOptions,
    ) -> serde_json::Value {
        let mut body = json!({
            "model": model,
            "messages": messages,
        });

        if let Some(max_tokens) = options.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }

        if let Some(temperature) = options.temperature {
            body["temperature"] = json!(temperature);
        }

        if let Some(top_p) = options.top_p {
            body["top_p"] = json!(top_p);
        }

        if let Some(ref provider_config) = options.provider_config {
            body["provider"] = provider_config.to_json();
        }

        body
    }

    /// Send chat completion request
    pub async fn chat_completion(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        options: ChatCompletionOptions,
    ) -> Result<ChatCompletionResponse, OpenRouterError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = self.build_request_body(model, &messages, &options);

        let mut last_error = None;
        let mut retry_count = 0;

        while retry_count <= self.max_retries {
            match self.send_request(&url, &body).await {
                Ok(response) => return Ok(response),
                Err(OpenRouterError::RateLimit { retry_after_secs }) => {
                    // Exponential backoff with rate limit hint
                    let wait_time = std::cmp::max(retry_after_secs, 2_u64.pow(retry_count));
                    tokio::time::sleep(Duration::from_secs(wait_time)).await;
                    retry_count += 1;
                    last_error = Some(OpenRouterError::RateLimit { retry_after_secs });
                }
                Err(OpenRouterError::Network(msg)) if retry_count < self.max_retries => {
                    // Retry on network errors
                    let wait_time = 2_u64.pow(retry_count);
                    tokio::time::sleep(Duration::from_secs(wait_time)).await;
                    retry_count += 1;
                    last_error = Some(OpenRouterError::Network(msg));
                }
                Err(e) => return Err(e),
            }
        }

        Err(last_error.unwrap_or(OpenRouterError::Network("Max retries exceeded".to_string())))
    }

    async fn send_request(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<ChatCompletionResponse, OpenRouterError> {
        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/takets/digrag")
            .header("X-Title", "digrag")
            .timeout(self.timeout)
            .json(body)
            .send()
            .await?;

        let status = response.status();

        // Handle rate limiting
        if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(60);
            return Err(OpenRouterError::RateLimit {
                retry_after_secs: retry_after,
            });
        }

        // Handle unauthorized
        if status.as_u16() == 401 {
            return Err(OpenRouterError::Unauthorized);
        }

        let response_text = response.text().await?;
        let api_response: ApiResponse = serde_json::from_str(&response_text)
            .map_err(|e| OpenRouterError::Parse(format!("{}: {}", e, response_text)))?;

        // Check for API error in response body
        if let Some(error) = api_response.error {
            let status_code = status.as_u16();
            if error.code.as_deref() == Some("model_not_found") {
                return Err(OpenRouterError::ModelNotFound(error.message));
            }
            return Err(OpenRouterError::Api {
                status: status_code,
                message: error.message,
            });
        }

        // Handle non-success status
        if !status.is_success() {
            return Err(OpenRouterError::Api {
                status: status.as_u16(),
                message: response_text,
            });
        }

        // Extract response content
        let content = api_response
            .choices
            .as_ref()
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.message.as_ref())
            .and_then(|msg| msg.content.clone())
            .ok_or_else(|| OpenRouterError::Parse("No content in response".to_string()))?;

        let finish_reason = api_response
            .choices
            .as_ref()
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.finish_reason.clone());

        let usage = api_response.usage.map(|u| UsageStats {
            prompt_tokens: u.prompt_tokens.unwrap_or(0),
            completion_tokens: u.completion_tokens.unwrap_or(0),
            total_tokens: u.total_tokens.unwrap_or(0),
        });

        Ok(ChatCompletionResponse {
            content,
            model: api_response.model.unwrap_or_default(),
            usage,
            finish_reason,
        })
    }

    /// Get API key
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Get base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_new() {
        let client = OpenRouterClient::new("test-api-key");
        assert_eq!(client.api_key(), "test-api-key");
        assert_eq!(client.base_url(), OpenRouterClient::DEFAULT_BASE_URL);
    }

    #[test]
    fn test_client_with_config() {
        let client = OpenRouterClient::with_config(
            "custom-key",
            Some("http://localhost:8080".to_string()),
            Some(Duration::from_secs(60)),
            Some(5),
        );
        assert_eq!(client.api_key(), "custom-key");
        assert_eq!(client.base_url(), "http://localhost:8080");
    }

    #[test]
    fn test_chat_message_constructors() {
        let system = ChatMessage::system("System prompt");
        assert_eq!(system.role, "system");
        assert_eq!(system.content, "System prompt");

        let user = ChatMessage::user("User message");
        assert_eq!(user.role, "user");

        let assistant = ChatMessage::assistant("Assistant response");
        assert_eq!(assistant.role, "assistant");
    }

    #[test]
    fn test_build_request_body_minimal() {
        let client = OpenRouterClient::new("key");
        let messages = vec![ChatMessage::user("Hello")];
        let options = ChatCompletionOptions::default();

        let body = client.build_request_body("test-model", &messages, &options);

        assert_eq!(body["model"], "test-model");
        assert!(body["messages"].is_array());
        assert!(body.get("max_tokens").is_none());
    }

    #[test]
    fn test_build_request_body_with_options() {
        let client = OpenRouterClient::new("key");
        let messages = vec![
            ChatMessage::system("You are helpful"),
            ChatMessage::user("Hello"),
        ];
        let options = ChatCompletionOptions {
            max_tokens: Some(100),
            temperature: Some(0.7),
            top_p: Some(0.9),
            provider_config: Some(ProviderConfig {
                order: Some(vec!["Cerebras".to_string()]),
                allow_fallbacks: true,
                only: None,
                ignore: None,
                sort: None,
                require_parameters: false,
            }),
        };

        let body = client.build_request_body("cerebras/llama-3.3-70b", &messages, &options);

        assert_eq!(body["model"], "cerebras/llama-3.3-70b");
        assert_eq!(body["max_tokens"], 100);
        // Use approximate comparison for floats
        let temp = body["temperature"].as_f64().unwrap();
        assert!((temp - 0.7).abs() < 0.01);
        let top_p = body["top_p"].as_f64().unwrap();
        assert!((top_p - 0.9).abs() < 0.01);
        assert!(body["provider"].is_object());
    }

    #[test]
    fn test_error_from_reqwest() {
        // Just test that the From trait is implemented
        // We can't easily create a reqwest::Error for testing
    }
}
