//! InterfaceAdapter trait — the contract for external system adapters.

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// The core adapter trait. Every external system protocol implements this.
///
/// Adapters are stateful: they hold connection handles, auth tokens, etc.
/// The lifecycle is: `connect()` → `execute()` → `disconnect()`.
#[async_trait]
pub trait InterfaceAdapter: Send + Sync {
    /// Human-readable adapter name (e.g., "Minecraft RCON", "Microsoft Graph")
    fn name(&self) -> &str;

    /// Protocol identifier matching `InterfaceProtocol`
    fn protocol(&self) -> &str;

    /// Initialize the adapter with configuration from the capability.
    ///
    /// The config map comes from `CapabilityInterface.config` and may include
    /// endpoint URLs, ports, auth tokens, etc.
    async fn connect(&mut self, config: &HashMap<String, Value>) -> Result<(), AdapterError>;

    /// Execute a tool call through this adapter.
    ///
    /// The tool_name comes from `CapabilityTool.name` and args from the agent's
    /// tool call. The adapter translates this into the appropriate protocol
    /// operation (HTTP request, RCON command, SQL query, etc.).
    async fn execute(
        &self,
        tool_name: &str,
        args: &Value,
    ) -> Result<Value, AdapterError>;

    /// Disconnect and clean up resources.
    async fn disconnect(&mut self) -> Result<(), AdapterError>;

    /// Health check: is the adapter connected and working?
    async fn health_check(&self) -> Result<AdapterHealth, AdapterError>;

    /// List the operations this adapter supports.
    /// Used for capability auto-discovery.
    fn supported_operations(&self) -> Vec<AdapterOperation>;

    /// Whether the adapter is currently connected.
    fn is_connected(&self) -> bool;
}

/// Adapter health status
#[derive(Debug, Clone)]
pub struct AdapterHealth {
    pub connected: bool,
    pub latency_ms: Option<u64>,
    pub message: String,
}

/// An operation supported by the adapter
#[derive(Debug, Clone)]
pub struct AdapterOperation {
    pub name: String,
    pub description: String,
    pub read_only: bool,
    pub idempotent: bool,
}

/// Adapter error types
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Operation not supported: {0}")]
    OperationNotSupported(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("Rate limited: retry after {0}ms")]
    RateLimited(u64),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}
