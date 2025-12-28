//! Configuration module tests (Process 2: Red Phase)
//!
//! Test cases for configuration file and environment variable support:
//! 1. Load config from TOML file
//! 2. Environment variable override
//! 3. Default values
//! 4. Priority: CLI > ENV > Config > Default

use digrag::config::app_config::AppConfig;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_default_config() {
    let config = AppConfig::default();
    assert_eq!(config.index_dir(), ".rag");
}

#[test]
fn test_load_from_toml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    
    std::fs::write(&config_path, r#"
index_dir = "/custom/index"
openrouter_api_key = "test-key"
default_top_k = 20
"#).unwrap();
    
    let config = AppConfig::from_file(&config_path).unwrap();
    assert_eq!(config.index_dir(), "/custom/index");
    assert_eq!(config.openrouter_api_key(), Some("test-key".to_string()));
    assert_eq!(config.default_top_k(), 20);
}

#[test]
fn test_env_override() {
    std::env::set_var("DIGRAG_INDEX_DIR", "/env/index");
    
    let config = AppConfig::from_env();
    assert_eq!(config.index_dir(), "/env/index");
    
    std::env::remove_var("DIGRAG_INDEX_DIR");
}

#[test]
fn test_merge_priority() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    
    std::fs::write(&config_path, r#"
index_dir = "/file/index"
default_top_k = 15
"#).unwrap();
    
    std::env::set_var("DIGRAG_INDEX_DIR", "/env/index");
    
    let file_config = AppConfig::from_file(&config_path).unwrap();
    let env_config = AppConfig::from_env();
    let merged = file_config.merge_with(&env_config);
    
    // ENV should override file
    assert_eq!(merged.index_dir(), "/env/index");
    // File value should be preserved where ENV is not set
    assert_eq!(merged.default_top_k(), 15);
    
    std::env::remove_var("DIGRAG_INDEX_DIR");
}

#[test]
fn test_config_with_cli_override() {
    let base_config = AppConfig::default();
    let with_override = base_config.with_index_dir("/cli/index");
    
    assert_eq!(with_override.index_dir(), "/cli/index");
}

#[test]
fn test_validate_config() {
    let config = AppConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_serialization() {
    let config = AppConfig::default()
        .with_index_dir("/test/index")
        .with_default_top_k(25);
    
    let toml_str = config.to_toml().unwrap();
    assert!(toml_str.contains("index_dir"));
    assert!(toml_str.contains("/test/index"));
}

#[test]
fn test_missing_file_returns_error() {
    let result = AppConfig::from_file(&PathBuf::from("/nonexistent/config.toml"));
    assert!(result.is_err());
}

// =============================================================================
// Process 1: Content Extraction Configuration Tests (TDD Red Phase)
// =============================================================================

#[test]
fn test_extraction_config_defaults() {
    let config = AppConfig::default();

    // 抽出設定のデフォルト値を確認
    assert_eq!(config.extraction_mode(), "snippet");
    assert_eq!(config.extraction_max_chars(), 5000);
    assert!(config.extraction_include_summary());
    assert!(config.extraction_include_raw());
}

#[test]
fn test_extraction_config_from_toml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    std::fs::write(&config_path, r#"
index_dir = ".rag"
extraction_mode = "entry"
extraction_max_chars = 10000
extraction_include_summary = true
extraction_include_raw = false
"#).unwrap();

    let config = AppConfig::from_file(&config_path).unwrap();
    assert_eq!(config.extraction_mode(), "entry");
    assert_eq!(config.extraction_max_chars(), 10000);
    assert!(config.extraction_include_summary());
    assert!(!config.extraction_include_raw());
}

#[test]
fn test_extraction_mode_env_override() {
    std::env::set_var("DIGRAG_EXTRACTION_MODE", "full");
    std::env::set_var("DIGRAG_EXTRACTION_MAX_CHARS", "8000");

    let config = AppConfig::from_env();
    assert_eq!(config.extraction_mode(), "full");
    assert_eq!(config.extraction_max_chars(), 8000);

    std::env::remove_var("DIGRAG_EXTRACTION_MODE");
    std::env::remove_var("DIGRAG_EXTRACTION_MAX_CHARS");
}

#[test]
fn test_summarization_config_defaults() {
    let config = AppConfig::default();

    // LLM要約設定のデフォルト値
    assert!(!config.summarization_enabled());
    assert_eq!(config.summarization_model(), "cerebras/llama-3.3-70b");
    assert_eq!(config.summarization_max_tokens(), 500);
    assert!((config.summarization_temperature() - 0.3).abs() < 0.001);
}

#[test]
fn test_summarization_config_from_toml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    std::fs::write(&config_path, r#"
index_dir = ".rag"
summarization_enabled = true
summarization_model = "anthropic/claude-3-haiku"
summarization_max_tokens = 1000
summarization_temperature = 0.5
"#).unwrap();

    let config = AppConfig::from_file(&config_path).unwrap();
    assert!(config.summarization_enabled());
    assert_eq!(config.summarization_model(), "anthropic/claude-3-haiku");
    assert_eq!(config.summarization_max_tokens(), 1000);
    assert!((config.summarization_temperature() - 0.5).abs() < 0.001);
}

#[test]
fn test_provider_config_defaults() {
    let config = AppConfig::default();

    // OpenRouterプロバイダー設定のデフォルト値
    assert!(config.provider_order().is_none());
    assert!(config.provider_allow_fallbacks());
    assert!(config.provider_only().is_none());
    assert!(config.provider_ignore().is_none());
    assert!(config.provider_sort().is_none());
    assert!(!config.provider_require_parameters());
}

#[test]
fn test_provider_config_from_toml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    std::fs::write(&config_path, r#"
index_dir = ".rag"
provider_order = ["Cerebras", "Together"]
provider_allow_fallbacks = false
provider_only = ["Cerebras"]
provider_ignore = ["OpenAI"]
provider_sort = "price"
provider_require_parameters = true
"#).unwrap();

    let config = AppConfig::from_file(&config_path).unwrap();
    assert_eq!(config.provider_order(), Some(vec!["Cerebras".to_string(), "Together".to_string()]));
    assert!(!config.provider_allow_fallbacks());
    assert_eq!(config.provider_only(), Some(vec!["Cerebras".to_string()]));
    assert_eq!(config.provider_ignore(), Some(vec!["OpenAI".to_string()]));
    assert_eq!(config.provider_sort(), Some("price".to_string()));
    assert!(config.provider_require_parameters());
}

#[test]
fn test_provider_order_env_override() {
    std::env::set_var("DIGRAG_PROVIDER_ORDER", "Cerebras,Together,Fireworks");
    std::env::set_var("DIGRAG_PROVIDER_ALLOW_FALLBACKS", "false");

    let config = AppConfig::from_env();
    assert_eq!(config.provider_order(), Some(vec![
        "Cerebras".to_string(),
        "Together".to_string(),
        "Fireworks".to_string()
    ]));
    assert!(!config.provider_allow_fallbacks());

    std::env::remove_var("DIGRAG_PROVIDER_ORDER");
    std::env::remove_var("DIGRAG_PROVIDER_ALLOW_FALLBACKS");
}

#[test]
fn test_validate_extraction_mode() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // 有効なモード
    for mode in &["snippet", "entry", "full"] {
        std::fs::write(&config_path, format!(r#"
index_dir = ".rag"
extraction_mode = "{}"
"#, mode)).unwrap();

        let config = AppConfig::from_file(&config_path).unwrap();
        assert!(config.validate().is_ok(), "Mode '{}' should be valid", mode);
    }
}

#[test]
fn test_validate_invalid_extraction_mode() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    std::fs::write(&config_path, r#"
index_dir = ".rag"
extraction_mode = "invalid_mode"
"#).unwrap();

    let config = AppConfig::from_file(&config_path).unwrap();
    assert!(config.validate().is_err());
}

#[test]
fn test_full_config_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    std::fs::write(&config_path, r#"
index_dir = "/custom/index"
openrouter_api_key = "test-key"
default_top_k = 20
default_search_mode = "hybrid"

extraction_mode = "entry"
extraction_max_chars = 8000
extraction_include_summary = true
extraction_include_raw = true

summarization_enabled = true
summarization_model = "cerebras/llama-3.3-70b"
summarization_max_tokens = 500
summarization_temperature = 0.3

provider_order = ["Cerebras", "Together"]
provider_allow_fallbacks = true
"#).unwrap();

    let config = AppConfig::from_file(&config_path).unwrap();
    assert!(config.validate().is_ok());

    // Serialize and verify
    let toml_str = config.to_toml().unwrap();
    assert!(toml_str.contains("extraction_mode"));
    assert!(toml_str.contains("summarization_model"));
    assert!(toml_str.contains("provider_order"));
}
