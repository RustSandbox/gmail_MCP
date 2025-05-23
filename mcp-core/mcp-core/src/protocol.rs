//! # MCP Protocol Implementation
//!
//! This module implements the core JSON-RPC protocol layer used by the MCP system.
//! It provides the infrastructure for sending and receiving JSON-RPC requests,
//! notifications, and responses between MCP clients and servers.
//!
//! The protocol layer is transport-agnostic and can work with any transport
//! implementation that conforms to the `Transport` trait.
//!
//! Key components include:
//! - `Protocol`: The main protocol handler
//! - `ProtocolBuilder`: A builder for configuring protocols
//! - Request and notification handlers
//! - Timeout and error handling

use super::transport::{JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use super::types::ErrorCode;
use anyhow::Result;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::json;
use std::pin::Pin;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{oneshot, Mutex};

/// The core protocol handler for MCP.
///
/// The `Protocol` struct manages the lifecycle of JSON-RPC requests and responses,
/// dispatches incoming requests to the appropriate handlers, and manages
/// pending requests and their responses.
#[derive(Clone)]
pub struct Protocol {
    request_id: Arc<AtomicU64>,
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
    request_handlers: Arc<Mutex<HashMap<String, Box<dyn RequestHandler>>>>,
    notification_handlers: Arc<Mutex<HashMap<String, Box<dyn NotificationHandler>>>>,
}

impl Protocol {
    /// Creates a new protocol builder.
    ///
    /// # Returns
    ///
    /// A `ProtocolBuilder` for configuring the protocol
    pub fn builder() -> ProtocolBuilder {
        ProtocolBuilder::new()
    }

    /// Handles an incoming JSON-RPC request.
    ///
    /// This method dispatches the request to the appropriate handler based on
    /// the request method, and returns the handler's response.
    ///
    /// # Arguments
    ///
    /// * `request` - The incoming JSON-RPC request
    ///
    /// # Returns
    ///
    /// A `JsonRpcResponse` containing the handler's response or an error
    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let handlers = self.request_handlers.lock().await;
        if let Some(handler) = handlers.get(&request.method) {
            match handler.handle(request.clone()).await {
                Ok(response) => response,
                Err(e) => JsonRpcResponse {
                    id: request.id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: ErrorCode::InternalError as i32,
                        message: e.to_string(),
                        data: None,
                    }),
                    ..Default::default()
                },
            }
        } else {
            JsonRpcResponse {
                id: request.id,
                error: Some(JsonRpcError {
                    code: ErrorCode::MethodNotFound as i32,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
                ..Default::default()
            }
        }
    }

    /// Handles an incoming JSON-RPC notification.
    ///
    /// This method dispatches the notification to the appropriate handler based on
    /// the notification method.
    ///
    /// # Arguments
    ///
    /// * `request` - The incoming JSON-RPC notification
    pub async fn handle_notification(&self, request: JsonRpcNotification) {
        let handlers = self.notification_handlers.lock().await;
        if let Some(handler) = handlers.get(&request.method) {
            match handler.handle(request.clone()).await {
                Ok(_) => tracing::info!("Received notification: {:?}", request.method),
                Err(e) => tracing::error!("Error handling notification: {}", e),
            }
        } else {
            tracing::debug!("No handler for notification: {}", request.method);
        }
    }

    /// Generates a new unique message ID for requests.
    ///
    /// # Returns
    ///
    /// A unique message ID
    pub fn new_message_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Creates a new request ID and channel for receiving the response.
    ///
    /// # Returns
    ///
    /// A tuple containing the request ID and a receiver for the response
    pub async fn create_request(&self) -> (u64, oneshot::Receiver<JsonRpcResponse>) {
        let id = self.new_message_id();
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(id, tx);
        }

        (id, rx)
    }

    /// Handles an incoming JSON-RPC response.
    ///
    /// This method delivers the response to the appropriate waiting request,
    /// if any.
    ///
    /// # Arguments
    ///
    /// * `response` - The incoming JSON-RPC response
    pub async fn handle_response(&self, response: JsonRpcResponse) {
        if let Some(tx) = self.pending_requests.lock().await.remove(&response.id) {
            let _ = tx.send(response);
        }
    }

    /// Cancels a pending request and sends an error response.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the request to cancel
    pub async fn cancel_response(&self, id: u64) {
        if let Some(tx) = self.pending_requests.lock().await.remove(&id) {
            let _ = tx.send(JsonRpcResponse {
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: ErrorCode::RequestTimeout as i32,
                    message: "Request cancelled".to_string(),
                    data: None,
                }),
                ..Default::default()
            });
        }
    }
}

/// The default request timeout, in milliseconds
pub const DEFAULT_REQUEST_TIMEOUT_MSEC: u64 = 60000;

/// Options for customizing requests.
///
/// This struct allows configuring various aspects of request handling,
/// such as timeouts.
pub struct RequestOptions {
    /// The timeout duration for the request
    pub timeout: Duration,
}

impl RequestOptions {
    /// Sets the timeout for the request.
    ///
    /// # Arguments
    ///
    /// * `timeout` - The timeout duration
    ///
    /// # Returns
    ///
    /// The modified options instance
    pub fn timeout(self, timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl Default for RequestOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_millis(DEFAULT_REQUEST_TIMEOUT_MSEC),
        }
    }
}

/// Builder for creating configured protocols.
///
/// The `ProtocolBuilder` provides a fluent API for configuring and creating
/// protocols with specific request and notification handlers.
#[derive(Clone)]
pub struct ProtocolBuilder {
    request_handlers: Arc<Mutex<HashMap<String, Box<dyn RequestHandler>>>>,
    notification_handlers: Arc<Mutex<HashMap<String, Box<dyn NotificationHandler>>>>,
}

impl ProtocolBuilder {
    /// Creates a new protocol builder.
    ///
    /// # Returns
    ///
    /// A new `ProtocolBuilder` instance
    pub fn new() -> Self {
        Self {
            request_handlers: Arc::new(Mutex::new(HashMap::new())),
            notification_handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Registers a typed request handler.
    ///
    /// # Arguments
    ///
    /// * `method` - The method name to handle
    /// * `handler` - The handler function
    ///
    /// # Returns
    ///
    /// The modified builder instance
    pub fn request_handler<Req, Resp>(
        self,
        method: &str,
        handler: impl Fn(Req) -> Pin<Box<dyn std::future::Future<Output = Result<Resp>> + Send>>
            + Send
            + Sync
            + 'static,
    ) -> Self
    where
        Req: DeserializeOwned + Send + Sync + 'static,
        Resp: Serialize + Send + Sync + 'static,
    {
        let handler = TypedRequestHandler {
            handler: Box::new(handler),
            _phantom: std::marker::PhantomData,
        };

        if let Ok(mut handlers) = self.request_handlers.try_lock() {
            handlers.insert(method.to_string(), Box::new(handler));
        }
        self
    }

    /// Checks if a request handler exists for a method.
    ///
    /// # Arguments
    ///
    /// * `method` - The method name to check
    ///
    /// # Returns
    ///
    /// `true` if a handler exists, `false` otherwise
    pub fn has_request_handler(&self, method: &str) -> bool {
        self.request_handlers
            .try_lock()
            .map(|handlers| handlers.contains_key(method))
            .unwrap_or(false)
    }

    /// Registers a typed notification handler.
    ///
    /// # Arguments
    ///
    /// * `method` - The method name to handle
    /// * `handler` - The handler function
    ///
    /// # Returns
    ///
    /// The modified builder instance
    pub fn notification_handler<N>(
        self,
        method: &str,
        handler: impl Fn(N) -> Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
            + Send
            + Sync
            + 'static,
    ) -> Self
    where
        N: DeserializeOwned + Send + Sync + 'static,
    {
        let handler = TypedNotificationHandler {
            handler: Box::new(handler),
            _phantom: std::marker::PhantomData,
        };

        if let Ok(mut handlers) = self.notification_handlers.try_lock() {
            handlers.insert(method.to_string(), Box::new(handler));
        }
        self
    }

    /// Checks if a notification handler exists for a method.
    ///
    /// # Arguments
    ///
    /// * `method` - The method name to check
    ///
    /// # Returns
    ///
    /// `true` if a handler exists, `false` otherwise
    pub fn has_notification_handler(&self, method: &str) -> bool {
        self.notification_handlers
            .try_lock()
            .map(|handlers| handlers.contains_key(method))
            .unwrap_or(false)
    }

    /// Builds the protocol with the configured handlers.
    ///
    /// # Returns
    ///
    /// A new `Protocol` instance
    pub fn build(self) -> Protocol {
        Protocol {
            request_id: Arc::new(AtomicU64::new(0)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            request_handlers: self.request_handlers,
            notification_handlers: self.notification_handlers,
        }
    }
}

/// Trait for handling JSON-RPC requests.
///
/// Implementors of this trait can handle incoming JSON-RPC requests
/// and produce responses.
#[async_trait]
trait RequestHandler: Send + Sync {
    /// Handles an incoming JSON-RPC request.
    ///
    /// # Arguments
    ///
    /// * `request` - The incoming JSON-RPC request
    ///
    /// # Returns
    ///
    /// A `Result` containing the response or an error
    async fn handle(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse>;
}

/// Trait for handling JSON-RPC notifications.
///
/// Implementors of this trait can handle incoming JSON-RPC notifications.
#[async_trait]
trait NotificationHandler: Send + Sync {
    /// Handles an incoming JSON-RPC notification.
    ///
    /// # Arguments
    ///
    /// * `notification` - The incoming JSON-RPC notification
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure
    async fn handle(&self, notification: JsonRpcNotification) -> Result<()>;
}

/// A typed request handler.
///
/// This struct adapts a typed handler function to the `RequestHandler` trait,
/// handling the deserialization of the request and serialization of the response.
struct TypedRequestHandler<Req, Resp>
where
    Req: DeserializeOwned + Send + Sync + 'static,
    Resp: Serialize + Send + Sync + 'static,
{
    handler: Box<
        dyn Fn(Req) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Resp>> + Send>>
            + Send
            + Sync,
    >,
    _phantom: std::marker::PhantomData<(Req, Resp)>,
}

#[async_trait]
impl<Req, Resp> RequestHandler for TypedRequestHandler<Req, Resp>
where
    Req: DeserializeOwned + Send + Sync + 'static,
    Resp: Serialize + Send + Sync + 'static,
{
    async fn handle(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let params: Req = if request.params.is_none() || request.params.as_ref().unwrap().is_null()
        {
            serde_json::from_value(json!({}))?
        } else {
            serde_json::from_value(request.params.unwrap())?
        };
        let result = (self.handler)(params).await?;
        Ok(JsonRpcResponse {
            id: request.id,
            result: Some(serde_json::to_value(result)?),
            error: None,
            ..Default::default()
        })
    }
}

/// A typed notification handler.
///
/// This struct adapts a typed handler function to the `NotificationHandler` trait,
/// handling the deserialization of the notification.
struct TypedNotificationHandler<N>
where
    N: DeserializeOwned + Send + Sync + 'static,
{
    handler: Box<
        dyn Fn(N) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
            + Send
            + Sync,
    >,
    _phantom: std::marker::PhantomData<N>,
}

#[async_trait]
impl<N> NotificationHandler for TypedNotificationHandler<N>
where
    N: DeserializeOwned + Send + Sync + 'static,
{
    async fn handle(&self, notification: JsonRpcNotification) -> Result<()> {
        let params: N =
            if notification.params.is_none() || notification.params.as_ref().unwrap().is_null() {
                serde_json::from_value(serde_json::Value::Null)?
            } else {
                serde_json::from_value(notification.params.unwrap())?
            };
        (self.handler)(params).await
    }
}
