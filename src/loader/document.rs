//! Document data structures
//!
//! Defines the core document types used throughout the search engine.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Document metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metadata {
    /// Document title
    pub title: String,
    /// Document date
    pub date: DateTime<Utc>,
    /// Document tags
    pub tags: Vec<String>,
}

/// A document in the search index
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    /// Unique document identifier
    pub id: String,
    /// Document metadata
    pub metadata: Metadata,
    /// Document text content
    pub text: String,
}

impl Document {
    /// Create a new document with a generated UUID
    pub fn new(title: String, date: DateTime<Utc>, tags: Vec<String>, text: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            metadata: Metadata { title, date, tags },
            text,
        }
    }

    /// Create a document with a specific ID
    pub fn with_id(
        id: String,
        title: String,
        date: DateTime<Utc>,
        tags: Vec<String>,
        text: String,
    ) -> Self {
        Self {
            id,
            metadata: Metadata { title, date, tags },
            text,
        }
    }

    /// Get the document title
    pub fn title(&self) -> &str {
        &self.metadata.title
    }

    /// Get the document date
    pub fn date(&self) -> DateTime<Utc> {
        self.metadata.date
    }

    /// Get the document tags
    pub fn tags(&self) -> &[String] {
        &self.metadata.tags
    }

    /// Check if document has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.metadata.tags.iter().any(|t| t == tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_document_creation() {
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let doc = Document::new(
            "Test Entry".to_string(),
            date,
            vec!["memo".to_string(), "worklog".to_string()],
            "Content line".to_string(),
        );

        assert!(!doc.id.is_empty());
        assert_eq!(doc.title(), "Test Entry");
        assert_eq!(doc.date(), date);
        assert_eq!(doc.tags(), &["memo", "worklog"]);
        assert_eq!(doc.text, "Content line");
    }

    #[test]
    fn test_document_with_id() {
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let doc = Document::with_id(
            "custom-id".to_string(),
            "Test Entry".to_string(),
            date,
            vec!["memo".to_string()],
            "Content".to_string(),
        );

        assert_eq!(doc.id, "custom-id");
    }

    #[test]
    fn test_document_has_tag() {
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let doc = Document::new(
            "Test".to_string(),
            date,
            vec!["memo".to_string(), "worklog".to_string()],
            "Content".to_string(),
        );

        assert!(doc.has_tag("memo"));
        assert!(doc.has_tag("worklog"));
        assert!(!doc.has_tag("tips"));
    }

    #[test]
    fn test_document_serialization() {
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let doc = Document::with_id(
            "test-id".to_string(),
            "Test Entry".to_string(),
            date,
            vec!["memo".to_string()],
            "Content".to_string(),
        );

        let json = serde_json::to_string(&doc).unwrap();
        let deserialized: Document = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized, doc);
    }

    #[test]
    fn test_metadata_serialization() {
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let metadata = Metadata {
            title: "Test".to_string(),
            date,
            tags: vec!["memo".to_string()],
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: Metadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized, metadata);
    }
}
