//! Document data structures
//!
//! Defines the core document types used throughout the search engine.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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

    /// Compute content hash from title and text
    ///
    /// Uses SHA256 hash of "title\0text" and returns first 16 hex characters.
    /// This ensures reproducible document IDs based on content only.
    pub fn compute_content_hash(title: &str, text: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(title.as_bytes());
        hasher.update(b"\0");
        hasher.update(text.as_bytes());
        let result = hasher.finalize();
        hex::encode(&result[..8]) // First 8 bytes = 16 hex chars
    }

    /// Create a document with content-based ID
    ///
    /// The document ID is computed from the content hash of title and text,
    /// making it reproducible across builds.
    pub fn with_content_id(
        title: String,
        date: DateTime<Utc>,
        tags: Vec<String>,
        text: String,
    ) -> Self {
        let id = Self::compute_content_hash(&title, &text);
        Self {
            id,
            metadata: Metadata { title, date, tags },
            text,
        }
    }

    /// Get the content hash of this document
    ///
    /// Returns hash based on title and text only (metadata excluded).
    pub fn content_hash(&self) -> String {
        Self::compute_content_hash(&self.metadata.title, &self.text)
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

    /// Get the category from the title
    ///
    /// Extracts the first part of a hierarchical title separated by " / ".
    /// For example, "Claude Code / hookタイミング" returns "Claude Code".
    ///
    /// # Returns
    /// - `Some(&str)` with the category if title is non-empty
    /// - `None` if the title is empty
    pub fn category(&self) -> Option<&str> {
        let title = self.title();
        if title.is_empty() {
            None
        } else {
            Some(title.split(" / ").next().unwrap_or(title))
        }
    }

    /// Get the subcategory from the title
    ///
    /// Extracts the second part of a hierarchical title separated by " / ".
    /// For example, "Claude Code / hookタイミング" returns "hookタイミング".
    ///
    /// # Returns
    /// - `Some(&str)` with the subcategory if title has hierarchy
    /// - `None` if title has no " / " separator or is empty
    pub fn subcategory(&self) -> Option<&str> {
        let parts: Vec<&str> = self.title().split(" / ").collect();
        if parts.len() > 1 {
            Some(parts[1])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    // Process 2: TDD Tests for category/subcategory

    #[test]
    fn test_category_with_hierarchy() {
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let doc = Document::new(
            "Claude Code / hookタイミング".to_string(),
            date,
            vec![],
            "Content".to_string(),
        );

        assert_eq!(doc.category(), Some("Claude Code"));
    }

    #[test]
    fn test_category_without_hierarchy() {
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let doc = Document::new(
            "単一カテゴリ".to_string(),
            date,
            vec![],
            "Content".to_string(),
        );

        assert_eq!(doc.category(), Some("単一カテゴリ"));
    }

    #[test]
    fn test_category_empty_title() {
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let doc = Document::new(
            "".to_string(),
            date,
            vec![],
            "Content".to_string(),
        );

        assert_eq!(doc.category(), None);
    }

    #[test]
    fn test_subcategory_with_hierarchy() {
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let doc = Document::new(
            "Claude Code / hookタイミング".to_string(),
            date,
            vec![],
            "Content".to_string(),
        );

        assert_eq!(doc.subcategory(), Some("hookタイミング"));
    }

    #[test]
    fn test_subcategory_without_hierarchy() {
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let doc = Document::new(
            "単一カテゴリ".to_string(),
            date,
            vec![],
            "Content".to_string(),
        );

        assert_eq!(doc.subcategory(), None);
    }

    #[test]
    fn test_subcategory_multiple_levels() {
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let doc = Document::new(
            "カテゴリ / サブカテゴリ / 詳細".to_string(),
            date,
            vec![],
            "Content".to_string(),
        );

        // 最初のスラッシュで分割、サブカテゴリは2番目の部分
        assert_eq!(doc.subcategory(), Some("サブカテゴリ"));
    }

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
