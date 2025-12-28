//! Test for content hashing functionality
//!
//! Process 1: TDD Red Phase - Content Hash Tests

use chrono::{TimeZone, Utc};
use digrag::loader::Document;

/// Test: compute_content_hash returns SHA256[:16] hex string
#[test]
fn test_compute_content_hash_returns_16_char_hex() {
    let hash = Document::compute_content_hash("Test Title", "Test content text");

    // Hash should be 16 characters (SHA256 first 64 bits as hex)
    assert_eq!(hash.len(), 16, "Hash should be 16 characters");

    // Hash should be valid hex
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "Hash should be valid hex");
}

/// Test: Same content produces same hash (reproducibility)
#[test]
fn test_same_content_produces_same_hash() {
    let hash1 = Document::compute_content_hash("Test Title", "Test content");
    let hash2 = Document::compute_content_hash("Test Title", "Test content");

    assert_eq!(hash1, hash2, "Same content should produce same hash");
}

/// Test: Different content produces different hash
#[test]
fn test_different_content_produces_different_hash() {
    let hash1 = Document::compute_content_hash("Title A", "Content A");
    let hash2 = Document::compute_content_hash("Title B", "Content B");

    assert_ne!(hash1, hash2, "Different content should produce different hash");
}

/// Test: Different title produces different hash
#[test]
fn test_different_title_produces_different_hash() {
    let hash1 = Document::compute_content_hash("Title A", "Same Content");
    let hash2 = Document::compute_content_hash("Title B", "Same Content");

    assert_ne!(hash1, hash2, "Different title should produce different hash");
}

/// Test: Different text produces different hash
#[test]
fn test_different_text_produces_different_hash() {
    let hash1 = Document::compute_content_hash("Same Title", "Content A");
    let hash2 = Document::compute_content_hash("Same Title", "Content B");

    assert_ne!(hash1, hash2, "Different text should produce different hash");
}

/// Test: Empty content still produces valid hash
#[test]
fn test_empty_content_produces_valid_hash() {
    let hash = Document::compute_content_hash("", "");

    assert_eq!(hash.len(), 16, "Empty content should still produce 16-char hash");
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "Hash should be valid hex");
}

/// Test: Metadata changes do NOT affect hash (date, tags)
#[test]
fn test_metadata_does_not_affect_hash() {
    let date1 = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
    let date2 = Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap();

    let doc1 = Document::with_content_id(
        "Test Title".to_string(),
        date1,
        vec!["tag1".to_string(), "tag2".to_string()],
        "Test content".to_string(),
    );

    let doc2 = Document::with_content_id(
        "Test Title".to_string(),
        date2,
        vec!["completely".to_string(), "different".to_string(), "tags".to_string()],
        "Test content".to_string(),
    );

    assert_eq!(doc1.id, doc2.id, "Same title+text should produce same ID regardless of metadata");
}

/// Test: with_content_id constructor creates document with content-based ID
#[test]
fn test_with_content_id_creates_content_based_id() {
    let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();

    let doc = Document::with_content_id(
        "Test Title".to_string(),
        date,
        vec!["memo".to_string()],
        "Test content".to_string(),
    );

    let expected_hash = Document::compute_content_hash("Test Title", "Test content");
    assert_eq!(doc.id, expected_hash, "Document ID should be content hash");
}

/// Test: content_hash method returns hash based on title and text
#[test]
fn test_document_content_hash_method() {
    let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();

    let doc = Document::with_id(
        "some-uuid".to_string(),
        "Test Title".to_string(),
        date,
        vec!["memo".to_string()],
        "Test content".to_string(),
    );

    let hash = doc.content_hash();
    let expected = Document::compute_content_hash("Test Title", "Test content");

    assert_eq!(hash, expected, "content_hash() should return hash of title and text");
}

/// Test: Unicode content is handled correctly
#[test]
fn test_unicode_content_hash() {
    let hash1 = Document::compute_content_hash("日本語タイトル", "日本語テキスト");
    let hash2 = Document::compute_content_hash("日本語タイトル", "日本語テキスト");

    assert_eq!(hash1, hash2, "Unicode content should produce consistent hash");
    assert_eq!(hash1.len(), 16, "Unicode hash should be 16 characters");
}
