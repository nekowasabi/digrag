//! Path resolution module tests (Process 1: Red Phase)
//!
//! Test cases for the path resolution functionality:
//! 1. Absolute paths should be returned as-is
//! 2. Tilde (~) should expand to home directory
//! 3. Relative paths should resolve from current directory
//! 4. Non-existent paths should return appropriate errors

use digrag::config::path_resolver::{expand_home, get_config_dir, get_data_dir, resolve_path};
use tempfile::TempDir;

#[test]
fn test_absolute_path_unchanged() {
    let path = "/tmp/test.txt";
    let resolved = resolve_path(path).unwrap();
    assert_eq!(resolved.to_str().unwrap(), path);
}

#[test]
fn test_tilde_expansion() {
    let path = "~/config";
    let resolved = expand_home(path).unwrap();
    let home = std::env::var("HOME").unwrap();
    assert_eq!(resolved.to_str().unwrap(), format!("{}/config", home));
}

#[test]
fn test_tilde_only() {
    let path = "~";
    let resolved = expand_home(path).unwrap();
    let home = std::env::var("HOME").unwrap();
    assert_eq!(resolved.to_str().unwrap(), home);
}

#[test]
fn test_relative_path_from_current_dir() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "test").unwrap();

    // Change to temp dir and resolve relative path
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    let resolved = resolve_path("test.txt").unwrap();
    assert!(resolved.exists());
    assert!(resolved.is_absolute());

    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_config_dir_xdg() {
    let config_dir = get_config_dir();
    assert!(config_dir.ends_with("digrag") || config_dir.to_str().unwrap().contains("digrag"));
}

#[test]
fn test_data_dir_xdg() {
    let data_dir = get_data_dir();
    assert!(data_dir.ends_with("digrag") || data_dir.to_str().unwrap().contains("digrag"));
}

#[test]
fn test_resolve_existing_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("existing.txt");
    std::fs::write(&file_path, "content").unwrap();

    let resolved = resolve_path(file_path.to_str().unwrap()).unwrap();
    assert!(resolved.exists());
}

#[test]
fn test_resolve_with_tilde_and_subdirs() {
    let path = "~/.config/digrag/config.toml";
    let resolved = expand_home(path).unwrap();
    let home = std::env::var("HOME").unwrap();
    assert!(resolved.to_str().unwrap().starts_with(&home));
    assert!(resolved.to_str().unwrap().ends_with("config.toml"));
}
