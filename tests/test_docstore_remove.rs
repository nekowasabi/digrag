//! Test for Docstore remove functionality
//!
//! Process 4: TDD Red Phase - Docstore Remove Tests

use chrono::Utc;
use digrag::index::Docstore;
use digrag::loader::Document;

fn create_test_doc(id: &str, title: &str) -> Document {
    Document::with_id(
        id.to_string(),
        title.to_string(),
        Utc::now(),
        vec![],
        "Content".to_string(),
    )
}

/// Test: remove single document by ID
#[test]
fn test_remove_single_document() {
    let mut store = Docstore::new();
    store.add(create_test_doc("doc1", "Title 1"));
    store.add(create_test_doc("doc2", "Title 2"));

    assert_eq!(store.len(), 2);

    store.remove("doc1");

    assert_eq!(store.len(), 1);
    assert!(!store.contains("doc1"));
    assert!(store.contains("doc2"));
}

/// Test: remove non-existent document does not panic
#[test]
fn test_remove_nonexistent_document() {
    let mut store = Docstore::new();
    store.add(create_test_doc("doc1", "Title 1"));

    // Should not panic
    store.remove("nonexistent");

    assert_eq!(store.len(), 1);
}

/// Test: remove_batch removes multiple documents
#[test]
fn test_remove_batch() {
    let mut store = Docstore::new();
    store.add(create_test_doc("doc1", "Title 1"));
    store.add(create_test_doc("doc2", "Title 2"));
    store.add(create_test_doc("doc3", "Title 3"));
    store.add(create_test_doc("doc4", "Title 4"));

    assert_eq!(store.len(), 4);

    store.remove_batch(&["doc1".to_string(), "doc3".to_string()]);

    assert_eq!(store.len(), 2);
    assert!(!store.contains("doc1"));
    assert!(store.contains("doc2"));
    assert!(!store.contains("doc3"));
    assert!(store.contains("doc4"));
}

/// Test: remove_batch with empty list does nothing
#[test]
fn test_remove_batch_empty() {
    let mut store = Docstore::new();
    store.add(create_test_doc("doc1", "Title 1"));

    store.remove_batch(&[]);

    assert_eq!(store.len(), 1);
}

/// Test: remove_batch with some non-existent IDs
#[test]
fn test_remove_batch_partial_nonexistent() {
    let mut store = Docstore::new();
    store.add(create_test_doc("doc1", "Title 1"));
    store.add(create_test_doc("doc2", "Title 2"));

    store.remove_batch(&["doc1".to_string(), "nonexistent".to_string()]);

    assert_eq!(store.len(), 1);
    assert!(!store.contains("doc1"));
    assert!(store.contains("doc2"));
}

/// Test: remove all documents
#[test]
fn test_remove_all_documents() {
    let mut store = Docstore::new();
    store.add(create_test_doc("doc1", "Title 1"));
    store.add(create_test_doc("doc2", "Title 2"));

    store.remove("doc1");
    store.remove("doc2");

    assert!(store.is_empty());
}
