use anyhow::Result;
use gmail_mcp_server::reademail::read_emails;
use mcp_core::{
    server::Server,
    tool_text_content,
    transport::ServerSseTransport,
    types::{ServerCapabilities, ToolCapabilities, ToolResponseContent},
};
use mcp_core_macros::{tool, tool_param};
use tracing::info;

#[tool(
    name = "gmail_reader",
    description = "Read Gmail emails with automatic authentication."
)]
async fn gmail(
    action: tool_param!(String, description = "Action to perform on emails"),
    max_results: tool_param!(Option<u32>, description = "Max emails to fetch (1-500)"),
) -> Result<ToolResponseContent, Box<dyn std::error::Error>> {
    let max_results = max_results.unwrap_or(10);

    info!(
        "Gmail tool called with action: '{}', max_results: {}",
        action, max_results
    );

    match read_emails(max_results).await {
        Ok(emails) => Ok(tool_text_content!(emails)),
        Err(e) => {
            info!("Error fetching emails: {}", e);
            Err(e)
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize simple logging
    tracing_subscriber::fmt::init();

    // Create MCP server
    let server = Server::builder(
        "gmail-server".to_string(),
        "1.0.0".to_string(),
        mcp_core::types::ProtocolVersion::V2025_03_26,
    )
    .set_capabilities(ServerCapabilities {
        tools: Some(ToolCapabilities::default()),
        ..Default::default()
    })
    .register_tool(Gmail::tool(), Gmail::call())
    .build();

    // Start server transport
    let transport = ServerSseTransport::new("0.0.0.0".to_string(), 3003, server);

    println!("Gmail MCP Server running on http://0.0.0.0:3003/sse");

    Server::start(transport).await?;
    Ok(())
}
