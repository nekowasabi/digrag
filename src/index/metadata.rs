//! Index Metadata implementation
//!
//! Provides metadata storage for index with schema versioning and document hashes.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Current schema version for incremental build support
pub const CURRENT_SCHEMA_VERSION: &str = "2.0";

/// Index metadata with schema versioning and document hashes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    /// Number of documents
    pub doc_count: usize,
    /// Index creation timestamp
    pub created_at: String,
    /// Model used for embeddings
    pub embedding_model: Option<String>,
    /// Schema version for compatibility checking
    #[serde(default)]
    pub schema_version: String,
    /// Map of document ID to content hash for incremental builds
    #[serde(default)]
    pub doc_hashes: HashMap<String, String>,
}

impl IndexMetadata {
    /// Create new metadata with current schema version
    pub fn new(doc_count: usize, embedding_model: Option<String>) -> Self {
        Self {
            doc_count,
            created_at: chrono::Utc::now().to_rfc3339(),
            embedding_model,
            schema_version: CURRENT_SCHEMA_VERSION.to_string(),
            doc_hashes: HashMap::new(),
        }
    }

    /// Check if a full rebuild is needed (old or missing schema version)
    pub fn needs_full_rebuild(&self) -> bool {
        if self.schema_version.is_empty() {
            return true;
        }

        // Parse version and compare
        let version = self.schema_version.parse::<f32>().unwrap_or(0.0);
        version < 2.0
    }

    /// Update or insert a document hash
    pub fn update_doc_hash(&mut self, doc_id: String, hash: String) {
        self.doc_hashes.insert(doc_id, hash);
    }

    /// Remove a document hash
    pub fn remove_doc_hash(&mut self, doc_id: &str) {
        self.doc_hashes.remove(doc_id);
    }

    /// Get a document hash
    pub fn get_doc_hash(&self, doc_id: &str) -> Option<&String> {
        self.doc_hashes.get(doc_id)
    }

    /// Save metadata to file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load metadata from file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let metadata: Self = serde_json::from_str(&content)?;
        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_metadata() {
        let metadata = IndexMetadata::new(10, Some("model".to_string()));
        assert_eq!(metadata.doc_count, 10);
        assert_eq!(metadata.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(metadata.doc_hashes.is_empty());
    }

    #[test]
    fn test_needs_full_rebuild() {
        let old = IndexMetadata {
            doc_count: 0,
            created_at: String::new(),
            embedding_model: None,
            schema_version: "1.0".to_string(),
            doc_hashes: HashMap::new(),
        };
        assert!(old.needs_full_rebuild());

        let current = IndexMetadata::new(0, None);
        assert!(!current.needs_full_rebuild());
    }
}
