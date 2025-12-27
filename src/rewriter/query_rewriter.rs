//! Query rewriter implementation
//!
//! Uses LLM to optimize queries for search.

use super::RewriteCache;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

/// System prompt for query rewriting
const SYSTEM_PROMPT: &str = r#"You are a query optimizer for a Japanese changelog/memo search system.
Your task is to rewrite the user's search query to improve search results.

Rules:
1. Expand abbreviations and acronyms
2. Add relevant synonyms or related terms
3. Keep the query focused on the main intent
4. Output ONLY the rewritten query, nothing else
5. Preserve important Japanese terms
6. If the query is already good, return it unchanged

Example:
Input: "MCP server"
Output: "MCP Model Context Protocol サーバー 実装"

Input: "rust bm25"
Output: "Rust BM25 検索 インデックス 実装"
"#;

/// Chat completion request
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    temperature: f32,
}

/// Chat message
#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

/// Chat completion response
#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

/// Response choice
#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

/// Response message
#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

/// Query rewriter using LLM
pub struct QueryRewriter {
    /// API key for OpenRouter
    api_key: String,
    /// Cache for rewrites
    cache: Option<RewriteCache>,
    /// Model to use
    model: String,
    /// HTTP client
    client: Client,
    /// Base URL
    base_url: String,
}

impl QueryRewriter {
    /// Create a new query rewriter
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            cache: None,
            model: "anthropic/claude-3.5-haiku".to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: "https://openrouter.ai/api/v1".to_string(),
        }
    }

    /// Create with cache
    pub fn with_cache<P: AsRef<Path>>(api_key: String, cache_path: P) -> Result<Self> {
        let cache = RewriteCache::new(cache_path)?;
        Ok(Self {
            api_key,
            cache: Some(cache),
            model: "anthropic/claude-3.5-haiku".to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: "https://openrouter.ai/api/v1".to_string(),
        })
    }

    /// Set custom model
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    /// Rewrite a query for better search results
    pub async fn rewrite(&self, query: &str) -> Result<String> {
        // Check cache first
        if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get(query)? {
                return Ok(cached);
            }
        }

        // Call LLM for rewriting
        let rewritten = self
            .call_llm(query)
            .await
            .unwrap_or_else(|_| query.to_string());

        // Cache the result
        if let Some(cache) = &self.cache {
            cache.set(query, &rewritten)?;
        }

        Ok(rewritten)
    }

    /// Call LLM API for query rewriting
    async fn call_llm(&self, query: &str) -> Result<String> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: SYSTEM_PROMPT.to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: query.to_string(),
                },
            ],
            max_tokens: 100,
            temperature: 0.3,
        };

        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/takets/changelog")
            .header("X-Title", "cl-search")
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            let chat_response: ChatResponse = response.json().await?;
            if let Some(choice) = chat_response.choices.first() {
                return Ok(choice.message.content.trim().to_string());
            }
        }

        Err(anyhow!("Failed to get LLM response"))
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
    fn test_query_rewriter_creation() {
        let rewriter = QueryRewriter::new("test-key".to_string());
        assert_eq!(rewriter.model(), "anthropic/claude-3.5-haiku");
    }

    #[test]
    fn test_query_rewriter_with_model() {
        let rewriter = QueryRewriter::new("test-key".to_string()).with_model("custom".to_string());
        assert_eq!(rewriter.model(), "custom");
    }

    #[tokio::test]
    async fn test_query_rewriter_passthrough() {
        let rewriter = QueryRewriter::new("test-key".to_string());
        let result = rewriter.rewrite("test query").await.unwrap();
        // Currently just passes through
        assert_eq!(result, "test query");
    }

    // TODO: Add more tests in Process 11
}
