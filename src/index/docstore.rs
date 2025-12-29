//! Document Store implementation
//!
//! Provides document storage and retrieval.

use crate::loader::Document;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Document store for retrieving full document content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Docstore {
    /// Documents indexed by ID
    documents: HashMap<String, Document>,
}

impl Default for Docstore {
    fn default() -> Self {
        Self::new()
    }
}

impl Docstore {
    /// Create a new empty document store
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    /// Add a document to the store
    pub fn add(&mut self, doc: Document) {
        self.documents.insert(doc.id.clone(), doc);
    }

    /// Get a document by ID
    pub fn get(&self, doc_id: &str) -> Option<&Document> {
        self.documents.get(doc_id)
    }

    /// Check if a document exists
    pub fn contains(&self, doc_id: &str) -> bool {
        self.documents.contains_key(doc_id)
    }

    /// Get all document IDs
    pub fn doc_ids(&self) -> Vec<&String> {
        self.documents.keys().collect()
    }

    /// Get all documents
    pub fn documents(&self) -> &HashMap<String, Document> {
        &self.documents
    }

    /// Get documents filtered by tag
    pub fn get_by_tag(&self, tag: &str) -> Vec<&Document> {
        self.documents
            .values()
            .filter(|doc| doc.has_tag(tag))
            .collect()
    }

    /// Get all unique tags
    pub fn get_all_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self
            .documents
            .values()
            .flat_map(|doc| doc.tags().iter().cloned())
            .collect();
        tags.sort();
        tags.dedup();
        tags
    }

    /// Get recent documents (sorted by date descending)
    pub fn get_recent(&self, limit: usize) -> Vec<&Document> {
        let mut docs: Vec<_> = self.documents.values().collect();
        docs.sort_by_key(|d| std::cmp::Reverse(d.date()));
        docs.into_iter().take(limit).collect()
    }

    /// Save store to file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load store from file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read docstore from {:?}", path))?;

        let store = serde_json::from_str(&content).with_context(|| "Failed to parse docstore")?;
        Ok(store)
    }

    /// Remove a document by ID
    pub fn remove(&mut self, doc_id: &str) {
        self.documents.remove(doc_id);
    }

    /// Remove multiple documents by ID
    pub fn remove_batch(&mut self, doc_ids: &[String]) {
        for doc_id in doc_ids {
            self.documents.remove(doc_id);
        }
    }

    /// Get document count
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Check if store is empty
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_doc(id: &str, title: &str, tags: Vec<&str>, days_ago: i64) -> Document {
        let date = Utc::now() - chrono::Duration::days(days_ago);
        Document::with_id(
            id.to_string(),
            title.to_string(),
            date,
            tags.into_iter().map(|s| s.to_string()).collect(),
            "Content".to_string(),
        )
    }

    #[test]
    fn test_docstore_creation() {
        let store = Docstore::new();
        assert!(store.is_empty());
    }

    #[test]
    fn test_add_and_get() {
        let mut store = Docstore::new();
        let doc = create_test_doc("doc1", "Test", vec!["memo"], 0);
        store.add(doc.clone());

        let retrieved = store.get("doc1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title(), "Test");
    }

    #[test]
    fn test_contains() {
        let mut store = Docstore::new();
        store.add(create_test_doc("doc1", "Test", vec!["memo"], 0));

        assert!(store.contains("doc1"));
        assert!(!store.contains("doc2"));
    }

    #[test]
    fn test_get_by_tag() {
        let mut store = Docstore::new();
        store.add(create_test_doc("doc1", "Test 1", vec!["memo"], 0));
        store.add(create_test_doc("doc2", "Test 2", vec!["worklog"], 0));
        store.add(create_test_doc(
            "doc3",
            "Test 3",
            vec!["memo", "worklog"],
            0,
        ));

        let memo_docs = store.get_by_tag("memo");
        assert_eq!(memo_docs.len(), 2);

        let worklog_docs = store.get_by_tag("worklog");
        assert_eq!(worklog_docs.len(), 2);
    }

    #[test]
    fn test_get_all_tags() {
        let mut store = Docstore::new();
        store.add(create_test_doc("doc1", "Test 1", vec!["memo", "tips"], 0));
        store.add(create_test_doc("doc2", "Test 2", vec!["worklog"], 0));

        let tags = store.get_all_tags();
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"memo".to_string()));
        assert!(tags.contains(&"tips".to_string()));
        assert!(tags.contains(&"worklog".to_string()));
    }

    #[test]
    fn test_get_recent() {
        let mut store = Docstore::new();
        store.add(create_test_doc("doc1", "Oldest", vec!["memo"], 10));
        store.add(create_test_doc("doc2", "Middle", vec!["memo"], 5));
        store.add(create_test_doc("doc3", "Newest", vec!["memo"], 0));

        let recent = store.get_recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].title(), "Newest");
        assert_eq!(recent[1].title(), "Middle");
    }

    #[test]
    fn test_docstore_serialization() {
        let mut store = Docstore::new();
        store.add(create_test_doc("doc1", "Test", vec!["memo"], 0));

        let json = serde_json::to_string(&store).unwrap();
        let deserialized: Docstore = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.len(), 1);
        assert!(deserialized.contains("doc1"));
    }

    // TODO: Add more tests in Process 7
}
