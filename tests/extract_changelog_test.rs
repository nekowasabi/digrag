//! Changelog extractor tests (Process 10: TDD)
//!
//! Tests for changelog entry extraction functionality

use digrag::extract::changelog::{extract_current_entry, ChangelogEntryExtractor};
use digrag::extract::TruncationConfig;

// =============================================================================
// Parse Entry Tests
// =============================================================================

#[test]
fn test_parse_single_entry_basic() {
    let extractor = ChangelogEntryExtractor::new(TruncationConfig::default());
    let text = "* Test Entry 2025-01-15 [memo]:\nContent line 1\nContent line 2";
    let entries = extractor.parse_entries(text);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "Test Entry");
    assert_eq!(entries[0].date, "2025-01-15");
    assert_eq!(entries[0].tags, vec!["memo"]);
    assert!(entries[0].content.contains("Content line 1"));
}

#[test]
fn test_parse_entry_with_time() {
    let extractor = ChangelogEntryExtractor::new(TruncationConfig::default());
    let text = "* Test Entry 2025-01-15 10:30:00 [memo]:[dev]:\nContent here";
    let entries = extractor.parse_entries(text);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].date, "2025-01-15");
    assert_eq!(entries[0].tags, vec!["memo", "dev"]);
}

#[test]
fn test_parse_multiple_entries() {
    let extractor = ChangelogEntryExtractor::new(TruncationConfig::default());
    let text = r#"* First Entry 2025-01-15 [memo]:
First content

* Second Entry 2025-01-16 [dev]:
Second content

* Third Entry 2025-01-17 [memo]:[work]:
Third content
"#;
    let entries = extractor.parse_entries(text);

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].title, "First Entry");
    assert_eq!(entries[1].title, "Second Entry");
    assert_eq!(entries[2].title, "Third Entry");
    assert_eq!(entries[2].tags, vec!["memo", "work"]);
}

#[test]
fn test_parse_entry_multiline_content() {
    let extractor = ChangelogEntryExtractor::new(TruncationConfig::default());
    let text = r#"* Complex Entry 2025-01-15 [memo]:
Line 1
Line 2
Line 3

More content after blank line
- Bullet point
"#;
    let entries = extractor.parse_entries(text);

    assert_eq!(entries.len(), 1);
    assert!(entries[0].content.contains("Line 1"));
    assert!(entries[0].content.contains("Line 2"));
    assert!(entries[0].content.contains("More content"));
    assert!(entries[0].content.contains("Bullet point"));
}

// =============================================================================
// Extract by Title Tests
// =============================================================================

#[test]
fn test_extract_by_title_exact_match() {
    let extractor = ChangelogEntryExtractor::new(TruncationConfig::default());
    let text = r#"* Entry Alpha 2025-01-15 [memo]:
Alpha content

* Entry Beta 2025-01-16 [dev]:
Beta content

* Entry Gamma 2025-01-17 [work]:
Gamma content
"#;

    let result = extractor.extract_by_title(text, "Beta");
    assert!(result.is_some());
    let extracted = result.unwrap();
    assert!(extracted.text.contains("Entry Beta"));
    assert!(extracted.text.contains("Beta content"));
}

#[test]
fn test_extract_by_title_partial_match() {
    let extractor = ChangelogEntryExtractor::new(TruncationConfig::default());
    let text = r#"* Claude Code hookタイミング 2025-12-27 [memo]:
hookのタイミング調査
"#;

    let result = extractor.extract_by_title(text, "Claude");
    assert!(result.is_some());
}

#[test]
fn test_extract_by_title_not_found() {
    let extractor = ChangelogEntryExtractor::new(TruncationConfig::default());
    let text = "* Entry One 2025-01-15 [memo]:\nContent";

    let result = extractor.extract_by_title(text, "NonExistent");
    assert!(result.is_none());
}

// =============================================================================
// Truncation Tests
// =============================================================================

#[test]
fn test_extract_with_truncation() {
    let truncation = TruncationConfig {
        max_chars: Some(50),
        max_lines: None,
        max_sections: None,
    };
    let extractor = ChangelogEntryExtractor::new(truncation);

    let text = "* Long Entry 2025-01-15 [memo]:\n".to_owned() + &"A".repeat(1000);

    let _entries = extractor.parse_entries(&text);
    let result = extractor.extract_by_title(&text, "Long Entry").unwrap();

    assert!(result.truncated);
    assert_eq!(result.stats.extracted_chars, 50);
    assert!(result.stats.total_chars > 50);
}

#[test]
fn test_extract_no_truncation_needed() {
    let truncation = TruncationConfig {
        max_chars: Some(5000),
        max_lines: None,
        max_sections: None,
    };
    let extractor = ChangelogEntryExtractor::new(truncation);

    let text = "* Short Entry 2025-01-15 [memo]:\nShort content";

    let result = extractor.extract_by_title(text, "Short Entry").unwrap();
    assert!(!result.truncated);
}

// =============================================================================
// extract_current_entry Tests
// =============================================================================

#[test]
fn test_extract_current_entry_first() {
    let truncation = TruncationConfig::default();
    let text = r#"* Current Entry 2025-01-15 [memo]:
Current content

* Old Entry 2025-01-10 [dev]:
Old content
"#;

    let result = extract_current_entry(text, &truncation);
    assert!(result.text.contains("Current Entry"));
    assert!(result.text.contains("Current content"));
    assert!(!result.text.contains("Old Entry"));
}

#[test]
fn test_extract_current_entry_no_entry() {
    let truncation = TruncationConfig::default();
    let text = "This is just plain text without any changelog entry format.";

    let result = extract_current_entry(text, &truncation);
    // Should return the full text
    assert_eq!(result.text, text);
    assert!(!result.truncated);
}

#[test]
fn test_extract_current_entry_with_truncation() {
    let truncation = TruncationConfig {
        max_chars: Some(20),
        max_lines: None,
        max_sections: None,
    };
    let text = "This is a long text that will be truncated when no entry is found.";

    let result = extract_current_entry(text, &truncation);
    assert!(result.truncated);
    assert_eq!(result.text.len(), 20);
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_empty_text() {
    let extractor = ChangelogEntryExtractor::new(TruncationConfig::default());
    let entries = extractor.parse_entries("");
    assert!(entries.is_empty());
}

#[test]
fn test_text_without_entries() {
    let extractor = ChangelogEntryExtractor::new(TruncationConfig::default());
    let text = "Regular text\nNo entries here\n** Double star doesn't match";
    let entries = extractor.parse_entries(text);
    assert!(entries.is_empty());
}

#[test]
fn test_entry_with_special_characters() {
    let extractor = ChangelogEntryExtractor::new(TruncationConfig::default());
    let text = "* Test/Entry-Name_V2.0 2025-01-15 [memo]:\nContent with 日本語 and émojis 🎉";
    let entries = extractor.parse_entries(text);

    assert_eq!(entries.len(), 1);
    assert!(entries[0].content.contains("日本語"));
    assert!(entries[0].content.contains("🎉"));
}

#[test]
fn test_entry_japanese_title() {
    let extractor = ChangelogEntryExtractor::new(TruncationConfig::default());
    let text = "* テスト項目 2025-01-15 [memo]:\n日本語のコンテンツ";
    let entries = extractor.parse_entries(text);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "テスト項目");
}
