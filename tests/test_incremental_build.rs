//! Test for IndexBuilder incremental build functionality
//!
//! Process 5: TDD Red Phase - Incremental Build Tests

use chrono::{TimeZone, Utc};
use digrag::index::{Docstore, IndexBuilder, IndexMetadata};
use digrag::loader::Document;
use tempfile::tempdir;

fn create_test_doc(title: &str, text: &str) -> Document {
    let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
    Document::with_content_id(title.to_string(), date, vec![], text.to_string())
}

/// Test: load_existing_metadata returns None for non-existent directory
#[test]
fn test_load_existing_metadata_nonexistent() {
    let result = IndexBuilder::load_existing_metadata(std::path::Path::new("/nonexistent/path"));
    assert!(result.is_none());
}

/// Test: load_existing_metadata returns metadata for valid directory
#[test]
fn test_load_existing_metadata_valid() {
    let dir = tempdir().unwrap();
    let metadata = IndexMetadata::new(5, Some("test-model".to_string()));
    metadata.save_to_file(&dir.path().join("metadata.json")).unwrap();

    let loaded = IndexBuilder::load_existing_metadata(dir.path());
    assert!(loaded.is_some());

    let loaded = loaded.unwrap();
    assert_eq!(loaded.doc_count, 5);
    assert_eq!(loaded.schema_version, "2.0");
}

/// Test: load_existing_metadata returns None for old schema
#[test]
fn test_load_existing_metadata_old_schema_returns_none() {
    let dir = tempdir().unwrap();

    // Create old-format metadata
    let old_metadata = r#"{
        "doc_count": 5,
        "created_at": "2025-01-01T00:00:00Z",
        "embedding_model": "old-model"
    }"#;
    std::fs::write(dir.path().join("metadata.json"), old_metadata).unwrap();

    // Should return None for old schema (requires full rebuild)
    let loaded = IndexBuilder::load_existing_metadata(dir.path());
    assert!(loaded.is_none());
}

/// Test: build_from_documents populates doc_hashes in metadata
#[test]
fn test_build_populates_doc_hashes() {
    let dir = tempdir().unwrap();
    let builder = IndexBuilder::new();

    let docs = vec![
        create_test_doc("Title 1", "Content 1"),
        create_test_doc("Title 2", "Content 2"),
    ];

    builder.build_from_documents(docs.clone(), dir.path()).unwrap();

    let metadata = IndexMetadata::load_from_file(&dir.path().join("metadata.json")).unwrap();

    assert_eq!(metadata.doc_hashes.len(), 2);
    for doc in &docs {
        assert!(metadata.doc_hashes.contains_key(&doc.id));
        assert_eq!(metadata.doc_hashes.get(&doc.id), Some(&doc.content_hash()));
    }
}

/// Test: incremental build only processes added/modified documents
#[test]
fn test_incremental_build_processes_only_changes() {
    let dir = tempdir().unwrap();
    let builder = IndexBuilder::new();

    // Initial build
    let initial_docs = vec![
        create_test_doc("Doc 1", "Content 1"),
        create_test_doc("Doc 2", "Content 2"),
    ];
    builder.build_from_documents(initial_docs.clone(), dir.path()).unwrap();

    // Load existing metadata
    let existing_metadata = IndexBuilder::load_existing_metadata(dir.path()).unwrap();

    // Second build with one new, one unchanged
    let new_docs = vec![
        create_test_doc("Doc 1", "Content 1"), // Unchanged
        create_test_doc("Doc 3", "Content 3"), // New
    ];

    let diff = digrag::index::IncrementalDiff::compute(new_docs, &existing_metadata.doc_hashes);

    assert_eq!(diff.added_count(), 1);
    assert_eq!(diff.unchanged_count(), 1);
    assert_eq!(diff.removed_count(), 1);
    assert_eq!(diff.embeddings_needed(), 1);
}

/// Test: incremental build removes deleted documents from indices
#[test]
fn test_incremental_build_removes_deleted() {
    let dir = tempdir().unwrap();
    let builder = IndexBuilder::new();

    // Initial build with 3 docs
    let initial_docs = vec![
        create_test_doc("Doc 1", "Content 1"),
        create_test_doc("Doc 2", "Content 2"),
        create_test_doc("Doc 3", "Content 3"),
    ];
    builder.build_from_documents(initial_docs.clone(), dir.path()).unwrap();

    // Load docstore
    let docstore = Docstore::load_from_file(&dir.path().join("docstore.json")).unwrap();
    assert_eq!(docstore.len(), 3);

    // Simulate incremental build removing Doc 2
    let new_docs = vec![
        create_test_doc("Doc 1", "Content 1"),
        create_test_doc("Doc 3", "Content 3"),
    ];

    builder.build_from_documents(new_docs, dir.path()).unwrap();

    // Verify Doc 2 is removed
    let updated_docstore = Docstore::load_from_file(&dir.path().join("docstore.json")).unwrap();
    assert_eq!(updated_docstore.len(), 2);
    assert!(!updated_docstore.contains(&initial_docs[1].id));
}

/// Test: has_incremental_support method
#[test]
fn test_has_incremental_support() {
    let dir = tempdir().unwrap();

    // No metadata - no support
    assert!(!IndexBuilder::has_incremental_support(dir.path()));

    // Create metadata with current schema
    let metadata = IndexMetadata::new(0, None);
    metadata.save_to_file(&dir.path().join("metadata.json")).unwrap();

    // Now has support
    assert!(IndexBuilder::has_incremental_support(dir.path()));
}

/// Test: metadata updated correctly after incremental build
#[test]
fn test_metadata_updated_after_build() {
    let dir = tempdir().unwrap();
    let builder = IndexBuilder::new();

    let docs = vec![
        create_test_doc("Doc 1", "Content 1"),
    ];

    builder.build_from_documents(docs.clone(), dir.path()).unwrap();

    let metadata = IndexMetadata::load_from_file(&dir.path().join("metadata.json")).unwrap();
    assert_eq!(metadata.doc_count, 1);
    assert_eq!(metadata.doc_hashes.len(), 1);

    // Add another document
    let more_docs = vec![
        create_test_doc("Doc 1", "Content 1"),
        create_test_doc("Doc 2", "Content 2"),
    ];

    builder.build_from_documents(more_docs, dir.path()).unwrap();

    let updated_metadata = IndexMetadata::load_from_file(&dir.path().join("metadata.json")).unwrap();
    assert_eq!(updated_metadata.doc_count, 2);
    assert_eq!(updated_metadata.doc_hashes.len(), 2);
}
