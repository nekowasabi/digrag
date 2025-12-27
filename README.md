# cl-search: Rust-based Changelog Search Engine

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

High-performance search engine for changelog memos, implemented in Rust with support for BM25 keyword search, semantic vector search, and hybrid search via MCP (Model Context Protocol).

## Features

- **Fast Search**: BM25 keyword-based search (~30 µs per query)
- **Semantic Search**: Vector-based semantic search with OpenRouter embeddings
- **Hybrid Search**: Combines BM25 and semantic results using Reciprocal Rank Fusion
- **Japanese Support**: Full Japanese text tokenization with Lindera IPADIC
- **MCP Integration**: Expose search functionality to Claude and other AI assistants
- **Portable Binary**: Single self-contained binary (~70MB) for macOS, Linux, Windows
- **Python Compatible**: 100% compatible with existing Python RAG indices
- **Query Rewriting**: LLM-based query optimization with caching

## Quick Start

### Build from Source

```bash
cd cl-search
cargo build --release
./target/release/cl-search --help
```

### Build Indices

```bash
# Create indices from changelogmemo file
./target/release/cl-search build --changelog ../changelogmemo --output ../.rag
```

### Search

```bash
# BM25 search
./target/release/cl-search search --index-dir ../.rag --query "メモ" --mode bm25

# Hybrid search (default)
./target/release/cl-search search --index-dir ../.rag --query "メモ"

# Semantic search
OPENROUTER_API_KEY=your-key ./target/release/cl-search search --index-dir ../.rag --query "メモ" --mode semantic
```

## Development

### Testing

```bash
cargo test                          # All tests
cargo test --test compatibility_test  # Integration tests
cargo test --lib                    # Unit tests
```

### Benchmarks

```bash
cargo bench
```

Results:
- BM25 search: ~30 µs per query
- Semantic search: ~3 ns (vector lookup)
- Hybrid search: ~34 µs per query

### Code Quality

```bash
cargo fmt --check
cargo clippy -- -D warnings
```

## Project Structure

```
cl-search/
├── src/
│   ├── lib.rs                 # Library exports
│   ├── main.rs                # CLI binary
│   ├── config/                # Configuration structures
│   ├── loader/                # Document loading and parsing
│   ├── tokenizer/             # Japanese tokenization
│   ├── index/                 # Search indices
│   ├── search/                # Search integration
│   ├── embedding/             # OpenRouter API client
│   ├── rewriter/              # Query rewriting
│   └── mcp/                   # MCP server implementation
├── tests/
│   └── compatibility_test.rs  # Python compatibility tests
├── benches/
│   └── search_bench.rs        # Performance benchmarks
├── Cargo.toml                 # Dependencies
└── README.md                  # This file
```

## Documentation

- [API Reference](../docs/API.md) - Complete API documentation
- [Migration Guide](../docs/MIGRATION.md) - Migrating from Python
- [Main README](../README.md) - Project overview

## License

MIT License - see LICENSE file for details
