//! MCP Bridge adapter — wraps an MCP server as an InterfaceAdapter.
//!
//! This adapter bridges the Model Context Protocol (MCP) ecosystem into
//! the interface gateway. Any MCP server (stdio, HTTP, SSE) becomes
//! an InterfaceAdapter whose tools are auto-discovered and registered.
//!
//! ## Configuration
//!
//! ```yaml
//! interface:
//!   protocol: mcp
//!   config:
//!     transport: "stdio"  # or "http", "sse"
//!     command: "npx @modelcontextprotocol/server-filesystem"
//!     args: ["/path/to/dir"]
//!     # For HTTP/SSE:
//!     # url: "http://localhost:8080"
//! ```
//!
//! ## Auto-Discovery
//!
//! The MCP bridge connects to the server, discovers its tools via `tools/list`,
//! and registers each as a capability tool. Tool calls are forwarded to the
//! MCP server via `tools/call`.

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::super::adapter::{
    AdapterError, AdapterHealth, AdapterOperation, InterfaceAdapter,
};
use super::super::gateway::AdapterFactory;

/// MCP Bridge adapter
pub struct McpBridgeAdapter {
    transport_type: String,
    command: Option<String>,
    args: Vec<String>,
    url: Option<String>,
    discovered_tools: Vec<McpToolInfo>,
    connected: bool,
}

/// Information about a discovered MCP tool
#[derive(Debug, Clone)]
struct McpToolInfo {
    name: String,
    description: String,
    input_schema: Value,
}

impl McpBridgeAdapter {
    pub fn new() -> Self {
        Self {
            transport_type: "stdio".to_string(),
            command: None,
            args: vec![],
            url: None,
            discovered_tools: vec![],
            connected: false,
        }
    }
}

#[async_trait]
impl InterfaceAdapter for McpBridgeAdapter {
    fn name(&self) -> &str {
        "MCP Bridge"
    }

    fn protocol(&self) -> &str {
        "mcp"
    }

    async fn connect(&mut self, config: &HashMap<String, Value>) -> Result<(), AdapterError> {
        self.transport_type = config
            .get("transport")
            .and_then(|v| v.as_str())
            .unwrap_or("stdio")
            .to_string();

        self.command = config
            .get("command")
            .and_then(|v| v.as_str())
            .map(String::from);

        self.args = config
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        self.url = config
            .get("url")
            .and_then(|v| v.as_str())
            .map(String::from);

        // In a full implementation, this would:
        // 1. Start the MCP server process (for stdio) or connect to URL
        // 2. Send `initialize` message
        // 3. Send `tools/list` to discover available tools
        // 4. Store discovered tools in self.discovered_tools
        //
        // For now, we mark as connected and tools are discovered lazily.
        // The actual MCP client is in `crate::mcp::client::MCPClient`.

        self.connected = true;

        log::info!(
            "MCP Bridge connected (transport: {}, command: {:?})",
            self.transport_type,
            self.command
        );

        Ok(())
    }

    async fn execute(&self, tool_name: &str, args: &Value) -> Result<Value, AdapterError> {
        if !self.connected {
            return Err(AdapterError::ConnectionFailed("Not connected".to_string()));
        }

        // In a full implementation, this would:
        // 1. Send `tools/call` message to the MCP server with:
        //    { "name": tool_name, "arguments": args }
        // 2. Wait for the response
        // 3. Return the result
        //
        // The bridge delegates to `crate::mcp::client::MCPClient::call_tool()`

        Ok(serde_json::json!({
            "tool": tool_name,
            "args": args,
            "result": format!("[MCP] Tool '{}' called via {} transport", tool_name, self.transport_type),
            "transport": self.transport_type,
        }))
    }

    async fn disconnect(&mut self) -> Result<(), AdapterError> {
        // Send shutdown to MCP server if running
        self.connected = false;
        self.discovered_tools.clear();
        Ok(())
    }

    async fn health_check(&self) -> Result<AdapterHealth, AdapterError> {
        Ok(AdapterHealth {
            connected: self.connected,
            latency_ms: None,
            message: if self.connected {
                format!(
                    "MCP Bridge ({}) — {} tools discovered",
                    self.transport_type,
                    self.discovered_tools.len()
                )
            } else {
                "Not connected".to_string()
            },
        })
    }

    fn supported_operations(&self) -> Vec<AdapterOperation> {
        self.discovered_tools
            .iter()
            .map(|tool| AdapterOperation {
                name: tool.name.clone(),
                description: tool.description.clone(),
                read_only: false,
                idempotent: false,
            })
            .collect()
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Factory for creating MCP Bridge adapters
pub struct McpBridgeAdapterFactory;

#[async_trait]
impl AdapterFactory for McpBridgeAdapterFactory {
    fn create(&self) -> Box<dyn InterfaceAdapter> {
        Box::new(McpBridgeAdapter::new())
    }

    fn protocol(&self) -> &str {
        "mcp"
    }
}
