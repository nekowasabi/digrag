# digrag: Portable Text RAG Search Engine

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

[日本語版](README_ja.md)

A high-performance, portable RAG (Retrieval-Augmented Generation) search engine for any text files. Search your notes, memos, documents, and other text content using BM25 keyword search, semantic vector search, or hybrid search. Seamlessly integrates with Claude via MCP (Model Context Protocol).

## Features

- **Japanese Support**: Full Japanese text tokenization with Lindera IPADIC
- **MCP Integration**: Expose search functionality to Claude Code and Claude Desktop
- **Fast Search**: BM25 keyword-based search (~30 microseconds per query)
- **Semantic Search**: Vector-based semantic search with OpenRouter embeddings
- **Hybrid Search**: Combines BM25 and semantic results using Reciprocal Rank Fusion
- **Portable Binary**: Single self-contained binary (~70MB) for macOS, Linux, Windows
- **Query Rewriting**: LLM-based query optimization with caching

## Installation

### Binary Download

Download the latest binary from [GitHub Releases](https://github.com/takets/digrag/releases):

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | `digrag-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `digrag-x86_64-apple-darwin.tar.gz` |
| Linux (x86_64) | `digrag-x86_64-unknown-linux-gnu.tar.gz` |
| Windows (x86_64) | `digrag-x86_64-pc-windows-msvc.zip` |

```bash
# Example: macOS Apple Silicon
curl -LO https://github.com/takets/digrag/releases/latest/download/digrag-aarch64-apple-darwin.tar.gz
tar xzf digrag-aarch64-apple-darwin.tar.gz
sudo mv digrag /usr/local/bin/
digrag --version
```

### Using install.sh

```bash
curl -sSL https://raw.githubusercontent.com/takets/digrag/main/install.sh | bash
```

### cargo install

If you have Rust 1.70+ installed:

```bash
cargo install digrag
```

### Build from Source

```bash
git clone https://github.com/takets/digrag.git
cd digrag
make build-release
./target/release/digrag --version
```

## Quick Start

### 1. Initialize Configuration

```bash
digrag init
```

This creates a configuration file at `~/.config/digrag/config.toml` (Linux/macOS) or `%APPDATA%\digrag\config.toml` (Windows).

### 2. Build Index

```bash
# BM25 only (fast, no API key required)
digrag build --input ~/notes --output ~/.digrag/index

# With semantic embeddings (requires OPENROUTER_API_KEY)
export OPENROUTER_API_KEY="sk-or-v1-..."
digrag build --input ~/notes --output ~/.digrag/index --with-embeddings
```

### 3. Search

```bash
# BM25 search
digrag search "meeting notes" --index-dir ~/.digrag/index

# Hybrid search
digrag search "project ideas" --index-dir ~/.digrag/index --mode hybrid

# Semantic search
digrag search "similar concepts" --index-dir ~/.digrag/index --mode semantic
```

## MCP Setup

### Claude Code

Add to your MCP configuration (`.mcp.json` or settings):

```json
{
  "mcpServers": {
    "digrag": {
      "command": "digrag",
      "args": ["serve", "--index-dir", "~/.digrag/index"],
      "env": {
        "OPENROUTER_API_KEY": "sk-or-v1-..."
      }
    }
  }
}
```

### Claude Desktop

Add to `~/.claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "digrag": {
      "command": "/usr/local/bin/digrag",
      "args": ["serve", "--index-dir", "/Users/yourname/.digrag/index"],
      "env": {
        "OPENROUTER_API_KEY": "sk-or-v1-..."
      }
    }
  }
}
```

### Available MCP Tools

Once configured, Claude can use these tools:

| Tool | Description |
|------|-------------|
| `query_memos` | Search documents using BM25, semantic, or hybrid mode |
| `list_tags` | List all available tags with document counts |
| `get_recent_memos` | Get the most recently modified documents |

### Initial Setup Steps

```bash
# 1. Initialize configuration
digrag init

# 2. Build your index
digrag build --input ~/Documents/notes --output ~/.digrag/index --with-embeddings

# 3. Test the server manually (optional)
digrag serve --index-dir ~/.digrag/index

# 4. Configure Claude Code or Claude Desktop (see above)
```

## Command Reference

### init

Initialize digrag configuration file.

```bash
digrag init [OPTIONS]
```

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--force` | `-f` | Overwrite existing configuration | `false` |

### serve

Start the MCP server (communicates via stdin/stdout).

```bash
digrag serve [OPTIONS]
```

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--index-dir` | `-i` | Path to index directory | `.rag` |

### build

Build search indices from text files.

```bash
digrag build --input <PATH> [OPTIONS]
```

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--input` | `-i` | Source file or directory (required) | - |
| `--output` | `-o` | Output index directory | `.rag` |
| `--with-embeddings` | - | Generate embeddings (requires `OPENROUTER_API_KEY`) | `false` |
| `--skip-embeddings` | - | Skip embedding generation (BM25 only) | `false` |
| `--incremental` | - | Use incremental build (only process changed documents) | `false` |
| `--force` | - | Force full rebuild even with `--incremental` | `false` |

### search

Search the index from command line (for testing).

```bash
digrag search <QUERY> [OPTIONS]
```

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `<query>` | - | Search query (required) | - |
| `--index-dir` | `-i` | Path to index directory | `.rag` |
| `--top-k` | `-k` | Number of results to return | `10` |
| `--mode` | `-m` | Search mode: `bm25`, `semantic`, `hybrid` | `bm25` |
| `--tag` | `-t` | Filter results by tag | - |

### Global Options

| Option | Short | Description |
|--------|-------|-------------|
| `--verbose` | `-v` | Enable verbose logging |
| `--help` | `-h` | Show help information |
| `--version` | `-V` | Show version |

## Incremental Build

The incremental build feature significantly reduces embedding API costs by only processing documents that have changed since the last build.

### How It Works

1. **Content Hashing**: Each document gets a unique ID based on SHA256 hash of its title and content
2. **Change Detection**: When rebuilding, digrag compares new documents against stored hashes
3. **Selective Processing**: Only added/modified documents get new embeddings generated
4. **Automatic Cleanup**: Removed documents are automatically deleted from the index

### Usage

```bash
# Initial build (full)
digrag build --input ~/notes --output ~/.digrag/index --with-embeddings

# Subsequent builds (incremental - only processes changes)
digrag build --input ~/notes --output ~/.digrag/index --with-embeddings --incremental

# Force full rebuild if needed
digrag build --input ~/notes --output ~/.digrag/index --with-embeddings --incremental --force
```

### Output Example

```
Using incremental build mode
Loaded 640 documents total

Incremental build summary:
  Added: 5 documents
  Modified: 2 documents
  Removed: 1 documents
  Unchanged: 632 documents
  Embeddings needed: 7
```

In this example, only 7 documents need new embeddings instead of all 640, reducing API costs by ~99%.

## Use Cases

- **Personal Note Search**: Search through your markdown notes, journals, and memos
- **Documentation Search**: Index and search project documentation, wikis, or knowledge bases
- **Learning Materials**: Organize and search study notes, research papers, and references
- **Code Documentation**: Search through code comments, READMEs, and technical documents

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
- BM25 search: ~30 microseconds per query
- Semantic search: ~3 ns (vector lookup)
- Hybrid search: ~34 microseconds per query

### Code Quality

```bash
cargo fmt --check
cargo clippy -- -D warnings
```

## Project Structure

```
digrag/
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

## License

MIT License - see [LICENSE](LICENSE) file for details.
