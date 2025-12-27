//! MCP server module
//!
//! This module provides types for the MCP server.
//! The actual MCP server implementation using rmcp is in main.rs.

mod server;

pub use server::{
    GetRecentMemosRequest, GetRecentMemosResponse, ListTagsResponse, MemoResult,
    QueryMemosRequest, QueryMemosResponse, TagInfo,
};
