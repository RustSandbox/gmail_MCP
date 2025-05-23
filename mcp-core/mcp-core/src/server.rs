//! # MCP Server
//!
//! This module provides the server-side implementation of the Model Context Protocol (MCP).
//! It allows creating MCP servers that expose tools for clients to discover and invoke.
//!
//! The core components include:
//! - The `Server` for managing server lifetime
//! - The `ServerProtocolBuilder` for configuring servers
//! - Client connection tracking
//!
//! Servers expose tools that can be discovered and called by clients, with
//! customizable capabilities and metadata.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::{
    protocol::Protocol,
    tools::{ToolHandler, ToolHandlerFn, Tools},
    types::{
        CallToolRequest, ListRequest, ProtocolVersion, Tool, ToolsListResponse,
        LATEST_PROTOCOL_VERSION,
    },
};

use super::{
    protocol::ProtocolBuilder,
    transport::Transport,
    types::{
        ClientCapabilities, Implementation, InitializeRequest, InitializeResponse,
        ServerCapabilities,
    },
};
use anyhow::Result;
use std::pin::Pin;

/// Represents a connected MCP client.
///
/// Tracks information about a client that has connected to the server,
/// including its capabilities, info, and initialization state.
#[derive(Clone)]
pub struct ClientConnection {
    /// The capabilities reported by the client
    pub client_capabilities: Option<ClientCapabilities>,
    /// Information about the client implementation
    pub client_info: Option<Implementation>,
    /// Whether the client has completed initialization
    pub initialized: bool,
}

/// The main MCP server type.
///
/// Provides static methods for creating and starting MCP servers.
#[derive(Clone)]
pub struct Server;

impl Server {
    /// Creates a new server builder with the specified server information.
    ///
    /// # Arguments
    ///
    /// * `name` - The server name
    /// * `version` - The server version
    /// * `protocol_version` - The protocol version to use
    ///
    /// # Returns
    ///
    /// A `ServerProtocolBuilder` for configuring the server
    pub fn builder(
        name: String,
        version: String,
        protocol_version: ProtocolVersion,
    ) -> ServerProtocolBuilder {
        ServerProtocolBuilder::new(name, version).set_protocol_version(protocol_version)
    }

    /// Starts the server with the given transport.
    ///
    /// # Arguments
    ///
    /// * `transport` - The transport to use for communication with clients
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure
    pub async fn start<T: Transport>(transport: T) -> Result<()> {
        transport.open().await
    }
}

/// Builder for creating configured server protocols.
///
/// The `ServerProtocolBuilder` provides a fluent API for configuring and creating
/// MCP server protocols with specific settings, tools, and capabilities.
pub struct ServerProtocolBuilder {
    protocol_version: ProtocolVersion,
    protocol_builder: ProtocolBuilder,
    server_info: Implementation,
    capabilities: ServerCapabilities,
    instructions: Option<String>,
    tools: HashMap<String, ToolHandler>,
    client_connection: Arc<RwLock<ClientConnection>>,
}

impl ServerProtocolBuilder {
    /// Creates a new server protocol builder.
    ///
    /// # Arguments
    ///
    /// * `name` - The server name
    /// * `version` - The server version
    ///
    /// # Returns
    ///
    /// A new `ServerProtocolBuilder` instance
    pub fn new(name: String, version: String) -> Self {
        ServerProtocolBuilder {
            protocol_version: LATEST_PROTOCOL_VERSION,
            protocol_builder: ProtocolBuilder::new(),
            server_info: Implementation { name, version },
            capabilities: ServerCapabilities::default(),
            instructions: None,
            tools: HashMap::new(),
            client_connection: Arc::new(RwLock::new(ClientConnection {
                client_capabilities: None,
                client_info: None,
                initialized: false,
            })),
        }
    }

    /// Sets the protocol version for the server.
    ///
    /// # Arguments
    ///
    /// * `protocol_version` - The protocol version to use
    ///
    /// # Returns
    ///
    /// The modified builder instance
    pub fn set_protocol_version(mut self, protocol_version: ProtocolVersion) -> Self {
        self.protocol_version = protocol_version;
        self
    }

    /// Sets the server capabilities.
    ///
    /// # Arguments
    ///
    /// * `capabilities` - The server capabilities
    ///
    /// # Returns
    ///
    /// The modified builder instance
    pub fn set_capabilities(mut self, capabilities: ServerCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Sets the server instructions.
    ///
    /// Instructions provide guidance for AI models on how to use the server's tools.
    ///
    /// # Arguments
    ///
    /// * `instructions` - The instructions for using the server
    ///
    /// # Returns
    ///
    /// The modified builder instance
    pub fn set_instructions(mut self, instructions: String) -> Self {
        self.instructions = Some(instructions);
        self
    }

    /// Removes the server instructions.
    ///
    /// # Returns
    ///
    /// The modified builder instance
    pub fn remove_instructions(mut self) -> Self {
        self.instructions = None;
        self
    }

    /// Registers a tool with the server.
    ///
    /// # Arguments
    ///
    /// * `tool` - The tool definition
    /// * `f` - The handler function for the tool
    ///
    /// # Returns
    ///
    /// The modified builder instance
    pub fn register_tool(mut self, tool: Tool, f: ToolHandlerFn) -> Self {
        self.tools.insert(
            tool.name.clone(),
            ToolHandler {
                tool,
                f: Box::new(f),
            },
        );
        self
    }

    /// Helper function for creating an initialize request handler.
    ///
    /// # Arguments
    ///
    /// * `protocol_version` - The protocol version to use
    /// * `state` - The client connection state
    /// * `server_info` - The server information
    /// * `capabilities` - The server capabilities
    /// * `instructions` - Optional server instructions
    ///
    /// # Returns
    ///
    /// A handler function for initialize requests
    fn handle_init(
        protocol_version: ProtocolVersion,
        state: Arc<RwLock<ClientConnection>>,
        server_info: Implementation,
        capabilities: ServerCapabilities,
        instructions: Option<String>,
    ) -> impl Fn(
        InitializeRequest,
    )
        -> Pin<Box<dyn std::future::Future<Output = Result<InitializeResponse>> + Send>> {
        move |req| {
            let state = state.clone();
            let server_info = server_info.clone();
            let capabilities = capabilities.clone();
            let instructions = instructions.clone();
            let protocol_version = protocol_version.clone();

            Box::pin(async move {
                let mut state = state
                    .write()
                    .map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
                state.client_capabilities = Some(req.capabilities);
                state.client_info = Some(req.client_info);

                Ok(InitializeResponse {
                    protocol_version: protocol_version.as_str().to_string(),
                    capabilities,
                    server_info,
                    instructions,
                })
            })
        }
    }

    /// Helper function for creating an initialized notification handler.
    ///
    /// # Arguments
    ///
    /// * `state` - The client connection state
    ///
    /// # Returns
    ///
    /// A handler function for initialized notifications
    fn handle_initialized(
        state: Arc<RwLock<ClientConnection>>,
    ) -> impl Fn(()) -> Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>> {
        move |_| {
            let state = state.clone();
            Box::pin(async move {
                let mut state = state
                    .write()
                    .map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
                state.initialized = true;
                Ok(())
            })
        }
    }

    /// Gets the client capabilities, if available.
    ///
    /// # Returns
    ///
    /// An `Option` containing the client capabilities if available
    pub fn get_client_capabilities(&self) -> Option<ClientCapabilities> {
        self.client_connection
            .read()
            .ok()?
            .client_capabilities
            .clone()
    }

    /// Gets the client information, if available.
    ///
    /// # Returns
    ///
    /// An `Option` containing the client information if available
    pub fn get_client_info(&self) -> Option<Implementation> {
        self.client_connection.read().ok()?.client_info.clone()
    }

    /// Checks if the client has completed initialization.
    ///
    /// # Returns
    ///
    /// `true` if the client is initialized, `false` otherwise
    pub fn is_initialized(&self) -> bool {
        self.client_connection
            .read()
            .ok()
            .map(|client_connection| client_connection.initialized)
            .unwrap_or(false)
    }

    /// Builds the server protocol.
    ///
    /// # Returns
    ///
    /// A `Protocol` instance configured with the server's settings
    pub fn build(self) -> Protocol {
        let tools = Arc::new(Tools::new(self.tools));
        let tools_clone = tools.clone();
        let tools_list = tools.clone();
        let tools_call = tools_clone.clone();

        let conn_for_list = self.client_connection.clone();
        let conn_for_call = self.client_connection.clone();

        self.protocol_builder
            .request_handler(
                "initialize",
                Self::handle_init(
                    self.protocol_version.clone(),
                    self.client_connection.clone(),
                    self.server_info,
                    self.capabilities,
                    self.instructions,
                ),
            )
            .notification_handler(
                "notifications/initialized",
                Self::handle_initialized(self.client_connection),
            )
            .request_handler("tools/list", move |_req: ListRequest| {
                let tools_list = tools_list.clone();
                let conn = conn_for_list.clone();
                Box::pin(async move {
                    match conn.read() {
                        Ok(conn) => {
                            if !conn.initialized {
                                return Err(anyhow::anyhow!("Client not initialized"));
                            }
                        }
                        Err(_) => return Err(anyhow::anyhow!("Lock poisoned")),
                    }

                    let tools = tools_list.list_tools();

                    Ok(ToolsListResponse {
                        tools,
                        next_cursor: None,
                        meta: None,
                    })
                })
            })
            .request_handler("tools/call", move |req: CallToolRequest| {
                let tools_call = tools_call.clone();
                let conn = conn_for_call.clone();
                Box::pin(async move {
                    match conn.read() {
                        Ok(conn) => {
                            if !conn.initialized {
                                return Err(anyhow::anyhow!("Client not initialized"));
                            }
                        }
                        Err(_) => return Err(anyhow::anyhow!("Lock poisoned")),
                    }

                    match tools_call.call_tool(req).await {
                        Ok(resp) => Ok(resp),
                        Err(e) => Err(e),
                    }
                })
            })
            .build()
    }
}
