//! Test for IndexMetadata extension
//!
//! Process 3: TDD Red Phase - IndexMetadata Tests

use digrag::index::IndexMetadata;
use std::collections::HashMap;
use tempfile::tempdir;

/// Test: IndexMetadata has schema_version field
#[test]
fn test_metadata_has_schema_version() {
    let metadata = IndexMetadata::new(10, Some("text-embedding-3-small".to_string()));

    assert_eq!(metadata.schema_version, "2.0");
}

/// Test: IndexMetadata has doc_hashes field
#[test]
fn test_metadata_has_doc_hashes() {
    let mut metadata = IndexMetadata::new(10, Some("model".to_string()));

    metadata.doc_hashes.insert("doc1".to_string(), "hash1".to_string());
    metadata.doc_hashes.insert("doc2".to_string(), "hash2".to_string());

    assert_eq!(metadata.doc_hashes.len(), 2);
    assert_eq!(metadata.doc_hashes.get("doc1"), Some(&"hash1".to_string()));
}

/// Test: IndexMetadata serialization/deserialization works correctly
#[test]
fn test_metadata_serialization() {
    let mut metadata = IndexMetadata::new(5, Some("model".to_string()));
    metadata.doc_hashes.insert("doc1".to_string(), "abc123".to_string());

    let json = serde_json::to_string(&metadata).unwrap();
    let deserialized: IndexMetadata = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.doc_count, 5);
    assert_eq!(deserialized.schema_version, "2.0");
    assert_eq!(deserialized.doc_hashes.get("doc1"), Some(&"abc123".to_string()));
}

/// Test: IndexMetadata save/load from file
#[test]
fn test_metadata_save_load() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("metadata.json");

    let mut metadata = IndexMetadata::new(3, Some("test-model".to_string()));
    metadata.doc_hashes.insert("id1".to_string(), "hash1".to_string());

    metadata.save_to_file(&path).unwrap();

    let loaded = IndexMetadata::load_from_file(&path).unwrap();

    assert_eq!(loaded.doc_count, 3);
    assert_eq!(loaded.schema_version, "2.0");
    assert_eq!(loaded.embedding_model, Some("test-model".to_string()));
    assert_eq!(loaded.doc_hashes.get("id1"), Some(&"hash1".to_string()));
}

/// Test: needs_full_rebuild returns true for old schema
#[test]
fn test_needs_full_rebuild_for_old_schema() {
    // Schema version 1.0 or lower requires full rebuild
    let old_metadata = IndexMetadata {
        doc_count: 10,
        created_at: "2025-01-01T00:00:00Z".to_string(),
        embedding_model: Some("model".to_string()),
        schema_version: "1.0".to_string(),
        doc_hashes: HashMap::new(),
    };

    assert!(old_metadata.needs_full_rebuild());
}

/// Test: needs_full_rebuild returns false for current schema
#[test]
fn test_no_full_rebuild_for_current_schema() {
    let metadata = IndexMetadata::new(10, Some("model".to_string()));
    assert!(!metadata.needs_full_rebuild());
}

/// Test: needs_full_rebuild returns true for missing schema
#[test]
fn test_needs_full_rebuild_for_missing_schema() {
    let old_metadata = IndexMetadata {
        doc_count: 10,
        created_at: "2025-01-01T00:00:00Z".to_string(),
        embedding_model: Some("model".to_string()),
        schema_version: "".to_string(),
        doc_hashes: HashMap::new(),
    };

    assert!(old_metadata.needs_full_rebuild());
}

/// Test: Default doc_hashes is empty
#[test]
fn test_default_doc_hashes_is_empty() {
    let metadata = IndexMetadata::new(0, None);
    assert!(metadata.doc_hashes.is_empty());
}

/// Test: Backward compatibility - deserialize old format without schema_version
#[test]
fn test_backward_compatibility_old_format() {
    // Old format without schema_version and doc_hashes
    let old_json = r#"{
        "doc_count": 5,
        "created_at": "2025-01-01T00:00:00Z",
        "embedding_model": "old-model"
    }"#;

    let metadata: IndexMetadata = serde_json::from_str(old_json).unwrap();

    // Should have default values
    assert_eq!(metadata.doc_count, 5);
    assert!(metadata.schema_version.is_empty() || metadata.schema_version == "1.0");
    assert!(metadata.doc_hashes.is_empty());
    assert!(metadata.needs_full_rebuild());
}

/// Test: update_doc_hash adds/updates hash
#[test]
fn test_update_doc_hash() {
    let mut metadata = IndexMetadata::new(0, None);

    metadata.update_doc_hash("doc1".to_string(), "hash1".to_string());
    assert_eq!(metadata.doc_hashes.get("doc1"), Some(&"hash1".to_string()));

    metadata.update_doc_hash("doc1".to_string(), "hash2".to_string());
    assert_eq!(metadata.doc_hashes.get("doc1"), Some(&"hash2".to_string()));
}

/// Test: remove_doc_hash removes hash
#[test]
fn test_remove_doc_hash() {
    let mut metadata = IndexMetadata::new(0, None);
    metadata.doc_hashes.insert("doc1".to_string(), "hash1".to_string());

    metadata.remove_doc_hash("doc1");
    assert!(!metadata.doc_hashes.contains_key("doc1"));
}

/// Test: get_doc_hash returns hash if exists
#[test]
fn test_get_doc_hash() {
    let mut metadata = IndexMetadata::new(0, None);
    metadata.doc_hashes.insert("doc1".to_string(), "hash1".to_string());

    assert_eq!(metadata.get_doc_hash("doc1"), Some(&"hash1".to_string()));
    assert_eq!(metadata.get_doc_hash("doc2"), None);
}
