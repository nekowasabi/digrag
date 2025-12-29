//! Changelog entry extractor
//!
//! Extracts changelog entries in the format:
//! `* Title YYYY-MM-DD HH:MM:SS [tag1]:[tag2]:`
//!
//! Each entry starts with `* ` and continues until the next `* ` line.

use once_cell::sync::Lazy;
use regex::Regex;

use super::{ContentStats, ExtractedContent, TruncationConfig};

/// Regex pattern for changelog entry header
/// Matches: * Title YYYY-MM-DD or * Title YYYY-MM-DD HH:MM:SS [tags]:
static ENTRY_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\* .+ \d{4}-\d{2}-\d{2}").unwrap());

/// Parsed changelog entry
#[derive(Debug, Clone)]
pub struct ChangelogEntry {
    /// Entry title (without `* ` prefix and date)
    pub title: String,
    /// Date string (YYYY-MM-DD format)
    pub date: String,
    /// Tags extracted from [tag1]:[tag2]: pattern
    pub tags: Vec<String>,
    /// Full entry content including header
    pub content: String,
    /// Start byte offset in original text
    pub start_offset: usize,
    /// End byte offset in original text
    pub end_offset: usize,
}

/// Changelog entry extractor
pub struct ChangelogEntryExtractor {
    truncation: TruncationConfig,
}

impl ChangelogEntryExtractor {
    /// Create a new changelog entry extractor
    pub fn new(truncation: TruncationConfig) -> Self {
        Self { truncation }
    }

    /// Parse all entries from text
    pub fn parse_entries(&self, text: &str) -> Vec<ChangelogEntry> {
        let mut entries = Vec::new();
        let mut current_start: Option<usize> = None;
        let mut current_header: Option<&str> = None;

        for line in text.lines() {
            if ENTRY_PATTERN.is_match(line) {
                // Found new entry header
                if let Some(start_offset) = current_start {
                    // Save previous entry
                    let end_offset = text[..].find(line).unwrap_or(text.len());
                    if let Some(header) = current_header {
                        if let Some(entry) = self.parse_single_entry(
                            header,
                            &text[start_offset..end_offset],
                            start_offset,
                            end_offset,
                        ) {
                            entries.push(entry);
                        }
                    }
                }
                current_start = Some(text.find(line).unwrap_or(0));
                current_header = Some(line);
            }
        }

        // Handle last entry
        if let (Some(start_offset), Some(header)) = (current_start, current_header) {
            if let Some(entry) =
                self.parse_single_entry(header, &text[start_offset..], start_offset, text.len())
            {
                entries.push(entry);
            }
        }

        entries
    }

    /// Extract entry by title match
    pub fn extract_by_title(&self, text: &str, title: &str) -> Option<ExtractedContent> {
        let entries = self.parse_entries(text);

        for entry in entries {
            if entry.title.contains(title) || entry.content.contains(title) {
                return Some(self.truncate_entry(&entry, text));
            }
        }

        None
    }

    fn parse_single_entry(
        &self,
        header: &str,
        content: &str,
        start_offset: usize,
        end_offset: usize,
    ) -> Option<ChangelogEntry> {
        // Extract title and date from header
        // Format: * Title YYYY-MM-DD HH:MM:SS [tags]:
        let header_trimmed = header.trim_start_matches("* ");

        // Find date pattern
        let date_pattern = Regex::new(r"(\d{4}-\d{2}-\d{2})").ok()?;
        let date_match = date_pattern.find(header_trimmed)?;
        let date = date_match.as_str().to_string();

        // Title is everything before the date
        let title = header_trimmed[..date_match.start()].trim().to_string();

        // Extract tags from [tag1]:[tag2]: pattern
        let tags = extract_tags(header);

        Some(ChangelogEntry {
            title,
            date,
            tags,
            content: content.trim_end().to_string(),
            start_offset,
            end_offset,
        })
    }

    fn truncate_entry(&self, entry: &ChangelogEntry, full_text: &str) -> ExtractedContent {
        let total_chars = full_text.chars().count();
        let total_lines = full_text.lines().count();
        let entry_chars = entry.content.chars().count();

        if let Some(max_chars) = self.truncation.max_chars {
            if entry_chars > max_chars {
                let truncated_text: String = entry.content.chars().take(max_chars).collect();
                return ExtractedContent {
                    text: truncated_text,
                    truncated: true,
                    stats: ContentStats {
                        total_chars,
                        total_lines,
                        extracted_chars: max_chars,
                    },
                };
            }
        }

        ExtractedContent {
            text: entry.content.clone(),
            truncated: false,
            stats: ContentStats {
                total_chars,
                total_lines,
                extracted_chars: entry_chars,
            },
        }
    }
}

/// Extract the current/first entry from text
pub fn extract_current_entry(text: &str, truncation: &TruncationConfig) -> ExtractedContent {
    let extractor = ChangelogEntryExtractor::new(truncation.clone());
    let entries = extractor.parse_entries(text);

    if let Some(first_entry) = entries.first() {
        extractor.truncate_entry(first_entry, text)
    } else {
        // No entry found, return full text with truncation
        let total_chars = text.chars().count();
        let total_lines = text.lines().count();

        if let Some(max_chars) = truncation.max_chars {
            if total_chars > max_chars {
                let truncated: String = text.chars().take(max_chars).collect();
                return ExtractedContent {
                    text: truncated,
                    truncated: true,
                    stats: ContentStats {
                        total_chars,
                        total_lines,
                        extracted_chars: max_chars,
                    },
                };
            }
        }

        ExtractedContent {
            text: text.to_string(),
            truncated: false,
            stats: ContentStats {
                total_chars,
                total_lines,
                extracted_chars: total_chars,
            },
        }
    }
}

/// Extract tags from header line
fn extract_tags(header: &str) -> Vec<String> {
    let tag_pattern = Regex::new(r"\[([^\]]+)\]").unwrap();
    tag_pattern
        .captures_iter(header)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_pattern_matches() {
        assert!(ENTRY_PATTERN.is_match("* Test Entry 2025-01-15"));
        assert!(ENTRY_PATTERN.is_match("* Test Entry 2025-01-15 10:30:00 [memo]:[dev]:"));
        assert!(!ENTRY_PATTERN.is_match("Regular line"));
        assert!(!ENTRY_PATTERN.is_match("** Nested"));
    }

    #[test]
    fn test_extract_tags() {
        let tags = extract_tags("* Title 2025-01-15 [memo]:[dev]:");
        assert_eq!(tags, vec!["memo", "dev"]);
    }

    #[test]
    fn test_parse_single_entry() {
        let extractor = ChangelogEntryExtractor::new(TruncationConfig::default());
        let text = "* Test Entry 2025-01-15 [memo]:\nContent line 1\nContent line 2";
        let entries = extractor.parse_entries(text);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Test Entry");
        assert_eq!(entries[0].date, "2025-01-15");
        assert_eq!(entries[0].tags, vec!["memo"]);
    }

    #[test]
    fn test_parse_multiple_entries() {
        let extractor = ChangelogEntryExtractor::new(TruncationConfig::default());
        let text = r#"* Entry One 2025-01-15 [memo]:
Content for entry one

* Entry Two 2025-01-16 [dev]:
Content for entry two
"#;
        let entries = extractor.parse_entries(text);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].title, "Entry One");
        assert_eq!(entries[1].title, "Entry Two");
    }

    #[test]
    fn test_extract_by_title() {
        let extractor = ChangelogEntryExtractor::new(TruncationConfig::default());
        let text = r#"* First Entry 2025-01-15 [memo]:
First content

* Target Entry 2025-01-16 [dev]:
Target content here
"#;
        let result = extractor.extract_by_title(text, "Target");

        assert!(result.is_some());
        let extracted = result.unwrap();
        assert!(extracted.text.contains("Target Entry"));
        assert!(extracted.text.contains("Target content"));
    }
}
