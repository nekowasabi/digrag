//! digrag: Command-line interface for the changelog search MCP server

use anyhow::Result;
use digrag::config::{SearchConfig, SearchMode, path_resolver, app_config::AppConfig};
use digrag::index::IndexBuilder;
use digrag::search::Searcher;
use clap::{ArgAction, Parser, Subcommand};
use rmcp::{
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool, ServerHandler, ServiceExt,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{stdin, stdout};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// ============================================================================
// Path Resolution Helper
// ============================================================================

/// Resolve a path using the path_resolver module
fn resolve_path(path: &str) -> String {
    path_resolver::resolve_path(path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string())
}

// ============================================================================
// MCP Server Implementation
// ============================================================================

/// MCP Server for changelog search
#[derive(Clone)]
struct DigragMcpServer {
    searcher: Arc<Searcher>,
}

/// Request parameters for query_memos tool
#[derive(Debug, Deserialize, JsonSchema)]
struct QueryMemosParams {
    /// Search query string (required for search)
    #[serde(default)]
    query: String,
    /// Number of results to return (default: 10)
    #[serde(default = "default_top_k")]
    top_k: usize,
    /// Optional tag filter
    tag_filter: Option<String>,
    /// Search mode: "bm25", "semantic", or "hybrid" (default: "bm25")
    #[serde(default = "default_mode")]
    mode: String,
}

fn default_top_k() -> usize {
    10
}

fn default_mode() -> String {
    "bm25".to_string()
}

/// Request parameters for get_recent_memos tool
#[derive(Debug, Deserialize, JsonSchema)]
struct GetRecentMemosParams {
    /// Number of memos to return (default: 10)
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    10
}

#[tool(tool_box)]
impl DigragMcpServer {
    fn new(index_dir: String) -> Result<Self> {
        // Check if API key is available for semantic search
        let searcher = if let Ok(api_key) = std::env::var("OPENROUTER_API_KEY") {
            tracing::info!("OPENROUTER_API_KEY found, enabling semantic search");
            let embedding_client = digrag::embedding::OpenRouterEmbedding::new(api_key);
            Searcher::with_embedding_client(&index_dir, embedding_client)?
        } else {
            tracing::info!("OPENROUTER_API_KEY not set, semantic search disabled");
            Searcher::new(&index_dir)?
        };
        Ok(Self {
            searcher: Arc::new(searcher),
        })
    }

    /// Search memos by query with optional filters
    #[tool(description = "Search changelog memos using BM25 or semantic search")]
    fn query_memos(&self, #[tool(aggr)] params: QueryMemosParams) -> Result<CallToolResult, rmcp::Error> {
        let search_mode = match params.mode.as_str() {
            "semantic" => SearchMode::Semantic,
            "hybrid" => SearchMode::Hybrid,
            _ => SearchMode::Bm25,
        };

        let config = SearchConfig::new()
            .with_mode(search_mode)
            .with_top_k(params.top_k)
            .with_tag_filter(params.tag_filter);

        let results = self.searcher.search(&params.query, &config)
            .map_err(|e| rmcp::Error::internal_error(e.to_string(), None))?;

        let mut output = format!("Found {} results for '{}':\n\n", results.len(), params.query);

        // Add warning if semantic/hybrid search was requested but no vector index
        if (search_mode == SearchMode::Semantic || search_mode == SearchMode::Hybrid)
            && !self.searcher.has_vector_index()
        {
            output.push_str("Note: Vector index is not available. Semantic search requires embeddings.\n");
            output.push_str("To enable semantic search, rebuild the index with embeddings using:\n");
            output.push_str("  digrag build --input <file> --output <dir> --with-embeddings\n\n");
        }
        for (i, result) in results.iter().enumerate() {
            if let Some(doc) = self.searcher.docstore().get(&result.doc_id) {
                output.push_str(&format!(
                    "{}. [score: {:.4}] {}\n   Date: {}\n   Tags: {:?}\n   {}\n\n",
                    i + 1,
                    result.score,
                    doc.title(),
                    doc.date().format("%Y-%m-%d"),
                    doc.tags(),
                    doc.text.chars().take(150).collect::<String>()
                ));
            }
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// List all available tags in the changelog
    #[tool(description = "List all tags in the changelog with their document counts")]
    fn list_tags(&self) -> Result<CallToolResult, rmcp::Error> {
        let tags = self.searcher.list_tags();
        let mut output = format!("Found {} tags:\n\n", tags.len());

        for tag in &tags {
            let count = self.searcher.docstore().get_by_tag(tag).len();
            output.push_str(&format!("- {} ({})\n", tag, count));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Get recent memos
    #[tool(description = "Get the most recent changelog memos")]
    fn get_recent_memos(&self, #[tool(aggr)] params: GetRecentMemosParams) -> Result<CallToolResult, rmcp::Error> {
        let memos = self.searcher.get_recent_memos(params.limit);
        let mut output = format!("Recent {} memos:\n\n", memos.len());

        for (i, doc) in memos.iter().enumerate() {
            output.push_str(&format!(
                "{}. {}\n   Date: {}\n   Tags: {:?}\n   {}\n\n",
                i + 1,
                doc.title(),
                doc.date().format("%Y-%m-%d %H:%M"),
                doc.tags(),
                doc.text.chars().take(150).collect::<String>()
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }
}

#[tool(tool_box)]
impl ServerHandler for DigragMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Changelog memo search server with BM25 and semantic search capabilities".into()),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            ..Default::default()
        }
    }
}

// ============================================================================
// CLI Implementation
// ============================================================================

/// digrag: Rust-based MCP server for changelog memo search
#[derive(Parser)]
#[command(name = "digrag")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize digrag configuration
    Init {
        /// Force overwrite existing configuration
        #[arg(short, long)]
        force: bool,
    },
    /// Start the MCP server
    Serve {
        /// Path to the index directory (default: .rag)
        #[arg(short, long, default_value = ".rag")]
        index_dir: String,
    },
    /// Build search indices from changelog file
    Build {
        /// Path to the changelog file(s) or directory(s) - can be specified multiple times
        #[arg(short, long, action = ArgAction::Append)]
        input: Vec<String>,

        /// Path to the output index directory
        #[arg(short, long, default_value = ".rag")]
        output: String,

        /// Skip embedding generation (BM25 only)
        #[arg(long)]
        skip_embeddings: bool,

        /// Generate embeddings for semantic search (requires OPENROUTER_API_KEY)
        #[arg(long)]
        with_embeddings: bool,
    },
    /// Search the changelog (for testing)
    Search {
        /// Search query
        query: String,

        /// Path to the index directory
        #[arg(short, long, default_value = ".rag")]
        index_dir: String,

        /// Number of results to return
        #[arg(short, long, default_value = "10")]
        top_k: usize,

        /// Search mode: bm25, semantic, or hybrid
        #[arg(short, long, default_value = "bm25")]
        mode: String,

        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging (to stderr to not interfere with MCP stdio)
    let log_level = if cli.verbose { "debug" } else { "warn" };
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| log_level.to_string()),
        ))
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_writer(std::io::stderr),
        )
        .init();

    match cli.command {
        Commands::Init { force } => {
            let config_dir = path_resolver::get_config_dir();
            let config_path = config_dir.join("config.toml");
            
            eprintln!("Initializing digrag configuration...");
            eprintln!("Config directory: {}", config_dir.display());
            
            // Create config directory
            if !config_dir.exists() {
                std::fs::create_dir_all(&config_dir)?;
                eprintln!("Created config directory");
            }
            
            // Check if config already exists
            if config_path.exists() && !force {
                eprintln!("Configuration file already exists: {}", config_path.display());
                eprintln!("Use --force to overwrite");
                return Ok(());
            }
            
            // Create default config
            let default_config = AppConfig::default();
            let toml_content = default_config.to_toml()?;
            std::fs::write(&config_path, &toml_content)?;
            
            eprintln!("Created configuration file: {}", config_path.display());
            eprintln!("\nConfiguration initialized successfully!");
            eprintln!("Edit {} to customize settings.", config_path.display());
            
            Ok(())
        }
        Commands::Serve { index_dir } => {
            let resolved_index_dir = resolve_path(&index_dir);
            tracing::info!("Starting MCP server with index directory: {}", resolved_index_dir);
            eprintln!("digrag MCP server starting... (index_dir: {})", resolved_index_dir);

            // Create MCP server with searcher
            let server = DigragMcpServer::new(resolved_index_dir)?;
            eprintln!("Index loaded. Starting MCP stdio transport...");

            // Serve via stdio transport
            let transport = (stdin(), stdout());
            let service = server.serve(transport).await?;

            // Wait for service to complete
            let _quit_reason = service.waiting().await?;
            Ok(())
        }
        Commands::Build {
            input,
            output,
            skip_embeddings: _,
            with_embeddings,
        } => {
            if input.is_empty() {
                return Err(anyhow::anyhow!("At least one --input is required"));
            }

            let resolved_inputs: Vec<String> = input.iter().map(|i| resolve_path(i)).collect();
            let resolved_output = resolve_path(&output);

            eprintln!("Building indices from {} input(s) to {}", resolved_inputs.len(), resolved_output);
            for (i, path) in resolved_inputs.iter().enumerate() {
                eprintln!("  Input {}: {}", i + 1, path);
            }

            if with_embeddings {
                // Get API key from environment
                let api_key = std::env::var("OPENROUTER_API_KEY")
                    .map_err(|_| anyhow::anyhow!("OPENROUTER_API_KEY environment variable not set. Required for --with-embeddings"))?;

                eprintln!("Embedding generation enabled (using OpenRouter API)");

                let builder = IndexBuilder::with_embeddings(api_key);

                // Process each input
                for (idx, resolved_input) in resolved_inputs.iter().enumerate() {
                    eprintln!("\nProcessing input {}/{}: {}", idx + 1, resolved_inputs.len(), resolved_input);
                    builder.build_with_embeddings(
                        Path::new(resolved_input),
                        Path::new(&resolved_output),
                        |step, total, msg| {
                            eprintln!("[{}/{}] {}", step, total, msg);
                        },
                    ).await?;
                }
            } else {
                let builder = IndexBuilder::new();

                // Process each input
                for (idx, resolved_input) in resolved_inputs.iter().enumerate() {
                    eprintln!("\nProcessing input {}/{}: {}", idx + 1, resolved_inputs.len(), resolved_input);
                    builder.build_with_progress(
                        Path::new(resolved_input),
                        Path::new(&resolved_output),
                        |step, total, msg| {
                            eprintln!("[{}/{}] {}", step, total, msg);
                        },
                    )?;
                }
            }

            eprintln!("\nIndex build complete!");
            Ok(())
        }
        Commands::Search {
            query,
            index_dir,
            top_k,
            mode,
            tag,
        } => {
            let resolved_index_dir = resolve_path(&index_dir);
            let search_mode = match mode.as_str() {
                "bm25" => SearchMode::Bm25,
                "semantic" => SearchMode::Semantic,
                "hybrid" => SearchMode::Hybrid,
                _ => {
                    eprintln!("Unknown mode '{}', using bm25", mode);
                    SearchMode::Bm25
                }
            };

            let searcher = Searcher::new(&resolved_index_dir)?;
            let config = SearchConfig::new()
                .with_mode(search_mode)
                .with_top_k(top_k)
                .with_tag_filter(tag);

            let results = searcher.search(&query, &config)?;

            if results.is_empty() {
                println!("No results found for '{}'", query);
            } else {
                println!("Found {} results for '{}':\n", results.len(), query);
                for (i, result) in results.iter().enumerate() {
                    println!("{}. [score: {:.4}] {}", i + 1, result.score, result.doc_id);
                    if let Some(doc) = searcher.docstore().get(&result.doc_id) {
                        println!("   Title: {}", doc.title());
                        println!("   Date: {}", doc.date().format("%Y-%m-%d"));
                        println!("   Tags: {:?}", doc.tags());
                        let snippet: String = doc.text.chars().take(100).collect();
                        println!("   {}", snippet);
                    }
                    println!();
                }
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        // Test that CLI can be parsed without errors
        let cli = Cli::try_parse_from(["digrag", "serve", "--index-dir", ".rag"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_build_command() {
        let cli = Cli::try_parse_from([
            "digrag",
            "build",
            "--input",
            "changelogmemo",
            "--output",
            ".rag",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_build_command_multiple_inputs() {
        let cli = Cli::try_parse_from([
            "digrag",
            "build",
            "--input",
            "changelogmemo",
            "--input",
            "archive/old_changelogmemo",
            "--output",
            ".rag",
        ]);
        assert!(cli.is_ok());
        if let Ok(parsed) = cli {
            if let Commands::Build { input, .. } = parsed.command {
                assert_eq!(input.len(), 2);
                assert_eq!(input[0], "changelogmemo");
                assert_eq!(input[1], "archive/old_changelogmemo");
            }
        }
    }

    #[test]
    fn test_cli_search_command() {
        let cli = Cli::try_parse_from(["digrag", "search", "test query", "--top-k", "5"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_query_memos_params_empty() {
        // Test that empty JSON object can be deserialized (fixes "missing field query" error)
        let params: QueryMemosParams = serde_json::from_str("{}").expect("Empty params should work");
        assert_eq!(params.query, "");
        assert_eq!(params.top_k, 10);
        assert_eq!(params.mode, "bm25");
        assert!(params.tag_filter.is_none());
    }

    #[test]
    fn test_query_memos_params_with_query() {
        let params: QueryMemosParams = serde_json::from_str(r#"{"query":"test"}"#).unwrap();
        assert_eq!(params.query, "test");
        assert_eq!(params.top_k, 10);
    }
}
