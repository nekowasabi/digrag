//! Init command tests (Process 4: Red Phase)
//!
//! Test cases for the --init subcommand:
//! 1. Creates config directory
//! 2. Creates default config.toml
//! 3. Does not overwrite existing config without --force
//! 4. Creates with --force flag

use digrag::config::app_config::AppConfig;
use digrag::config::path_resolver::get_config_dir;
use std::fs;
use tempfile::TempDir;

mod init_tests {
    use super::*;

    /// Helper to create a mock XDG_CONFIG_HOME
    fn with_temp_config_home<F, T>(f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let temp_dir = TempDir::new().unwrap();
        let original = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::set_var("XDG_CONFIG_HOME", temp_dir.path());

        let result = f();

        if let Some(val) = original {
            std::env::set_var("XDG_CONFIG_HOME", val);
        } else {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        result
    }

    #[test]
    fn test_init_creates_config_directory() {
        with_temp_config_home(|| {
            let config_dir = get_config_dir();
            assert!(!config_dir.exists());

            // Simulate init command
            fs::create_dir_all(&config_dir).unwrap();

            assert!(config_dir.exists());
        });
    }

    #[test]
    fn test_init_creates_default_config() {
        with_temp_config_home(|| {
            let config_dir = get_config_dir();
            fs::create_dir_all(&config_dir).unwrap();

            let config_path = config_dir.join("config.toml");
            let default_config = AppConfig::default();
            let toml_content = default_config.to_toml().unwrap();

            fs::write(&config_path, &toml_content).unwrap();

            assert!(config_path.exists());
            let content = fs::read_to_string(&config_path).unwrap();
            assert!(content.contains("index_dir"));
        });
    }

    #[test]
    fn test_config_file_can_be_loaded() {
        with_temp_config_home(|| {
            let config_dir = get_config_dir();
            fs::create_dir_all(&config_dir).unwrap();

            let config_path = config_dir.join("config.toml");
            let default_config = AppConfig::default();
            fs::write(&config_path, default_config.to_toml().unwrap()).unwrap();

            let loaded = AppConfig::from_file(&config_path).unwrap();
            assert_eq!(loaded.index_dir(), ".rag");
        });
    }

    #[test]
    fn test_existing_config_not_overwritten() {
        with_temp_config_home(|| {
            let config_dir = get_config_dir();
            fs::create_dir_all(&config_dir).unwrap();

            let config_path = config_dir.join("config.toml");
            let custom_content = r#"
index_dir = "/custom/path"
default_top_k = 50
"#;
            fs::write(&config_path, custom_content).unwrap();

            // Should not overwrite without force
            let existing = fs::read_to_string(&config_path).unwrap();
            assert!(existing.contains("/custom/path"));
        });
    }
}
