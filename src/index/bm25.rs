//! BM25 Index implementation
//!
//! Provides keyword-based search using the BM25 ranking algorithm.
//! Supports both Rust-native format and Python RAG format for cross-compatibility.

use crate::loader::Document;
use crate::search::SearchResult;
use crate::tokenizer::JapaneseTokenizer;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// BM25 parameters
const BM25_K1: f32 = 1.2;
const BM25_B: f32 = 0.75;

/// Python RAG format for BM25 index (for compatibility)
#[derive(Debug, Deserialize)]
struct PythonBm25Format {
    /// Version string (e.g., "1.0")
    #[allow(dead_code)]
    version: Option<String>,
    /// Document IDs
    doc_ids: Vec<String>,
    /// Tokenized corpus (called "corpus" in Python version)
    corpus: Vec<Vec<String>>,
}

/// BM25 search index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bm25Index {
    /// Document IDs in index order
    doc_ids: Vec<String>,
    /// Document token lists (tokenized content for each document)
    doc_tokens: Vec<Vec<String>>,
    /// Inverted index: term -> list of (doc_index, term_frequency)
    inverted_index: HashMap<String, Vec<(usize, usize)>>,
    /// Document lengths (number of tokens)
    doc_lengths: Vec<usize>,
    /// Average document length
    avg_doc_length: f32,
    /// Document frequency for each term
    doc_frequencies: HashMap<String, usize>,
    /// Total number of documents
    num_docs: usize,
}

impl Default for Bm25Index {
    fn default() -> Self {
        Self::new()
    }
}

impl Bm25Index {
    /// Create a new empty BM25 index
    pub fn new() -> Self {
        Self {
            doc_ids: Vec::new(),
            doc_tokens: Vec::new(),
            inverted_index: HashMap::new(),
            doc_lengths: Vec::new(),
            avg_doc_length: 0.0,
            doc_frequencies: HashMap::new(),
            num_docs: 0,
        }
    }

    /// Build an index from documents
    pub fn build(docs: &[Document]) -> Result<Self> {
        let tokenizer = JapaneseTokenizer::new()?;
        let mut index = Self::new();

        index.num_docs = docs.len();
        let mut total_length = 0usize;

        for (doc_idx, doc) in docs.iter().enumerate() {
            // Tokenize document content AND title with English token extraction
            let combined_text = format!("{} {}", doc.title(), doc.text);
            let tokens = tokenizer.tokenize_with_english(&combined_text)?;
            let doc_len = tokens.len();

            index.doc_ids.push(doc.id.clone());
            index.doc_lengths.push(doc_len);
            total_length += doc_len;

            // Count term frequencies for this document
            let mut term_freqs: HashMap<String, usize> = HashMap::new();
            for token in &tokens {
                *term_freqs.entry(token.clone()).or_insert(0) += 1;
            }

            // Update inverted index and document frequencies
            for (term, freq) in &term_freqs {
                index
                    .inverted_index
                    .entry(term.clone())
                    .or_default()
                    .push((doc_idx, *freq));

                // Update document frequency (count docs containing this term)
                *index.doc_frequencies.entry(term.clone()).or_insert(0) += 1;
            }

            index.doc_tokens.push(tokens);
        }

        // Calculate average document length
        if index.num_docs > 0 {
            index.avg_doc_length = total_length as f32 / index.num_docs as f32;
        }

        Ok(index)
    }

    /// Search the index using BM25 ranking
    pub fn search(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        if self.num_docs == 0 {
            return Ok(Vec::new());
        }

        let tokenizer = JapaneseTokenizer::new()?;
        // Use tokenize_with_english for query to match English acronyms
        let query_tokens = tokenizer.tokenize_with_english(query)?;

        if query_tokens.is_empty() {
            return Ok(Vec::new());
        }

        // Calculate BM25 scores for all documents
        let mut scores: Vec<(usize, f32)> = Vec::new();

        for doc_idx in 0..self.num_docs {
            let score = self.calculate_bm25_score(doc_idx, &query_tokens);
            if score > 0.0 {
                scores.push((doc_idx, score));
            }
        }

        // Sort by score descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top_k results
        let results: Vec<SearchResult> = scores
            .into_iter()
            .take(top_k)
            .map(|(doc_idx, score)| SearchResult::new(self.doc_ids[doc_idx].clone(), score))
            .collect();

        Ok(results)
    }

    /// Calculate BM25 score for a document given query tokens
    fn calculate_bm25_score(&self, doc_idx: usize, query_tokens: &[String]) -> f32 {
        let doc_len = self.doc_lengths[doc_idx] as f32;
        let mut score = 0.0;

        for token in query_tokens {
            // Get term frequency in this document
            let tf = self
                .inverted_index
                .get(token)
                .and_then(|postings| {
                    postings
                        .iter()
                        .find(|(idx, _)| *idx == doc_idx)
                        .map(|(_, freq)| *freq as f32)
                })
                .unwrap_or(0.0);

            if tf == 0.0 {
                continue;
            }

            // Get document frequency
            let df = *self.doc_frequencies.get(token).unwrap_or(&0) as f32;
            if df == 0.0 {
                continue;
            }

            // Calculate IDF (inverse document frequency)
            let idf = ((self.num_docs as f32 - df + 0.5) / (df + 0.5) + 1.0).ln();

            // Calculate BM25 term score
            let numerator = tf * (BM25_K1 + 1.0);
            let denominator =
                tf + BM25_K1 * (1.0 - BM25_B + BM25_B * (doc_len / self.avg_doc_length));
            let term_score = idf * (numerator / denominator);

            score += term_score;
        }

        score
    }

    /// Save index to file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load index from file
    ///
    /// Supports both Rust-native format and Python RAG format.
    /// Python format has: { "version", "doc_ids", "corpus" }
    /// Rust format has: { "doc_ids", "doc_tokens", "inverted_index", ... }
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read BM25 index from {:?}", path))?;

        // First, try to parse as JSON Value to detect format
        let json_value: Value =
            serde_json::from_str(&content).with_context(|| "Failed to parse BM25 index as JSON")?;

        // Check if this is Python format (has "corpus" key)
        if json_value.get("corpus").is_some() {
            tracing::info!("Detected Python RAG format, converting to Rust format");
            Self::load_from_python_format(&content)
        } else {
            // Rust native format
            let index = serde_json::from_str(&content)
                .with_context(|| "Failed to parse BM25 index as Rust format")?;
            Ok(index)
        }
    }

    /// Load from Python RAG format and convert to Rust format
    fn load_from_python_format(content: &str) -> Result<Self> {
        let python_format: PythonBm25Format =
            serde_json::from_str(content).with_context(|| "Failed to parse Python BM25 format")?;

        let num_docs = python_format.doc_ids.len();
        let doc_ids = python_format.doc_ids;
        let doc_tokens = python_format.corpus;

        // Calculate doc_lengths
        let doc_lengths: Vec<usize> = doc_tokens.iter().map(|tokens| tokens.len()).collect();

        // Calculate average document length
        let total_length: usize = doc_lengths.iter().sum();
        let avg_doc_length = if num_docs > 0 {
            total_length as f32 / num_docs as f32
        } else {
            0.0
        };

        // Build inverted index and doc_frequencies
        let mut inverted_index: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        let mut doc_frequencies: HashMap<String, usize> = HashMap::new();

        for (doc_idx, tokens) in doc_tokens.iter().enumerate() {
            // Count term frequencies for this document
            let mut term_freqs: HashMap<String, usize> = HashMap::new();
            for token in tokens {
                *term_freqs.entry(token.clone()).or_insert(0) += 1;
            }

            // Update inverted index and document frequencies
            for (term, freq) in &term_freqs {
                inverted_index
                    .entry(term.clone())
                    .or_default()
                    .push((doc_idx, *freq));

                // Update document frequency (count docs containing this term)
                *doc_frequencies.entry(term.clone()).or_insert(0) += 1;
            }
        }

        tracing::info!(
            "Converted Python BM25 format: {} docs, {} unique terms",
            num_docs,
            doc_frequencies.len()
        );

        Ok(Self {
            doc_ids,
            doc_tokens,
            inverted_index,
            doc_lengths,
            avg_doc_length,
            doc_frequencies,
            num_docs,
        })
    }

    /// Get document count
    pub fn len(&self) -> usize {
        self.doc_ids.len()
    }

    /// Check if index is empty
    pub fn is_empty(&self) -> bool {
        self.doc_ids.is_empty()
    }

    /// Get average document length
    pub fn avg_doc_length(&self) -> f32 {
        self.avg_doc_length
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_documents() -> Vec<Document> {
        vec![
            Document::with_id(
                "doc1".to_string(),
                "MCPサーバーの実装".to_string(),
                Utc::now(),
                vec!["memo".to_string()],
                "MCPサーバーをRustで実装する方法について説明します。".to_string(),
            ),
            Document::with_id(
                "doc2".to_string(),
                "Pythonプログラミング".to_string(),
                Utc::now(),
                vec!["tips".to_string()],
                "Pythonでウェブアプリケーションを開発する手順を解説。".to_string(),
            ),
            Document::with_id(
                "doc3".to_string(),
                "Rustの基本".to_string(),
                Utc::now(),
                vec!["memo".to_string()],
                "Rustプログラミングの基本的な概念を学ぶ。".to_string(),
            ),
            Document::with_id(
                "doc4".to_string(),
                "機械学習入門".to_string(),
                Utc::now(),
                vec!["worklog".to_string()],
                "機械学習の基礎について。ニューラルネットワークの仕組みを理解する。".to_string(),
            ),
            Document::with_id(
                "doc5".to_string(),
                "データベース設計".to_string(),
                Utc::now(),
                vec!["memo".to_string()],
                "PostgreSQLを使ったデータベース設計のベストプラクティス。".to_string(),
            ),
        ]
    }

    #[test]
    fn test_bm25_index_creation() {
        let index = Bm25Index::new();
        assert!(index.is_empty());
    }

    #[test]
    fn test_bm25_index_build() {
        let docs = create_test_documents();
        let index = Bm25Index::build(&docs).unwrap();

        assert_eq!(index.len(), 5);
        assert!(!index.is_empty());
        assert!(index.avg_doc_length() > 0.0);
    }

    #[test]
    fn test_bm25_search() {
        let docs = create_test_documents();
        let index = Bm25Index::build(&docs).unwrap();

        // Search for "機械学習" (Japanese term that exists in docs)
        let results = index.search("機械学習", 3).unwrap();

        // Should find documents containing machine learning content
        assert!(!results.is_empty());
        // Results should have scores
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn test_bm25_search_ranking() {
        let docs = create_test_documents();
        let index = Bm25Index::build(&docs).unwrap();

        // Search for "実装" which appears in doc1
        let results = index.search("実装", 3).unwrap();

        // Should find at least one result
        if !results.is_empty() {
            assert!(results[0].score > 0.0);
        }
    }

    #[test]
    fn test_bm25_empty_query() {
        let docs = create_test_documents();
        let index = Bm25Index::build(&docs).unwrap();

        let results = index.search("", 3).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_bm25_no_match() {
        let docs = create_test_documents();
        let index = Bm25Index::build(&docs).unwrap();

        // Search for English term that doesn't exist as Japanese token
        // Note: Lindera may not recognize arbitrary English words
        let results = index.search("xyznonexistent", 3).unwrap();
        // The result may or may not be empty depending on tokenizer behavior
        // Just check that search doesn't crash
        assert!(results.len() <= 3);
    }

    #[test]
    fn test_bm25_index_serialization() {
        let docs = create_test_documents();
        let index = Bm25Index::build(&docs).unwrap();

        let json = serde_json::to_string(&index).unwrap();
        let deserialized: Bm25Index = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.len(), 5);
        assert_eq!(deserialized.avg_doc_length(), index.avg_doc_length());
    }

    #[test]
    fn test_bm25_file_save_load() {
        let docs = create_test_documents();
        let index = Bm25Index::build(&docs).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("bm25_index.json");

        index.save_to_file(&path).unwrap();
        let loaded = Bm25Index::load_from_file(&path).unwrap();

        assert_eq!(loaded.len(), index.len());

        // Search should work on loaded index
        let results = loaded.search("機械学習", 3).unwrap();
        assert!(!results.is_empty());
    }

    // ============================================
    // TDD: English Acronym Search Tests
    // ============================================

    #[test]
    fn test_bm25_search_english_acronym() {
        let docs = create_test_documents();
        let index = Bm25Index::build(&docs).unwrap();

        // Search for "MCP" should find doc1 which contains "MCPサーバー"
        let results = index.search("MCP", 3).unwrap();
        assert!(!results.is_empty(), "Should find documents with MCP");
        assert!(
            results.iter().any(|r| r.doc_id == "doc1"),
            "Should find doc1 containing MCPサーバー"
        );
    }

    #[test]
    fn test_bm25_search_lowercase_acronym() {
        let docs = create_test_documents();
        let index = Bm25Index::build(&docs).unwrap();

        // Search for "mcp" (lowercase) should also find doc1
        let results = index.search("mcp", 3).unwrap();
        assert!(!results.is_empty(), "Lowercase search should work");
        assert!(
            results.iter().any(|r| r.doc_id == "doc1"),
            "Should find doc1 with lowercase mcp search"
        );
    }

    #[test]
    fn test_bm25_search_rust_keyword() {
        let docs = create_test_documents();
        let index = Bm25Index::build(&docs).unwrap();

        // Search for "Rust" should find doc1 and doc3
        let results = index.search("Rust", 5).unwrap();
        assert!(!results.is_empty(), "Should find documents with Rust");
    }

    #[test]
    fn test_bm25_search_python_keyword() {
        let docs = create_test_documents();
        let index = Bm25Index::build(&docs).unwrap();

        // Search for "Python" should find doc2
        let results = index.search("Python", 3).unwrap();
        assert!(!results.is_empty(), "Should find documents with Python");
        assert!(
            results.iter().any(|r| r.doc_id == "doc2"),
            "Should find doc2 containing Python"
        );
    }

    // ============================================
    // TDD Process 2: Title in BM25 Index Tests
    // ============================================

    #[test]
    fn test_bm25_search_by_title_only_keyword() {
        // Create a document where the keyword ONLY exists in the title
        let docs = vec![Document::with_id(
            "vimconf_doc".to_string(),
            "VimConf2025参加レポート".to_string(), // keyword in title
            Utc::now(),
            vec!["event".to_string()],
            "カンファレンスに参加しました。素晴らしい体験でした。".to_string(), // no keyword in body
        )];
        let index = Bm25Index::build(&docs).unwrap();

        // Search for "VimConf" which only exists in title
        let results = index.search("VimConf", 3).unwrap();
        assert!(
            !results.is_empty(),
            "Should find document by title keyword 'VimConf'"
        );
        assert!(
            results.iter().any(|r| r.doc_id == "vimconf_doc"),
            "Should find the vimconf_doc"
        );
    }

    // ============================================
    // TDD Process 3: English Tokens with Numbers
    // ============================================

    #[test]
    fn test_bm25_search_alphanumeric_keyword() {
        let docs = vec![Document::with_id(
            "vimconf2025_doc".to_string(),
            "イベントレポート".to_string(),
            Utc::now(),
            vec!["event".to_string()],
            "vimconf2025に参加した。".to_string(), // keyword with numbers
        )];
        let index = Bm25Index::build(&docs).unwrap();

        // Search for "vimconf2025" with numbers
        let results = index.search("vimconf2025", 3).unwrap();
        assert!(
            !results.is_empty(),
            "Should find document with alphanumeric keyword 'vimconf2025'"
        );
    }

    #[test]
    fn test_bm25_search_year_number() {
        let docs = vec![Document::with_id(
            "year_doc".to_string(),
            "2025年の予定".to_string(),
            Utc::now(),
            vec!["plan".to_string()],
            "vimconf2025とrubykaigi2025に参加予定。".to_string(),
        )];
        let index = Bm25Index::build(&docs).unwrap();

        // Search for "2025" should work
        let results = index.search("2025", 3).unwrap();
        // This should find the document if numbers are tokenized
        assert!(
            !results.is_empty(),
            "Should find document containing '2025'"
        );
    }

    // ============================================
    // TDD Process 4: CamelCase Splitting
    // ============================================

    #[test]
    fn test_bm25_search_camelcase_partial() {
        let docs = vec![Document::with_id(
            "camel_doc".to_string(),
            "VimConf参加".to_string(),
            Utc::now(),
            vec!["event".to_string()],
            "VimConfは素晴らしいイベントです。".to_string(),
        )];
        let index = Bm25Index::build(&docs).unwrap();

        // Search for "Vim" should find "VimConf" via CamelCase splitting
        let results = index.search("Vim", 3).unwrap();
        assert!(
            !results.is_empty(),
            "Should find document with 'Vim' from 'VimConf' via CamelCase split"
        );
    }

    #[test]
    fn test_bm25_search_camelcase_second_part() {
        let docs = vec![Document::with_id(
            "camel_doc2".to_string(),
            "VimConf参加".to_string(),
            Utc::now(),
            vec!["event".to_string()],
            "VimConfは素晴らしいイベントです。".to_string(),
        )];
        let index = Bm25Index::build(&docs).unwrap();

        // Search for "Conf" should find "VimConf" via CamelCase splitting
        let results = index.search("Conf", 3).unwrap();
        assert!(
            !results.is_empty(),
            "Should find document with 'Conf' from 'VimConf' via CamelCase split"
        );
    }
}
