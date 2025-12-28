//! Changelog file parser
//!
//! Parses the changelog memo file format into Document structures.

use super::Document;
use anyhow::{Context, Result};
use chrono::{NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use std::fs;
use std::path::Path;

/// Changelog file loader and parser
pub struct ChangelogLoader {
    /// Regex pattern for entry headers
    entry_pattern: Regex,
    /// Regex pattern for extracting tags
    tag_pattern: Regex,
}

impl Default for ChangelogLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ChangelogLoader {
    /// Create a new changelog loader
    pub fn new() -> Self {
        // Pattern: * Title YYYY-MM-DD HH:MM:SS [tags]:
        let entry_pattern =
            Regex::new(r"^\* (.+?) (\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2})\s*(.*)$").unwrap();
        // Pattern: [tag]:
        let tag_pattern = Regex::new(r"\[([^\]]+)\]:").unwrap();

        Self {
            entry_pattern,
            tag_pattern,
        }
    }

    /// Load documents from a file
    pub fn load_from_file<P: AsRef<Path>>(&self, path: P) -> Result<Vec<Document>> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read file: {:?}", path.as_ref()))?;
        self.load_from_string(&content)
    }

    /// Load documents from a string
    pub fn load_from_string(&self, content: &str) -> Result<Vec<Document>> {
        let mut documents = Vec::new();
        let mut current_entry: Option<(String, String, Vec<String>, Vec<String>)> = None;

        for line in content.lines() {
            if let Some(caps) = self.entry_pattern.captures(line) {
                // Save previous entry if exists
                if let Some((title, date_str, tags, content_lines)) = current_entry.take() {
                    if let Some(doc) = self.create_document(&title, &date_str, tags, content_lines)
                    {
                        documents.push(doc);
                    }
                }

                // Parse new entry header
                let title = caps
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                let date_str = caps
                    .get(2)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                let tags_str = caps.get(3).map(|m| m.as_str()).unwrap_or("");

                // Extract tags
                let tags: Vec<String> = self
                    .tag_pattern
                    .captures_iter(tags_str)
                    .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
                    .collect();

                current_entry = Some((title, date_str, tags, Vec::new()));
            } else if let Some((_, _, _, ref mut content_lines)) = current_entry {
                // Add content line to current entry
                content_lines.push(line.to_string());
            }
        }

        // Don't forget the last entry
        if let Some((title, date_str, tags, content_lines)) = current_entry {
            if let Some(doc) = self.create_document(&title, &date_str, tags, content_lines) {
                documents.push(doc);
            }
        }

        Ok(documents)
    }

    /// Create a document from parsed components
    fn create_document(
        &self,
        title: &str,
        date_str: &str,
        tags: Vec<String>,
        content_lines: Vec<String>,
    ) -> Option<Document> {
        // Parse date
        let date = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S")
            .ok()
            .map(|dt| Utc.from_utc_datetime(&dt))?;

        // Join content lines
        let text = content_lines.join("\n").trim().to_string();

        // Use content-based ID for incremental build support
        Some(Document::with_content_id(title.to_string(), date, tags, text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_entry() {
        let loader = ChangelogLoader::new();
        let content = "* Test Entry 2025-01-15 10:00:00 [memo]:[worklog]:\n・Content line";

        let docs = loader.load_from_string(content).unwrap();
        assert_eq!(docs.len(), 1);

        let doc = &docs[0];
        assert_eq!(doc.title(), "Test Entry");
        assert_eq!(doc.tags(), &["memo", "worklog"]);
        assert_eq!(doc.text, "・Content line");
    }

    #[test]
    fn test_parse_multiple_entries() {
        let loader = ChangelogLoader::new();
        let content = r#"* First Entry 2025-01-15 10:00:00 [memo]:
First content
* Second Entry 2025-01-14 09:00:00 [worklog]:
Second content"#;

        let docs = loader.load_from_string(content).unwrap();
        assert_eq!(docs.len(), 2);

        assert_eq!(docs[0].title(), "First Entry");
        assert_eq!(docs[0].tags(), &["memo"]);

        assert_eq!(docs[1].title(), "Second Entry");
        assert_eq!(docs[1].tags(), &["worklog"]);
    }

    #[test]
    fn test_parse_multiline_content() {
        let loader = ChangelogLoader::new();
        let content = r#"* Entry 2025-01-15 10:00:00 [memo]:
・First line
	・Second line (indented)
		・Third line (double indented)"#;

        let docs = loader.load_from_string(content).unwrap();
        assert_eq!(docs.len(), 1);
        assert!(docs[0].text.contains("First line"));
        assert!(docs[0].text.contains("Second line"));
        assert!(docs[0].text.contains("Third line"));
    }

    #[test]
    fn test_parse_no_tags() {
        let loader = ChangelogLoader::new();
        let content = "* Entry Without Tags 2025-01-15 10:00:00 \nContent";

        let docs = loader.load_from_string(content).unwrap();
        assert_eq!(docs.len(), 1);
        assert!(docs[0].tags().is_empty());
    }

    #[test]
    fn test_parse_empty_content() {
        let loader = ChangelogLoader::new();
        let content = "";

        let docs = loader.load_from_string(content).unwrap();
        assert!(docs.is_empty());
    }

    #[test]
    fn test_date_parsing() {
        let loader = ChangelogLoader::new();
        let content = "* Entry 2025-01-15 14:30:45 [memo]:\nContent";

        let docs = loader.load_from_string(content).unwrap();
        assert_eq!(docs.len(), 1);

        let date = docs[0].date();
        assert_eq!(
            date.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2025-01-15 14:30:45"
        );
    }
}
