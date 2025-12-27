//! Configuration module for digrag
//!
//! This module defines configuration structures for search modes and options.

mod search_config;
pub mod path_resolver;
pub mod app_config;

pub use search_config::{SearchConfig, SearchMode};
