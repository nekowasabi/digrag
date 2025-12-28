//! Content summarizer
//!
//! Provides summarization strategies:
//! - RuleBased: Extract preview + statistics (no API call)
//! - LlmBased: Use OpenRouter API for LLM summarization

use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, info, warn};

use super::openrouter_client::{ChatCompletionOptions, ChatMessage, OpenRouterClient};
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
    /// Token usage (if LLM was used)
    pub usage: Option<SummaryUsage>,
}

/// Usage statistics for LLM summarization
#[derive(Debug, Clone)]
pub struct SummaryUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    pub model: String,
}

/// Content summarizer
pub struct ContentSummarizer {
    strategy: SummarizationStrategy,
    client: Option<OpenRouterClient>,
}

impl ContentSummarizer {
    /// Create a new content summarizer
    pub fn new(strategy: SummarizationStrategy, api_key: Option<String>) -> Self {
        let client = api_key.as_ref().map(|key| OpenRouterClient::new(key.clone()));
        Self {
            strategy,
            client,
        }
    }

    /// Create a rule-based summarizer
    pub fn rule_based(preview_chars: usize) -> Self {
        Self {
            strategy: SummarizationStrategy::RuleBased { preview_chars },
            client: None,
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
        let client = OpenRouterClient::new(api_key);
        Self {
            strategy: SummarizationStrategy::LlmBased {
                model,
                max_tokens,
                temperature,
                provider_config,
            },
            client: Some(client),
        }
    }

    /// Create summarizer from AppConfig
    pub fn from_config(config: &crate::config::app_config::AppConfig) -> Self {
        if config.summarization_enabled() {
            if let Some(api_key) = config.openrouter_api_key() {
                let provider_config = ProviderConfig {
                    order: config.provider_order(),
                    allow_fallbacks: config.provider_allow_fallbacks(),
                    only: config.provider_only(),
                    ignore: config.provider_ignore(),
                    sort: config.provider_sort(),
                    require_parameters: config.provider_require_parameters(),
                };

                return Self::llm_based(
                    config.summarization_model().to_string(),
                    config.summarization_max_tokens(),
                    config.summarization_temperature(),
                    provider_config,
                    api_key,
                );
            } else {
                warn!("Summarization enabled but no API key configured, falling back to rule-based");
            }
        }

        Self::rule_based(200)
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
                if let Some(ref client) = self.client {
                    let start = std::time::Instant::now();
                    match self
                        .llm_summary(
                            client,
                            content,
                            model,
                            *max_tokens,
                            *temperature,
                            provider_config,
                        )
                        .await
                    {
                        Ok(summary) => {
                            let elapsed = start.elapsed();
                            info!(
                                model = %model,
                                duration_ms = %elapsed.as_millis(),
                                "LLM summarization completed"
                            );
                            summary
                        }
                        Err(e) => {
                            warn!(error = %e, "LLM summarization failed, falling back to rule-based");
                            self.rule_based_summary(content, 200)
                        }
                    }
                } else {
                    debug!("No API client configured, using rule-based summary");
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
            format!(
                "{}...\n[{} chars, {} lines]",
                preview, content.stats.total_chars, content.stats.total_lines
            )
        } else {
            preview
        };

        Summary {
            text: summary_text,
            method: "rule-based".to_string(),
            stats: content.stats.clone(),
            usage: None,
        }
    }

    /// Generate LLM-based summary via OpenRouter
    async fn llm_summary(
        &self,
        client: &OpenRouterClient,
        content: &ExtractedContent,
        model: &str,
        max_tokens: usize,
        temperature: f32,
        provider_config: &ProviderConfig,
    ) -> Result<Summary, Box<dyn std::error::Error + Send + Sync>> {
        let system_prompt = "以下のテキストを簡潔に要約してください。重要なポイントを箇条書きで抽出してください。";

        let messages = vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(&content.text),
        ];

        let options = ChatCompletionOptions {
            max_tokens: Some(max_tokens),
            temperature: Some(temperature),
            top_p: None,
            provider_config: Some(provider_config.clone()),
        };

        debug!(model = %model, content_len = content.text.len(), "Calling LLM API");

        let response = client.chat_completion(model, messages, options).await?;

        let usage = response.usage.map(|u| SummaryUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
            model: response.model.clone(),
        });

        Ok(Summary {
            text: response.content,
            method: "llm".to_string(),
            stats: content.stats.clone(),
            usage,
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
        assert!(summary.usage.is_none());
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

    #[test]
    fn test_llm_based_factory() {
        let summarizer = ContentSummarizer::llm_based(
            "cerebras/llama-3.3-70b".to_string(),
            500,
            0.3,
            ProviderConfig::default(),
            "test-key".to_string(),
        );
        assert!(summarizer.client.is_some());
    }
}
