//! OpenRouter Embedding API client
//!
//! Provides embedding generation using OpenRouter's API.

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Default embedding model
const DEFAULT_MODEL: &str = "openai/text-embedding-3-small";

/// API base URL
const BASE_URL: &str = "https://openrouter.ai/api/v1";

/// Maximum characters per text for embedding
/// text-embedding-3-small has 8192 token limit
/// Japanese text averages ~1.3 tokens per character, so we use 6000 chars to be safe
const MAX_TEXT_CHARS: usize = 6000;

/// Request payload for embedding API
#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
}

/// Response from embedding API
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
    #[allow(dead_code)]
    model: Option<String>,
    #[allow(dead_code)]
    usage: Option<EmbeddingUsage>,
}

/// Individual embedding data
#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    #[allow(dead_code)]
    index: usize,
}

/// Usage information from embedding API
#[derive(Debug, Deserialize)]
struct EmbeddingUsage {
    #[allow(dead_code)]
    prompt_tokens: Option<u32>,
    #[allow(dead_code)]
    total_tokens: Option<u32>,
}

/// Error response from embedding API
#[derive(Debug, Deserialize)]
struct EmbeddingErrorResponse {
    error: EmbeddingError,
}

/// Error details
#[derive(Debug, Deserialize)]
struct EmbeddingError {
    message: String,
    #[allow(dead_code)]
    code: Option<i32>,
}

/// OpenRouter embedding client
pub struct OpenRouterEmbedding {
    /// API key
    api_key: String,
    /// API base URL
    base_url: String,
    /// Model to use
    model: String,
    /// HTTP client
    client: Client,
}

impl OpenRouterEmbedding {
    /// Create a new OpenRouter embedding client
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: BASE_URL.to_string(),
            model: DEFAULT_MODEL.to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Create with custom model
    pub fn with_model(api_key: String, model: String) -> Self {
        Self {
            api_key,
            base_url: BASE_URL.to_string(),
            model,
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Create with custom base URL (for testing with mock servers)
    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
            model: DEFAULT_MODEL.to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Create with custom model and base URL (for testing with mock servers)
    pub fn with_model_and_base_url(api_key: String, model: String, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
            model,
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Generate embedding for a single text
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let results = self.embed_batch(&[text.to_string()]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No embedding returned"))
    }

    /// Truncate text to fit within token limits and handle empty texts
    fn truncate_text(text: &str) -> String {
        let trimmed = text.trim();
        // Handle empty text - use placeholder to avoid API error
        if trimmed.is_empty() {
            return "(empty)".to_string();
        }
        if trimmed.chars().count() <= MAX_TEXT_CHARS {
            trimmed.to_string()
        } else {
            // Truncate to MAX_TEXT_CHARS and add indicator
            let truncated: String = trimmed.chars().take(MAX_TEXT_CHARS - 3).collect();
            format!("{}...", truncated)
        }
    }

    /// Generate embeddings for multiple texts with retry logic
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // Truncate texts that exceed the maximum length
        let truncated_texts: Vec<String> = texts.iter().map(|t| Self::truncate_text(t)).collect();

        let request = EmbeddingRequest {
            model: self.model.clone(),
            input: truncated_texts,
        };

        let url = format!("{}/embeddings", self.base_url);

        // Retry with exponential backoff
        let max_retries = 3;
        let mut last_error = None;

        for attempt in 0..max_retries {
            if attempt > 0 {
                let delay = Duration::from_millis(1000 * 2u64.pow(attempt as u32));
                tokio::time::sleep(delay).await;
            }

            let response = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .header("HTTP-Referer", "https://github.com/takets/changelog")
                .header("X-Title", "cl-search")
                .json(&request)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();

                    if status.is_success() {
                        // Get response body as text first for better error diagnostics
                        let body_text = match resp.text().await {
                            Ok(text) => text,
                            Err(e) => {
                                last_error = Some(anyhow!("Failed to read response body: {}", e));
                                continue;
                            }
                        };

                        // Try to parse as successful response
                        match serde_json::from_str::<EmbeddingResponse>(&body_text) {
                            Ok(embedding_response) => {
                                let mut embeddings: Vec<Vec<f32>> = embedding_response
                                    .data
                                    .into_iter()
                                    .map(|d| d.embedding)
                                    .collect();

                                // Ensure correct order
                                embeddings.sort_by_key(|_| 0); // Already in order from API

                                return Ok(embeddings);
                            }
                            Err(parse_err) => {
                                // Try to parse as error response
                                if let Ok(error_response) =
                                    serde_json::from_str::<EmbeddingErrorResponse>(&body_text)
                                {
                                    return Err(anyhow!(
                                        "API error: {}",
                                        error_response.error.message
                                    ));
                                }
                                // If both fail, return parsing error with body preview
                                let preview: String = body_text.chars().take(300).collect();
                                return Err(anyhow!(
                                    "Failed to parse response: {}. Body preview: {}",
                                    parse_err,
                                    preview
                                ));
                            }
                        }
                    } else if status.as_u16() == 429 {
                        // Rate limited, retry
                        last_error = Some(anyhow!("Rate limited (429)"));
                        continue;
                    } else {
                        // Try to parse error response from non-200 status
                        match resp.text().await {
                            Ok(error_text) => {
                                if let Ok(error_response) =
                                    serde_json::from_str::<EmbeddingErrorResponse>(&error_text)
                                {
                                    return Err(anyhow!(
                                        "API error {}: {}",
                                        status,
                                        error_response.error.message
                                    ));
                                }
                                return Err(anyhow!("API error {}: {}", status, error_text));
                            }
                            Err(e) => {
                                return Err(anyhow!(
                                    "API error {} (failed to read body: {})",
                                    status,
                                    e
                                ));
                            }
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(anyhow!("Request failed: {}", e));
                    continue;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("Max retries exceeded")))
    }

    /// Get the API key
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Get the model
    pub fn model(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openrouter_embedding_creation() {
        let client = OpenRouterEmbedding::new("test-key".to_string());
        assert_eq!(client.api_key(), "test-key");
        assert_eq!(client.model(), "openai/text-embedding-3-small");
    }

    #[test]
    fn test_openrouter_embedding_with_model() {
        let client =
            OpenRouterEmbedding::with_model("test-key".to_string(), "custom-model".to_string());
        assert_eq!(client.model(), "custom-model");
    }

    // TODO: Add more tests in Process 10
}
