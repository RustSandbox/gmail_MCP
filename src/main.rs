use colored_json::to_colored_json_auto;
use gmailrs::reademail::read_emails;

use anyhow::Result;
use mcp_core::types::ToolCapabilities;
use mcp_core::{
    server::Server,
    tool_text_content,
    transport::ServerSseTransport,
    types::{ServerCapabilities, ToolResponseContent},
};
use mcp_core_macros::{tool, tool_param};
use tracing::{debug, info};

#[tool(
    name = "gmail_reader",
    description = "You can use this tool to read my gmail. all authentication and configuration is done automatically.",
    annotations(
        title = "Gmail Reader",
        read_only_hint = true,
        destructive_hint = false,
        idempotent_hint = false,
        open_world_hint = false
    )
)]
/*
title - Display title for the tool (defaults to function name)
read_only_hint - Whether the tool only reads data (defaults to false)
destructive_hint - Whether the tool makes destructive changes (defaults to true)
idempotent_hint - Whether the tool is idempotent (defaults to false)
open_world_hint
*/
async fn gmail(
    action: tool_param!(
        String,
        description = "an action to perform on the emails like summery, classify, etc."
    ),
) -> Result<ToolResponseContent, Box<dyn std::error::Error>> {
    info!("[gmail] Tool called with action: {}", action);
    debug!("[gmail] Starting to read emails...");

    let result = read_emails().await;
    debug!(
        "[gmail] Read emails completed with status: {:?}",
        result.is_ok()
    );

    match result {
        Ok(output) => {
            debug!("[gmail] Successfully retrieved emails");
            // Only try to serialize the successful output
            if let Ok(colored) = to_colored_json_auto(&output) {
                debug!("[gmail] Colored output: {}", colored);
            }
            info!("[gmail] Returning successful response");
            Ok(tool_text_content!(output))
        }
        Err(e) => {
            let error_msg = format!("Error reading Gmail: {:#?}", e);
            info!("[gmail] Error occurred: {}", error_msg);
            Err(error_msg.into())
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize the logging framework with a custom configuration
    // that ensures no ANSI codes or extra formatting
    tracing_subscriber::fmt()
        .with_ansi(true)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_level(true) // Disable level prefix
        .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339()) // Use UTC time in RFC3339 format
        .event_format(tracing_subscriber::fmt::format().with_ansi(false)) // Ensure no ANSI in event format
        .init();

    // Log server startup
    tracing::info!("Starting MCP Gmail Server...");

    // Step 1: Build the MCP server protocol handler
    // This creates the core server logic that will process MCP requests
    let mcp_server_protocol = Server::builder(
        "Gmail-reader-server".to_string(), // Server name for identification
        "1.0.0".to_string(),               // Server version
        mcp_core::types::ProtocolVersion::V2025_03_26, // MCP protocol version
    )
    // Configure server capabilities to indicate what features this server supports
    .set_capabilities(ServerCapabilities {
        tools: Some(ToolCapabilities::default()),
        ..Default::default()
    })
    // Register our add_tool function as an available tool
    // The tool() method provides metadata, call() provides the implementation
    .register_tool(Gmail::tool(), Gmail::call())
    .build();

    // Step 2: Create the transport layer for the server
    // This sets up the Server-Sent Events (SSE) transport on the specified address
    let mcp_server_transport = ServerSseTransport::new(
        "0.0.0.0".to_string(), // Bind to all interfaces
        3003,                  // Listen on port 3003
        mcp_server_protocol,   // Use our configured protocol handler
    );

    // Step 3: Start the server
    // The server will run indefinitely, handling client connections and requests
    tracing::info!("MCP Gmail Server listening on http://0.0.0.0:3003/sse");
    tracing::info!("Server will run until terminated (Ctrl+C)");

    // Start the server and await its completion (which should never happen
    // unless there's an error or the server is shut down)
    match Server::start(mcp_server_transport).await {
        Ok(_) => {
            tracing::error!("Server stopped unexpectedly");
            Ok(())
        }
        Err(e) => {
            if e.to_string().contains("Address already in use") {
                tracing::error!(
                    "Port 3003 is already in use. Please ensure no other instance of the server is running."
                );
                tracing::error!(
                    "You can kill the existing process using: lsof -i :3003 | grep LISTEN"
                );
            }
            Err(e.into())
        }
    }
}
