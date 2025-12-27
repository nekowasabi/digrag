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
