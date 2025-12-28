//! Content extraction engine for digrag
//!
//! Provides extraction strategies for different content types:
//! - Head: Extract first N characters (legacy compatibility)
//! - ChangelogEntry: Extract `*`-prefixed changelog entries
//! - Full: Extract entire content with optional truncation

pub mod cache;
pub mod changelog;
pub mod openrouter_client;
pub mod summarizer;
pub mod telemetry;

/// Extraction strategy enum
#[derive(Debug, Clone)]
pub enum ExtractionStrategy {
    /// Extract first N characters (legacy snippet mode)
    Head(usize),
    /// Extract changelog entry (`* Title YYYY-MM-DD` pattern)
    ChangelogEntry,
    /// Extract full content
    Full,
}

/// Truncation configuration
#[derive(Debug, Clone)]
pub struct TruncationConfig {
    /// Maximum characters to extract
    pub max_chars: Option<usize>,
    /// Maximum lines to extract
    pub max_lines: Option<usize>,
    /// Maximum sections to extract
    pub max_sections: Option<usize>,
}

impl Default for TruncationConfig {
    fn default() -> Self {
        Self {
            max_chars: Some(5000),
            max_lines: None,
            max_sections: None,
        }
    }
}

/// Content statistics
#[derive(Debug, Clone)]
pub struct ContentStats {
    /// Total characters in original content
    pub total_chars: usize,
    /// Total lines in original content
    pub total_lines: usize,
    /// Characters actually extracted
    pub extracted_chars: usize,
}

/// Extracted content result
#[derive(Debug, Clone)]
pub struct ExtractedContent {
    /// Extracted text
    pub text: String,
    /// Whether content was truncated
    pub truncated: bool,
    /// Content statistics
    pub stats: ContentStats,
}

/// Content extractor
pub struct ContentExtractor {
    strategy: ExtractionStrategy,
    truncation: TruncationConfig,
}

impl ContentExtractor {
    /// Create a new content extractor
    pub fn new(strategy: ExtractionStrategy, truncation: TruncationConfig) -> Self {
        Self { strategy, truncation }
    }

    /// Extract content from text using configured strategy
    pub fn extract(&self, full_text: &str) -> ExtractedContent {
        let total_chars = full_text.chars().count();
        let total_lines = full_text.lines().count();

        match &self.strategy {
            ExtractionStrategy::Head(n) => {
                self.extract_head(full_text, *n, total_chars, total_lines)
            }
            ExtractionStrategy::ChangelogEntry => {
                // Delegate to changelog module
                changelog::extract_current_entry(full_text, &self.truncation)
            }
            ExtractionStrategy::Full => {
                self.extract_full(full_text, total_chars, total_lines)
            }
        }
    }

    fn extract_head(&self, text: &str, n: usize, total_chars: usize, total_lines: usize) -> ExtractedContent {
        let max_chars = self.truncation.max_chars.unwrap_or(n).min(n);
        let extracted: String = text.chars().take(max_chars).collect();
        let extracted_chars = extracted.chars().count();
        let truncated = extracted_chars < total_chars;

        ExtractedContent {
            text: extracted,
            truncated,
            stats: ContentStats {
                total_chars,
                total_lines,
                extracted_chars,
            },
        }
    }

    fn extract_full(&self, text: &str, total_chars: usize, total_lines: usize) -> ExtractedContent {
        if let Some(max_chars) = self.truncation.max_chars {
            if total_chars > max_chars {
                let extracted: String = text.chars().take(max_chars).collect();
                return ExtractedContent {
                    text: extracted,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncation_config_default() {
        let config = TruncationConfig::default();
        assert_eq!(config.max_chars, Some(5000));
    }

    #[test]
    fn test_extractor_head_basic() {
        let extractor = ContentExtractor::new(
            ExtractionStrategy::Head(5),
            TruncationConfig::default(),
        );
        let result = extractor.extract("Hello, World!");
        assert_eq!(result.text, "Hello");
        assert!(result.truncated);
    }
}
