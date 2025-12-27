# digrag Makefile
# Cross-compilation targets for portable binary distribution

.PHONY: all build build-release build-linux-static build-macos-arm build-macos-intel \
        install install-local test clean check fmt lint help

# Default target
all: build

# Development build
build:
	cargo build

# Release build (optimized)
build-release:
	cargo build --release

# Linux static binary (musl)
build-linux-static:
	cargo build --release --target x86_64-unknown-linux-musl

# macOS ARM64 (Apple Silicon)
build-macos-arm:
	cargo build --release --target aarch64-apple-darwin

# macOS x86_64 (Intel)
build-macos-intel:
	cargo build --release --target x86_64-apple-darwin

# Install to ~/.cargo/bin
install:
	cargo install --path . --locked

# Install to ~/.local/bin
install-local: build-release
	mkdir -p ~/.local/bin
	cp target/release/digrag ~/.local/bin/
	@echo "Installed to ~/.local/bin/digrag"
	@echo "Make sure ~/.local/bin is in your PATH"

# Run tests
test:
	cargo test

# Run tests with verbose output
test-verbose:
	cargo test -- --nocapture

# Type checking
check:
	cargo check

# Format code
fmt:
	cargo fmt

# Lint with clippy
lint:
	cargo clippy -- -D warnings

# Clean build artifacts
clean:
	cargo clean

# Show help
help:
	@echo "digrag Makefile targets:"
	@echo ""
	@echo "  build              - Development build"
	@echo "  build-release      - Release build (optimized)"
	@echo "  build-linux-static - Linux static binary (musl)"
	@echo "  build-macos-arm    - macOS ARM64 (Apple Silicon)"
	@echo "  build-macos-intel  - macOS x86_64 (Intel)"
	@echo ""
	@echo "  install            - Install via cargo"
	@echo "  install-local      - Install to ~/.local/bin"
	@echo ""
	@echo "  test               - Run tests"
	@echo "  test-verbose       - Run tests with output"
	@echo "  check              - Type checking"
	@echo "  fmt                - Format code"
	@echo "  lint               - Run clippy"
	@echo "  clean              - Clean build artifacts"
