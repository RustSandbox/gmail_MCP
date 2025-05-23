//! # MCP Server Transports
//!
//! This module provides different transport implementations for MCP servers.
//!
//! Available transports include:
//! - `ServerStdioTransport`: Communicates with MCP clients over standard I/O
//! - `ServerSseTransport`: Communicates with MCP clients over Server-Sent Events (SSE)
//!
//! Each transport implements the `Transport` trait and provides server-specific
//! functionality for accepting connections from MCP clients and handling
//! communication.

mod stdio;
pub use stdio::ServerStdioTransport;

#[cfg(feature = "sse")]
mod sse;
#[cfg(feature = "sse")]
pub use sse::ServerSseTransport;
