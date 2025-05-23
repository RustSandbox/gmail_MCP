//! # MCP Transport Layer
//!
//! This module implements the transport layer for the Model Context Protocol (MCP).
//! It provides abstractions for sending and receiving JSON-RPC messages between
//! clients and servers using different transport mechanisms.
//!
//! The transport layer:
//! - Handles serialization and deserialization of messages
//! - Provides interfaces for sending and receiving messages
//! - Defines transport-specific implementations (SSE, stdio)
//! - Abstracts the underlying communication protocol
//!
//! The core component is the `Transport` trait, which defines the operations that
//! any MCP transport must support, regardless of the underlying mechanism.

use std::{future::Future, pin::Pin};

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

mod client;
pub use client::*;

mod server;
pub use server::*;

use crate::protocol::RequestOptions;

/// A message in the MCP protocol.
///
/// Currently, only JSON-RPC messages are supported, as defined in the
/// [MCP specification](https://spec.modelcontextprotocol.io/specification/basic/messages/).
pub type Message = JsonRpcMessage;

/// Core trait that defines operations for MCP transports.
///
/// This trait abstracts the transport layer, allowing the protocol to work
/// with different communication mechanisms (SSE, stdio, etc.).
#[async_trait()]
pub trait Transport: Send + Sync + 'static {
    /// Opens the transport connection.
    ///
    /// This initializes the transport and prepares it for communication.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure
    async fn open(&self) -> Result<()>;

    /// Closes the transport connection.
    ///
    /// This terminates the transport and releases any resources.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure
    async fn close(&self) -> Result<()>;

    /// Polls for incoming messages.
    ///
    /// This checks for any new messages from the other endpoint.
    ///
    /// # Returns
    ///
    /// A `Result` containing an `Option<Message>` if a message is available
    async fn poll_message(&self) -> Result<Option<Message>>;

    /// Sends a request and waits for the response.
    ///
    /// # Arguments
    ///
    /// * `method` - The method name for the request
    /// * `params` - Optional parameters for the request
    /// * `options` - Request options (like timeout)
    ///
    /// # Returns
    ///
    /// A `Future` that resolves to a `Result` containing the response
    fn request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
        options: RequestOptions,
    ) -> Pin<Box<dyn Future<Output = Result<JsonRpcResponse>> + Send + Sync>>;

    /// Sends a notification.
    ///
    /// Unlike requests, notifications do not expect a response.
    ///
    /// # Arguments
    ///
    /// * `method` - The method name for the notification
    /// * `params` - Optional parameters for the notification
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure
    async fn send_notification(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<()>;

    /// Sends a response to a request.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the request being responded to
    /// * `result` - Optional successful result
    /// * `error` - Optional error information
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure
    async fn send_response(
        &self,
        id: RequestId,
        result: Option<serde_json::Value>,
        error: Option<JsonRpcError>,
    ) -> Result<()>;
}

/// Type representing a JSON-RPC request ID.
///
/// Request IDs are used to match responses to their corresponding requests.
pub type RequestId = u64;

/// Represents a JSON-RPC protocol version.
///
/// The JSON-RPC version is included in all JSON-RPC messages and
/// is typically "2.0" for the current version of the protocol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct JsonRpcVersion(String);

impl Default for JsonRpcVersion {
    /// Creates a default JSON-RPC version (2.0).
    ///
    /// # Returns
    ///
    /// A new `JsonRpcVersion` with value "2.0"
    fn default() -> Self {
        JsonRpcVersion("2.0".to_owned())
    }
}

impl JsonRpcVersion {
    /// Returns the version as a string slice.
    ///
    /// # Returns
    ///
    /// A string slice containing the version
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Represents a JSON-RPC message.
///
/// This enum can be a request, a response, or a notification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    /// A response to a request
    Response(JsonRpcResponse),
    /// A request that expects a response
    Request(JsonRpcRequest),
    /// A notification that does not expect a response
    Notification(JsonRpcNotification),
}

/// Represents a JSON-RPC request.
///
/// A request is a message that expects a response with the same ID.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JsonRpcRequest {
    /// The request ID, used to match with the response
    pub id: RequestId,
    /// The method name to call
    pub method: String,
    /// Optional parameters for the method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    /// The JSON-RPC version
    pub jsonrpc: JsonRpcVersion,
}

/// Represents a JSON-RPC notification.
///
/// A notification is a message that does not expect a response.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct JsonRpcNotification {
    /// The method name for the notification
    pub method: String,
    /// Optional parameters for the notification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    /// The JSON-RPC version
    pub jsonrpc: JsonRpcVersion,
}

/// Represents a JSON-RPC response.
///
/// A response is a message sent in reply to a request with the same ID.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct JsonRpcResponse {
    /// The request ID this response corresponds to
    pub id: RequestId,
    /// The result of the request, if successful
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// The error, if the request failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    /// The JSON-RPC version
    pub jsonrpc: JsonRpcVersion,
}

/// Represents a JSON-RPC error.
///
/// An error is included in a response when the request fails.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Optional additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}
