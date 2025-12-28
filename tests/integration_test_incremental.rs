//! Integration tests for incremental build functionality
//!
//! Process 10: E2E Tests for Incremental Build

use std::process::Command;
use tempfile::tempdir;
use std::fs;

/// Helper to create a changelog file
/// Uses the format: * Title YYYY-MM-DD HH:MM:SS [tags]:
fn create_changelog(path: &std::path::Path, entries: &[(&str, &str, &str)]) {
    let mut content = String::new();
    for (date, title, text) in entries {
        // Format: * Title 2025-01-15 10:00:00 [memo]:
        content.push_str(&format!("* {} {} 10:00:00 [memo]:\n{}\n", title, date, text));
    }
    fs::write(path, content).unwrap();
}

/// E2E Test: Full build -> Add document -> Incremental build
#[test]
fn test_e2e_incremental_add_document() {
    let dir = tempdir().unwrap();
    let changelog = dir.path().join("changelog.md");
    let output = dir.path().join("output");

    // Initial: 2 documents
    create_changelog(&changelog, &[
        ("2025-01-15", "Entry 1", "Content for entry 1"),
        ("2025-01-16", "Entry 2", "Content for entry 2"),
    ]);

    // First build (full)
    let result1 = Command::new("cargo")
        .args([
            "run", "--",
            "build",
            "-i", changelog.to_str().unwrap(),
            "-o", output.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run build");

    assert!(result1.status.success(), "Initial build failed: {:?}", result1);

    // Verify initial metadata
    let metadata_path = output.join("metadata.json");
    let metadata_content = fs::read_to_string(&metadata_path).unwrap();
    assert!(metadata_content.contains("\"doc_count\": 2"), "Should have 2 docs: {}", metadata_content);

    // Add a third document
    create_changelog(&changelog, &[
        ("2025-01-15", "Entry 1", "Content for entry 1"),
        ("2025-01-16", "Entry 2", "Content for entry 2"),
        ("2025-01-17", "Entry 3", "Content for entry 3"),
    ]);

    // Incremental build
    let result2 = Command::new("cargo")
        .args([
            "run", "--",
            "build",
            "-i", changelog.to_str().unwrap(),
            "-o", output.to_str().unwrap(),
            "--incremental",
        ])
        .output()
        .expect("Failed to run incremental build");

    assert!(result2.status.success(), "Incremental build failed: {:?}", result2);

    // Verify updated metadata
    let updated_metadata = fs::read_to_string(&metadata_path).unwrap();
    assert!(updated_metadata.contains("\"doc_count\": 3"), "Should have 3 docs: {}", updated_metadata);

    // Check stderr for incremental message
    let stderr = String::from_utf8_lossy(&result2.stderr);
    assert!(
        stderr.contains("incremental") || stderr.contains("Incremental"),
        "Should indicate incremental mode: {}", stderr
    );
}

/// E2E Test: Full build -> Modify document -> Incremental build
#[test]
fn test_e2e_incremental_modify_document() {
    let dir = tempdir().unwrap();
    let changelog = dir.path().join("changelog.md");
    let output = dir.path().join("output");

    // Initial
    create_changelog(&changelog, &[
        ("2025-01-15", "Entry 1", "Original content"),
    ]);

    // First build
    let result1 = Command::new("cargo")
        .args([
            "run", "--",
            "build",
            "-i", changelog.to_str().unwrap(),
            "-o", output.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run build");

    assert!(result1.status.success());

    // Get initial hash
    let metadata1 = fs::read_to_string(output.join("metadata.json")).unwrap();

    // Modify document content
    create_changelog(&changelog, &[
        ("2025-01-15", "Entry 1", "Modified content"),
    ]);

    // Incremental build
    let result2 = Command::new("cargo")
        .args([
            "run", "--",
            "build",
            "-i", changelog.to_str().unwrap(),
            "-o", output.to_str().unwrap(),
            "--incremental",
        ])
        .output()
        .expect("Failed to run incremental build");

    assert!(result2.status.success());

    // Hash should be different
    let metadata2 = fs::read_to_string(output.join("metadata.json")).unwrap();

    // Parse and compare doc_hashes
    let hashes1: serde_json::Value = serde_json::from_str(&metadata1).unwrap();
    let hashes2: serde_json::Value = serde_json::from_str(&metadata2).unwrap();

    assert_ne!(
        hashes1["doc_hashes"], hashes2["doc_hashes"],
        "Doc hashes should be different after modification"
    );
}

/// E2E Test: Full build -> Remove document -> Incremental build
#[test]
fn test_e2e_incremental_remove_document() {
    let dir = tempdir().unwrap();
    let changelog = dir.path().join("changelog.md");
    let output = dir.path().join("output");

    // Initial: 3 documents
    create_changelog(&changelog, &[
        ("2025-01-15", "Entry 1", "Content 1"),
        ("2025-01-16", "Entry 2", "Content 2"),
        ("2025-01-17", "Entry 3", "Content 3"),
    ]);

    // First build
    let result1 = Command::new("cargo")
        .args([
            "run", "--",
            "build",
            "-i", changelog.to_str().unwrap(),
            "-o", output.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run build");

    assert!(result1.status.success());

    // Remove middle document
    create_changelog(&changelog, &[
        ("2025-01-15", "Entry 1", "Content 1"),
        ("2025-01-17", "Entry 3", "Content 3"),
    ]);

    // Incremental build
    let result2 = Command::new("cargo")
        .args([
            "run", "--",
            "build",
            "-i", changelog.to_str().unwrap(),
            "-o", output.to_str().unwrap(),
            "--incremental",
        ])
        .output()
        .expect("Failed to run incremental build");

    assert!(result2.status.success());

    // Verify doc count
    let metadata = fs::read_to_string(output.join("metadata.json")).unwrap();
    assert!(metadata.contains("\"doc_count\": 2"), "Should have 2 docs after removal: {}", metadata);
}

/// E2E Test: Unchanged documents are skipped
#[test]
fn test_e2e_unchanged_documents_skipped() {
    let dir = tempdir().unwrap();
    let changelog = dir.path().join("changelog.md");
    let output = dir.path().join("output");

    // Initial
    create_changelog(&changelog, &[
        ("2025-01-15", "Entry 1", "Content 1"),
        ("2025-01-16", "Entry 2", "Content 2"),
    ]);

    // First build
    let _result1 = Command::new("cargo")
        .args([
            "run", "--",
            "build",
            "-i", changelog.to_str().unwrap(),
            "-o", output.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run build");

    // Same content, incremental build
    let result2 = Command::new("cargo")
        .args([
            "run", "--",
            "build",
            "-i", changelog.to_str().unwrap(),
            "-o", output.to_str().unwrap(),
            "--incremental",
        ])
        .output()
        .expect("Failed to run incremental build");

    assert!(result2.status.success());

    let stderr = String::from_utf8_lossy(&result2.stderr);
    // Should indicate documents are unchanged
    assert!(
        stderr.contains("unchanged") || stderr.contains("Unchanged") ||
        stderr.contains("0 added") || stderr.contains("skip"),
        "Should indicate documents are unchanged: {}", stderr
    );
}

/// E2E Test: Force flag triggers full rebuild
#[test]
fn test_e2e_force_full_rebuild() {
    let dir = tempdir().unwrap();
    let changelog = dir.path().join("changelog.md");
    let output = dir.path().join("output");

    create_changelog(&changelog, &[
        ("2025-01-15", "Entry 1", "Content 1"),
    ]);

    // First build
    let _result1 = Command::new("cargo")
        .args([
            "run", "--",
            "build",
            "-i", changelog.to_str().unwrap(),
            "-o", output.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run build");

    // Force rebuild
    let result2 = Command::new("cargo")
        .args([
            "run", "--",
            "build",
            "-i", changelog.to_str().unwrap(),
            "-o", output.to_str().unwrap(),
            "--incremental",
            "--force",
        ])
        .output()
        .expect("Failed to run force rebuild");

    assert!(result2.status.success());

    let stderr = String::from_utf8_lossy(&result2.stderr);
    assert!(
        stderr.contains("Force") || stderr.contains("force") || stderr.contains("full"),
        "Should indicate force/full rebuild: {}", stderr
    );
}
