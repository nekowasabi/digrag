//! Test for CLI incremental build options
//!
//! Process 6: TDD Red Phase - CLI Incremental Tests

use std::process::Command;
use tempfile::tempdir;

/// Test: --incremental flag is recognized
#[test]
fn test_incremental_flag_recognized() {
    let output = Command::new("cargo")
        .args(["run", "--", "build", "--help"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    assert!(
        combined.contains("--incremental"),
        "CLI should recognize --incremental flag"
    );
}

/// Test: --force flag is recognized
#[test]
fn test_force_flag_recognized() {
    let output = Command::new("cargo")
        .args(["run", "--", "build", "--help"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    assert!(
        combined.contains("--force"),
        "CLI should recognize --force flag"
    );
}

/// Test: build without flags uses full build (default behavior)
#[test]
fn test_default_build_behavior() {
    let dir = tempdir().unwrap();
    let test_file = dir.path().join("test.md");
    std::fs::write(&test_file, "# 2025-01-15\n## Test Entry\n#memo\nContent").unwrap();

    let output_dir = dir.path().join("output");

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "build",
            "--input",
            test_file.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Build should succeed: {:?}",
        output
    );
    assert!(
        output_dir.join("metadata.json").exists(),
        "Metadata should be created"
    );
}

/// Test: build with --incremental uses incremental build when possible
#[test]
fn test_incremental_build_flag() {
    let dir = tempdir().unwrap();
    let test_file = dir.path().join("test.md");
    std::fs::write(&test_file, "# 2025-01-15\n## Test Entry\n#memo\nContent").unwrap();

    let output_dir = dir.path().join("output");

    // First build (full)
    let output1 = Command::new("cargo")
        .args([
            "run",
            "--",
            "build",
            "--input",
            test_file.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output1.status.success(), "First build should succeed");

    // Second build (incremental)
    let output2 = Command::new("cargo")
        .args([
            "run",
            "--",
            "build",
            "--input",
            test_file.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--incremental",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output2.status.success(), "Incremental build should succeed");

    let stderr = String::from_utf8_lossy(&output2.stderr);
    // Should indicate incremental mode
    assert!(
        stderr.contains("Incremental")
            || stderr.contains("incremental")
            || stderr.contains("unchanged"),
        "Output should mention incremental build: {}",
        stderr
    );
}

/// Test: --force with --incremental forces full rebuild
#[test]
fn test_force_full_rebuild() {
    let dir = tempdir().unwrap();
    let test_file = dir.path().join("test.md");
    std::fs::write(&test_file, "# 2025-01-15\n## Test Entry\n#memo\nContent").unwrap();

    let output_dir = dir.path().join("output");

    // First build
    let _output1 = Command::new("cargo")
        .args([
            "run",
            "--",
            "build",
            "--input",
            test_file.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Force full rebuild
    let output2 = Command::new("cargo")
        .args([
            "run",
            "--",
            "build",
            "--input",
            test_file.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--incremental",
            "--force",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output2.status.success(), "Force rebuild should succeed");

    let stderr = String::from_utf8_lossy(&output2.stderr);
    // Should indicate full rebuild
    assert!(
        stderr.contains("full")
            || stderr.contains("Full")
            || stderr.contains("rebuild")
            || stderr.contains("force"),
        "Output should mention full rebuild when --force is used: {}",
        stderr
    );
}
