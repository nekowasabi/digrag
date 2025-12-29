//! Incremental Diff implementation
//!
//! Provides functionality to compute the difference between new documents
//! and existing index for incremental builds.

use crate::loader::Document;
use std::collections::HashMap;

/// Result of computing the difference between new documents and existing index
#[derive(Debug, Clone)]
pub struct IncrementalDiff {
    /// Documents that are new (not in existing index)
    pub added: Vec<Document>,
    /// Documents that exist but have been modified
    pub modified: Vec<Document>,
    /// Document IDs that were in existing index but not in new documents
    pub removed: Vec<String>,
    /// Document IDs that are unchanged
    pub unchanged: Vec<String>,
}

impl IncrementalDiff {
    /// Compute the difference between new documents and existing index
    ///
    /// # Arguments
    /// * `new_docs` - The new set of documents to be indexed
    /// * `existing_hashes` - Map of doc_id -> content_hash from existing index
    ///
    /// # Returns
    /// An IncrementalDiff containing added, modified, removed, and unchanged documents
    pub fn compute(new_docs: Vec<Document>, existing_hashes: &HashMap<String, String>) -> Self {
        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut unchanged = Vec::new();

        // Track which existing IDs we've seen
        let mut seen_ids: Vec<String> = Vec::new();

        for doc in new_docs {
            if let Some(existing_hash) = existing_hashes.get(&doc.id) {
                // Document exists in index
                seen_ids.push(doc.id.clone());
                let current_hash = doc.content_hash();
                if &current_hash == existing_hash {
                    // Content unchanged
                    unchanged.push(doc.id);
                } else {
                    // Content modified
                    modified.push(doc);
                }
            } else {
                // New document
                added.push(doc);
            }
        }

        // Find removed documents (in existing but not in new)
        let removed: Vec<String> = existing_hashes
            .keys()
            .filter(|id| !seen_ids.contains(id))
            .cloned()
            .collect();

        Self {
            added,
            modified,
            removed,
            unchanged,
        }
    }

    /// Get count of added documents
    pub fn added_count(&self) -> usize {
        self.added.len()
    }

    /// Get count of modified documents
    pub fn modified_count(&self) -> usize {
        self.modified.len()
    }

    /// Get count of removed documents
    pub fn removed_count(&self) -> usize {
        self.removed.len()
    }

    /// Get count of unchanged documents
    pub fn unchanged_count(&self) -> usize {
        self.unchanged.len()
    }

    /// Get count of documents that need new embeddings (added + modified)
    pub fn embeddings_needed(&self) -> usize {
        self.added.len() + self.modified.len()
    }

    /// Get all documents that need new embeddings
    pub fn needs_embedding(&self) -> Vec<&Document> {
        self.added.iter().chain(self.modified.iter()).collect()
    }

    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        !self.added.is_empty() || !self.modified.is_empty() || !self.removed.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn create_doc(title: &str, text: &str) -> Document {
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        Document::with_content_id(title.to_string(), date, vec![], text.to_string())
    }

    #[test]
    fn test_empty_diff() {
        let diff = IncrementalDiff::compute(vec![], &HashMap::new());
        assert!(!diff.has_changes());
    }

    #[test]
    fn test_all_new() {
        let docs = vec![create_doc("Title", "Text")];
        let diff = IncrementalDiff::compute(docs, &HashMap::new());

        assert_eq!(diff.added_count(), 1);
        assert!(diff.has_changes());
    }
}
