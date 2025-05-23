//! # MCP Tools Management
//!
//! This module provides the infrastructure for registering, managing, and invoking
//! MCP tools. Tools are the primary way for clients to interact with server capabilities.
//!
//! The module implements a registry for tools and handlers that process tool invocations.

use crate::types::{CallToolRequest, CallToolResponse, Tool};
use anyhow::Result;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

/// Registry and dispatcher for MCP tools.
///
/// The `Tools` struct manages a collection of tools and their associated handlers,
/// providing methods to register, list, and invoke tools.
pub struct Tools {
    tool_handlers: HashMap<String, ToolHandler>,
}

impl Tools {
    /// Creates a new tool registry with the given tool handlers.
    pub(crate) fn new(map: HashMap<String, ToolHandler>) -> Self {
        Self { tool_handlers: map }
    }

    /// Retrieves a tool definition by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the tool to retrieve
    ///
    /// # Returns
    ///
    /// An `Option` containing the tool if found, or `None` if not found.
    pub fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tool_handlers
            .get(name)
            .map(|tool_handler| tool_handler.tool.clone())
    }

    /// Invokes a tool with the given request.
    ///
    /// # Arguments
    ///
    /// * `req` - The request containing the tool name and arguments
    ///
    /// # Returns
    ///
    /// A `Result` containing the tool response if successful, or an error if
    /// the tool is not found or the invocation fails.
    pub async fn call_tool(&self, req: CallToolRequest) -> Result<CallToolResponse> {
        let handler = self
            .tool_handlers
            .get(&req.name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", req.name))?;

        Ok((handler.f)(req).await)
    }

    /// Lists all registered tools.
    ///
    /// # Returns
    ///
    /// A vector containing all registered tools.
    pub fn list_tools(&self) -> Vec<Tool> {
        self.tool_handlers
            .values()
            .map(|tool_handler| tool_handler.tool.clone())
            .collect()
    }
}

/// Type alias for a tool handler function.
///
/// A tool handler is a function that takes a `CallToolRequest` and returns a
/// future that resolves to a `CallToolResponse`.
pub type ToolHandlerFn =
    fn(CallToolRequest) -> Pin<Box<dyn Future<Output = CallToolResponse> + Send>>;

/// Container for a tool definition and its handler function.
///
/// The `ToolHandler` struct couples a tool definition with the function
/// that implements the tool's behavior.
pub(crate) struct ToolHandler {
    /// The tool definition (name, description, parameters, etc.)
    pub tool: Tool,
    /// The handler function that implements the tool
    pub f: Box<ToolHandlerFn>,
}
