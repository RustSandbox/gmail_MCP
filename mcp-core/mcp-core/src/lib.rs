//! # Model Context Protocol (MCP) Core Library
//!
//! `mcp-core` is a Rust implementation of the Model Context Protocol (MCP), an open
//! protocol for interaction between AI models and tools/external systems.
//!
//! This library provides a comprehensive framework for building both MCP servers (tool providers)
//! and MCP clients (model interfaces), with support for:
//!
//! - Bidirectional communication between AI models and external tools
//! - Tool registration, discovery, and invocation
//! - Resource management
//! - Transport-agnostic design (supporting both SSE and stdio)
//! - Standardized request/response formats using JSON-RPC
//!
//! ## Architecture
//!
//! The library is organized into several main components:
//!
//! - **Client**: Implementation of the MCP client for connecting to servers
//! - **Server**: Implementation of the MCP server for exposing tools to clients
//! - **Protocol**: Core protocol implementation using JSON-RPC
//! - **Types**: Data structures representing MCP concepts
//! - **Transport**: Network transport abstraction (SSE, stdio)
//! - **Tools**: Framework for registering and invoking tools
//!
//! ## Usage
//!
//! For examples of how to use this library, see the `examples/` directory:
//! - `echo_server.rs`: A simple MCP server implementation
//! - `echo_server_macro.rs`: Using the `#[tool]` macro for simpler integration
//! - `echo_client.rs`: A client connecting to an MCP server
//!
//! ## Macros
//!
//! This library includes a set of utility macros to make working with the MCP protocol
//! easier, including helpers for creating various types of tool responses.

pub mod client;
pub mod protocol;
pub mod server;
pub mod tools;
pub mod transport;
pub mod types;

/// Creates a tool response with error information.
///
/// This macro generates a `CallToolResponse` containing a text error message
/// and sets the `is_error` flag to `true`.
///
/// # Examples
///
/// ```
/// use mcp_core::tool_error_response;
/// use anyhow::Error;
///
/// let error = Error::msg("Something went wrong");
/// let response = tool_error_response!(error);
/// assert_eq!(response.is_error, Some(true));
/// ```
#[macro_export]
macro_rules! tool_error_response {
    ($e:expr) => {{
        let error_message = $e.to_string();
        $crate::types::CallToolResponse {
            content: vec![$crate::types::ToolResponseContent::Text(
                $crate::types::TextContent {
                    content_type: "text".to_string(),
                    text: error_message,
                    annotations: None,
                },
            )],
            is_error: Some(true),
            meta: None,
        }
    }};
}

/// Creates a tool response with text content.
///
/// This macro generates a `CallToolResponse` containing the provided text.
///
/// # Examples
///
/// ```
/// use mcp_core::tool_text_response;
///
/// let response = tool_text_response!("Hello, world!");
/// ```
#[macro_export]
macro_rules! tool_text_response {
    ($e:expr) => {{
        $crate::types::CallToolResponse {
            content: vec![$crate::types::ToolResponseContent::Text(
                $crate::types::TextContent {
                    content_type: "text".to_string(),
                    text: $e.to_string(),
                    annotations: None,
                },
            )],
            is_error: None,
            meta: None,
        }
    }};
}

/// Creates a text content object for tool responses.
///
/// This macro generates a `ToolResponseContent::Text` object with the provided text.
///
/// # Examples
///
/// ```
/// use mcp_core::tool_text_content;
///
/// let content = tool_text_content!("Hello, world!");
/// ```
#[macro_export]
macro_rules! tool_text_content {
    ($e:expr) => {{
        $crate::types::ToolResponseContent::Text($crate::types::TextContent {
            content_type: "text".to_string(),
            text: $e.to_string(),
            annotations: None,
        })
    }};
}

/// Creates an image content object for tool responses.
///
/// This macro generates a `ToolResponseContent::Image` object with the provided data and MIME type.
///
/// # Examples
///
/// ```
/// use mcp_core::tool_image_content;
///
/// let image_data = "base64_encoded_data".to_string();
/// let content = tool_image_content!(image_data, "image/jpeg".to_string());
/// ```
#[macro_export]
macro_rules! tool_image_content {
    ($data:expr, $mime_type:expr) => {{
        $crate::types::ToolResponseContent::Image($crate::types::ImageContent {
            content_type: "image".to_string(),
            data: $data,
            mime_type: $mime_type,
            annotations: None,
        })
    }};
}

/// Creates an audio content object for tool responses.
///
/// This macro generates a `ToolResponseContent::Audio` object with the provided data and MIME type.
///
/// # Examples
///
/// ```
/// use mcp_core::tool_audio_content;
///
/// let audio_data = "base64_encoded_audio".to_string();
/// let content = tool_audio_content!(audio_data, "audio/mp3".to_string());
/// ```
#[macro_export]
macro_rules! tool_audio_content {
    ($data:expr, $mime_type:expr) => {{
        $crate::types::ToolResponseContent::Audio($crate::types::AudioContent {
            content_type: "audio".to_string(),
            data: $data,
            mime_type: $mime_type,
            annotations: None,
        })
    }};
}

/// Creates a resource content object for tool responses.
///
/// This macro generates a `ToolResponseContent::Resource` object with the provided URI and optional MIME type.
///
/// # Examples
///
/// ```
/// use mcp_core::tool_resource_content;
/// use url::Url;
///
/// let uri = Url::parse("https://example.com/resource.png").unwrap();
/// let content = tool_resource_content!(uri, "image/png".to_string());
/// ```
#[macro_export]
macro_rules! tool_resource_content {
    ($uri:expr, $mime_type:expr) => {{
        $crate::types::ToolResponseContent::Resource($crate::types::EmbeddedResource {
            content_type: "resource".to_string(),
            resource: $crate::types::ResourceContents {
                uri: $uri,
                mime_type: Some($mime_type),
                text: None,
                blob: None,
            },
            annotations: None,
        })
    }};
    ($uri:expr) => {{
        $crate::types::ToolResponseContent::Resource($crate::types::EmbeddedResource {
            content_type: "resource".to_string(),
            resource: $crate::types::ResourceContents {
                uri: $uri,
                mime_type: None,
                text: None,
                blob: None,
            },
            annotations: None,
        })
    }};
}
