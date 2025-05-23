use crate::protocol::{Protocol, ProtocolBuilder, RequestOptions};
use crate::transport::{
    JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, Message, RequestId,
    Transport,
};
use crate::types::ErrorCode;
use anyhow::Result;
use async_trait::async_trait;
use std::future::Future;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::pin::Pin;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::debug;

/// Client transport that communicates with an MCP server over standard I/O.
///
/// The `ClientStdioTransport` launches a child process specified by the provided
/// program and arguments, then communicates with it using the standard input and output
/// streams. It implements the `Transport` trait to send requests and receive responses
/// over these streams.
///
/// This transport is useful for:
/// - Running local MCP servers as child processes
/// - Command-line tools that need to communicate with MCP servers
/// - Testing and development scenarios
///
/// # Example
///
/// ```
/// use mcp_core::transport::{ClientStdioTransport, Transport};
/// use anyhow::Result;
///
/// async fn example() -> Result<()> {
///     let transport = ClientStdioTransport::new("my-mcp-server", &["--flag"])?;
///     transport.open().await?;
///     // Use transport...
///     transport.close().await?;
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct ClientStdioTransport {
    protocol: Protocol,
    stdin: Arc<Mutex<Option<BufWriter<std::process::ChildStdin>>>>,
    stdout: Arc<Mutex<Option<BufReader<std::process::ChildStdout>>>>,
    child: Arc<Mutex<Option<std::process::Child>>>,
    program: String,
    args: Vec<String>,
}

impl ClientStdioTransport {
    /// Creates a new `ClientStdioTransport` instance.
    ///
    /// # Arguments
    ///
    /// * `program` - The path or name of the program to execute
    /// * `args` - Command-line arguments to pass to the program
    ///
    /// # Returns
    ///
    /// A `Result` containing the new transport instance if successful
    pub fn new(program: &str, args: &[&str]) -> Result<Self> {
        Ok(ClientStdioTransport {
            protocol: ProtocolBuilder::new().build(),
            stdin: Arc::new(Mutex::new(None)),
            stdout: Arc::new(Mutex::new(None)),
            child: Arc::new(Mutex::new(None)),
            program: program.to_string(),
            args: args.iter().map(|&s| s.to_string()).collect(),
        })
    }
}

#[async_trait()]
impl Transport for ClientStdioTransport {
    /// Opens the transport by launching the child process and setting up the communication channels.
    ///
    /// This method:
    /// 1. Spawns the child process with the configured program and arguments
    /// 2. Sets up pipes for stdin and stdout
    /// 3. Starts a background task for handling incoming messages
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure
    async fn open(&self) -> Result<()> {
        debug!("ClientStdioTransport: Opening transport");
        let mut child = Command::new(&self.program)
            .args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Child process stdin not available"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Child process stdout not available"))?;

        {
            let mut stdin_lock = self.stdin.lock().await;
            *stdin_lock = Some(BufWriter::new(stdin));
        }
        {
            let mut stdout_lock = self.stdout.lock().await;
            *stdout_lock = Some(BufReader::new(stdout));
        }
        {
            let mut child_lock = self.child.lock().await;
            *child_lock = Some(child);
        }

        // Spawn a background task to continuously poll messages.
        let transport_clone = self.clone();
        tokio::spawn(async move {
            loop {
                match transport_clone.poll_message().await {
                    Ok(Some(message)) => match message {
                        Message::Request(request) => {
                            let response = transport_clone.protocol.handle_request(request).await;
                            let _ = transport_clone
                                .send_response(response.id, response.result, response.error)
                                .await;
                        }
                        Message::Notification(notification) => {
                            let _ = transport_clone
                                .protocol
                                .handle_notification(notification)
                                .await;
                        }
                        Message::Response(response) => {
                            transport_clone.protocol.handle_response(response).await;
                        }
                    },
                    Ok(None) => break, // EOF encountered.
                    Err(e) => {
                        debug!("ClientStdioTransport: Error polling message: {:?}", e);
                        break;
                    }
                }
            }
        });
        Ok(())
    }

    /// Closes the transport by terminating the child process and cleaning up resources.
    ///
    /// This method:
    /// 1. Kills the child process
    /// 2. Clears the stdin and stdout handles
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure
    async fn close(&self) -> Result<()> {
        let mut child_lock = self.child.lock().await;
        if let Some(child) = child_lock.as_mut() {
            let _ = child.kill();
        }
        *child_lock = None;

        // Clear stdin and stdout
        *self.stdin.lock().await = None;
        *self.stdout.lock().await = None;

        Ok(())
    }

    /// Polls for incoming messages from the child process's stdout.
    ///
    /// This method reads a line from the child process's stdout and parses it
    /// as a JSON-RPC message.
    ///
    /// # Returns
    ///
    /// A `Result` containing an `Option<Message>`. `None` indicates EOF.
    async fn poll_message(&self) -> Result<Option<Message>> {
        debug!("ClientStdioTransport: Starting to receive message");

        // Take ownership of stdout temporarily
        let mut stdout_guard = self.stdout.lock().await;
        let mut stdout = stdout_guard
            .take()
            .ok_or_else(|| anyhow::anyhow!("Transport not opened"))?;

        // Drop the lock before spawning the blocking task
        drop(stdout_guard);

        // Use a blocking operation in a spawn_blocking task
        let (line_result, stdout) = tokio::task::spawn_blocking(move || {
            let mut line = String::new();
            let result = match stdout.read_line(&mut line) {
                Ok(0) => Ok(None), // EOF
                Ok(_) => Ok(Some(line)),
                Err(e) => Err(anyhow::anyhow!("Error reading line: {}", e)),
            };
            // Return both the result and the stdout so we can put it back
            (result, stdout)
        })
        .await?;

        // Put stdout back
        let mut stdout_guard = self.stdout.lock().await;
        *stdout_guard = Some(stdout);

        // Process the result
        match line_result? {
            Some(line) => {
                debug!(
                    "ClientStdioTransport: Received from process: {}",
                    line.trim()
                );
                let message: Message = serde_json::from_str(&line)?;
                debug!("ClientStdioTransport: Successfully parsed message");
                Ok(Some(message))
            }
            None => {
                debug!("ClientStdioTransport: Received EOF from process");
                Ok(None)
            }
        }
    }

    /// Sends a request to the child process and waits for a response.
    ///
    /// This method:
    /// 1. Creates a new request ID
    /// 2. Constructs a JSON-RPC request
    /// 3. Sends it to the child process's stdin
    /// 4. Waits for a response with the same ID
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
    ) -> Pin<Box<dyn Future<Output = Result<JsonRpcResponse>> + Send + Sync>> {
        let protocol = self.protocol.clone();
        let stdin_arc = self.stdin.clone();
        let method = method.to_owned();
        Box::pin(async move {
            let (id, rx) = protocol.create_request().await;
            let request = JsonRpcRequest {
                id,
                method,
                jsonrpc: Default::default(),
                params,
            };
            let serialized = serde_json::to_string(&request)?;
            debug!("ClientStdioTransport: Sending request: {}", serialized);

            // Get the stdin writer
            let mut stdin_guard = stdin_arc.lock().await;
            let mut stdin = stdin_guard
                .take()
                .ok_or_else(|| anyhow::anyhow!("Transport not opened"))?;

            // Use a blocking operation in a spawn_blocking task
            let stdin_result = tokio::task::spawn_blocking(move || {
                stdin.write_all(serialized.as_bytes())?;
                stdin.write_all(b"\n")?;
                stdin.flush()?;
                Ok::<_, anyhow::Error>(stdin)
            })
            .await??;

            // Put the writer back
            *stdin_guard = Some(stdin_result);

            debug!("ClientStdioTransport: Request sent successfully");
            let result = timeout(options.timeout, rx).await;
            match result {
                Ok(inner_result) => match inner_result {
                    Ok(response) => Ok(response),
                    Err(_) => {
                        protocol.cancel_response(id).await;
                        Ok(JsonRpcResponse {
                            id,
                            result: None,
                            error: Some(JsonRpcError {
                                code: ErrorCode::RequestTimeout as i32,
                                message: "Request cancelled".to_string(),
                                data: None,
                            }),
                            ..Default::default()
                        })
                    }
                },
                Err(_) => {
                    protocol.cancel_response(id).await;
                    Ok(JsonRpcResponse {
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: ErrorCode::RequestTimeout as i32,
                            message: "Request timed out".to_string(),
                            data: None,
                        }),
                        ..Default::default()
                    })
                }
            }
        })
    }

    /// Sends a response to a request previously received from the child process.
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
    ) -> Result<()> {
        let response = JsonRpcResponse {
            id,
            result,
            error,
            jsonrpc: Default::default(),
        };
        let serialized = serde_json::to_string(&response)?;
        debug!("ClientStdioTransport: Sending response: {}", serialized);

        // Get the stdin writer
        let mut stdin_guard = self.stdin.lock().await;
        let mut stdin = stdin_guard
            .take()
            .ok_or_else(|| anyhow::anyhow!("Transport not opened"))?;

        // Use a blocking operation in a spawn_blocking task
        let stdin_result = tokio::task::spawn_blocking(move || {
            stdin.write_all(serialized.as_bytes())?;
            stdin.write_all(b"\n")?;
            stdin.flush()?;
            Ok::<_, anyhow::Error>(stdin)
        })
        .await??;

        // Put the writer back
        *stdin_guard = Some(stdin_result);

        Ok(())
    }

    /// Sends a notification to the child process.
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
    ) -> Result<()> {
        let notification = JsonRpcNotification {
            jsonrpc: Default::default(),
            method: method.to_owned(),
            params,
        };
        let serialized = serde_json::to_string(&notification)?;
        debug!("ClientStdioTransport: Sending notification: {}", serialized);

        // Get the stdin writer
        let mut stdin_guard = self.stdin.lock().await;
        let mut stdin = stdin_guard
            .take()
            .ok_or_else(|| anyhow::anyhow!("Transport not opened"))?;

        // Use a blocking operation in a spawn_blocking task
        let stdin_result = tokio::task::spawn_blocking(move || {
            stdin.write_all(serialized.as_bytes())?;
            stdin.write_all(b"\n")?;
            stdin.flush()?;
            Ok::<_, anyhow::Error>(stdin)
        })
        .await??;

        // Put the writer back
        *stdin_guard = Some(stdin_result);

        Ok(())
    }
}
