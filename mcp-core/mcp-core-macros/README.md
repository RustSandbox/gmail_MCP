# MCP Core Macros

[![Crates.io](https://img.shields.io/crates/v/mcp-core-macros.svg)](https://crates.io/crates/mcp-core-macros)
[![Documentation](https://docs.rs/mcp-core-macros/badge.svg)](https://docs.rs/mcp-core-macros)

A Rust library providing procedural macros for the MCP Core system.

## Overview

This crate provides procedural macros that simplify the process of creating tool definitions for the MCP system. The macros handle generating tool metadata, parameter schemas, and the necessary boilerplate code for tool registration.

## Macros

### `#[tool]` Attribute Macro

The `tool` attribute macro transforms an async function into a tool that can be registered with the MCP system. It automatically generates:

- A struct named after the function (e.g., `web_search_tool` → `WebSearchTool`)
- Tool definitions with proper metadata
- JSON schema for input parameters 
- Methods to handle tool invocation

#### Arguments

- `name` - The name of the tool (optional, defaults to the function name)
- `description` - A description of what the tool does
- `annotations` - Additional metadata for the tool:
  - `title` - Display title for the tool (defaults to function name)
  - `read_only_hint` - Whether the tool only reads data (defaults to false)
  - `destructive_hint` - Whether the tool makes destructive changes (defaults to true)
  - `idempotent_hint` - Whether the tool is idempotent (defaults to false)
  - `open_world_hint` - Whether the tool can access resources outside the system (defaults to true)

#### Example

```rust
use mcp_core_macros::{tool, tool_param};
use mcp_core::types::ToolResponseContent;
use mcp_core::tool_text_content;
use anyhow::Result;

#[tool(
    name = "web_search",
    description = "Search the web for information",
    annotations(
        title = "Web Search",
        read_only_hint = true,
        open_world_hint = true
    )
)]
async fn web_search_tool(query: String) -> Result<ToolResponseContent> {
    // Tool implementation
    Ok(tool_text_content!("Results for: ".to_string() + &query))
}
```

### `tool_param!` Macro

The `tool_param!` macro allows specifying parameter attributes such as descriptions and visibility in the generated schema.

#### Arguments

- `hidden` - Excludes the parameter from the generated schema
- `description` - Adds a description to the parameter in the schema

#### Example

```rust
use mcp_core_macros::{tool, tool_param};
use mcp_core::types::ToolResponseContent;
use mcp_core::tool_text_content;
use anyhow::Result;

#[tool(name = "my_tool", description = "A tool with documented parameters", annotations(title = "My Tool"))]
async fn my_tool(
    // A required parameter with description
    required_param: tool_param!(String, description = "A required parameter"),
    
    // An optional parameter
    optional_param: tool_param!(Option<String>, description = "An optional parameter"),
    
    // A hidden parameter that won't appear in the schema
    internal_param: tool_param!(String, hidden)
) -> Result<ToolResponseContent> {
    // Implementation
    Ok(tool_text_content!("Tool executed".to_string()))
}
```

## Generated Code

The `tool` macro generates a structure with methods to handle tool registration and invocation. For example, the function:

```rust
#[tool(name = "example", description = "An example tool")]
async fn example_tool(param: String) -> Result<ToolResponseContent> {
    // Implementation
}
```

Will generate code equivalent to:

```rust
struct ExampleTool;

impl ExampleTool {
    pub fn tool() -> Tool {
        Tool {
            name: "example".to_string(),
            description: Some("An example tool".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "param": {
                        "type": "string"
                    }
                },
                "required": ["param"]
            }),
            annotations: Some(json!({
                "title": "example",
                "readOnlyHint": false,
                "destructiveHint": true,
                "idempotentHint": false,
                "openWorldHint": true
            })),
        }
    }
    
    pub async fn call(params: serde_json::Value) -> Result<ToolResponseContent> {
        // Deserialize parameters and call the implementation
        let param: String = serde_json::from_value(params["param"].clone())?;
        example_tool(param).await
    }
}
```

## License

This project is licensed under the Apache-2.0 License. 