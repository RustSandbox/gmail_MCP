use anyhow::Result;
use clap::{Parser, ValueEnum};
use mcp_core::{
    server::Server,
    tool_text_response,
    tools::ToolHandlerFn,
    transport::{ServerSseTransport, ServerStdioTransport},
    types::{CallToolRequest, ServerCapabilities, Tool, ToolCapabilities},
};
use serde_json::json;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Transport type to use
    #[arg(value_enum, default_value_t = TransportType::Stdio)]
    transport: TransportType,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum TransportType {
    Stdio,
    Sse,
}

struct EchoTool;

impl EchoTool {
    fn tool() -> Tool {
        Tool {
            name: "echo".to_string(),
            description: Some("Echo back the message you send".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "The message to echo back"
                    }
                },
                "required": ["message"]
            }),
            annotations: None,
        }
    }

    fn call() -> ToolHandlerFn {
        move |request: CallToolRequest| {
            Box::pin(async move {
                let message = request
                    .arguments
                    .as_ref()
                    .and_then(|args| args.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                tool_text_response!(message)
            })
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    let server_protocol = Server::builder(
        "echo".to_string(),
        "1.0".to_string(),
        mcp_core::types::ProtocolVersion::V2024_11_05,
    )
    .set_capabilities(ServerCapabilities {
        tools: Some(ToolCapabilities::default()),
        ..Default::default()
    })
    .register_tool(EchoTool::tool(), EchoTool::call())
    .build();

    match cli.transport {
        TransportType::Stdio => {
            let transport = ServerStdioTransport::new(server_protocol);
            Server::start(transport).await
        }
        TransportType::Sse => {
            let transport = ServerSseTransport::new("127.0.0.1".to_string(), 3000, server_protocol);
            Server::start(transport).await
        }
    }
}
