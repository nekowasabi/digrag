//! JSONL document loader
//!
//! Provides functionality to load documents from JSONL format (one JSON object per line).
//! This is used for stdin input and pipe-based workflows.

use super::Document;
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Read};

/// JSONL loader for reading documents from a reader
pub struct JsonlLoader;

impl JsonlLoader {
    /// Load documents from a reader (e.g., stdin)
    ///
    /// Each line should be a valid JSON representation of a Document.
    /// Empty lines and lines starting with # are skipped.
    pub fn load_from_reader<R: Read>(reader: R) -> Result<Vec<Document>> {
        let buf_reader = BufReader::new(reader);
        let mut documents = Vec::new();
        let mut line_number = 0;

        for line_result in buf_reader.lines() {
            line_number += 1;
            let line = line_result.context(format!("Failed to read line {}", line_number))?;

            // Skip empty lines and comments
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Parse JSON
            let doc: Document = serde_json::from_str(trimmed)
                .with_context(|| format!("Failed to parse JSON at line {}: {}", line_number, trimmed))?;

            documents.push(doc);
        }

        Ok(documents)
    }

    /// Load documents from a string
    pub fn load_from_string(content: &str) -> Result<Vec<Document>> {
        Self::load_from_reader(content.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_load_single_document() {
        let jsonl = r#"{"id":"doc1","metadata":{"title":"Test Title","date":"2025-01-15T10:00:00Z","tags":["memo"]},"text":"Test content"}"#;

        let docs = JsonlLoader::load_from_string(jsonl).unwrap();

        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id, "doc1");
        assert_eq!(docs[0].title(), "Test Title");
        assert_eq!(docs[0].tags(), &["memo"]);
        assert_eq!(docs[0].text, "Test content");
    }

    #[test]
    fn test_load_multiple_documents() {
        let jsonl = r#"{"id":"doc1","metadata":{"title":"First","date":"2025-01-15T10:00:00Z","tags":[]},"text":"First content"}
{"id":"doc2","metadata":{"title":"Second","date":"2025-01-14T09:00:00Z","tags":["worklog"]},"text":"Second content"}"#;

        let docs = JsonlLoader::load_from_string(jsonl).unwrap();

        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].title(), "First");
        assert_eq!(docs[1].title(), "Second");
    }

    #[test]
    fn test_skip_empty_lines() {
        let jsonl = r#"{"id":"doc1","metadata":{"title":"First","date":"2025-01-15T10:00:00Z","tags":[]},"text":"A"}

{"id":"doc2","metadata":{"title":"Second","date":"2025-01-14T09:00:00Z","tags":[]},"text":"B"}"#;

        let docs = JsonlLoader::load_from_string(jsonl).unwrap();

        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn test_skip_comment_lines() {
        let jsonl = r#"# This is a comment
{"id":"doc1","metadata":{"title":"Test","date":"2025-01-15T10:00:00Z","tags":[]},"text":"Content"}
# Another comment"#;

        let docs = JsonlLoader::load_from_string(jsonl).unwrap();

        assert_eq!(docs.len(), 1);
    }

    #[test]
    fn test_invalid_json_error() {
        let jsonl = "not valid json";

        let result = JsonlLoader::load_from_string(jsonl);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("line 1"));
    }

    #[test]
    fn test_empty_input() {
        let jsonl = "";

        let docs = JsonlLoader::load_from_string(jsonl).unwrap();

        assert!(docs.is_empty());
    }

    #[test]
    fn test_document_with_category_hierarchy() {
        let jsonl = r#"{"id":"doc1","metadata":{"title":"Claude Code / hookタイミング","date":"2025-01-15T10:00:00Z","tags":["tips"]},"text":"Content"}"#;

        let docs = JsonlLoader::load_from_string(jsonl).unwrap();

        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].category(), Some("Claude Code"));
        assert_eq!(docs[0].subcategory(), Some("hookタイミング"));
    }
}
