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

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            index_dir: default_index_dir(),
            openrouter_api_key: None,
            default_top_k: default_top_k(),
            default_search_mode: default_search_mode(),
        }
    }
}

impl AppConfig {
    /// Create config from a TOML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read config file {}: {}", path.display(), e))?;
        let config: AppConfig = toml::from_str(&content)
            .map_err(|e| anyhow!("Failed to parse config file: {}", e))?;
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
            openrouter_api_key: other.openrouter_api_key.clone()
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
        
        Ok(())
    }
    
    /// Serialize to TOML string
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self)
            .map_err(|e| anyhow!("Failed to serialize config: {}", e))
    }
    
    // Getters
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
