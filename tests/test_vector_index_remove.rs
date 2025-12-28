//! Test for VectorIndex remove functionality
//!
//! Process 4: TDD Red Phase - VectorIndex Remove Tests

use digrag::index::VectorIndex;

/// Test: remove single vector by ID
#[test]
fn test_remove_single_vector() {
    let mut index = VectorIndex::new(3);
    index.add("doc1".to_string(), vec![1.0, 0.0, 0.0]).unwrap();
    index.add("doc2".to_string(), vec![0.0, 1.0, 0.0]).unwrap();

    assert_eq!(index.len(), 2);

    index.remove("doc1");

    assert_eq!(index.len(), 1);
    assert!(!index.contains("doc1"));
    assert!(index.contains("doc2"));
}

/// Test: remove non-existent vector does not panic
#[test]
fn test_remove_nonexistent_vector() {
    let mut index = VectorIndex::new(3);
    index.add("doc1".to_string(), vec![1.0, 0.0, 0.0]).unwrap();

    // Should not panic
    index.remove("nonexistent");

    assert_eq!(index.len(), 1);
}

/// Test: remove_batch removes multiple vectors
#[test]
fn test_remove_batch() {
    let mut index = VectorIndex::new(3);
    index.add("doc1".to_string(), vec![1.0, 0.0, 0.0]).unwrap();
    index.add("doc2".to_string(), vec![0.0, 1.0, 0.0]).unwrap();
    index.add("doc3".to_string(), vec![0.0, 0.0, 1.0]).unwrap();
    index.add("doc4".to_string(), vec![1.0, 1.0, 0.0]).unwrap();

    assert_eq!(index.len(), 4);

    index.remove_batch(&["doc1".to_string(), "doc3".to_string()]);

    assert_eq!(index.len(), 2);
    assert!(!index.contains("doc1"));
    assert!(index.contains("doc2"));
    assert!(!index.contains("doc3"));
    assert!(index.contains("doc4"));
}

/// Test: remove_batch with empty list does nothing
#[test]
fn test_remove_batch_empty() {
    let mut index = VectorIndex::new(3);
    index.add("doc1".to_string(), vec![1.0, 0.0, 0.0]).unwrap();

    index.remove_batch(&[]);

    assert_eq!(index.len(), 1);
}

/// Test: search after remove works correctly
#[test]
fn test_search_after_remove() {
    let mut index = VectorIndex::new(3);
    index.add("doc1".to_string(), vec![1.0, 0.0, 0.0]).unwrap();
    index.add("doc2".to_string(), vec![0.0, 1.0, 0.0]).unwrap();
    index.add("doc3".to_string(), vec![0.5, 0.5, 0.0]).unwrap();

    // Remove doc1
    index.remove("doc1");

    // Search with query similar to removed doc
    let results = index.search(&[1.0, 0.0, 0.0], 3).unwrap();

    // Should not find doc1, but find others
    assert!(!results.iter().any(|r| r.doc_id == "doc1"));
    assert!(results.len() <= 2);
}

/// Test: contains method works
#[test]
fn test_contains() {
    let mut index = VectorIndex::new(3);
    index.add("doc1".to_string(), vec![1.0, 0.0, 0.0]).unwrap();

    assert!(index.contains("doc1"));
    assert!(!index.contains("doc2"));
}

/// Test: remove all vectors
#[test]
fn test_remove_all_vectors() {
    let mut index = VectorIndex::new(3);
    index.add("doc1".to_string(), vec![1.0, 0.0, 0.0]).unwrap();
    index.add("doc2".to_string(), vec![0.0, 1.0, 0.0]).unwrap();

    index.remove("doc1");
    index.remove("doc2");

    assert!(index.is_empty());

    // Search on empty index should return empty results
    let results = index.search(&[1.0, 0.0, 0.0], 3).unwrap();
    assert!(results.is_empty());
}
