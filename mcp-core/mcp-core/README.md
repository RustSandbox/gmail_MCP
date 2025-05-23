<p align="center">
    <img src="imgs/mcp_logo.png" alt="mcp_logo" style="width: 15%; margin-right:3%;" />
    <img src="imgs/plus.svg" alt="plus_svg" style="width: 10%; margin-bottom: 2%;" />
    <img src="imgs/rust_logo.png" alt="rust_logo" style="width: 15%; margin-left:3%;" />
</p>
<p align="center">
<h1 align="center">MCP Core</h1>
<p align="center">
A Rust library implementing the <a href="https://modelcontextprotocol.io/introduction">Modern Context Protocol (MCP)</a>
</p>
<p align="center">
<a href="https://github.com/stevohuncho/mcp-core"><img src="https://img.shields.io/github/stars/stevohuncho/mcp-core?style=social" alt="stars" /></a>
&nbsp;
<a href="https://crates.io/crates/mcp-core"><img src="https://img.shields.io/crates/v/mcp-core" alt="Crates.io" /></a>
&nbsp;
</p>

## Project Goals
Combine efforts with [Offical MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk). The offical SDK repo is new and collaborations are in works to bring these features to the adopted platform.
- **Efficiency & Scalability**
  - Handles many concurrent connections with low overhead.
  - Scales easily across multiple nodes.
- **Security**
  - Strong authentication and authorization.
  - Built-in rate limiting and quota management.
- **Rust Advantages**
  - High performance and predictable latency.
  - Memory safety with no runtime overhead.

## Installation

Use the `cargo add` command to automatically add it to your `Cargo.toml`
```bash
cargo add mcp-core
```
Or add `mcp-core` to your `Cargo.toml` dependencies directly
```toml
[dependencies]
mcp-core = "0.1.50"
```

## Server Implementation
Easily start your own local SSE MCP Servers with tooling capabilities. To use SSE functionality, make sure to enable the "http" feature in your Cargo.toml `mcp-core = { version = "0.1.50", features = ["sse"] }`
```rust
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
        mcp_core::types::ProtocolVersion::V2024_11_05
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
```

## Creating MCP Tools
There are two ways to create tools in MCP Core: using macros (recommended) or manually implementing the tool trait.

### Using Macros (Recommended)
The easiest way to create a tool is using the `mcp-core-macros` crate. First, add it to your dependencies:
```toml
[dependencies]
mcp-core-macros = "0.1.30"
```

Then create your tool using the `#[tool]` macro:
```rust
use mcp_core::{tool_text_content, types::ToolResponseContent};
use mcp_core_macros::{tool, tool_param};
use anyhow::Result;

#[tool(
    name = "echo",
    description = "Echo back the message you send",
    annotations(
        title = "Echo Tool",
        read_only_hint = true,
        destructive_hint = false
    )
)]
async fn echo_tool(
    message: tool_param!(String, description = "The message to echo back")
) -> Result<ToolResponseContent> {
    Ok(tool_text_content!(message))
}
```

The macro automatically generates all the necessary boilerplate code for your tool. You can then register it with your server:

```rust
let server_protocol = Server::builder(
    "echo".to_string(), 
    "1.0".to_string(),
    mcp_core::types::ProtocolVersion::V2024_11_05
)
.set_capabilities(ServerCapabilities {
    tools: Some(ToolCapabilities::default()),
    ..Default::default()
})
.register_tool(EchoTool::tool(), EchoTool::call())
.build();
```

### Tool Parameters
Tools can have various parameter types that are automatically deserialized from the client's JSON input:
- Basic types (String, f64, bool)
- Optional types (Option<T>)
- Custom parameter attributes

For example:
```rust
#[tool(
    name = "complex_tool",
    description = "A tool with complex parameters"
)]
async fn complex_tool(
    // A required parameter with description
    text: tool_param!(String, description = "A text parameter"),
    
    // An optional parameter
    number: tool_param!(Option<f64>, description = "An optional number parameter"),
    
    // A hidden parameter that won't appear in the schema
    internal_param: tool_param!(String, hidden)
) -> Result<ToolResponseContent> {
    // Tool implementation
    Ok(tool_text_content!("Tool executed successfully"))
}
```

## SSE Client Connection
Connect to an SSE MCP Server using the `ClientSseTransport`. Here is an example of connecting to one and listing the tools from that server.
```rust
use std::time::Duration;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use mcp_core::{
    client::ClientBuilder,
    protocol::RequestOptions,
    transport::{ClientSseTransportBuilder, ClientStdioTransport},
};
use serde_json::json;
use tracing::info;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Transport type to use
    #[arg(value_enum, default_value_t = TransportType::Sse)]
    transport: TransportType,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum TransportType {
    Stdio,
    Sse,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    let response = match cli.transport {
        TransportType::Stdio => {
            // Build the server first
            // cargo run --example echo_server --features="sse"
            let transport = ClientStdioTransport::new("./target/debug/examples/echo_server", &[])?;
            let client = ClientBuilder::new(transport.clone())
                .set_protocol_version(mcp_core::types::ProtocolVersion::V2024_11_05)
                .set_client_info("echo_client".to_string(), "0.1.0".to_string())
                .build();
            tokio::time::sleep(Duration::from_millis(100)).await;
            client.open().await?;

            client.initialize().await?;

            client
                .call_tool(
                    "echo",
                    Some(json!({
                        "message": "Hello, world!"
                    })),
                )
                .await?
        }
        TransportType::Sse => {
            let client = ClientBuilder::new(
                ClientSseTransportBuilder::new("http://localhost:3000/sse".to_string()).build(),
            )
            .set_protocol_version(mcp_core::types::ProtocolVersion::V2024_11_05)
            .set_client_info("echo_client".to_string(), "0.1.0".to_string())
            .build();
            client.open().await?;

            client.initialize().await?;

            client
                .request(
                    "tools/list",
                    None,
                    RequestOptions::default().timeout(Duration::from_secs(5)),
                )
                .await?;

            client
                .call_tool(
                    "echo",
                    Some(json!({
                        "message": "Hello, world!"
                    })),
                )
                .await?
        }
    };
    info!("response: {:?}", response);
    Ok(())
}
```

### Setting `SecureValues` to your SSE MCP Client
Have API Keys or Secrets needed to be passed to MCP Tool Calls, but you don't want to pass this information to the LLM you are prompting? Use `mcp_core::client::SecureValue`!
```rust
ClientBuilder::new(
    ClientSseTransportBuilder::new("http://localhost:3000/sse".to_string()).build(),
)
.with_secure_value(
    "discord_token",
    mcp_core::client::SecureValue::Static(discord_token),
)
.with_secure_value(
    "anthropic_api_key",
    mcp_core::client::SecureValue::Env("ANTHROPIC_API_KEY".to_string()),
)
.use_strict()
.build()
```
#### mcp_core::client::SecureValue::Static
Automatically have **MCP Tool Call Parameters** be replaced by the string value set to it.
#### mcp_core::client::SecureValue::Env
Automatically have **MCP Tool Call Parameters** be replaced by the value in your `.env` from the string set to it.