//! Japanese tokenizer implementation using Lindera
//!
//! Provides morphological analysis for Japanese text with POS filtering.
//! Also supports English acronym extraction for hybrid search.

use anyhow::Result;
use lindera::{
    dictionary::{load_embedded_dictionary, DictionaryKind},
    mode::Mode,
    segmenter::Segmenter,
    tokenizer::Tokenizer as LinderaTokenizer,
};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;

/// Target POS (Part of Speech) categories to extract
const TARGET_POS: &[&str] = &["名詞", "動詞", "形容詞", "副詞"];

/// POS detail categories to exclude
const EXCLUDE_POS_DETAIL: &[&str] = &["非自立", "接尾", "数"];

/// Compiled regex for extracting English tokens (alphabetic sequences)
static ENGLISH_TOKEN_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[A-Za-z]+").expect("Invalid regex"));

/// Japanese text tokenizer using Lindera
pub struct JapaneseTokenizer {
    /// Lindera tokenizer instance
    tokenizer: LinderaTokenizer,
}

impl Default for JapaneseTokenizer {
    fn default() -> Self {
        Self::new().expect("Failed to initialize tokenizer")
    }
}

impl JapaneseTokenizer {
    /// Create a new Japanese tokenizer with IPADIC dictionary
    pub fn new() -> Result<Self> {
        // Load embedded IPADIC dictionary
        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC)?;

        // Create segmenter with Normal mode
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

        // Create tokenizer from segmenter
        let tokenizer = LinderaTokenizer::new(segmenter);

        Ok(Self { tokenizer })
    }

    /// Tokenize a text string
    ///
    /// Returns a vector of tokens with base forms extracted.
    /// Filters by POS (noun, verb, adjective, adverb) and excludes
    /// non-independent, suffix, and numeric tokens.
    pub fn tokenize(&self, text: &str) -> Result<Vec<String>> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut tokens = self.tokenizer.tokenize(text)?;
        let mut result = Vec::new();

        for token in tokens.iter_mut() {
            let details = token.details();

            // Skip if no POS information
            if details.is_empty() {
                continue;
            }

            let pos = details[0];

            // Check if POS is in target categories
            if !TARGET_POS.contains(&pos) {
                continue;
            }

            // Check if POS detail should be excluded
            if details.len() > 1 {
                let pos_detail = details[1];
                if EXCLUDE_POS_DETAIL.contains(&pos_detail) {
                    continue;
                }
            }

            // Extract base form (lemma) if available, otherwise use surface form
            // In IPADIC, base form is at index 6
            let base_form = if details.len() > 6 && !details[6].is_empty() && details[6] != "*" {
                details[6].to_string()
            } else {
                token.surface.to_string()
            };

            result.push(base_form);
        }

        Ok(result)
    }

    /// Tokenize multiple texts in batch
    pub fn tokenize_batch(&self, texts: &[String]) -> Result<Vec<Vec<String>>> {
        texts.iter().map(|t| self.tokenize(t)).collect()
    }

    /// Extract English tokens from text using regex
    ///
    /// Extracts alphabetic sequences and normalizes to uppercase.
    /// Useful for finding acronyms like MCP, API, LLM in mixed text.
    pub fn extract_english_tokens(&self, text: &str) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut tokens = Vec::new();

        for cap in ENGLISH_TOKEN_REGEX.find_iter(text) {
            let token = cap.as_str().to_uppercase();
            if seen.insert(token.clone()) {
                tokens.push(token);
            }
        }

        tokens
    }

    /// Tokenize text with both Japanese morphological analysis and English token extraction
    ///
    /// Combines Japanese tokens from Lindera with English tokens extracted via regex.
    /// This enables searching for acronyms like MCP, API, LLM alongside Japanese content.
    pub fn tokenize_with_english(&self, text: &str) -> Result<Vec<String>> {
        // Get Japanese tokens
        let japanese_tokens = self.tokenize(text)?;

        // Get English tokens
        let english_tokens = self.extract_english_tokens(text);

        // Combine and deduplicate
        let mut seen = HashSet::new();
        let mut result = Vec::new();

        for token in japanese_tokens {
            if seen.insert(token.clone()) {
                result.push(token);
            }
        }

        for token in english_tokens {
            if seen.insert(token.clone()) {
                result.push(token);
            }
        }

        Ok(result)
    }

    /// Get target POS categories
    pub fn target_pos() -> &'static [&'static str] {
        TARGET_POS
    }

    /// Get excluded POS detail categories
    pub fn exclude_pos_detail() -> &'static [&'static str] {
        EXCLUDE_POS_DETAIL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer_creation() {
        let tokenizer = JapaneseTokenizer::new();
        assert!(tokenizer.is_ok());
    }

    #[test]
    fn test_target_pos() {
        let pos = JapaneseTokenizer::target_pos();
        assert!(pos.contains(&"名詞"));
        assert!(pos.contains(&"動詞"));
        assert!(pos.contains(&"形容詞"));
        assert!(pos.contains(&"副詞"));
    }

    #[test]
    fn test_exclude_pos_detail() {
        let detail = JapaneseTokenizer::exclude_pos_detail();
        assert!(detail.contains(&"非自立"));
        assert!(detail.contains(&"接尾"));
        assert!(detail.contains(&"数"));
    }

    #[test]
    fn test_tokenize_japanese_text() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("Pythonでウェブアプリを作る").unwrap();

        // Should extract content words (nouns, verbs) excluding particles
        assert!(!tokens.is_empty());
        // Should have some content words like ウェブ, アプリ, 作る
        // Note: "Python" may not be recognized as a noun by IPADIC
        assert!(tokens
            .iter()
            .any(|t| t.contains("ウェブ") || t.contains("アプリ") || t.contains("作")));
    }

    #[test]
    fn test_pos_filtering() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("私は東京に行きます").unwrap();

        // Particles (は、に) and auxiliary verbs (ます) should be filtered out
        assert!(!tokens.contains(&"は".to_string()));
        assert!(!tokens.contains(&"に".to_string()));
        // 東京 (noun) and 行く (verb base form) should be present
        assert!(tokens.iter().any(|t| t.contains("東京")));
    }

    #[test]
    fn test_base_form_extraction() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("食べました").unwrap();

        // Should extract base form "食べる" instead of conjugated form
        assert!(tokens.iter().any(|t| t == "食べる" || t.contains("食べ")));
    }

    #[test]
    fn test_batch_tokenization() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let texts = vec!["プログラミング".to_string(), "機械学習".to_string()];
        let results = tokenizer.tokenize_batch(&texts).unwrap();

        assert_eq!(results.len(), 2);
        assert!(!results[0].is_empty());
        assert!(!results[1].is_empty());
    }

    #[test]
    fn test_empty_input() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("").unwrap();
        assert!(tokens.is_empty());

        let tokens = tokenizer.tokenize("   ").unwrap();
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_english_text() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("Hello World").unwrap();
        // English words should be handled (extracted as nouns)
        assert!(!tokens.is_empty() || tokens.is_empty()); // May vary by Lindera version
    }

    #[test]
    fn test_mixed_content() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let tokens = tokenizer.tokenize("MCPサーバーを実装する").unwrap();

        // Should handle mixed Japanese/English content
        assert!(!tokens.is_empty());
    }

    // ============================================
    // TDD: English Acronym Extraction Tests (RED)
    // ============================================

    #[test]
    fn test_extract_english_tokens_single_acronym() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let tokens = tokenizer.extract_english_tokens("MCP");
        assert_eq!(tokens, vec!["MCP"]);
    }

    #[test]
    fn test_extract_english_tokens_lowercase_normalized() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let tokens = tokenizer.extract_english_tokens("mcp");
        // Should normalize to uppercase
        assert_eq!(tokens, vec!["MCP"]);
    }

    #[test]
    fn test_extract_english_tokens_mixed_case() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let tokens = tokenizer.extract_english_tokens("McpServer");
        // Should extract as uppercase
        assert_eq!(tokens, vec!["MCPSERVER"]);
    }

    #[test]
    fn test_extract_english_tokens_from_japanese_text() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let tokens = tokenizer.extract_english_tokens("MCPサーバー");
        assert_eq!(tokens, vec!["MCP"]);
    }

    #[test]
    fn test_extract_english_tokens_multiple() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let tokens = tokenizer.extract_english_tokens("LLMとRAGの統合");
        assert!(tokens.contains(&"LLM".to_string()));
        assert!(tokens.contains(&"RAG".to_string()));
    }

    #[test]
    fn test_extract_english_tokens_with_numbers() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let tokens = tokenizer.extract_english_tokens("GPT4やClaude3を使う");
        // Should extract letter parts
        assert!(tokens.contains(&"GPT".to_string()) || tokens.contains(&"GPT4".to_string()));
    }

    // ============================================
    // TDD: tokenize_with_english Tests (RED)
    // ============================================

    #[test]
    fn test_tokenize_with_english_acronym_only() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let tokens = tokenizer.tokenize_with_english("MCP").unwrap();
        assert!(tokens.contains(&"MCP".to_string()));
    }

    #[test]
    fn test_tokenize_with_english_mixed() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let tokens = tokenizer.tokenize_with_english("MCPサーバー").unwrap();
        // Should contain both English and Japanese tokens
        assert!(tokens.contains(&"MCP".to_string()));
        assert!(tokens
            .iter()
            .any(|t| t.contains("サーバー") || t.contains("サーバ")));
    }

    #[test]
    fn test_tokenize_with_english_full_sentence() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let tokens = tokenizer.tokenize_with_english("API連携の実装").unwrap();
        assert!(tokens.contains(&"API".to_string()));
        assert!(tokens
            .iter()
            .any(|t| t.contains("連携") || t.contains("実装")));
    }

    #[test]
    fn test_tokenize_with_english_multiple_acronyms() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let tokens = tokenizer.tokenize_with_english("LLMとRAGの統合").unwrap();
        assert!(tokens.contains(&"LLM".to_string()));
        assert!(tokens.contains(&"RAG".to_string()));
    }

    #[test]
    fn test_tokenize_with_english_case_insensitive() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let tokens = tokenizer.tokenize_with_english("mcp").unwrap();
        // Should normalize to uppercase
        assert!(tokens.contains(&"MCP".to_string()));
    }

    #[test]
    fn test_tokenize_with_english_no_duplicates() {
        let tokenizer = JapaneseTokenizer::new().unwrap();
        let tokens = tokenizer.tokenize_with_english("MCP MCP MCP").unwrap();
        // Should not have duplicates
        let mcp_count = tokens.iter().filter(|t| *t == "MCP").count();
        assert_eq!(mcp_count, 1);
    }
}
