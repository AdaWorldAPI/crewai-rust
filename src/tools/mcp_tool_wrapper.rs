//! MCP Tool Wrapper for on-demand MCP server connections.
//!
//! Corresponds to `crewai/tools/mcp_tool_wrapper.py`.
//!
//! Provides a lightweight wrapper for MCP tools that connects to the
//! MCP server on-demand for each tool invocation, with retry logic
//! and exponential backoff.

use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use serde_json::Value;

/// Connection timeout in seconds for MCP server.
pub const MCP_CONNECTION_TIMEOUT: u64 = 15;

/// Tool execution timeout in seconds.
pub const MCP_TOOL_EXECUTION_TIMEOUT: u64 = 60;

/// Discovery timeout in seconds.
pub const MCP_DISCOVERY_TIMEOUT: u64 = 15;

/// Maximum number of retry attempts.
pub const MCP_MAX_RETRIES: u32 = 3;

/// Lightweight wrapper for MCP tools that connects on-demand.
///
/// Each invocation establishes a new connection to the MCP server,
/// executes the tool, and tears down the connection. This approach
/// is simpler but has higher latency per call compared to
/// `MCPNativeTool`.
#[derive(Clone)]
pub struct MCPToolWrapper {
    /// Prefixed tool name (server_name + "_" + tool_name).
    pub name: String,
    /// Tool description.
    pub description: String,
    /// JSON Schema for the tool's arguments.
    pub args_schema: Value,
    /// Parameters for connecting to the MCP server (e.g., URL).
    pub mcp_server_params: HashMap<String, String>,
    /// Original tool name on the MCP server (without prefix).
    pub original_tool_name: String,
    /// Name of the MCP server.
    pub server_name: String,
}

impl fmt::Debug for MCPToolWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MCPToolWrapper")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("original_tool_name", &self.original_tool_name)
            .field("server_name", &self.server_name)
            .finish()
    }
}

impl MCPToolWrapper {
    /// Create a new `MCPToolWrapper`.
    ///
    /// # Arguments
    ///
    /// * `mcp_server_params` - Parameters for connecting to the MCP server.
    /// * `tool_name` - Original name of the tool on the MCP server.
    /// * `tool_schema` - Schema information for the tool (description, args_schema).
    /// * `server_name` - Name of the MCP server for prefixing.
    pub fn new(
        mcp_server_params: HashMap<String, String>,
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
            mcp_server_params,
            original_tool_name: tool_name,
            server_name,
        }
    }

    /// Execute the MCP tool synchronously.
    ///
    /// Connects to the MCP server, calls the tool, and returns the result.
    /// Uses retry logic with exponential backoff for transient failures.
    pub fn run(
        &self,
        _args: HashMap<String, Value>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // In a full implementation, this would:
        // 1. Connect to the MCP server via the configured transport
        // 2. Initialize the session
        // 3. Call the tool with the given arguments
        // 4. Extract and return the result
        //
        // For now, return an error indicating the MCP client is not yet implemented.
        Err(format!(
            "MCP tool execution for '{}' not yet implemented in Rust port. \
             Server: {}, URL: {}",
            self.original_tool_name,
            self.server_name,
            self.mcp_server_params
                .get("url")
                .unwrap_or(&"<not set>".to_string()),
        )
        .into())
    }

    /// Execute the MCP tool asynchronously with retry logic.
    pub async fn run_async(
        &self,
        args: HashMap<String, Value>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.retry_with_exponential_backoff(args).await
    }

    /// Retry operation with exponential backoff.
    async fn retry_with_exponential_backoff(
        &self,
        args: HashMap<String, Value>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut last_error = String::new();

        for attempt in 0..MCP_MAX_RETRIES {
            match self.execute_tool_with_timeout(&args).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    let error_str = e.to_string().to_lowercase();

                    // Non-retryable errors
                    if error_str.contains("authentication") || error_str.contains("unauthorized") {
                        return Err(format!("Authentication failed for MCP server: {}", e).into());
                    }
                    if error_str.contains("not found") {
                        return Err(format!(
                            "Tool '{}' not found on MCP server",
                            self.original_tool_name
                        )
                        .into());
                    }

                    // Retryable errors
                    last_error = e.to_string();
                    if attempt < MCP_MAX_RETRIES - 1 {
                        let wait_time = Duration::from_secs(2u64.pow(attempt));
                        tokio::time::sleep(wait_time).await;
                    }
                }
            }
        }

        Err(format!(
            "MCP tool execution failed after {} attempts: {}",
            MCP_MAX_RETRIES, last_error
        )
        .into())
    }

    /// Execute tool with timeout wrapper.
    async fn execute_tool_with_timeout(
        &self,
        args: &HashMap<String, Value>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let timeout = Duration::from_secs(MCP_TOOL_EXECUTION_TIMEOUT);

        match tokio::time::timeout(timeout, self.execute_tool(args)).await {
            Ok(result) => result,
            Err(_) => Err(format!(
                "MCP tool '{}' timed out after {} seconds",
                self.original_tool_name, MCP_TOOL_EXECUTION_TIMEOUT
            )
            .into()),
        }
    }

    /// Execute the actual MCP tool call.
    ///
    /// This is a stub that will be implemented when MCP client support is added.
    async fn execute_tool(
        &self,
        _args: &HashMap<String, Value>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement actual MCP client connection and tool call.
        // This requires an MCP client library for Rust.
        Err(format!(
            "MCP tool execution for '{}' not yet implemented in Rust port",
            self.original_tool_name
        )
        .into())
    }
}
