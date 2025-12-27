//! Document loading module
//!
//! This module provides functionality for loading and parsing changelog documents.

mod changelog;
mod document;
mod jsonl;

pub use changelog::ChangelogLoader;
pub use document::{Document, Metadata};
pub use jsonl::JsonlLoader;
