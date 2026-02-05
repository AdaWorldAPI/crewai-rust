//! InterfaceGateway — routes tool calls to the appropriate adapter.
//!
//! The gateway is the central hub that:
//! 1. Maps capabilities to adapters
//! 2. Routes tool calls to the right adapter
//! 3. Manages adapter lifecycle (connect/disconnect)
//! 4. Enforces rate limits per-capability

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::RwLock;

use super::adapter::{AdapterError, AdapterHealth, InterfaceAdapter};
use crate::capabilities::{Capability, InterfaceProtocol};

/// The interface gateway: maps capabilities to adapters and routes tool calls.
pub struct InterfaceGateway {
    /// Registered adapters indexed by protocol name
    adapter_factories: HashMap<String, Box<dyn AdapterFactory>>,

    /// Active adapter instances indexed by capability ID
    active_adapters: HashMap<String, Arc<RwLock<Box<dyn InterfaceAdapter>>>>,

    /// Tool → capability mapping for routing
    tool_routing: HashMap<String, String>,

    /// Rate limiting state per capability
    rate_limits: HashMap<String, RateLimitState>,
}

/// Factory for creating adapter instances
#[async_trait]
pub trait AdapterFactory: Send + Sync {
    /// Create a new adapter instance for the given protocol
    fn create(&self) -> Box<dyn InterfaceAdapter>;

    /// Protocol identifier
    fn protocol(&self) -> &str;
}

/// Rate limiting state
struct RateLimitState {
    max_rpm: u32,
    window_start: std::time::Instant,
    count: u32,
}

impl InterfaceGateway {
    /// Create a new gateway.
    pub fn new() -> Self {
        Self {
            adapter_factories: HashMap::new(),
            active_adapters: HashMap::new(),
            tool_routing: HashMap::new(),
            rate_limits: HashMap::new(),
        }
    }

    /// Create a gateway with all built-in adapter factories registered.
    pub fn with_defaults() -> Self {
        let mut gw = Self::new();
        gw.register_factory(Box::new(super::adapters::rest_api::RestApiAdapterFactory));
        gw.register_factory(Box::new(super::adapters::rcon::RconAdapterFactory));
        gw.register_factory(Box::new(super::adapters::graph_api::GraphApiAdapterFactory));
        gw.register_factory(Box::new(super::adapters::mcp_bridge::McpBridgeAdapterFactory));
        gw
    }

    /// Register an adapter factory for a protocol.
    pub fn register_factory(&mut self, factory: Box<dyn AdapterFactory>) {
        let protocol = factory.protocol().to_string();
        self.adapter_factories.insert(protocol, factory);
    }

    /// Bind a capability: create an adapter, connect it, and register its tools.
    ///
    /// After binding, the capability's tools become invocable through `invoke()`.
    pub async fn bind_capability(
        &mut self,
        capability: &Capability,
        connection_config: &HashMap<String, Value>,
    ) -> Result<(), AdapterError> {
        let protocol_key = protocol_to_key(&capability.interface.protocol);

        // Find the factory for this protocol
        let factory = self
            .adapter_factories
            .get(&protocol_key)
            .ok_or_else(|| {
                AdapterError::InvalidConfig(format!(
                    "No adapter factory registered for protocol: {}",
                    protocol_key
                ))
            })?;

        // Create and connect the adapter
        let mut adapter = factory.create();

        // Merge capability config with connection config
        let mut merged_config = capability.interface.config.clone();
        for (k, v) in connection_config {
            merged_config.insert(k.clone(), v.clone());
        }

        adapter.connect(&merged_config).await?;

        // Register tool routing
        for tool in &capability.tools {
            let qualified_name = format!("{}::{}", capability.id, tool.name);
            self.tool_routing
                .insert(qualified_name.clone(), capability.id.clone());
            // Also register unqualified name for convenience
            self.tool_routing
                .insert(tool.name.clone(), capability.id.clone());
        }

        // Set up rate limiting
        if let Some(max_rpm) = capability.policy.max_rpm {
            self.rate_limits.insert(
                capability.id.clone(),
                RateLimitState {
                    max_rpm,
                    window_start: std::time::Instant::now(),
                    count: 0,
                },
            );
        }

        // Store the active adapter
        self.active_adapters
            .insert(capability.id.clone(), Arc::new(RwLock::new(adapter)));

        Ok(())
    }

    /// Invoke a tool by name. Routes to the appropriate adapter.
    ///
    /// The tool name can be qualified ("minecraft:server_control::mc_execute")
    /// or unqualified ("mc_execute").
    pub async fn invoke(
        &mut self,
        tool_name: &str,
        args: &Value,
    ) -> Result<Value, AdapterError> {
        // Find which capability owns this tool
        let capability_id = self
            .tool_routing
            .get(tool_name)
            .ok_or_else(|| {
                AdapterError::OperationNotSupported(format!(
                    "No capability bound for tool: {}",
                    tool_name
                ))
            })?
            .clone();

        // Check rate limits
        if let Some(rate_limit) = self.rate_limits.get_mut(&capability_id) {
            let elapsed = rate_limit.window_start.elapsed();
            if elapsed >= std::time::Duration::from_secs(60) {
                // Reset window
                rate_limit.window_start = std::time::Instant::now();
                rate_limit.count = 0;
            }
            if rate_limit.count >= rate_limit.max_rpm {
                let remaining_ms =
                    (std::time::Duration::from_secs(60) - elapsed).as_millis() as u64;
                return Err(AdapterError::RateLimited(remaining_ms));
            }
            rate_limit.count += 1;
        }

        // Get the adapter and execute
        let adapter = self
            .active_adapters
            .get(&capability_id)
            .ok_or_else(|| {
                AdapterError::ConnectionFailed(format!(
                    "Adapter for {} not connected",
                    capability_id
                ))
            })?
            .clone();

        let adapter_guard = adapter.read().await;

        // Extract the unqualified tool name for the adapter
        let bare_name = tool_name.rsplit("::").next().unwrap_or(tool_name);
        adapter_guard.execute(bare_name, args).await
    }

    /// Disconnect a capability's adapter and unregister its tools.
    pub async fn unbind_capability(
        &mut self,
        capability_id: &str,
    ) -> Result<(), AdapterError> {
        if let Some(adapter) = self.active_adapters.remove(capability_id) {
            let mut adapter_guard = adapter.write().await;
            adapter_guard.disconnect().await?;
        }

        // Remove tool routing entries for this capability
        self.tool_routing
            .retain(|_, cap_id| cap_id != capability_id);

        self.rate_limits.remove(capability_id);

        Ok(())
    }

    /// Health check all active adapters.
    pub async fn health_check_all(&self) -> Vec<(String, Result<AdapterHealth, AdapterError>)> {
        let mut results = Vec::new();
        for (cap_id, adapter) in &self.active_adapters {
            let adapter_guard = adapter.read().await;
            let health = adapter_guard.health_check().await;
            results.push((cap_id.clone(), health));
        }
        results
    }

    /// List all bound capability IDs.
    pub fn bound_capabilities(&self) -> Vec<&str> {
        self.active_adapters.keys().map(|s| s.as_str()).collect()
    }

    /// List all routable tool names.
    pub fn available_tools(&self) -> Vec<(&str, &str)> {
        self.tool_routing
            .iter()
            .map(|(tool, cap)| (tool.as_str(), cap.as_str()))
            .collect()
    }

    /// Disconnect all adapters.
    pub async fn shutdown(&mut self) {
        let cap_ids: Vec<String> = self.active_adapters.keys().cloned().collect();
        for cap_id in cap_ids {
            let _ = self.unbind_capability(&cap_id).await;
        }
    }
}

impl Default for InterfaceGateway {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert an InterfaceProtocol enum to a string key for the factory registry
fn protocol_to_key(protocol: &InterfaceProtocol) -> String {
    match protocol {
        InterfaceProtocol::RestApi => "rest_api".to_string(),
        InterfaceProtocol::Graphql => "graphql".to_string(),
        InterfaceProtocol::Grpc => "grpc".to_string(),
        InterfaceProtocol::Mcp => "mcp".to_string(),
        InterfaceProtocol::Rcon => "rcon".to_string(),
        InterfaceProtocol::Websocket => "websocket".to_string(),
        InterfaceProtocol::ArrowFlight => "arrow_flight".to_string(),
        InterfaceProtocol::MsGraph => "ms_graph".to_string(),
        InterfaceProtocol::AwsSdk => "aws_sdk".to_string(),
        InterfaceProtocol::Ssh => "ssh".to_string(),
        InterfaceProtocol::Database => "database".to_string(),
        InterfaceProtocol::Native => "native".to_string(),
        InterfaceProtocol::Custom(name) => name.clone(),
    }
}
