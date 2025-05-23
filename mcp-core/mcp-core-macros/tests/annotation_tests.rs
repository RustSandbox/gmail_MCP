use anyhow::Result;
use mcp_core::{tool_text_content, types::ToolResponseContent};
use mcp_core_macros::{tool, tool_param};
use serde_json::json;

#[tokio::test]
async fn test_readonly_tool_annotations() {
    #[tool(
        name = "web_search",
        description = "Search the web for information",
        annotations(title = "web_search", read_only_hint = true, open_world_hint = true)
    )]
    async fn web_search_tool(query: String) -> Result<ToolResponseContent> {
        Ok(tool_text_content!("Success"))
    }

    let tool = WebSearchTool::tool();

    assert_eq!(tool.name, "web_search");
    assert_eq!(
        tool.description,
        Some("Search the web for information".to_string())
    );

    let expected_schema = json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string"
            }
        },
        "required": ["query"]
    });

    assert_eq!(tool.input_schema, expected_schema);

    let annotations = tool.annotations.unwrap();
    assert_eq!(annotations.title, Some("web_search".to_string()));
    assert_eq!(annotations.read_only_hint, Some(true));
    assert_eq!(annotations.destructive_hint, Some(true)); // Default value
    assert_eq!(annotations.idempotent_hint, Some(false)); // Default value
    assert_eq!(annotations.open_world_hint, Some(true));
}

#[tokio::test]
async fn test_destructive_tool_annotations() {
    #[tool(
        name = "delete_file",
        description = "Delete a file from the filesystem",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false,
            title = "Delete File"
        )
    )]
    async fn delete_file_tool(path: String) -> Result<ToolResponseContent> {
        Ok(tool_text_content!("Success"))
    }

    let tool = DeleteFileTool::tool();

    assert_eq!(tool.name, "delete_file");
    assert_eq!(
        tool.description,
        Some("Delete a file from the filesystem".to_string())
    );

    let expected_schema = json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string"
            }
        },
        "required": ["path"]
    });

    assert_eq!(tool.input_schema, expected_schema);

    let annotations = tool.annotations.unwrap();
    assert_eq!(annotations.title, Some("Delete File".to_string()));
    assert_eq!(annotations.read_only_hint, Some(false));
    assert_eq!(annotations.destructive_hint, Some(true));
    assert_eq!(annotations.idempotent_hint, Some(true));
    assert_eq!(annotations.open_world_hint, Some(false));
}

#[tokio::test]
async fn test_annotation_nested_syntax() {
    #[tool(
        name = "create_record",
        description = "Create a new record in the database",
        annotations(
            title = "Create Database Record",
            readOnlyHint = false,
            destructiveHint = false,
            idempotentHint = false,
            openWorldHint = false
        )
    )]
    async fn create_record_tool(table: String, data: String) -> Result<ToolResponseContent> {
        Ok(tool_text_content!("Success"))
    }

    let tool = CreateRecordTool::tool();

    assert_eq!(tool.name, "create_record");
    assert_eq!(
        tool.description,
        Some("Create a new record in the database".to_string())
    );

    let expected_schema = json!({
        "type": "object",
        "properties": {
            "table": {
                "type": "string"
            },
            "data": {
                "type": "string"
            }
        },
        "required": ["table", "data"]
    });

    assert_eq!(tool.input_schema, expected_schema);

    let annotations = tool.annotations.unwrap();
    assert_eq!(
        annotations.title,
        Some("Create Database Record".to_string())
    );
    assert_eq!(annotations.read_only_hint, Some(false));
    assert_eq!(annotations.destructive_hint, Some(false));
    assert_eq!(annotations.idempotent_hint, Some(false));
    assert_eq!(annotations.open_world_hint, Some(false));
}

#[tokio::test]
async fn test_numeric_parameters() {
    #[tool(name = "calculate", description = "Perform a calculation")]
    async fn calculate_tool(
        value1: f64,
        value2: i32,
        operation: String,
    ) -> Result<ToolResponseContent> {
        Ok(tool_text_content!("Success"))
    }

    let tool = CalculateTool::tool();

    assert_eq!(tool.name, "calculate");

    let expected_schema = json!({
        "type": "object",
        "properties": {
            "value1": {
                "type": "number"
            },
            "value2": {
                "type": "number"
            },
            "operation": {
                "type": "string"
            }
        },
        "required": ["value1", "value2", "operation"]
    });

    assert_eq!(tool.input_schema, expected_schema);
}

#[tokio::test]
async fn test_optional_parameters() {
    #[tool(
        name = "optional_params_tool",
        description = "Tool with optional parameters"
    )]
    async fn optional_params_tool(
        required_param: tool_param!(String, description = "A required parameter"),
        optional_string: tool_param!(Option<String>, description = "An optional string parameter"),
        optional_number: tool_param!(Option<i32>, description = "An optional number parameter"),
    ) -> Result<ToolResponseContent> {
        Ok(tool_text_content!("Success"))
    }

    let tool = OptionalParamsTool::tool();

    let expected_schema = json!({
        "type": "object",
        "properties": {
            "required_param": {
                "type": "string",
                "description": "A required parameter"
            },
            "optional_string": {
                "type": "string",
                "description": "An optional string parameter"
            },
            "optional_number": {
                "type": "number",
                "description": "An optional number parameter"
            }
        },
        "required": ["required_param"]
    });

    assert_eq!(tool.input_schema, expected_schema);
}

#[tokio::test]
async fn test_parameter_descriptions() {
    #[tool(
        name = "QueryDatabase",
        description = "Query a database with parameters"
    )]
    async fn query_database_tool(
        db_name: tool_param!(String, description = "Name of the database to query"),
        query: tool_param!(String, description = "SQL query to execute"),
        timeout_ms: tool_param!(Option<i32>, description = "Query timeout in milliseconds"),
    ) -> Result<ToolResponseContent> {
        Ok(tool_text_content!("Success"))
    }

    let tool = QueryDatabaseTool::tool();

    assert_eq!(tool.name, "QueryDatabase");
    assert_eq!(
        tool.description,
        Some("Query a database with parameters".to_string())
    );

    // Validate schema structure
    let expected_schema = json!({
        "type": "object",
        "properties": {
            "db_name": {
                "type": "string",
                "description": "Name of the database to query"
            },
            "query": {
                "type": "string",
                "description": "SQL query to execute"
            },
            "timeout_ms": {
                "type": "number",
                "description": "Query timeout in milliseconds"
            }
        },
        "required": ["db_name", "query"]
    });

    assert_eq!(tool.input_schema, expected_schema);
}

#[tokio::test]
async fn test_parameter_hidden() {
    #[tool(
        name = "HideParameter",
        description = "Demo tool to show how to hide parameters"
    )]
    async fn hide_parameter_tool(
        shown: tool_param!(String, description = "Name of the shown parameter"),
        hidden: tool_param!(String, description = "Name of the hidden parameter", hidden),
    ) -> Result<ToolResponseContent> {
        Ok(tool_text_content!("Success"))
    }

    let tool = HideParameterTool::tool();

    assert_eq!(tool.name, "HideParameter");
    assert_eq!(
        tool.description,
        Some("Demo tool to show how to hide parameters".to_string())
    );

    // Validate schema structure
    let expected_schema = json!({
        "type": "object",
        "properties": {
            "shown": {
                "type": "string",
                "description": "Name of the shown parameter"
            },
        },
        "required": ["shown"]
    });

    assert_eq!(tool.input_schema, expected_schema);
}
