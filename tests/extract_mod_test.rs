//! Extract module tests (Process 2: TDD)
//!
//! Tests for content extraction engine basic structure

use digrag::extract::{
    ContentExtractor, ContentStats, ExtractedContent, ExtractionStrategy, TruncationConfig,
};

// =============================================================================
// TDD Red Phase: Basic Structure Tests
// =============================================================================

#[test]
fn test_extraction_strategy_head() {
    let strategy = ExtractionStrategy::Head(150);
    match strategy {
        ExtractionStrategy::Head(n) => assert_eq!(n, 150),
        _ => panic!("Expected Head strategy"),
    }
}

#[test]
fn test_extraction_strategy_changelog_entry() {
    let strategy = ExtractionStrategy::ChangelogEntry;
    assert!(matches!(strategy, ExtractionStrategy::ChangelogEntry));
}

#[test]
fn test_extraction_strategy_full() {
    let strategy = ExtractionStrategy::Full;
    assert!(matches!(strategy, ExtractionStrategy::Full));
}

#[test]
fn test_truncation_config_defaults() {
    let config = TruncationConfig::default();
    assert_eq!(config.max_chars, Some(5000));
    assert!(config.max_lines.is_none());
    assert!(config.max_sections.is_none());
}

#[test]
fn test_truncation_config_custom() {
    let config = TruncationConfig {
        max_chars: Some(10000),
        max_lines: Some(100),
        max_sections: Some(5),
    };
    assert_eq!(config.max_chars, Some(10000));
    assert_eq!(config.max_lines, Some(100));
    assert_eq!(config.max_sections, Some(5));
}

#[test]
fn test_content_stats_new() {
    let stats = ContentStats {
        total_chars: 1000,
        total_lines: 50,
        extracted_chars: 500,
    };
    assert_eq!(stats.total_chars, 1000);
    assert_eq!(stats.total_lines, 50);
    assert_eq!(stats.extracted_chars, 500);
}

#[test]
fn test_extracted_content_not_truncated() {
    let content = ExtractedContent {
        text: "Hello, World!".to_string(),
        truncated: false,
        stats: ContentStats {
            total_chars: 13,
            total_lines: 1,
            extracted_chars: 13,
        },
    };
    assert_eq!(content.text, "Hello, World!");
    assert!(!content.truncated);
    assert_eq!(content.stats.total_chars, 13);
}

#[test]
fn test_extracted_content_truncated() {
    let content = ExtractedContent {
        text: "Hello".to_string(),
        truncated: true,
        stats: ContentStats {
            total_chars: 100,
            total_lines: 10,
            extracted_chars: 5,
        },
    };
    assert!(content.truncated);
    assert_eq!(content.stats.extracted_chars, 5);
}

#[test]
fn test_content_extractor_new_head() {
    let _extractor =
        ContentExtractor::new(ExtractionStrategy::Head(150), TruncationConfig::default());
    // Should compile and create successfully
}

#[test]
fn test_content_extractor_extract_head() {
    let extractor = ContentExtractor::new(
        ExtractionStrategy::Head(10),
        TruncationConfig {
            max_chars: Some(10),
            max_lines: None,
            max_sections: None,
        },
    );

    let result = extractor.extract("Hello, World! This is a test.");
    assert_eq!(result.text, "Hello, Wor");
    assert!(result.truncated);
    assert_eq!(result.stats.total_chars, 29);
    assert_eq!(result.stats.extracted_chars, 10);
}

#[test]
fn test_content_extractor_extract_full_no_truncation() {
    let extractor = ContentExtractor::new(
        ExtractionStrategy::Full,
        TruncationConfig {
            max_chars: Some(1000),
            max_lines: None,
            max_sections: None,
        },
    );

    let input = "Short text";
    let result = extractor.extract(input);
    assert_eq!(result.text, input);
    assert!(!result.truncated);
}

#[test]
fn test_content_extractor_extract_full_with_truncation() {
    let extractor = ContentExtractor::new(
        ExtractionStrategy::Full,
        TruncationConfig {
            max_chars: Some(5),
            max_lines: None,
            max_sections: None,
        },
    );

    let result = extractor.extract("Hello, World!");
    assert_eq!(result.text, "Hello");
    assert!(result.truncated);
}

#[test]
fn test_content_extractor_line_counting() {
    let extractor = ContentExtractor::new(ExtractionStrategy::Full, TruncationConfig::default());

    let input = "Line 1\nLine 2\nLine 3";
    let result = extractor.extract(input);
    assert_eq!(result.stats.total_lines, 3);
}
