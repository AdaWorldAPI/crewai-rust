//! MCP (Model Context Protocol) event types.
//!
//! Corresponds to `crewai/events/types/mcp_events.py`.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::events::base_event::BaseEventData;
use crate::impl_base_event;

// ---------------------------------------------------------------------------
// MCPConnectionStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when starting to connect to an MCP server.
///
/// Corresponds to `MCPConnectionStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPConnectionStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the MCP server.
    pub server_name: String,
    /// MCP server URL.
    pub server_url: Option<String>,
    /// Transport type: "stdio", "http", "sse".
    pub transport_type: Option<String>,
    /// Connection timeout in seconds.
    pub connect_timeout: Option<i64>,
    /// Whether this is a reconnection attempt.
    pub is_reconnect: bool,
}

impl MCPConnectionStartedEvent {
    pub fn new(
        server_name: String,
        server_url: Option<String>,
        transport_type: Option<String>,
        connect_timeout: Option<i64>,
        is_reconnect: bool,
    ) -> Self {
        Self {
            base: BaseEventData::new("mcp_connection_started"),
            server_name,
            server_url,
            transport_type,
            connect_timeout,
            is_reconnect,
        }
    }
}

impl_base_event!(MCPConnectionStartedEvent);

// ---------------------------------------------------------------------------
// MCPConnectionCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when successfully connected to an MCP server.
///
/// Corresponds to `MCPConnectionCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPConnectionCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the MCP server.
    pub server_name: String,
    /// MCP server URL.
    pub server_url: Option<String>,
    /// Transport type.
    pub transport_type: Option<String>,
    /// When the connection started.
    pub started_at: Option<DateTime<Utc>>,
    /// When the connection completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Connection duration in milliseconds.
    pub connection_duration_ms: Option<f64>,
    /// Whether this was a reconnection.
    pub is_reconnect: bool,
}

impl MCPConnectionCompletedEvent {
    pub fn new(
        server_name: String,
        server_url: Option<String>,
        transport_type: Option<String>,
        connection_duration_ms: Option<f64>,
        is_reconnect: bool,
    ) -> Self {
        Self {
            base: BaseEventData::new("mcp_connection_completed"),
            server_name,
            server_url,
            transport_type,
            started_at: None,
            completed_at: None,
            connection_duration_ms,
            is_reconnect,
        }
    }
}

impl_base_event!(MCPConnectionCompletedEvent);

// ---------------------------------------------------------------------------
// MCPConnectionFailedEvent
// ---------------------------------------------------------------------------

/// Event emitted when connection to an MCP server fails.
///
/// Corresponds to `MCPConnectionFailedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPConnectionFailedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the MCP server.
    pub server_name: String,
    /// MCP server URL.
    pub server_url: Option<String>,
    /// Transport type.
    pub transport_type: Option<String>,
    /// Error message.
    pub error: String,
    /// Error type: "timeout", "authentication", "network", etc.
    pub error_type: Option<String>,
    /// When the connection started.
    pub started_at: Option<DateTime<Utc>>,
    /// When the connection failed.
    pub failed_at: Option<DateTime<Utc>>,
}

impl MCPConnectionFailedEvent {
    pub fn new(
        server_name: String,
        server_url: Option<String>,
        transport_type: Option<String>,
        error: String,
        error_type: Option<String>,
    ) -> Self {
        Self {
            base: BaseEventData::new("mcp_connection_failed"),
            server_name,
            server_url,
            transport_type,
            error,
            error_type,
            started_at: None,
            failed_at: None,
        }
    }
}

impl_base_event!(MCPConnectionFailedEvent);

// ---------------------------------------------------------------------------
// MCPToolExecutionStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when starting to execute an MCP tool.
///
/// Corresponds to `MCPToolExecutionStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolExecutionStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the MCP server.
    pub server_name: String,
    /// MCP server URL.
    pub server_url: Option<String>,
    /// Transport type.
    pub transport_type: Option<String>,
    /// Name of the tool being executed.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub tool_args: Option<HashMap<String, Value>>,
}

impl MCPToolExecutionStartedEvent {
    pub fn new(
        server_name: String,
        tool_name: String,
        tool_args: Option<HashMap<String, Value>>,
    ) -> Self {
        Self {
            base: BaseEventData::new("mcp_tool_execution_started"),
            server_name,
            server_url: None,
            transport_type: None,
            tool_name,
            tool_args,
        }
    }
}

impl_base_event!(MCPToolExecutionStartedEvent);

// ---------------------------------------------------------------------------
// MCPToolExecutionCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when MCP tool execution completes.
///
/// Corresponds to `MCPToolExecutionCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolExecutionCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the MCP server.
    pub server_name: String,
    /// MCP server URL.
    pub server_url: Option<String>,
    /// Transport type.
    pub transport_type: Option<String>,
    /// Name of the tool.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub tool_args: Option<HashMap<String, Value>>,
    /// Tool execution result (serialised).
    pub result: Option<Value>,
    /// When the execution started.
    pub started_at: Option<DateTime<Utc>>,
    /// When the execution completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Execution duration in milliseconds.
    pub execution_duration_ms: Option<f64>,
}

impl MCPToolExecutionCompletedEvent {
    pub fn new(
        server_name: String,
        tool_name: String,
        tool_args: Option<HashMap<String, Value>>,
        result: Option<Value>,
        execution_duration_ms: Option<f64>,
    ) -> Self {
        Self {
            base: BaseEventData::new("mcp_tool_execution_completed"),
            server_name,
            server_url: None,
            transport_type: None,
            tool_name,
            tool_args,
            result,
            started_at: None,
            completed_at: None,
            execution_duration_ms,
        }
    }
}

impl_base_event!(MCPToolExecutionCompletedEvent);

// ---------------------------------------------------------------------------
// MCPToolExecutionFailedEvent
// ---------------------------------------------------------------------------

/// Event emitted when MCP tool execution fails.
///
/// Corresponds to `MCPToolExecutionFailedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolExecutionFailedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the MCP server.
    pub server_name: String,
    /// MCP server URL.
    pub server_url: Option<String>,
    /// Transport type.
    pub transport_type: Option<String>,
    /// Name of the tool.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub tool_args: Option<HashMap<String, Value>>,
    /// Error message.
    pub error: String,
    /// Error type: "timeout", "validation", "server_error", etc.
    pub error_type: Option<String>,
    /// When the execution started.
    pub started_at: Option<DateTime<Utc>>,
    /// When the execution failed.
    pub failed_at: Option<DateTime<Utc>>,
}

impl MCPToolExecutionFailedEvent {
    pub fn new(
        server_name: String,
        tool_name: String,
        tool_args: Option<HashMap<String, Value>>,
        error: String,
        error_type: Option<String>,
    ) -> Self {
        Self {
            base: BaseEventData::new("mcp_tool_execution_failed"),
            server_name,
            server_url: None,
            transport_type: None,
            tool_name,
            tool_args,
            error,
            error_type,
            started_at: None,
            failed_at: None,
        }
    }
}

impl_base_event!(MCPToolExecutionFailedEvent);
