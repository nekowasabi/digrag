//! Test for IncrementalDiff functionality
//!
//! Process 2: TDD Red Phase - Incremental Diff Tests

use chrono::{TimeZone, Utc};
use digrag::index::IncrementalDiff;
use digrag::loader::Document;
use std::collections::HashMap;

fn create_test_doc(title: &str, text: &str) -> Document {
    let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
    Document::with_content_id(title.to_string(), date, vec![], text.to_string())
}

/// Test: New document is classified as added
#[test]
fn test_new_document_classified_as_added() {
    let doc = create_test_doc("New Title", "New content");
    let existing_hashes: HashMap<String, String> = HashMap::new();

    let diff = IncrementalDiff::compute(vec![doc.clone()], &existing_hashes);

    assert_eq!(diff.added.len(), 1);
    assert_eq!(diff.added[0].id, doc.id);
    assert!(diff.modified.is_empty());
    assert!(diff.removed.is_empty());
    assert!(diff.unchanged.is_empty());
}

/// Test: Existing document with same content is classified as unchanged
#[test]
fn test_unchanged_document_classified_correctly() {
    let doc = create_test_doc("Existing Title", "Existing content");
    let mut existing_hashes: HashMap<String, String> = HashMap::new();
    existing_hashes.insert(doc.id.clone(), doc.content_hash());

    let diff = IncrementalDiff::compute(vec![doc.clone()], &existing_hashes);

    assert!(diff.added.is_empty());
    assert!(diff.modified.is_empty());
    assert!(diff.removed.is_empty());
    assert_eq!(diff.unchanged.len(), 1);
    assert_eq!(diff.unchanged[0], doc.id);
}

/// Test: Existing document with modified content is classified as modified
#[test]
fn test_modified_document_classified_correctly() {
    // Original document
    let original = create_test_doc("Title", "Original content");
    let mut existing_hashes: HashMap<String, String> = HashMap::new();
    existing_hashes.insert(original.id.clone(), original.content_hash());

    // Modified document (same ID but different content hash)
    // Since content hash determines ID, we need to use with_id to simulate modification
    let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
    let modified = Document::with_id(
        original.id.clone(),  // Same ID
        "Title".to_string(),
        date,
        vec![],
        "Modified content".to_string(),  // Different content
    );

    let diff = IncrementalDiff::compute(vec![modified.clone()], &existing_hashes);

    assert!(diff.added.is_empty());
    assert_eq!(diff.modified.len(), 1);
    assert_eq!(diff.modified[0].id, original.id);
    assert!(diff.removed.is_empty());
    assert!(diff.unchanged.is_empty());
}

/// Test: Document not in new list is classified as removed
#[test]
fn test_removed_document_classified_correctly() {
    let doc = create_test_doc("Old Title", "Old content");
    let mut existing_hashes: HashMap<String, String> = HashMap::new();
    existing_hashes.insert(doc.id.clone(), doc.content_hash());

    // Empty new documents list
    let diff = IncrementalDiff::compute(vec![], &existing_hashes);

    assert!(diff.added.is_empty());
    assert!(diff.modified.is_empty());
    assert_eq!(diff.removed.len(), 1);
    assert_eq!(diff.removed[0], doc.id);
    assert!(diff.unchanged.is_empty());
}

/// Test: Mixed scenario with added, modified, unchanged, and removed documents
#[test]
fn test_mixed_scenario() {
    let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();

    // Existing documents
    let unchanged_doc = create_test_doc("Unchanged", "Same content");
    let to_modify_doc = create_test_doc("ToModify", "Original content");
    let to_remove_doc = create_test_doc("ToRemove", "Will be removed");

    let mut existing_hashes: HashMap<String, String> = HashMap::new();
    existing_hashes.insert(unchanged_doc.id.clone(), unchanged_doc.content_hash());
    existing_hashes.insert(to_modify_doc.id.clone(), to_modify_doc.content_hash());
    existing_hashes.insert(to_remove_doc.id.clone(), to_remove_doc.content_hash());

    // New documents
    let new_doc = create_test_doc("NewDoc", "Brand new content");
    let modified_doc = Document::with_id(
        to_modify_doc.id.clone(),
        "ToModify".to_string(),
        date,
        vec![],
        "Modified content".to_string(),
    );

    let new_docs = vec![unchanged_doc.clone(), modified_doc, new_doc.clone()];

    let diff = IncrementalDiff::compute(new_docs, &existing_hashes);

    assert_eq!(diff.added.len(), 1, "Should have 1 added document");
    assert_eq!(diff.modified.len(), 1, "Should have 1 modified document");
    assert_eq!(diff.removed.len(), 1, "Should have 1 removed document");
    assert_eq!(diff.unchanged.len(), 1, "Should have 1 unchanged document");

    assert_eq!(diff.added[0].id, new_doc.id);
    assert_eq!(diff.modified[0].id, to_modify_doc.id);
    assert_eq!(diff.removed[0], to_remove_doc.id);
    assert_eq!(diff.unchanged[0], unchanged_doc.id);
}

/// Test: Empty existing index means all documents are added
#[test]
fn test_empty_existing_index() {
    let docs = vec![
        create_test_doc("Doc1", "Content 1"),
        create_test_doc("Doc2", "Content 2"),
    ];
    let existing_hashes: HashMap<String, String> = HashMap::new();

    let diff = IncrementalDiff::compute(docs.clone(), &existing_hashes);

    assert_eq!(diff.added.len(), 2);
    assert!(diff.modified.is_empty());
    assert!(diff.removed.is_empty());
    assert!(diff.unchanged.is_empty());
}

/// Test: Empty new documents means all existing are removed
#[test]
fn test_empty_new_documents() {
    let doc1 = create_test_doc("Doc1", "Content 1");
    let doc2 = create_test_doc("Doc2", "Content 2");

    let mut existing_hashes: HashMap<String, String> = HashMap::new();
    existing_hashes.insert(doc1.id.clone(), doc1.content_hash());
    existing_hashes.insert(doc2.id.clone(), doc2.content_hash());

    let diff = IncrementalDiff::compute(vec![], &existing_hashes);

    assert!(diff.added.is_empty());
    assert!(diff.modified.is_empty());
    assert_eq!(diff.removed.len(), 2);
    assert!(diff.unchanged.is_empty());
}

/// Test: Summary statistics are correct
#[test]
fn test_summary_statistics() {
    let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();

    let unchanged = create_test_doc("Unchanged", "Same");
    let to_modify = create_test_doc("ToModify", "Original");
    let to_remove = create_test_doc("ToRemove", "Gone");

    let mut existing_hashes: HashMap<String, String> = HashMap::new();
    existing_hashes.insert(unchanged.id.clone(), unchanged.content_hash());
    existing_hashes.insert(to_modify.id.clone(), to_modify.content_hash());
    existing_hashes.insert(to_remove.id.clone(), to_remove.content_hash());

    let new_doc = create_test_doc("New", "Brand new");
    let modified = Document::with_id(
        to_modify.id.clone(),
        "ToModify".to_string(),
        date,
        vec![],
        "Changed".to_string(),
    );

    let diff = IncrementalDiff::compute(vec![unchanged, modified, new_doc], &existing_hashes);

    assert_eq!(diff.added_count(), 1);
    assert_eq!(diff.modified_count(), 1);
    assert_eq!(diff.removed_count(), 1);
    assert_eq!(diff.unchanged_count(), 1);
    assert_eq!(diff.embeddings_needed(), 2); // added + modified
}

/// Test: needs_embedding returns correct documents
#[test]
fn test_needs_embedding() {
    let date = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();

    let unchanged = create_test_doc("Unchanged", "Same");
    let to_modify = create_test_doc("ToModify", "Original");

    let mut existing_hashes: HashMap<String, String> = HashMap::new();
    existing_hashes.insert(unchanged.id.clone(), unchanged.content_hash());
    existing_hashes.insert(to_modify.id.clone(), to_modify.content_hash());

    let new_doc = create_test_doc("New", "Brand new");
    let modified = Document::with_id(
        to_modify.id.clone(),
        "ToModify".to_string(),
        date,
        vec![],
        "Changed".to_string(),
    );

    let diff = IncrementalDiff::compute(vec![unchanged, modified.clone(), new_doc.clone()], &existing_hashes);

    let needs_embedding = diff.needs_embedding();
    assert_eq!(needs_embedding.len(), 2);

    let ids: Vec<&String> = needs_embedding.iter().map(|d| &d.id).collect();
    assert!(ids.contains(&&new_doc.id));
    assert!(ids.contains(&&to_modify.id));
}
