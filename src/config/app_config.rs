//! Application configuration module for digrag
//!
//! Provides TOML-based configuration with environment variable override support.
//! Priority: CLI args > Environment variables > Config file > Defaults

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Directory for search indices (default: .rag)
    #[serde(default = "default_index_dir")]
    index_dir: String,

    /// OpenRouter API key for semantic search
    #[serde(default)]
    openrouter_api_key: Option<String>,

    /// Default number of results to return
    #[serde(default = "default_top_k")]
    default_top_k: usize,

    /// Default search mode: bm25, semantic, or hybrid
    #[serde(default = "default_search_mode")]
    default_search_mode: String,

    // =========================================================================
    // Content Extraction Settings
    // =========================================================================
    /// Extraction mode: "snippet" (default), "entry", or "full"
    #[serde(default = "default_extraction_mode")]
    extraction_mode: String,

    /// Maximum characters to extract (default: 5000)
    #[serde(default = "default_extraction_max_chars")]
    extraction_max_chars: usize,

    /// Include summary in output (default: true)
    #[serde(default = "default_true")]
    extraction_include_summary: bool,

    /// Include raw content in output (default: true)
    #[serde(default = "default_true")]
    extraction_include_raw: bool,

    // =========================================================================
    // LLM Summarization Settings
    // =========================================================================
    /// Enable LLM-based summarization (default: false)
    #[serde(default)]
    summarization_enabled: bool,

    /// LLM model for summarization (default: "cerebras/llama-3.3-70b")
    #[serde(default = "default_summarization_model")]
    summarization_model: String,

    /// Max tokens for summarization (default: 500)
    #[serde(default = "default_summarization_max_tokens")]
    summarization_max_tokens: usize,

    /// Temperature for summarization (default: 0.3)
    #[serde(default = "default_summarization_temperature")]
    summarization_temperature: f32,

    // =========================================================================
    // OpenRouter Provider Settings
    // =========================================================================
    /// Provider priority order (e.g., ["Cerebras", "Together"])
    #[serde(default)]
    provider_order: Option<Vec<String>>,

    /// Allow fallback to other providers (default: true)
    #[serde(default = "default_true")]
    provider_allow_fallbacks: bool,

    /// Only use these providers
    #[serde(default)]
    provider_only: Option<Vec<String>>,

    /// Ignore these providers
    #[serde(default)]
    provider_ignore: Option<Vec<String>>,

    /// Sort providers by: "price" or "throughput"
    #[serde(default)]
    provider_sort: Option<String>,

    /// Require full parameter support from provider
    #[serde(default)]
    provider_require_parameters: bool,
}

fn default_index_dir() -> String {
    ".rag".to_string()
}

fn default_top_k() -> usize {
    10
}

fn default_search_mode() -> String {
    "bm25".to_string()
}

fn default_extraction_mode() -> String {
    "snippet".to_string()
}

fn default_extraction_max_chars() -> usize {
    5000
}

fn default_true() -> bool {
    true
}

fn default_summarization_model() -> String {
    "cerebras/llama-3.3-70b".to_string()
}

fn default_summarization_max_tokens() -> usize {
    500
}

fn default_summarization_temperature() -> f32 {
    0.3
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            index_dir: default_index_dir(),
            openrouter_api_key: None,
            default_top_k: default_top_k(),
            default_search_mode: default_search_mode(),
            // Extraction settings
            extraction_mode: default_extraction_mode(),
            extraction_max_chars: default_extraction_max_chars(),
            extraction_include_summary: default_true(),
            extraction_include_raw: default_true(),
            // Summarization settings
            summarization_enabled: false,
            summarization_model: default_summarization_model(),
            summarization_max_tokens: default_summarization_max_tokens(),
            summarization_temperature: default_summarization_temperature(),
            // Provider settings
            provider_order: None,
            provider_allow_fallbacks: default_true(),
            provider_only: None,
            provider_ignore: None,
            provider_sort: None,
            provider_require_parameters: false,
        }
    }
}

impl AppConfig {
    /// Create config from a TOML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read config file {}: {}", path.display(), e))?;
        let config: AppConfig =
            toml::from_str(&content).map_err(|e| anyhow!("Failed to parse config file: {}", e))?;
        Ok(config)
    }

    /// Create config from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(index_dir) = std::env::var("DIGRAG_INDEX_DIR") {
            config.index_dir = index_dir;
        }

        if let Ok(api_key) = std::env::var("DIGRAG_OPENROUTER_API_KEY") {
            config.openrouter_api_key = Some(api_key);
        } else if let Ok(api_key) = std::env::var("OPENROUTER_API_KEY") {
            config.openrouter_api_key = Some(api_key);
        }

        if let Ok(top_k) = std::env::var("DIGRAG_TOP_K") {
            if let Ok(n) = top_k.parse() {
                config.default_top_k = n;
            }
        }

        if let Ok(mode) = std::env::var("DIGRAG_SEARCH_MODE") {
            config.default_search_mode = mode;
        }

        // Extraction settings from env
        if let Ok(mode) = std::env::var("DIGRAG_EXTRACTION_MODE") {
            config.extraction_mode = mode;
        }

        if let Ok(max_chars) = std::env::var("DIGRAG_EXTRACTION_MAX_CHARS") {
            if let Ok(n) = max_chars.parse() {
                config.extraction_max_chars = n;
            }
        }

        // Summarization settings from env
        if let Ok(enabled) = std::env::var("DIGRAG_SUMMARIZATION_ENABLED") {
            config.summarization_enabled = enabled.to_lowercase() == "true" || enabled == "1";
        }

        if let Ok(model) = std::env::var("DIGRAG_SUMMARIZATION_MODEL") {
            config.summarization_model = model;
        }

        // Provider settings from env
        if let Ok(order) = std::env::var("DIGRAG_PROVIDER_ORDER") {
            config.provider_order = Some(order.split(',').map(|s| s.trim().to_string()).collect());
        }

        if let Ok(fallbacks) = std::env::var("DIGRAG_PROVIDER_ALLOW_FALLBACKS") {
            config.provider_allow_fallbacks =
                fallbacks.to_lowercase() == "true" || fallbacks == "1";
        }

        config
    }

    /// Merge with another config (other takes priority for non-default values)
    pub fn merge_with(&self, other: &Self) -> Self {
        Self {
            index_dir: if other.index_dir != default_index_dir() {
                other.index_dir.clone()
            } else {
                self.index_dir.clone()
            },
            openrouter_api_key: other
                .openrouter_api_key
                .clone()
                .or_else(|| self.openrouter_api_key.clone()),
            default_top_k: if other.default_top_k != default_top_k() {
                other.default_top_k
            } else {
                self.default_top_k
            },
            default_search_mode: if other.default_search_mode != default_search_mode() {
                other.default_search_mode.clone()
            } else {
                self.default_search_mode.clone()
            },
            // Extraction settings
            extraction_mode: if other.extraction_mode != default_extraction_mode() {
                other.extraction_mode.clone()
            } else {
                self.extraction_mode.clone()
            },
            extraction_max_chars: if other.extraction_max_chars != default_extraction_max_chars() {
                other.extraction_max_chars
            } else {
                self.extraction_max_chars
            },
            extraction_include_summary: other.extraction_include_summary,
            extraction_include_raw: other.extraction_include_raw,
            // Summarization settings
            summarization_enabled: other.summarization_enabled || self.summarization_enabled,
            summarization_model: if other.summarization_model != default_summarization_model() {
                other.summarization_model.clone()
            } else {
                self.summarization_model.clone()
            },
            summarization_max_tokens: if other.summarization_max_tokens
                != default_summarization_max_tokens()
            {
                other.summarization_max_tokens
            } else {
                self.summarization_max_tokens
            },
            summarization_temperature: if (other.summarization_temperature
                - default_summarization_temperature())
            .abs()
                > 0.001
            {
                other.summarization_temperature
            } else {
                self.summarization_temperature
            },
            // Provider settings
            provider_order: other
                .provider_order
                .clone()
                .or_else(|| self.provider_order.clone()),
            provider_allow_fallbacks: other.provider_allow_fallbacks,
            provider_only: other
                .provider_only
                .clone()
                .or_else(|| self.provider_only.clone()),
            provider_ignore: other
                .provider_ignore
                .clone()
                .or_else(|| self.provider_ignore.clone()),
            provider_sort: other
                .provider_sort
                .clone()
                .or_else(|| self.provider_sort.clone()),
            provider_require_parameters: other.provider_require_parameters
                || self.provider_require_parameters,
        }
    }

    /// Override index_dir
    pub fn with_index_dir(mut self, dir: &str) -> Self {
        self.index_dir = dir.to_string();
        self
    }

    /// Override default_top_k
    pub fn with_default_top_k(mut self, k: usize) -> Self {
        self.default_top_k = k;
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.default_top_k == 0 {
            return Err(anyhow!("default_top_k must be greater than 0"));
        }

        let valid_modes = ["bm25", "semantic", "hybrid"];
        if !valid_modes.contains(&self.default_search_mode.as_str()) {
            return Err(anyhow!(
                "Invalid search mode '{}'. Valid modes: {:?}",
                self.default_search_mode,
                valid_modes
            ));
        }

        // Validate extraction mode
        let valid_extraction_modes = ["snippet", "entry", "full"];
        if !valid_extraction_modes.contains(&self.extraction_mode.as_str()) {
            return Err(anyhow!(
                "Invalid extraction mode '{}'. Valid modes: {:?}",
                self.extraction_mode,
                valid_extraction_modes
            ));
        }

        Ok(())
    }

    /// Serialize to TOML string
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self).map_err(|e| anyhow!("Failed to serialize config: {}", e))
    }

    // Getters - Basic settings
    pub fn index_dir(&self) -> &str {
        &self.index_dir
    }

    pub fn openrouter_api_key(&self) -> Option<String> {
        self.openrouter_api_key.clone()
    }

    pub fn default_top_k(&self) -> usize {
        self.default_top_k
    }

    pub fn default_search_mode(&self) -> &str {
        &self.default_search_mode
    }

    // Getters - Extraction settings
    pub fn extraction_mode(&self) -> &str {
        &self.extraction_mode
    }

    pub fn extraction_max_chars(&self) -> usize {
        self.extraction_max_chars
    }

    pub fn extraction_include_summary(&self) -> bool {
        self.extraction_include_summary
    }

    pub fn extraction_include_raw(&self) -> bool {
        self.extraction_include_raw
    }

    // Getters - Summarization settings
    pub fn summarization_enabled(&self) -> bool {
        self.summarization_enabled
    }

    pub fn summarization_model(&self) -> &str {
        &self.summarization_model
    }

    pub fn summarization_max_tokens(&self) -> usize {
        self.summarization_max_tokens
    }

    pub fn summarization_temperature(&self) -> f32 {
        self.summarization_temperature
    }

    // Getters - Provider settings
    pub fn provider_order(&self) -> Option<Vec<String>> {
        self.provider_order.clone()
    }

    pub fn provider_allow_fallbacks(&self) -> bool {
        self.provider_allow_fallbacks
    }

    pub fn provider_only(&self) -> Option<Vec<String>> {
        self.provider_only.clone()
    }

    pub fn provider_ignore(&self) -> Option<Vec<String>> {
        self.provider_ignore.clone()
    }

    pub fn provider_sort(&self) -> Option<String> {
        self.provider_sort.clone()
    }

    pub fn provider_require_parameters(&self) -> bool {
        self.provider_require_parameters
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.index_dir(), ".rag");
        assert_eq!(config.default_top_k(), 10);
        assert_eq!(config.default_search_mode(), "bm25");
    }

    #[test]
    fn test_validate_valid_config() {
        let config = AppConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_top_k() {
        let config = AppConfig::default().with_default_top_k(0);
        assert!(config.validate().is_err());
    }
}
