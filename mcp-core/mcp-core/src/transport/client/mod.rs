//! # MCP Client Transports
//!
//! This module provides different transport implementations for MCP clients.
//!
//! Available transports include:
//! - `ClientStdioTransport`: Communicates with an MCP server over standard I/O
//! - `ClientSseTransport`: Communicates with an MCP server over Server-Sent Events (SSE)
//!
//! Each transport implements the `Transport` trait and provides client-specific
//! functionality for connecting to MCP servers.

#[cfg(feature = "sse")]
mod sse;
mod stdio;

#[cfg(feature = "sse")]
pub use sse::{ClientSseTransport, ClientSseTransportBuilder};
pub use stdio::ClientStdioTransport;
