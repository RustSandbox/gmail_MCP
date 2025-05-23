//! # MCP Client
//!
//! This module provides the client-side implementation of the Model Context Protocol (MCP).
//! The client can connect to MCP servers, initialize the connection, and invoke tools
//! provided by the server.
//!
//! The core functionality includes:
//! - Establishing connections to MCP servers
//! - Managing the protocol handshake
//! - Discovering available tools
//! - Invoking tools with parameters
//! - Handling server resources

use std::{collections::HashMap, env, sync::Arc};

use crate::{
    protocol::RequestOptions,
    transport::Transport,
    types::{
        CallToolRequest, CallToolResponse, ClientCapabilities, Implementation, InitializeRequest,
        InitializeResponse, ListRequest, ProtocolVersion, ReadResourceRequest, Resource,
        ResourcesListResponse, ToolsListResponse, LATEST_PROTOCOL_VERSION,
    },
};

use anyhow::Result;
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::debug;

/// An MCP client for connecting to MCP servers and invoking their tools.
///
/// The `Client` provides a high-level API for interacting with MCP servers,
/// including initialization, tool discovery, and tool invocation.
#[derive(Clone)]
pub struct Client<T: Transport> {
    transport: T,
    strict: bool,
    protocol_version: ProtocolVersion,
    initialize_res: Arc<RwLock<Option<InitializeResponse>>>,
    env: Option<HashMap<String, SecureValue>>,
    client_info: Implementation,
    capabilities: ClientCapabilities,
}

impl<T: Transport> Client<T> {
    /// Creates a new client builder.
    ///
    /// # Arguments
    ///
    /// * `transport` - The transport to use for communication with the server
    ///
    /// # Returns
    ///
    /// A `ClientBuilder` for configuring and building the client
    pub fn builder(transport: T) -> ClientBuilder<T> {
        ClientBuilder::new(transport)
    }

    /// Sets the protocol version for the client.
    ///
    /// # Arguments
    ///
    /// * `protocol_version` - The protocol version to use
    ///
    /// # Returns
    ///
    /// The modified client instance
    pub fn set_protocol_version(mut self, protocol_version: ProtocolVersion) -> Self {
        self.protocol_version = protocol_version;
        self
    }

    /// Opens the transport connection.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure
    pub async fn open(&self) -> Result<()> {
        self.transport.open().await
    }

    /// Initializes the connection with the MCP server.
    ///
    /// This sends the initialize request to the server, negotiates protocol
    /// version and capabilities, and establishes the session.
    ///
    /// # Returns
    ///
    /// A `Result` containing the server's initialization response if successful
    pub async fn initialize(&self) -> Result<InitializeResponse> {
        let request = InitializeRequest {
            protocol_version: self.protocol_version.as_str().to_string(),
            capabilities: self.capabilities.clone(),
            client_info: self.client_info.clone(),
        };
        let response = self
            .request(
                "initialize",
                Some(serde_json::to_value(request)?),
                RequestOptions::default(),
            )
            .await?;
        let response: InitializeResponse = serde_json::from_value(response)
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;

        if response.protocol_version != self.protocol_version.as_str() {
            return Err(anyhow::anyhow!(
                "Unsupported protocol version: {}",
                response.protocol_version
            ));
        }

        // Save the response for later use
        let mut writer = self.initialize_res.write().await;
        *writer = Some(response.clone());

        debug!(
            "Initialized with protocol version: {}",
            response.protocol_version
        );
        self.transport
            .send_notification("notifications/initialized", None)
            .await?;

        Ok(response)
    }

    /// Checks if the client has been initialized.
    ///
    /// # Returns
    ///
    /// A `Result` indicating if the client is initialized
    pub async fn assert_initialized(&self) -> Result<(), anyhow::Error> {
        let reader = self.initialize_res.read().await;
        match &*reader {
            Some(_) => Ok(()),
            None => Err(anyhow::anyhow!("Not initialized")),
        }
    }

    /// Sends a request to the server.
    ///
    /// # Arguments
    ///
    /// * `method` - The method name
    /// * `params` - Optional parameters for the request
    /// * `options` - Request options (like timeout)
    ///
    /// # Returns
    ///
    /// A `Result` containing the server's response if successful
    pub async fn request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
        options: RequestOptions,
    ) -> Result<serde_json::Value> {
        let response = self.transport.request(method, params, options).await?;
        response
            .result
            .ok_or_else(|| anyhow::anyhow!("Request failed: {:?}", response.error))
    }

    /// Lists tools available on the server.
    ///
    /// # Arguments
    ///
    /// * `cursor` - Optional pagination cursor
    /// * `request_options` - Optional request options
    ///
    /// # Returns
    ///
    /// A `Result` containing the list of tools if successful
    pub async fn list_tools(
        &self,
        cursor: Option<String>,
        request_options: Option<RequestOptions>,
    ) -> Result<ToolsListResponse> {
        if self.strict {
            self.assert_initialized().await?;
        }

        let list_request = ListRequest { cursor, meta: None };

        let response = self
            .request(
                "tools/list",
                Some(serde_json::to_value(list_request)?),
                request_options.unwrap_or_else(RequestOptions::default),
            )
            .await?;

        Ok(serde_json::from_value(response)
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?)
    }

    /// Calls a tool on the server.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the tool to call
    /// * `arguments` - Optional arguments for the tool
    ///
    /// # Returns
    ///
    /// A `Result` containing the tool's response if successful
    pub async fn call_tool(
        &self,
        name: &str,
        arguements: Option<serde_json::Value>,
    ) -> Result<CallToolResponse> {
        if self.strict {
            self.assert_initialized().await?;
        }

        let arguments = if let Some(env) = &self.env {
            arguements
                .as_ref()
                .map(|args| apply_secure_replacements(args, env))
        } else {
            arguements
        };

        let arguments =
            arguments.map(|value| serde_json::from_value(value).unwrap_or_else(|_| HashMap::new()));

        let request = CallToolRequest {
            name: name.to_string(),
            arguments,
            meta: None,
        };

        let response = self
            .request(
                "tools/call",
                Some(serde_json::to_value(request)?),
                RequestOptions::default(),
            )
            .await?;

        Ok(serde_json::from_value(response)
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?)
    }

    /// Lists resources available on the server.
    ///
    /// # Arguments
    ///
    /// * `cursor` - Optional pagination cursor
    /// * `request_options` - Optional request options
    ///
    /// # Returns
    ///
    /// A `Result` containing the list of resources if successful
    pub async fn list_resources(
        &self,
        cursor: Option<String>,
        request_options: Option<RequestOptions>,
    ) -> Result<ResourcesListResponse> {
        if self.strict {
            self.assert_initialized().await?;
        }

        let list_request = ListRequest { cursor, meta: None };

        let response = self
            .request(
                "resources/list",
                Some(serde_json::to_value(list_request)?),
                request_options.unwrap_or_else(RequestOptions::default),
            )
            .await?;

        Ok(serde_json::from_value(response)
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?)
    }

    /// Reads a resource from the server.
    ///
    /// # Arguments
    ///
    /// * `uri` - The URI of the resource to read
    ///
    /// # Returns
    ///
    /// A `Result` containing the resource if successful
    pub async fn read_resource(&self, uri: url::Url) -> Result<Resource> {
        if self.strict {
            self.assert_initialized().await?;
        }

        let read_request = ReadResourceRequest { uri };

        let response = self
            .request(
                "resources/read",
                Some(serde_json::to_value(read_request)?),
                RequestOptions::default(),
            )
            .await?;

        Ok(serde_json::from_value(response)
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?)
    }

    pub async fn subscribe_to_resource(&self, uri: url::Url) -> Result<()> {
        if self.strict {
            self.assert_initialized().await?;
        }

        let subscribe_request = ReadResourceRequest { uri };

        self.request(
            "resources/subscribe",
            Some(serde_json::to_value(subscribe_request)?),
            RequestOptions::default(),
        )
        .await?;

        Ok(())
    }

    pub async fn unsubscribe_to_resource(&self, uri: url::Url) -> Result<()> {
        if self.strict {
            self.assert_initialized().await?;
        }

        let unsubscribe_request = ReadResourceRequest { uri };

        self.request(
            "resources/unsubscribe",
            Some(serde_json::to_value(unsubscribe_request)?),
            RequestOptions::default(),
        )
        .await?;

        Ok(())
    }
}

/// Represents a value that may contain sensitive information.
///
/// Secure values can be either static strings or environment variables.
#[derive(Clone, Debug)]
pub enum SecureValue {
    /// A static string value
    Static(String),
    /// An environment variable reference
    Env(String),
}

/// Builder for creating configured `Client` instances.
///
/// The `ClientBuilder` provides a fluent API for configuring and creating
/// MCP clients with specific settings.
pub struct ClientBuilder<T: Transport> {
    transport: T,
    strict: bool,
    env: Option<HashMap<String, SecureValue>>,
    protocol_version: ProtocolVersion,
    client_info: Implementation,
    capabilities: ClientCapabilities,
}

impl<T: Transport> ClientBuilder<T> {
    /// Creates a new client builder.
    ///
    /// # Arguments
    ///
    /// * `transport` - The transport to use for communication with the server
    ///
    /// # Returns
    ///
    /// A new `ClientBuilder` instance
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            strict: false,
            env: None,
            protocol_version: LATEST_PROTOCOL_VERSION,
            client_info: Implementation {
                name: env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "mcp-client".to_string()),
                version: env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string()),
            },
            capabilities: ClientCapabilities::default(),
        }
    }

    /// Sets the protocol version for the client.
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

    /// Sets the client information.
    ///
    /// # Arguments
    ///
    /// * `name` - The client name
    /// * `version` - The client version
    ///
    /// # Returns
    ///
    /// The modified builder instance
    pub fn set_client_info(mut self, name: impl Into<String>, version: impl Into<String>) -> Self {
        self.client_info = Implementation {
            name: name.into(),
            version: version.into(),
        };
        self
    }

    /// Sets the client capabilities.
    ///
    /// # Arguments
    ///
    /// * `capabilities` - The client capabilities
    ///
    /// # Returns
    ///
    /// The modified builder instance
    pub fn set_capabilities(mut self, capabilities: ClientCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Adds a secure value for substitution in tool arguments.
    ///
    /// # Arguments
    ///
    /// * `key` - The key for the secure value
    /// * `value` - The secure value
    ///
    /// # Returns
    ///
    /// The modified builder instance
    pub fn with_secure_value(mut self, key: impl Into<String>, value: SecureValue) -> Self {
        if self.env.is_none() {
            self.env = Some(HashMap::new());
        }

        if let Some(env) = &mut self.env {
            env.insert(key.into(), value);
        }

        self
    }

    /// Enables strict mode, which requires initialization before operations.
    ///
    /// # Returns
    ///
    /// The modified builder instance
    pub fn use_strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Sets the strict mode flag.
    ///
    /// # Arguments
    ///
    /// * `strict` - Whether to enable strict mode
    ///
    /// # Returns
    ///
    /// The modified builder instance
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// Builds the client with the configured settings.
    ///
    /// # Returns
    ///
    /// A new `Client` instance
    pub fn build(self) -> Client<T> {
        Client {
            transport: self.transport,
            strict: self.strict,
            env: self.env,
            protocol_version: self.protocol_version,
            initialize_res: Arc::new(RwLock::new(None)),
            client_info: self.client_info,
            capabilities: self.capabilities,
        }
    }
}

/// Recursively walk through the JSON value. If a JSON string exactly matches
/// one of the keys in the secure values map, replace it with the corresponding secure value.
pub fn apply_secure_replacements(
    value: &Value,
    secure_values: &HashMap<String, SecureValue>,
) -> Value {
    match value {
        Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (k, v) in map.iter() {
                let new_value = if let Value::String(_) = v {
                    if let Some(secure_val) = secure_values.get(k) {
                        let replacement = match secure_val {
                            SecureValue::Static(val) => val.clone(),
                            SecureValue::Env(env_key) => env::var(env_key)
                                .unwrap_or_else(|_| v.as_str().unwrap().to_string()),
                        };
                        Value::String(replacement)
                    } else {
                        apply_secure_replacements(v, secure_values)
                    }
                } else {
                    apply_secure_replacements(v, secure_values)
                };
                new_map.insert(k.clone(), new_value);
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => {
            let new_arr: Vec<Value> = arr
                .iter()
                .map(|v| apply_secure_replacements(v, secure_values))
                .collect();
            Value::Array(new_arr)
        }
        _ => value.clone(),
    }
}
