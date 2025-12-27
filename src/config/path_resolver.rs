//! Path resolution module for digrag
//!
//! Provides utilities for resolving file paths with support for:
//! - Absolute paths (returned as-is)
//! - Tilde (~) expansion to home directory
//! - Relative paths (resolved from current directory)
//! - XDG Base Directory specification compliance

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// Expand tilde (~) in path to home directory
pub fn expand_home(path: &str) -> Result<PathBuf> {
    if let Some(stripped) = path.strip_prefix('~') {
        let home = std::env::var("HOME")
            .map_err(|_| anyhow!("HOME environment variable not set"))?;
        if stripped.is_empty() {
            Ok(PathBuf::from(home))
        } else if stripped.starts_with('/') {
            Ok(PathBuf::from(format!("{}{}", home, stripped)))
        } else {
            // ~username format not supported, return as-is
            Ok(PathBuf::from(path))
        }
    } else {
        Ok(PathBuf::from(path))
    }
}

/// Resolve a path to an absolute path
/// 
/// Resolution order:
/// 1. Expand ~ to home directory
/// 2. If absolute, return as-is
/// 3. If relative, resolve from current directory
pub fn resolve_path(path: &str) -> Result<PathBuf> {
    let expanded = expand_home(path)?;
    
    if expanded.is_absolute() {
        Ok(expanded)
    } else {
        // Resolve relative path from current directory
        let current_dir = std::env::current_dir()
            .map_err(|e| anyhow!("Failed to get current directory: {}", e))?;
        Ok(current_dir.join(expanded))
    }
}

/// Get the XDG config directory for digrag
/// 
/// Returns: $XDG_CONFIG_HOME/digrag or ~/.config/digrag
pub fn get_config_dir() -> PathBuf {
    if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg_config).join("digrag")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config").join("digrag")
    } else {
        PathBuf::from(".config").join("digrag")
    }
}

/// Get the XDG data directory for digrag
/// 
/// Returns: $XDG_DATA_HOME/digrag or ~/.local/share/digrag
pub fn get_data_dir() -> PathBuf {
    if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(xdg_data).join("digrag")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".local").join("share").join("digrag")
    } else {
        PathBuf::from(".local").join("share").join("digrag")
    }
}

/// Get the XDG cache directory for digrag
/// 
/// Returns: $XDG_CACHE_HOME/digrag or ~/.cache/digrag
pub fn get_cache_dir() -> PathBuf {
    if let Ok(xdg_cache) = std::env::var("XDG_CACHE_HOME") {
        PathBuf::from(xdg_cache).join("digrag")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".cache").join("digrag")
    } else {
        PathBuf::from(".cache").join("digrag")
    }
}

/// Get the default config file path
pub fn get_default_config_path() -> PathBuf {
    get_config_dir().join("config.toml")
}

/// Get the directory of the current executable
pub fn get_exe_dir() -> Result<PathBuf> {
    let exe_path = std::env::current_exe()
        .map_err(|e| anyhow!("Failed to get executable path: {}", e))?;
    exe_path.parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| anyhow!("Executable has no parent directory"))
}

/// Find project root by looking for .git directory
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        
        if !current.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_home_with_tilde() {
        let result = expand_home("~").unwrap();
        assert!(!result.to_str().unwrap().contains('~'));
    }

    #[test]
    fn test_expand_home_with_subdir() {
        let result = expand_home("~/test").unwrap();
        assert!(result.to_str().unwrap().ends_with("/test"));
    }

    #[test]
    fn test_expand_home_absolute() {
        let result = expand_home("/absolute/path").unwrap();
        assert_eq!(result.to_str().unwrap(), "/absolute/path");
    }

    #[test]
    fn test_get_config_dir_contains_digrag() {
        let dir = get_config_dir();
        assert!(dir.to_str().unwrap().contains("digrag"));
    }

    #[test]
    fn test_get_data_dir_contains_digrag() {
        let dir = get_data_dir();
        assert!(dir.to_str().unwrap().contains("digrag"));
    }

    #[test]
    fn test_resolve_absolute_path() {
        let result = resolve_path("/tmp").unwrap();
        assert_eq!(result.to_str().unwrap(), "/tmp");
    }
}
