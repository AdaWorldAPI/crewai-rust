//! MCP client with session management for crewAI agents.
//!
//! Corresponds to `crewai/mcp/client.py`.
//!
//! This module provides the `MCPClient` struct which manages connections
//! to MCP servers, supports tool discovery, tool execution, prompt listing,
//! and prompt retrieval. It includes retry logic with exponential backoff,
//! configurable timeouts, an in-memory schema cache, and event emission
//! for observability.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::sync::Mutex;

use crate::mcp::transports::{BaseTransport, TransportType};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// MCP connection timeout in seconds.
pub const MCP_CONNECTION_TIMEOUT: u64 = 30;
/// MCP tool execution timeout in seconds.
pub const MCP_TOOL_EXECUTION_TIMEOUT: u64 = 30;
/// MCP tool discovery timeout in seconds.
pub const MCP_DISCOVERY_TIMEOUT: u64 = 30;
/// Maximum retry attempts.
pub const MCP_MAX_RETRIES: u32 = 3;

/// Simple in-memory cache TTL for MCP tool schemas (5 minutes).
const CACHE_TTL: Duration = Duration::from_secs(300);

// ---------------------------------------------------------------------------
// CacheEntry
// ---------------------------------------------------------------------------

/// Cached schema entry.
struct CacheEntry {
    data: Vec<HashMap<String, Value>>,
    created_at: Instant,
}

impl CacheEntry {
    /// Check if this cache entry has expired.
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= CACHE_TTL
    }
}

// ---------------------------------------------------------------------------
// ServerInfo (for event emission)
// ---------------------------------------------------------------------------

/// Server information extracted from transport, used for event emission.
#[derive(Debug, Clone)]
struct ServerInfo {
    /// Human-readable server name (command line or URL).
    server_name: String,
    /// Server URL (None for stdio transports).
    server_url: Option<String>,
    /// Transport type string.
    transport_type: String,
}

// ---------------------------------------------------------------------------
// MCPClient
// ---------------------------------------------------------------------------

/// MCP client with session management.
///
/// Manages connections to MCP servers and provides a high-level
/// interface for interacting with MCP tools, prompts, and resources.
/// Supports configurable timeouts, retry logic with exponential backoff,
/// and an in-memory schema cache for tool definitions.
///
/// Corresponds to `crewai.mcp.client.MCPClient`.
///
/// # Example
///
/// ```rust,no_run
/// use crewai::mcp::client::MCPClient;
/// use crewai::mcp::transports::StdioTransport;
///
/// let transport = StdioTransport::new("python", Some(vec!["server.py".into()]), None);
/// let mut client = MCPClient::new(Box::new(transport))
///     .with_connect_timeout(60)
///     .with_cache_tools_list(true);
///
/// // async {
/// //     client.connect().await.unwrap();
/// //     let tools = client.list_tools(None).await.unwrap();
/// //     let result = client.call_tool("tool_name", None).await.unwrap();
/// //     client.disconnect().await.unwrap();
/// // };
/// ```
pub struct MCPClient {
    /// The transport layer for communication.
    pub transport: Box<dyn BaseTransport>,
    /// Connection timeout in seconds.
    pub connect_timeout: u64,
    /// Tool execution timeout in seconds.
    pub execution_timeout: u64,
    /// Tool discovery timeout in seconds.
    pub discovery_timeout: u64,
    /// Maximum retry attempts.
    pub max_retries: u32,
    /// Whether to cache tool list results.
    pub cache_tools_list: bool,
    /// Whether the client has been initialized (session created).
    initialized: bool,
    /// Whether the client was previously connected (for reconnection tracking).
    was_connected: bool,
    /// Opaque MCP session handle.
    ///
    /// In the Python implementation, this is a `ClientSession` from the MCP SDK.
    /// Stored as `Option<Value>` until an actual MCP SDK binding is integrated.
    session: Option<Value>,
    /// In-memory schema cache (keyed by resource-type-qualified identifier).
    schema_cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
}

impl MCPClient {
    /// Create a new MCPClient.
    ///
    /// # Arguments
    ///
    /// * `transport` - Transport instance for MCP server connection.
    pub fn new(transport: Box<dyn BaseTransport>) -> Self {
        Self {
            transport,
            connect_timeout: MCP_CONNECTION_TIMEOUT,
            execution_timeout: MCP_TOOL_EXECUTION_TIMEOUT,
            discovery_timeout: MCP_DISCOVERY_TIMEOUT,
            max_retries: MCP_MAX_RETRIES,
            cache_tools_list: false,
            initialized: false,
            was_connected: false,
            session: None,
            schema_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // -----------------------------------------------------------------------
    // Builder methods
    // -----------------------------------------------------------------------

    /// Builder: set connection timeout.
    pub fn with_connect_timeout(mut self, timeout: u64) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Builder: set execution timeout.
    pub fn with_execution_timeout(mut self, timeout: u64) -> Self {
        self.execution_timeout = timeout;
        self
    }

    /// Builder: set discovery timeout.
    pub fn with_discovery_timeout(mut self, timeout: u64) -> Self {
        self.discovery_timeout = timeout;
        self
    }

    /// Builder: set max retries.
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Builder: enable or disable tool list caching.
    pub fn with_cache_tools_list(mut self, cache: bool) -> Self {
        self.cache_tools_list = cache;
        self
    }

    // -----------------------------------------------------------------------
    // Connection state
    // -----------------------------------------------------------------------

    /// Check if the client is connected to the MCP server.
    ///
    /// Returns `true` only when the transport is connected AND
    /// the session has been initialized.
    pub fn connected(&self) -> bool {
        self.transport.connected() && self.initialized
    }

    /// Get a reference to the MCP session.
    ///
    /// # Errors
    ///
    /// Returns an error if the client is not connected.
    pub fn get_session(&self) -> Result<&Value, anyhow::Error> {
        self.session
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Client not connected. Call connect() first."))
    }

    // -----------------------------------------------------------------------
    // Server info (for events / logging)
    // -----------------------------------------------------------------------

    /// Get server information from the transport for event emission.
    ///
    /// Extracts a human-readable server name, optional URL, and transport
    /// type from the underlying transport instance.
    fn get_server_info(&self) -> ServerInfo {
        let transport_type = self.transport.transport_type();
        let identifier = self.transport.server_identifier();

        match transport_type {
            TransportType::Stdio => ServerInfo {
                server_name: identifier.clone(),
                server_url: None,
                transport_type: transport_type.to_string(),
            },
            TransportType::Http | TransportType::StreamableHttp => {
                // Identifier format is "http:<url>"
                let url = identifier
                    .strip_prefix("http:")
                    .unwrap_or(&identifier)
                    .to_string();
                ServerInfo {
                    server_name: url.clone(),
                    server_url: Some(url),
                    transport_type: transport_type.to_string(),
                }
            }
            TransportType::Sse => {
                // Identifier format is "sse:<url>"
                let url = identifier
                    .strip_prefix("sse:")
                    .unwrap_or(&identifier)
                    .to_string();
                ServerInfo {
                    server_name: url.clone(),
                    server_url: Some(url),
                    transport_type: transport_type.to_string(),
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Connect / Disconnect
    // -----------------------------------------------------------------------

    /// Connect to the MCP server and initialize the session.
    ///
    /// If already connected, this is a no-op. On success, sets the internal
    /// `initialized` flag and marks the client as having been connected at
    /// least once (for reconnection tracking).
    ///
    /// Emits `MCPConnectionStartedEvent` before attempting connection,
    /// and either `MCPConnectionCompletedEvent` or `MCPConnectionFailedEvent`
    /// on completion.
    ///
    /// # Errors
    ///
    /// * Connection timeout after `connect_timeout` seconds.
    /// * Transport-level connection failures.
    pub async fn connect(&mut self) -> Result<(), anyhow::Error> {
        if self.connected() {
            return Ok(());
        }

        let is_reconnect = self.was_connected;
        let server_info = self.get_server_info();
        let started_at = Instant::now();

        log::info!(
            "MCP connection started: server='{}', transport='{}', reconnect={}",
            server_info.server_name,
            server_info.transport_type,
            is_reconnect
        );

        // TODO: Emit MCPConnectionStartedEvent via event bus when integrated.

        let timeout = Duration::from_secs(self.connect_timeout);
        let result = tokio::time::timeout(timeout, self.transport.connect()).await;

        match result {
            Ok(Ok(())) => {
                // TODO: Create ClientSession with transport read/write streams.
                // self.session = Some(ClientSession::new(
                //     self.transport.read_stream(),
                //     self.transport.write_stream(),
                // ));
                // Initialize the MCP protocol session:
                // self.session.as_mut().unwrap().initialize().await?;

                self.initialized = true;
                self.was_connected = true;

                let duration_ms = started_at.elapsed().as_millis();
                log::info!(
                    "MCP connection established: server='{}' ({}ms)",
                    server_info.server_name,
                    duration_ms
                );

                // TODO: Emit MCPConnectionCompletedEvent via event bus.

                Ok(())
            }
            Ok(Err(e)) => {
                self.cleanup_on_error().await;
                let error_msg = format!("Failed to connect to MCP server: {}", e);
                self.emit_connection_failed(&server_info, &error_msg, "network", started_at);
                Err(anyhow::anyhow!("{}", error_msg))
            }
            Err(_) => {
                self.cleanup_on_error().await;
                let error_msg = format!(
                    "MCP connection timed out after {} seconds. \
                     The server may be slow or unreachable.",
                    self.connect_timeout
                );
                self.emit_connection_failed(&server_info, &error_msg, "timeout", started_at);
                Err(anyhow::anyhow!("{}", error_msg))
            }
        }
    }

    /// Disconnect from the MCP server and clean up resources.
    ///
    /// If not connected, this is a no-op.
    pub async fn disconnect(&mut self) -> Result<(), anyhow::Error> {
        if !self.connected() {
            return Ok(());
        }

        let result = self.transport.disconnect().await;

        // Always clean up internal state.
        self.session = None;
        self.initialized = false;

        result.map_err(|e| anyhow::anyhow!("Error during MCP client disconnect: {}", e))
    }

    /// Clean up resources when an error occurs during connection.
    ///
    /// Best-effort cleanup: disconnects transport, clears session, resets
    /// initialized state.
    async fn cleanup_on_error(&mut self) {
        let _ = self.transport.disconnect().await;
        self.session = None;
        self.initialized = false;
    }

    /// Emit a connection failed event/log.
    ///
    /// Currently logs at error level. Will emit `MCPConnectionFailedEvent`
    /// through the event bus when the event system is integrated.
    fn emit_connection_failed(
        &self,
        server_info: &ServerInfo,
        error: &str,
        error_type: &str,
        started_at: Instant,
    ) {
        let duration_ms = started_at.elapsed().as_millis();
        log::error!(
            "MCP connection failed: server='{}', error_type='{}', error='{}', duration={}ms",
            server_info.server_name,
            error_type,
            error,
            duration_ms
        );

        // TODO: Emit MCPConnectionFailedEvent via event bus:
        // event_bus.emit(MCPConnectionFailedEvent {
        //     server_name: server_info.server_name.clone(),
        //     server_url: server_info.server_url.clone(),
        //     transport_type: Some(server_info.transport_type.clone()),
        //     error: error.to_string(),
        //     error_type: error_type.to_string(),
        //     started_at,
        //     failed_at: Instant::now(),
        // });
    }

    // -----------------------------------------------------------------------
    // Tool Operations
    // -----------------------------------------------------------------------

    /// List available tools from the MCP server.
    ///
    /// # Arguments
    ///
    /// * `use_cache` - Whether to use cached results. If `None`, uses
    ///   the client's `cache_tools_list` setting.
    ///
    /// # Returns
    ///
    /// List of tool definitions, each containing `name`, `description`,
    /// and `inputSchema` keys.
    pub async fn list_tools(
        &mut self,
        use_cache: Option<bool>,
    ) -> Result<Vec<HashMap<String, Value>>, anyhow::Error> {
        if !self.connected() {
            self.connect().await?;
        }

        let use_cache = use_cache.unwrap_or(self.cache_tools_list);

        // Check cache if enabled.
        if use_cache {
            let cache_key = self.get_cache_key("tools");
            let cache = self.schema_cache.lock().await;
            if let Some(entry) = cache.get(&cache_key) {
                if !entry.is_expired() {
                    return Ok(entry.data.clone());
                }
            }
        }

        // Discover tools with retry.
        let tools = self
            .retry_operation(|| self.list_tools_impl())
            .await?;

        // Cache results if enabled.
        if use_cache {
            let cache_key = self.get_cache_key("tools");
            let mut cache = self.schema_cache.lock().await;
            cache.insert(
                cache_key,
                CacheEntry {
                    data: tools.clone(),
                    created_at: Instant::now(),
                },
            );
        }

        Ok(tools)
    }

    /// Internal implementation of list_tools.
    ///
    /// Calls the MCP session's `list_tools()` method and converts
    /// the response into a list of tool definitions.
    async fn list_tools_impl(&self) -> Result<Vec<HashMap<String, Value>>, anyhow::Error> {
        // TODO: Integrate with actual MCP session list_tools
        // let tools_result = self.session.list_tools().await?;
        // return Ok(tools_result.tools.iter().map(|tool| {
        //     let mut def = HashMap::new();
        //     def.insert("name".into(), Value::String(sanitize_tool_name(&tool.name)));
        //     def.insert("description".into(), Value::String(tool.description.clone().unwrap_or_default()));
        //     def.insert("inputSchema".into(), tool.input_schema.clone().unwrap_or(Value::Object(Default::default())));
        //     def
        // }).collect());

        log::debug!("list_tools_impl called (MCP SDK integration pending)");
        Ok(Vec::new())
    }

    /// Call a tool on the MCP server.
    ///
    /// # Arguments
    ///
    /// * `tool_name` - Name of the tool to call.
    /// * `arguments` - Tool arguments as a JSON value map.
    ///
    /// # Returns
    ///
    /// Tool execution result as a string.
    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: Option<HashMap<String, Value>>,
    ) -> Result<String, anyhow::Error> {
        if !self.connected() {
            self.connect().await?;
        }

        let arguments = arguments.unwrap_or_default();
        let cleaned_arguments = Self::clean_tool_arguments(&arguments);
        let tool_name_owned = tool_name.to_string();
        let server_info = self.get_server_info();

        log::info!(
            "MCP tool execution started: tool='{}', server='{}'",
            tool_name_owned,
            server_info.server_name
        );

        // TODO: Emit MCPToolExecutionStartedEvent via event bus.

        let started_at = Instant::now();

        let result = self
            .retry_operation(|| self.call_tool_impl(&tool_name_owned, &cleaned_arguments))
            .await;

        let duration_ms = started_at.elapsed().as_millis();

        match result {
            Ok(val) => {
                log::info!(
                    "MCP tool execution completed: tool='{}' ({}ms)",
                    tool_name_owned,
                    duration_ms
                );
                // TODO: Emit MCPToolExecutionCompletedEvent via event bus.
                Ok(val)
            }
            Err(e) => {
                let error_type = if e.to_string().to_lowercase().contains("timeout") {
                    "timeout"
                } else {
                    "server_error"
                };
                log::error!(
                    "MCP tool execution failed: tool='{}', error_type='{}', error='{}' ({}ms)",
                    tool_name_owned,
                    error_type,
                    e,
                    duration_ms
                );
                // TODO: Emit MCPToolExecutionFailedEvent via event bus.
                Err(e)
            }
        }
    }

    /// Internal implementation of call_tool.
    ///
    /// Calls the MCP session's `call_tool()` method, extracts the text
    /// content from the response, and returns it as a string.
    async fn call_tool_impl(
        &self,
        tool_name: &str,
        arguments: &HashMap<String, Value>,
    ) -> Result<String, anyhow::Error> {
        // TODO: Integrate with actual MCP session call_tool
        // let result = self.session.call_tool(tool_name, arguments).await?;
        // if let Some(content) = result.content.first() {
        //     if let Some(text) = content.text.as_ref() {
        //         return Ok(text.clone());
        //     }
        //     return Ok(format!("{:?}", content));
        // }
        // return Ok(format!("{:?}", result));

        log::debug!(
            "call_tool_impl called: tool='{}', args={:?} (MCP SDK integration pending)",
            tool_name,
            arguments
        );
        Err(anyhow::anyhow!(
            "MCP tool execution not yet implemented: {}",
            tool_name
        ))
    }

    // -----------------------------------------------------------------------
    // Prompt Operations
    // -----------------------------------------------------------------------

    /// List available prompts from the MCP server.
    ///
    /// Corresponds to `MCPClient.list_prompts()` in Python.
    ///
    /// # Returns
    ///
    /// List of prompt definitions, each containing `name`, `description`,
    /// and `arguments` keys.
    pub async fn list_prompts(&mut self) -> Result<Vec<HashMap<String, Value>>, anyhow::Error> {
        if !self.connected() {
            self.connect().await?;
        }

        self.retry_operation(|| self.list_prompts_impl()).await
    }

    /// Internal implementation of list_prompts.
    async fn list_prompts_impl(&self) -> Result<Vec<HashMap<String, Value>>, anyhow::Error> {
        // TODO: Integrate with actual MCP session list_prompts
        // let prompts_result = self.session.list_prompts().await?;
        // return Ok(prompts_result.prompts.iter().map(|prompt| {
        //     let mut def = HashMap::new();
        //     def.insert("name".into(), Value::String(prompt.name.clone()));
        //     def.insert("description".into(), Value::String(prompt.description.clone().unwrap_or_default()));
        //     def.insert("arguments".into(), serde_json::to_value(&prompt.arguments).unwrap_or(Value::Array(vec![])));
        //     def
        // }).collect());

        log::debug!("list_prompts_impl called (MCP SDK integration pending)");
        Ok(Vec::new())
    }

    /// Get a prompt from the MCP server.
    ///
    /// Corresponds to `MCPClient.get_prompt()` in Python.
    ///
    /// # Arguments
    ///
    /// * `prompt_name` - Name of the prompt to get.
    /// * `arguments` - Optional prompt arguments.
    ///
    /// # Returns
    ///
    /// Prompt content and metadata as a JSON value map with `name`,
    /// `messages`, and `arguments` keys.
    pub async fn get_prompt(
        &mut self,
        prompt_name: &str,
        arguments: Option<HashMap<String, Value>>,
    ) -> Result<HashMap<String, Value>, anyhow::Error> {
        if !self.connected() {
            self.connect().await?;
        }

        let arguments = arguments.unwrap_or_default();
        let prompt_name_owned = prompt_name.to_string();

        self.retry_operation(|| self.get_prompt_impl(&prompt_name_owned, &arguments))
            .await
    }

    /// Internal implementation of get_prompt.
    async fn get_prompt_impl(
        &self,
        prompt_name: &str,
        arguments: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, anyhow::Error> {
        // TODO: Integrate with actual MCP session get_prompt
        // let result = self.session.get_prompt(prompt_name, arguments).await?;
        // let messages: Vec<Value> = result.messages.iter().map(|msg| {
        //     serde_json::json!({
        //         "role": msg.role,
        //         "content": msg.content,
        //     })
        // }).collect();
        // let mut response = HashMap::new();
        // response.insert("name".into(), Value::String(prompt_name.to_string()));
        // response.insert("messages".into(), Value::Array(messages));
        // response.insert("arguments".into(), serde_json::to_value(arguments)?);
        // return Ok(response);

        log::debug!(
            "get_prompt_impl called: prompt='{}', args={:?} (MCP SDK integration pending)",
            prompt_name,
            arguments
        );
        Err(anyhow::anyhow!(
            "MCP prompt retrieval not yet implemented: {}",
            prompt_name
        ))
    }

    // -----------------------------------------------------------------------
    // Argument cleaning
    // -----------------------------------------------------------------------

    /// Clean tool arguments by removing null values and fixing formats.
    ///
    /// Performs the following transformations:
    /// 1. Removes `null` values.
    /// 2. Converts `sources` arrays from `["web"]` to `[{"type": "web"}]`.
    /// 3. Recursively cleans nested objects and arrays.
    /// 4. Removes empty objects and arrays after cleaning.
    ///
    /// Corresponds to `MCPClient._clean_tool_arguments()` in Python.
    pub fn clean_tool_arguments(
        arguments: &HashMap<String, Value>,
    ) -> HashMap<String, Value> {
        let mut cleaned = HashMap::new();

        for (key, value) in arguments {
            // Skip null values.
            if value.is_null() {
                continue;
            }

            // Fix sources array format: convert ["web"] to [{"type": "web"}].
            if key == "sources" {
                if let Some(arr) = value.as_array() {
                    let fixed_sources: Vec<Value> = arr
                        .iter()
                        .map(|item| {
                            if let Some(s) = item.as_str() {
                                serde_json::json!({"type": s})
                            } else {
                                item.clone()
                            }
                        })
                        .collect();
                    if !fixed_sources.is_empty() {
                        cleaned.insert(key.clone(), Value::Array(fixed_sources));
                    }
                    continue;
                }
            }

            // Recursively clean nested objects.
            if let Some(obj) = value.as_object() {
                let nested_map: HashMap<String, Value> = obj
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                let nested_cleaned = Self::clean_tool_arguments(&nested_map);
                if !nested_cleaned.is_empty() {
                    cleaned.insert(
                        key.clone(),
                        serde_json::to_value(nested_cleaned).unwrap_or(Value::Null),
                    );
                }
            } else if let Some(arr) = value.as_array() {
                // Clean array items.
                let cleaned_list: Vec<Value> = arr
                    .iter()
                    .filter_map(|item| {
                        if item.is_null() {
                            return None;
                        }
                        if let Some(obj) = item.as_object() {
                            let nested_map: HashMap<String, Value> = obj
                                .iter()
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect();
                            let cleaned_item = Self::clean_tool_arguments(&nested_map);
                            if !cleaned_item.is_empty() {
                                return Some(
                                    serde_json::to_value(cleaned_item)
                                        .unwrap_or(Value::Null),
                                );
                            }
                            None
                        } else {
                            Some(item.clone())
                        }
                    })
                    .collect();
                if !cleaned_list.is_empty() {
                    cleaned.insert(key.clone(), Value::Array(cleaned_list));
                }
            } else {
                // Keep primitive values.
                cleaned.insert(key.clone(), value.clone());
            }
        }

        cleaned
    }

    // -----------------------------------------------------------------------
    // Retry logic
    // -----------------------------------------------------------------------

    /// Retry an async operation with exponential backoff.
    ///
    /// Non-retryable errors (authentication failures, not-found errors) are
    /// returned immediately without retrying.
    ///
    /// Corresponds to `MCPClient._retry_operation()` in Python.
    ///
    /// # Arguments
    ///
    /// * `operation` - Async closure producing the result.
    ///
    /// # Returns
    ///
    /// The operation result, or the last error after exhausting retries.
    async fn retry_operation<F, Fut, T>(&self, operation: F) -> Result<T, anyhow::Error>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, anyhow::Error>>,
    {
        let mut last_error = None;
        let timeout = Duration::from_secs(self.execution_timeout);

        for attempt in 0..self.max_retries {
            match tokio::time::timeout(timeout, operation()).await {
                Ok(Ok(result)) => return Ok(result),
                Ok(Err(e)) => {
                    let error_str = e.to_string().to_lowercase();

                    // Non-retryable errors: authentication.
                    if error_str.contains("authentication")
                        || error_str.contains("unauthorized")
                    {
                        return Err(anyhow::anyhow!("Authentication failed: {}", e));
                    }

                    // Non-retryable errors: not found.
                    if error_str.contains("not found") {
                        return Err(anyhow::anyhow!("Resource not found: {}", e));
                    }

                    // Retryable error.
                    last_error = Some(e);
                }
                Err(_) => {
                    last_error = Some(anyhow::anyhow!(
                        "Operation timed out after {} seconds",
                        self.execution_timeout
                    ));
                }
            }

            // Exponential backoff: 1s, 2s, 4s, ...
            if attempt < self.max_retries - 1 {
                let wait_time = Duration::from_secs(2u64.pow(attempt));
                tokio::time::sleep(wait_time).await;
            }
        }

        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!(
                "Operation failed after {} attempts",
                self.max_retries
            )
        }))
    }

    // -----------------------------------------------------------------------
    // Cache key generation
    // -----------------------------------------------------------------------

    /// Generate a cache key for a resource type.
    ///
    /// Uses the transport's server identifier and the resource type
    /// to create a unique cache key.
    ///
    /// Corresponds to `MCPClient._get_cache_key()` in Python.
    fn get_cache_key(&self, resource_type: &str) -> String {
        let transport_info = self.transport.server_identifier();
        format!("mcp:{}:{}", transport_info, resource_type)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_tool_arguments_removes_nulls() {
        let mut args = HashMap::new();
        args.insert("key1".to_string(), Value::String("value".to_string()));
        args.insert("key2".to_string(), Value::Null);
        args.insert("key3".to_string(), serde_json::json!(42));

        let cleaned = MCPClient::clean_tool_arguments(&args);
        assert_eq!(cleaned.len(), 2);
        assert!(cleaned.contains_key("key1"));
        assert!(cleaned.contains_key("key3"));
        assert!(!cleaned.contains_key("key2"));
    }

    #[test]
    fn test_clean_tool_arguments_fixes_sources() {
        let mut args = HashMap::new();
        args.insert(
            "sources".to_string(),
            serde_json::json!(["web", "file"]),
        );

        let cleaned = MCPClient::clean_tool_arguments(&args);
        let sources = cleaned.get("sources").unwrap().as_array().unwrap();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0], serde_json::json!({"type": "web"}));
        assert_eq!(sources[1], serde_json::json!({"type": "file"}));
    }

    #[test]
    fn test_clean_tool_arguments_keeps_sources_objects() {
        let mut args = HashMap::new();
        args.insert(
            "sources".to_string(),
            serde_json::json!([{"type": "web"}, {"type": "file"}]),
        );

        let cleaned = MCPClient::clean_tool_arguments(&args);
        let sources = cleaned.get("sources").unwrap().as_array().unwrap();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0], serde_json::json!({"type": "web"}));
    }

    #[test]
    fn test_clean_tool_arguments_nested_objects() {
        let mut args = HashMap::new();
        args.insert(
            "config".to_string(),
            serde_json::json!({
                "name": "test",
                "value": null,
                "nested": {"a": 1, "b": null}
            }),
        );

        let cleaned = MCPClient::clean_tool_arguments(&args);
        let config = cleaned.get("config").unwrap();
        assert!(config.get("name").is_some());
        // Null values should be removed from nested objects.
        assert!(config.get("value").is_none());
    }

    #[test]
    fn test_clean_tool_arguments_empty() {
        let args = HashMap::new();
        let cleaned = MCPClient::clean_tool_arguments(&args);
        assert!(cleaned.is_empty());
    }

    #[test]
    fn test_clean_tool_arguments_all_nulls() {
        let mut args = HashMap::new();
        args.insert("a".to_string(), Value::Null);
        args.insert("b".to_string(), Value::Null);

        let cleaned = MCPClient::clean_tool_arguments(&args);
        assert!(cleaned.is_empty());
    }

    #[test]
    fn test_clean_tool_arguments_array_with_nulls() {
        let mut args = HashMap::new();
        args.insert(
            "items".to_string(),
            serde_json::json!(["hello", null, "world"]),
        );

        let cleaned = MCPClient::clean_tool_arguments(&args);
        let items = cleaned.get("items").unwrap().as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], serde_json::json!("hello"));
        assert_eq!(items[1], serde_json::json!("world"));
    }

    #[test]
    fn test_client_new_defaults() {
        use crate::mcp::transports::stdio::StdioTransport;
        let transport = StdioTransport::new("echo", None, None);
        let client = MCPClient::new(Box::new(transport));

        assert_eq!(client.connect_timeout, MCP_CONNECTION_TIMEOUT);
        assert_eq!(client.execution_timeout, MCP_TOOL_EXECUTION_TIMEOUT);
        assert_eq!(client.discovery_timeout, MCP_DISCOVERY_TIMEOUT);
        assert_eq!(client.max_retries, MCP_MAX_RETRIES);
        assert!(!client.cache_tools_list);
        assert!(!client.connected());
        assert!(!client.initialized);
        assert!(!client.was_connected);
    }

    #[test]
    fn test_client_builder() {
        use crate::mcp::transports::stdio::StdioTransport;
        let transport = StdioTransport::new("echo", None, None);
        let client = MCPClient::new(Box::new(transport))
            .with_connect_timeout(60)
            .with_execution_timeout(120)
            .with_discovery_timeout(45)
            .with_max_retries(5)
            .with_cache_tools_list(true);

        assert_eq!(client.connect_timeout, 60);
        assert_eq!(client.execution_timeout, 120);
        assert_eq!(client.discovery_timeout, 45);
        assert_eq!(client.max_retries, 5);
        assert!(client.cache_tools_list);
    }

    #[test]
    fn test_get_session_not_connected() {
        use crate::mcp::transports::stdio::StdioTransport;
        let transport = StdioTransport::new("echo", None, None);
        let client = MCPClient::new(Box::new(transport));

        let result = client.get_session();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }

    #[test]
    fn test_get_cache_key() {
        use crate::mcp::transports::http::HTTPTransport;
        let transport = HTTPTransport::new("https://example.com/mcp", None, None);
        let client = MCPClient::new(Box::new(transport));

        let key = client.get_cache_key("tools");
        assert!(key.starts_with("mcp:"));
        assert!(key.contains("http:"));
        assert!(key.ends_with(":tools"));
    }

    #[test]
    fn test_get_server_info_stdio() {
        use crate::mcp::transports::stdio::StdioTransport;
        let transport = StdioTransport::new(
            "python",
            Some(vec!["server.py".into()]),
            None,
        );
        let client = MCPClient::new(Box::new(transport));
        let info = client.get_server_info();

        assert!(info.server_name.contains("python"));
        assert!(info.server_url.is_none());
        assert_eq!(info.transport_type, "stdio");
    }

    #[test]
    fn test_get_server_info_http() {
        use crate::mcp::transports::http::HTTPTransport;
        let transport = HTTPTransport::new("https://api.example.com/mcp", None, None);
        let client = MCPClient::new(Box::new(transport));
        let info = client.get_server_info();

        assert!(info.server_url.is_some());
        assert!(info.server_url.as_ref().unwrap().contains("example.com"));
    }

    #[test]
    fn test_get_server_info_sse() {
        use crate::mcp::transports::sse::SSETransport;
        let transport = SSETransport::new("https://api.example.com/sse", None);
        let client = MCPClient::new(Box::new(transport));
        let info = client.get_server_info();

        assert!(info.server_url.is_some());
        assert_eq!(info.transport_type, "sse");
    }
}
