use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use url::Url;

/// Supported versions of the Model Context Protocol
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProtocolVersion {
    /// 2024-11-05 protocol version
    #[serde(rename = "2024-11-05")]
    V2024_11_05,
    /// 2025-03-26 protocol version
    #[serde(rename = "2025-03-26")]
    V2025_03_26,
}

impl ProtocolVersion {
    /// Returns the string representation of the protocol version
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtocolVersion::V2024_11_05 => "2024-11-05",
            ProtocolVersion::V2025_03_26 => "2025-03-26",
        }
    }
}

/// The latest version of the Model Context Protocol
pub const LATEST_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::V2025_03_26;

/// Describes the name and version of an MCP implementation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Implementation {
    /// The name of the implementation
    pub name: String,
    /// The version of the implementation
    pub version: String,
}

/// Initialization request sent from the client to the server
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct InitializeRequest {
    /// The protocol version that the client supports
    pub protocol_version: String,
    /// The client's capabilities
    pub capabilities: ClientCapabilities,
    /// Information about the client implementation
    pub client_info: Implementation,
}

/// Response to an initialization request
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct InitializeResponse {
    /// The protocol version that the server supports
    pub protocol_version: String,
    /// The server's capabilities
    pub capabilities: ServerCapabilities,
    /// Information about the server implementation
    pub server_info: Implementation,
    /// Instructions describing how to use the server and its features
    ///
    /// This can be used by clients to improve the LLM's understanding of available tools,
    /// resources, etc. It can be thought of like a "hint" to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

/// Capabilities that a server supports
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct ServerCapabilities {
    /// Tool-related capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolCapabilities>,
    /// Experimental, non-standard capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<serde_json::Value>,
    /// Logging capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<serde_json::Value>,
    /// Completion capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completions: Option<serde_json::Value>,
    /// Prompt-related capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptCapabilities>,
    /// Resource-related capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceCapabilities>,
}

/// Tool-related capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct ToolCapabilities {
    /// Whether the server supports notifications for changes to the tool list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Prompt-related capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct PromptCapabilities {
    /// Whether the server supports notifications for changes to the prompt list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Resource-related capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct ResourceCapabilities {
    /// Whether the server supports subscribing to resource updates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
    /// Whether the server supports notifications for changes to the resource list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Capabilities that a client supports
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct ClientCapabilities {
    /// Experimental, non-standard capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<serde_json::Value>,
    /// Sampling capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<serde_json::Value>,
    /// Root directory capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootCapabilities>,
}

/// Root directory-related capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct RootCapabilities {
    /// Whether the client supports notifications for changes to the roots list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Definition for a tool the client can call
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// The name of the tool
    pub name: String,
    /// A human-readable description of the tool
    ///
    /// This can be used by clients to improve the LLM's understanding of available tools.
    /// It can be thought of like a "hint" to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// A JSON Schema object defining the expected parameters for the tool
    pub input_schema: serde_json::Value,
    /// Optional additional tool information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
}

/// Additional properties describing a Tool to clients
///
/// NOTE: all properties in ToolAnnotations are **hints**.
/// They are not guaranteed to provide a faithful description of
/// tool behavior (including descriptive properties like `title`).
///
/// Clients should never make tool use decisions based on ToolAnnotations
/// received from untrusted servers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolAnnotations {
    /// A human-readable title for the tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// If true, the tool does not modify its environment
    ///
    /// Default: false
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only_hint: Option<bool>,
    /// If true, the tool may perform destructive updates to its environment.
    /// If false, the tool performs only additive updates.
    ///
    /// (This property is meaningful only when `read_only_hint == false`)
    ///
    /// Default: true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destructive_hint: Option<bool>,
    /// If true, calling the tool repeatedly with the same arguments
    /// will have no additional effect on the its environment.
    ///
    /// (This property is meaningful only when `read_only_hint == false`)
    ///
    /// Default: false
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotent_hint: Option<bool>,
    /// If true, this tool may interact with an "open world" of external
    /// entities. If false, the tool's domain of interaction is closed.
    /// For example, the world of a web search tool is open, whereas that
    /// of a memory tool is not.
    ///
    /// Default: true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_world_hint: Option<bool>,
}

/// Request to call a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolRequest {
    /// The name of the tool to call
    pub name: String,
    /// Arguments to pass to the tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, serde_json::Value>>,
    /// Optional metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

/// Response from a tool call
///
/// Any errors that originate from the tool SHOULD be reported inside the result
/// object, with `is_error` set to true, _not_ as an MCP protocol-level error
/// response. Otherwise, the LLM would not be able to see that an error occurred
/// and self-correct.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResponse {
    /// The content returned by the tool
    pub content: Vec<ToolResponseContent>,
    /// Whether the tool call ended in an error
    ///
    /// If not set, this is assumed to be false (the call was successful).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    /// Optional metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

/// Content types that can be returned by a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ToolResponseContent {
    /// Text content
    #[serde(rename = "text")]
    Text(TextContent),
    /// Image content
    #[serde(rename = "image")]
    Image(ImageContent),
    /// Audio content
    #[serde(rename = "audio")]
    Audio(AudioContent),
    /// Resource content
    #[serde(rename = "resource")]
    Resource(EmbeddedResource),
}

/// Text content provided to or from an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextContent {
    /// The content type, always "text"
    #[serde(rename = "type")]
    #[serde(default = "default_text_type")]
    pub content_type: String,
    /// The text content
    pub text: String,
    /// Optional annotations for the client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// An image provided to or from an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageContent {
    /// The content type, always "image"
    #[serde(rename = "type")]
    #[serde(default = "default_image_type")]
    pub content_type: String,
    /// The base64-encoded image data
    pub data: String,
    /// The MIME type of the image. Different providers may support different image types.
    pub mime_type: String,
    /// Optional annotations for the client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// Audio provided to or from an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioContent {
    /// The content type, always "audio"
    #[serde(rename = "type")]
    #[serde(default = "default_audio_type")]
    pub content_type: String,
    /// The base64-encoded audio data
    pub data: String,
    /// The MIME type of the audio. Different providers may support different audio types.
    pub mime_type: String,
    /// Optional annotations for the client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// The contents of a resource, embedded into a prompt or tool call result
///
/// It is up to the client how best to render embedded resources for the benefit
/// of the LLM and/or the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedResource {
    /// The content type, always "resource"
    #[serde(rename = "type")]
    #[serde(default = "default_resource_type")]
    pub content_type: String,
    /// The resource contents
    pub resource: ResourceContents,
    /// Optional annotations for the client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// Optional annotations for the client
///
/// The client can use annotations to inform how objects are used or displayed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Annotations {
    /// Describes who the intended customer of this object or data is
    ///
    /// It can include multiple entries to indicate content useful for multiple audiences
    /// (e.g., `["user", "assistant"]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Role>>,
    /// Describes how important this data is for operating the server
    ///
    /// A value of 1 means "most important," and indicates that the data is
    /// effectively required, while 0 means "least important," and indicates that
    /// the data is entirely optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f32>,
}

/// The contents of a specific resource or sub-resource
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceContents {
    /// The URI of this resource
    pub uri: Url,
    /// The MIME type of this resource, if known
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// The text of the item. This must only be set if the item can actually be
    /// represented as text (not binary data).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// A base64-encoded string representing the binary data of the item
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

/// Request to read a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceRequest {
    /// The URI of the resource to read
    pub uri: Url,
}

/// Response to a resource read request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceResponse {
    /// The contents of the requested resource
    pub contents: Vec<ResourceContents>,
    /// Optional metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

/// Base request for paginated list operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRequest {
    /// An opaque token representing the current pagination position
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// Optional metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

/// Response to a tools/list request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsListResponse {
    /// The list of available tools
    pub tools: Vec<Tool>,
    /// An opaque token representing the pagination position after the last returned result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Optional metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

/// Response to a prompts/list request
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptsListResponse {
    /// The list of available prompts
    pub prompts: Vec<Prompt>,
    /// An opaque token representing the pagination position after the last returned result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Optional metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, serde_json::Value>>,
}

/// A prompt or prompt template that the server offers
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    /// The name of the prompt or prompt template
    pub name: String,
    /// An optional description of what this prompt provides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// A list of arguments to use for templating the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

/// Describes an argument that a prompt can accept
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptArgument {
    /// The name of the argument
    pub name: String,
    /// A human-readable description of the argument
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether this argument must be provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

/// Response to a resources/list request
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesListResponse {
    /// The list of available resources
    pub resources: Vec<Resource>,
    /// An opaque token representing the pagination position after the last returned result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Optional metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, serde_json::Value>>,
}

/// A known resource that the server is capable of reading
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    /// The URI of this resource
    pub uri: Url,
    /// A human-readable name for this resource
    pub name: String,
    /// A description of what this resource represents
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The MIME type of this resource, if known
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Optional annotations for the client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
    /// The size of the raw resource content, in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<usize>,
}

/// The sender or recipient of messages and data in a conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Content from the user
    User,
    /// Content from the assistant
    Assistant,
}

/// Describes a message returned as part of a prompt
///
/// This is similar to `SamplingMessage`, but also supports the embedding of
/// resources from the MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptMessage {
    /// The sender or recipient of the message
    pub role: Role,
    /// The content of the message
    pub content: PromptMessageContent,
}

/// Content types that can be included in a prompt message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PromptMessageContent {
    /// Text content
    #[serde(rename = "text")]
    Text(TextContent),
    /// Image content
    #[serde(rename = "image")]
    Image(ImageContent),
    /// Audio content
    #[serde(rename = "audio")]
    Audio(AudioContent),
    /// Resource content
    #[serde(rename = "resource")]
    Resource(EmbeddedResource),
}

/// The server's preferences for model selection, requested of the client during sampling
///
/// Because LLMs can vary along multiple dimensions, choosing the "best" model is
/// rarely straightforward. Different models excel in different areas—some are
/// faster but less capable, others are more capable but more expensive, and so
/// on. This interface allows servers to express their priorities across multiple
/// dimensions to help clients make an appropriate selection for their use case.
///
/// These preferences are always advisory. The client MAY ignore them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelPreferences {
    /// Optional hints to use for model selection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<Vec<ModelHint>>,
    /// How much to prioritize cost when selecting a model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_priority: Option<f32>,
    /// How much to prioritize sampling speed (latency) when selecting a model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_priority: Option<f32>,
    /// How much to prioritize intelligence and capabilities when selecting a model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence_priority: Option<f32>,
}

/// Hints to use for model selection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelHint {
    /// A hint for a model name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Error codes used in the Model Context Protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// The connection was closed
    ConnectionClosed = -1,
    /// The request timed out
    RequestTimeout = -2,

    // Standard JSON-RPC error codes
    /// Invalid JSON was received by the server
    ParseError = -32700,
    /// The JSON sent is not a valid Request object
    InvalidRequest = -32600,
    /// The method does not exist / is not available
    MethodNotFound = -32601,
    /// Invalid method parameter(s)
    InvalidParams = -32602,
    /// Internal JSON-RPC error
    InternalError = -32603,
}

fn default_text_type() -> String {
    "text".to_string()
}

fn default_image_type() -> String {
    "image".to_string()
}

fn default_audio_type() -> String {
    "audio".to_string()
}

fn default_resource_type() -> String {
    "resource".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_capabilities() {
        let capabilities = ServerCapabilities::default();
        let json = serde_json::to_string(&capabilities).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_tool_annotations() {
        let annotations = ToolAnnotations {
            title: Some("My Tool".to_string()),
            read_only_hint: Some(true),
            destructive_hint: None,
            idempotent_hint: Some(false),
            open_world_hint: Some(true),
        };

        let json = serde_json::to_string(&annotations).unwrap();
        let expected = r#"{"title":"My Tool","readOnlyHint":true,"idempotentHint":false,"openWorldHint":true}"#;
        assert_eq!(json, expected);

        let parsed: ToolAnnotations = serde_json::from_str(expected).unwrap();
        assert_eq!(parsed.title, Some("My Tool".to_string()));
        assert_eq!(parsed.read_only_hint, Some(true));
        assert_eq!(parsed.destructive_hint, None);
        assert_eq!(parsed.idempotent_hint, Some(false));
        assert_eq!(parsed.open_world_hint, Some(true));
    }

    #[test]
    fn test_text_content() {
        let content = TextContent {
            content_type: "text".to_string(),
            text: "Hello, world!".to_string(),
            annotations: None,
        };

        let json = serde_json::to_string(&content).unwrap();
        let expected = r#"{"type":"text","text":"Hello, world!"}"#;
        assert_eq!(json, expected);

        let parsed: TextContent = serde_json::from_str(expected).unwrap();
        assert_eq!(parsed.content_type, "text");
        assert_eq!(parsed.text, "Hello, world!");
        assert!(parsed.annotations.is_none());
    }
}
