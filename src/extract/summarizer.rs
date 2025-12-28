//! Content summarizer
//!
//! Provides summarization strategies:
//! - RuleBased: Extract preview + statistics (no API call)
//! - LlmBased: Use OpenRouter API for LLM summarization

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{ContentStats, ExtractedContent};

/// OpenRouter provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider priority order
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,
    /// Allow fallback to other providers
    #[serde(default = "default_true")]
    pub allow_fallbacks: bool,
    /// Only use these providers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only: Option<Vec<String>>,
    /// Ignore these providers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
    /// Sort by: "price" or "throughput"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    /// Require full parameter support
    #[serde(default)]
    pub require_parameters: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            order: None,
            allow_fallbacks: true,
            only: None,
            ignore: None,
            sort: None,
            require_parameters: false,
        }
    }
}

impl ProviderConfig {
    /// Convert to JSON for API request
    pub fn to_json(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();

        if let Some(ref order) = self.order {
            obj.insert("order".to_string(), json!(order));
        }

        obj.insert("allow_fallbacks".to_string(), json!(self.allow_fallbacks));

        if let Some(ref only) = self.only {
            obj.insert("only".to_string(), json!(only));
        }

        if let Some(ref ignore) = self.ignore {
            obj.insert("ignore".to_string(), json!(ignore));
        }

        if let Some(ref sort) = self.sort {
            obj.insert("sort".to_string(), json!(sort));
        }

        if self.require_parameters {
            obj.insert("require_parameters".to_string(), json!(true));
        }

        serde_json::Value::Object(obj)
    }
}

/// Summarization strategy
#[derive(Debug, Clone)]
pub enum SummarizationStrategy {
    /// Rule-based summarization (preview + stats)
    RuleBased {
        /// Number of preview characters
        preview_chars: usize,
    },
    /// LLM-based summarization via OpenRouter
    LlmBased {
        /// Model identifier (e.g., "cerebras/llama-3.3-70b")
        model: String,
        /// Maximum tokens for summary
        max_tokens: usize,
        /// Temperature for generation
        temperature: f32,
        /// Provider configuration
        provider_config: ProviderConfig,
    },
}

impl Default for SummarizationStrategy {
    fn default() -> Self {
        Self::RuleBased { preview_chars: 200 }
    }
}

/// Summary result
#[derive(Debug, Clone)]
pub struct Summary {
    /// Summary text
    pub text: String,
    /// Method used: "rule-based" or "llm"
    pub method: String,
    /// Content statistics
    pub stats: ContentStats,
}

/// Content summarizer
pub struct ContentSummarizer {
    strategy: SummarizationStrategy,
    api_key: Option<String>,
}

impl ContentSummarizer {
    /// Create a new content summarizer
    pub fn new(strategy: SummarizationStrategy, api_key: Option<String>) -> Self {
        Self { strategy, api_key }
    }

    /// Create a rule-based summarizer
    pub fn rule_based(preview_chars: usize) -> Self {
        Self {
            strategy: SummarizationStrategy::RuleBased { preview_chars },
            api_key: None,
        }
    }

    /// Create an LLM-based summarizer
    pub fn llm_based(
        model: String,
        max_tokens: usize,
        temperature: f32,
        provider_config: ProviderConfig,
        api_key: String,
    ) -> Self {
        Self {
            strategy: SummarizationStrategy::LlmBased {
                model,
                max_tokens,
                temperature,
                provider_config,
            },
            api_key: Some(api_key),
        }
    }

    /// Generate summary (async for LLM, sync-compatible for rule-based)
    pub async fn summarize(&self, content: &ExtractedContent) -> Summary {
        match &self.strategy {
            SummarizationStrategy::RuleBased { preview_chars } => {
                self.rule_based_summary(content, *preview_chars)
            }
            SummarizationStrategy::LlmBased {
                model,
                max_tokens,
                temperature,
                provider_config,
            } => {
                if let Some(ref api_key) = self.api_key {
                    match self
                        .llm_summary(
                            content,
                            model,
                            *max_tokens,
                            *temperature,
                            provider_config,
                            api_key,
                        )
                        .await
                    {
                        Ok(summary) => summary,
                        Err(_) => {
                            // Fallback to rule-based on error
                            self.rule_based_summary(content, 200)
                        }
                    }
                } else {
                    // No API key, use rule-based
                    self.rule_based_summary(content, 200)
                }
            }
        }
    }

    /// Generate rule-based summary
    fn rule_based_summary(&self, content: &ExtractedContent, preview_chars: usize) -> Summary {
        let preview: String = content.text.chars().take(preview_chars).collect();
        let truncated = content.text.chars().count() > preview_chars;

        let summary_text = if truncated {
            format!("{}...\n[{} chars, {} lines]",
                preview,
                content.stats.total_chars,
                content.stats.total_lines
            )
        } else {
            preview
        };

        Summary {
            text: summary_text,
            method: "rule-based".to_string(),
            stats: content.stats.clone(),
        }
    }

    /// Generate LLM-based summary via OpenRouter
    async fn llm_summary(
        &self,
        content: &ExtractedContent,
        model: &str,
        max_tokens: usize,
        temperature: f32,
        provider_config: &ProviderConfig,
        api_key: &str,
    ) -> Result<Summary, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();

        let request_body = json!({
            "model": model,
            "messages": [
                {
                    "role": "system",
                    "content": "以下のテキストを簡潔に要約してください。重要なポイントを箇条書きで抽出してください。"
                },
                {
                    "role": "user",
                    "content": content.text
                }
            ],
            "max_tokens": max_tokens,
            "temperature": temperature,
            "provider": provider_config.to_json()
        });

        let response = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let response_json: serde_json::Value = response.json().await?;

        let summary_text = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("Summary generation failed")
            .to_string();

        Ok(Summary {
            text: summary_text,
            method: "llm".to_string(),
            stats: content.stats.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_config_default() {
        let config = ProviderConfig::default();
        assert!(config.allow_fallbacks);
        assert!(config.order.is_none());
    }

    #[test]
    fn test_provider_config_to_json() {
        let config = ProviderConfig {
            order: Some(vec!["Cerebras".to_string(), "Together".to_string()]),
            allow_fallbacks: true,
            only: None,
            ignore: None,
            sort: Some("price".to_string()),
            require_parameters: false,
        };

        let json = config.to_json();
        assert_eq!(json["order"], json!(["Cerebras", "Together"]));
        assert_eq!(json["allow_fallbacks"], json!(true));
        assert_eq!(json["sort"], json!("price"));
    }

    #[test]
    fn test_rule_based_summary_short() {
        let summarizer = ContentSummarizer::rule_based(100);
        let content = ExtractedContent {
            text: "Short content".to_string(),
            truncated: false,
            stats: ContentStats {
                total_chars: 13,
                total_lines: 1,
                extracted_chars: 13,
            },
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let summary = rt.block_on(summarizer.summarize(&content));

        assert_eq!(summary.method, "rule-based");
        assert_eq!(summary.text, "Short content");
    }

    #[test]
    fn test_rule_based_summary_long() {
        let summarizer = ContentSummarizer::rule_based(10);
        let content = ExtractedContent {
            text: "This is a longer piece of content that will be truncated".to_string(),
            truncated: false,
            stats: ContentStats {
                total_chars: 55,
                total_lines: 1,
                extracted_chars: 55,
            },
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let summary = rt.block_on(summarizer.summarize(&content));

        assert_eq!(summary.method, "rule-based");
        assert!(summary.text.starts_with("This is a "));
        assert!(summary.text.contains("..."));
    }

    #[test]
    fn test_summarization_strategy_default() {
        let strategy = SummarizationStrategy::default();
        match strategy {
            SummarizationStrategy::RuleBased { preview_chars } => {
                assert_eq!(preview_chars, 200);
            }
            _ => panic!("Expected RuleBased strategy"),
        }
    }
}
