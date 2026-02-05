//! Native MCP tool wrapper for CrewAI agents.
//!
//! Corresponds to `crewai/tools/mcp_native_tool.py`.
//!
//! Provides a tool wrapper that reuses existing MCP client sessions
//! for better performance and connection management. Unlike
//! `MCPToolWrapper` which connects on-demand, this tool uses a shared
//! MCP client instance that maintains a persistent connection.

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::Mutex;

/// Native MCP tool that reuses client sessions.
///
/// Used when agents connect to MCP servers using structured configurations.
/// Reuses existing client sessions for better performance and proper
/// connection lifecycle management.
#[derive(Clone)]
pub struct MCPNativeTool {
    /// Prefixed tool name (server_name + "_" + tool_name).
    pub name: String,
    /// Tool description.
    pub description: String,
    /// JSON Schema for the tool's arguments.
    pub args_schema: Value,
    /// Shared MCP client instance (opaque, type-erased).
    /// In a full implementation, this would be a concrete MCP client type.
    mcp_client: Arc<Mutex<Box<dyn Any + Send + Sync>>>,
    /// Original tool name on the MCP server (without prefix).
    pub original_tool_name: String,
    /// Name of the MCP server.
    pub server_name: String,
}

impl fmt::Debug for MCPNativeTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MCPNativeTool")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("original_tool_name", &self.original_tool_name)
            .field("server_name", &self.server_name)
            .finish()
    }
}

impl MCPNativeTool {
    /// Create a new `MCPNativeTool`.
    ///
    /// # Arguments
    ///
    /// * `mcp_client` - MCPClient instance with active session (type-erased).
    /// * `tool_name` - Original name of the tool on the MCP server.
    /// * `tool_schema` - Schema information for the tool.
    /// * `server_name` - Name of the MCP server for prefixing.
    pub fn new(
        mcp_client: Box<dyn Any + Send + Sync>,
        tool_name: impl Into<String>,
        tool_schema: &Value,
        server_name: impl Into<String>,
    ) -> Self {
        let tool_name = tool_name.into();
        let server_name = server_name.into();
        let prefixed_name = format!("{}_{}", server_name, tool_name);

        let description = tool_schema
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Tool {} from {}", tool_name, server_name));

        let args_schema = tool_schema
            .get("args_schema")
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

        Self {
            name: prefixed_name,
            description,
            args_schema,
            mcp_client: Arc::new(Mutex::new(mcp_client)),
            original_tool_name: tool_name,
            server_name,
        }
    }

    /// Execute tool using the MCP client session (synchronous wrapper).
    ///
    /// Handles the complexity of running async code from a sync context,
    /// similar to the Python implementation which uses `asyncio.run()`.
    pub fn run(
        &self,
        _args: HashMap<String, Value>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // In a full implementation, this would:
        // 1. Lock the MCP client
        // 2. Reconnect if needed (since each sync call may be on a different tokio runtime)
        // 3. Call the tool
        // 4. Disconnect after the call
        // 5. Extract and return the result
        Err(format!(
            "MCP native tool execution for '{}' not yet implemented in Rust port. Server: {}",
            self.original_tool_name, self.server_name,
        )
        .into())
    }

    /// Execute tool asynchronously using the MCP client session.
    ///
    /// Reconnects on-demand because async runtimes may differ between calls.
    /// Always disconnects after the call to ensure clean context manager lifecycle.
    pub async fn run_async(
        &self,
        _args: HashMap<String, Value>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement actual MCP client connection and tool call.
        // The implementation should:
        // 1. Lock the MCP client
        // 2. Disconnect if already connected (new event loop context)
        // 3. Connect
        // 4. Call the tool
        // 5. Always disconnect in the finally block
        // 6. Handle reconnection on connection errors
        Err(format!(
            "MCP native tool async execution for '{}' not yet implemented in Rust port. Server: {}",
            self.original_tool_name, self.server_name,
        )
        .into())
    }

    /// Extract result content from an MCP tool call response.
    ///
    /// Handles various result formats including string results,
    /// content lists, and text content items.
    fn extract_result_content(result: &Value) -> String {
        if let Some(s) = result.as_str() {
            return s.to_string();
        }

        if let Some(content) = result.get("content") {
            if let Some(arr) = content.as_array() {
                if let Some(first) = arr.first() {
                    if let Some(text) = first.get("text").and_then(|t| t.as_str()) {
                        return text.to_string();
                    }
                    return first.to_string();
                }
            }
            return content.to_string();
        }

        result.to_string()
    }
}
