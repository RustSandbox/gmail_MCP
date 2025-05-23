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
use tracing::{debug, error, info, trace, warn};

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
    max_results: tool_param!(
        u32,
        description =
            "The maximum number of emails to fetch (1-500). Defaults to 10 if not specified.",
        optional = true
    ),
) -> Result<ToolResponseContent, Box<dyn std::error::Error>> {
    // Use max_results directly - if optional=true, it should provide a default
    info!(
        "[gmail] Tool called with action: {} and max_results: {}",
        action, max_results
    );
    debug!("[gmail] Starting to read emails...");
    trace!(
        "[gmail] Tool parameters: action={}, max_results={}",
        action, max_results
    );

    let result = read_emails(max_results).await;
    debug!(
        "[gmail] Read emails completed with status: {:?}",
        result.is_ok()
    );

    match result {
        Ok(output) => {
            debug!("[gmail] Successfully retrieved emails");
            trace!("[gmail] Raw output length: {} bytes", output.len());
            // Only try to serialize the successful output
            if let Ok(colored) = to_colored_json_auto(&output) {
                debug!("[gmail] Colored output: {}", colored);
                trace!("[gmail] Colored output length: {} bytes", colored.len());
            } else {
                warn!("[gmail] Failed to create colored JSON output");
            }
            info!("[gmail] Returning successful response");
            Ok(tool_text_content!(output))
        }
        Err(e) => {
            let error_msg = format!("Error reading Gmail: {:#?}", e);
            error!("[gmail] Error occurred: {}", error_msg);
            trace!("[gmail] Full error details: {:?}", e);
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
        .with_level(true)
        .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
        .event_format(tracing_subscriber::fmt::format().with_ansi(false))
        .with_max_level(tracing::Level::TRACE) // Enable all log levels including TRACE
        .with_thread_names(true) // Show thread names
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL) // Show span events
        .init();

    // Log server startup with more details
    info!(
        target = "server",
        version = env!("CARGO_PKG_VERSION"),
        "Starting MCP Gmail Server with enhanced logging..."
    );
    debug!(
        target = "server",
        "Server configuration: max_level=TRACE, thread_names=true, span_events=FULL"
    );
    trace!(
        target = "server",
        "Detailed server configuration: ansi=true, target=true, thread_ids=true, file=true, line_number=true"
    );

    // Step 1: Build the MCP server protocol handler
    info!("Building MCP server protocol handler");
    debug!("Server name: Gmail-reader-server");
    debug!("Server version: 1.0.0");
    debug!("Protocol version: V2025_03_26");

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

    debug!("MCP server protocol handler built successfully");
    trace!("Server capabilities configured: tools enabled");

    // Step 2: Create the transport layer for the server
    info!("Creating server transport layer");
    debug!("Transport type: Server-Sent Events (SSE)");
    debug!("Binding address: 0.0.0.0:3003");

    let mcp_server_transport = ServerSseTransport::new(
        "0.0.0.0".to_string(), // Bind to all interfaces
        3003,                  // Listen on port 3003
        mcp_server_protocol,   // Use our configured protocol handler
    );

    debug!("Server transport layer created successfully");
    trace!("Transport configuration: address=0.0.0.0, port=3003");

    // Step 3: Start the server
    info!("MCP Gmail Server listening on http://0.0.0.0:3003/sse");
    info!("Server will run until terminated (Ctrl+C)");
    debug!("Server startup complete, entering main event loop");
    trace!("Server state: initialized, ready to accept connections");

    // Start the server and await its completion (which should never happen
    // unless there's an error or the server is shut down)
    match Server::start(mcp_server_transport).await {
        Ok(_) => {
            error!("Server stopped unexpectedly");
            trace!("Server shutdown details: normal termination");
            Ok(())
        }
        Err(e) => {
            if e.to_string().contains("Address already in use") {
                error!(
                    "Port 3003 is already in use. Please ensure no other instance of the server is running."
                );
                error!("You can kill the existing process using: lsof -i :3003 | grep LISTEN");
                trace!("Port conflict details: port=3003, error={:?}", e);
            } else {
                error!("Server error: {:?}", e);
                trace!("Full error details: {:?}", e);
            }
            Err(e.into())
        }
    }
}
