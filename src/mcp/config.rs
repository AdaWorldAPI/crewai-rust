//! MCP server configuration models for crewAI agents.
//!
//! Corresponds to `crewai/mcp/config.py`.
//!
//! This module provides configuration structs for connecting to MCP servers
//! using different transport types: Stdio (local process), HTTP/Streamable HTTP,
//! and Server-Sent Events (SSE). Each configuration includes transport-specific
//! settings, optional tool filtering, and caching options.
//!
//! These configurations are used to construct appropriate transports and
//! `MCPClient` instances when an agent needs to communicate with MCP servers.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

/// Shared type for tool filter functions.
///
/// Uses `Arc` so it can be cloned across configs without requiring
/// ownership transfer. The filter takes a tool definition JSON value
/// and returns `true` if the tool should be included.
///
/// Corresponds to the `ToolFilter` type in Python's
/// `crewai.mcp.filters`.
pub type ArcToolFilter = Arc<dyn Fn(&Value) -> bool + Send + Sync>;

// ---------------------------------------------------------------------------
// MCPServerStdio
// ---------------------------------------------------------------------------

/// Stdio MCP server configuration.
///
/// Used for connecting to local MCP servers that run as child processes
/// and communicate via standard input/output streams.
///
/// Corresponds to `crewai.mcp.config.MCPServerStdio`.
///
/// # Example
///
/// ```rust
/// use crewai::mcp::config::MCPServerStdio;
///
/// let config = MCPServerStdio::new("python")
///     .with_args(vec!["path/to/server.py".to_string()])
///     .with_cache_tools_list(true);
/// ```
#[derive(Serialize, Deserialize)]
pub struct MCPServerStdio {
    /// Command to execute (e.g., "python", "node", "npx", "uvx").
    pub command: String,
    /// Command arguments (e.g., vec!["server.py"] or vec!["-y", "@mcp/server"]).
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables to pass to the process.
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    /// Optional tool filter for filtering available tools.
    /// Serialization is skipped since ToolFilter contains function pointers.
    #[serde(skip)]
    pub tool_filter: Option<ArcToolFilter>,
    /// Whether to cache the tool list for faster subsequent access.
    #[serde(default)]
    pub cache_tools_list: bool,
}

impl std::fmt::Debug for MCPServerStdio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MCPServerStdio")
            .field("command", &self.command)
            .field("args", &self.args)
            .field("env", &self.env)
            .field("tool_filter", &self.tool_filter.as_ref().map(|_| "<filter>"))
            .field("cache_tools_list", &self.cache_tools_list)
            .finish()
    }
}

impl Clone for MCPServerStdio {
    fn clone(&self) -> Self {
        Self {
            command: self.command.clone(),
            args: self.args.clone(),
            env: self.env.clone(),
            tool_filter: self.tool_filter.clone(),
            cache_tools_list: self.cache_tools_list,
        }
    }
}

impl MCPServerStdio {
    /// Create a new MCPServerStdio configuration.
    ///
    /// # Arguments
    ///
    /// * `command` - Command to execute (e.g., "python", "node", "npx").
    pub fn new(command: &str) -> Self {
        Self {
            command: command.to_string(),
            args: Vec::new(),
            env: None,
            tool_filter: None,
            cache_tools_list: false,
        }
    }

    /// Set the command arguments.
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Set the environment variables.
    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = Some(env);
        self
    }

    /// Set the tool filter.
    pub fn with_tool_filter(mut self, filter: ArcToolFilter) -> Self {
        self.tool_filter = Some(filter);
        self
    }

    /// Enable or disable tool list caching.
    pub fn with_cache_tools_list(mut self, cache: bool) -> Self {
        self.cache_tools_list = cache;
        self
    }

    /// Get the server identifier for logging and caching.
    pub fn server_identifier(&self) -> String {
        format!("stdio:{}:{}", self.command, self.args.join(":"))
    }
}

// ---------------------------------------------------------------------------
// MCPServerHTTP
// ---------------------------------------------------------------------------

/// HTTP/Streamable HTTP MCP server configuration.
///
/// Used for connecting to remote MCP servers over HTTP/HTTPS
/// using streamable HTTP transport.
///
/// Corresponds to `crewai.mcp.config.MCPServerHTTP`.
///
/// # Example
///
/// ```rust
/// use crewai::mcp::config::MCPServerHTTP;
/// use std::collections::HashMap;
///
/// let mut headers = HashMap::new();
/// headers.insert("Authorization".to_string(), "Bearer token123".to_string());
///
/// let config = MCPServerHTTP::new("https://api.example.com/mcp")
///     .with_headers(headers)
///     .with_cache_tools_list(true);
/// ```
#[derive(Serialize, Deserialize)]
pub struct MCPServerHTTP {
    /// Server URL (e.g., "https://api.example.com/mcp").
    pub url: String,
    /// Optional HTTP headers for authentication or other purposes.
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    /// Whether to use streamable HTTP transport (default: true).
    #[serde(default = "default_true")]
    pub streamable: bool,
    /// Optional tool filter for filtering available tools.
    #[serde(skip)]
    pub tool_filter: Option<ArcToolFilter>,
    /// Whether to cache the tool list for faster subsequent access.
    #[serde(default)]
    pub cache_tools_list: bool,
}

/// Default value for boolean fields that should default to true.
fn default_true() -> bool {
    true
}

impl std::fmt::Debug for MCPServerHTTP {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MCPServerHTTP")
            .field("url", &self.url)
            .field("headers", &self.headers.as_ref().map(|h| {
                // Mask header values for security.
                h.keys().map(|k| format!("{}=<masked>", k)).collect::<Vec<_>>()
            }))
            .field("streamable", &self.streamable)
            .field("tool_filter", &self.tool_filter.as_ref().map(|_| "<filter>"))
            .field("cache_tools_list", &self.cache_tools_list)
            .finish()
    }
}

impl Clone for MCPServerHTTP {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            headers: self.headers.clone(),
            streamable: self.streamable,
            tool_filter: self.tool_filter.clone(),
            cache_tools_list: self.cache_tools_list,
        }
    }
}

impl MCPServerHTTP {
    /// Create a new MCPServerHTTP configuration.
    ///
    /// # Arguments
    ///
    /// * `url` - Server URL (e.g., "https://api.example.com/mcp").
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            headers: None,
            streamable: true,
            tool_filter: None,
            cache_tools_list: false,
        }
    }

    /// Set the HTTP headers.
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Set whether to use streamable HTTP.
    pub fn with_streamable(mut self, streamable: bool) -> Self {
        self.streamable = streamable;
        self
    }

    /// Set the tool filter.
    pub fn with_tool_filter(mut self, filter: ArcToolFilter) -> Self {
        self.tool_filter = Some(filter);
        self
    }

    /// Enable or disable tool list caching.
    pub fn with_cache_tools_list(mut self, cache: bool) -> Self {
        self.cache_tools_list = cache;
        self
    }

    /// Get the server identifier for logging and caching.
    pub fn server_identifier(&self) -> String {
        format!("http:{}", self.url)
    }
}

// ---------------------------------------------------------------------------
// MCPServerSSE
// ---------------------------------------------------------------------------

/// Server-Sent Events (SSE) MCP server configuration.
///
/// Used for connecting to remote MCP servers using Server-Sent Events
/// for real-time streaming communication.
///
/// Corresponds to `crewai.mcp.config.MCPServerSSE`.
///
/// # Example
///
/// ```rust
/// use crewai::mcp::config::MCPServerSSE;
///
/// let config = MCPServerSSE::new("https://api.example.com/mcp/sse")
///     .with_cache_tools_list(true);
/// ```
#[derive(Serialize, Deserialize)]
pub struct MCPServerSSE {
    /// Server URL (e.g., "https://api.example.com/mcp/sse").
    pub url: String,
    /// Optional HTTP headers for authentication or other purposes.
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    /// Optional tool filter for filtering available tools.
    #[serde(skip)]
    pub tool_filter: Option<ArcToolFilter>,
    /// Whether to cache the tool list for faster subsequent access.
    #[serde(default)]
    pub cache_tools_list: bool,
}

impl std::fmt::Debug for MCPServerSSE {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MCPServerSSE")
            .field("url", &self.url)
            .field("headers", &self.headers.as_ref().map(|h| {
                // Mask header values for security.
                h.keys().map(|k| format!("{}=<masked>", k)).collect::<Vec<_>>()
            }))
            .field("tool_filter", &self.tool_filter.as_ref().map(|_| "<filter>"))
            .field("cache_tools_list", &self.cache_tools_list)
            .finish()
    }
}

impl Clone for MCPServerSSE {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            headers: self.headers.clone(),
            tool_filter: self.tool_filter.clone(),
            cache_tools_list: self.cache_tools_list,
        }
    }
}

impl MCPServerSSE {
    /// Create a new MCPServerSSE configuration.
    ///
    /// # Arguments
    ///
    /// * `url` - Server URL (e.g., "https://api.example.com/mcp/sse").
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            headers: None,
            tool_filter: None,
            cache_tools_list: false,
        }
    }

    /// Set the HTTP headers.
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Set the tool filter.
    pub fn with_tool_filter(mut self, filter: ArcToolFilter) -> Self {
        self.tool_filter = Some(filter);
        self
    }

    /// Enable or disable tool list caching.
    pub fn with_cache_tools_list(mut self, cache: bool) -> Self {
        self.cache_tools_list = cache;
        self
    }

    /// Get the server identifier for logging and caching.
    pub fn server_identifier(&self) -> String {
        format!("sse:{}", self.url)
    }
}

// ---------------------------------------------------------------------------
// MCPServerConfig (union enum)
// ---------------------------------------------------------------------------

/// Union of all MCP server configuration types.
///
/// Corresponds to the Python type alias:
/// `MCPServerConfig = MCPServerStdio | MCPServerHTTP | MCPServerSSE`
///
/// Used when a function or struct can accept any MCP server configuration.
#[derive(Debug, Clone)]
pub enum MCPServerConfig {
    /// Stdio-based local process server.
    Stdio(MCPServerStdio),
    /// HTTP/Streamable HTTP remote server.
    Http(MCPServerHTTP),
    /// Server-Sent Events remote server.
    Sse(MCPServerSSE),
}

impl MCPServerConfig {
    /// Get the tool filter for this server configuration.
    pub fn tool_filter(&self) -> &Option<ArcToolFilter> {
        match self {
            MCPServerConfig::Stdio(s) => &s.tool_filter,
            MCPServerConfig::Http(s) => &s.tool_filter,
            MCPServerConfig::Sse(s) => &s.tool_filter,
        }
    }

    /// Check if tool list caching is enabled.
    pub fn cache_tools_list(&self) -> bool {
        match self {
            MCPServerConfig::Stdio(s) => s.cache_tools_list,
            MCPServerConfig::Http(s) => s.cache_tools_list,
            MCPServerConfig::Sse(s) => s.cache_tools_list,
        }
    }

    /// Get the server identifier for logging and caching.
    pub fn server_identifier(&self) -> String {
        match self {
            MCPServerConfig::Stdio(s) => s.server_identifier(),
            MCPServerConfig::Http(s) => s.server_identifier(),
            MCPServerConfig::Sse(s) => s.server_identifier(),
        }
    }
}

// Convenience From implementations for ergonomic enum construction.

impl From<MCPServerStdio> for MCPServerConfig {
    fn from(config: MCPServerStdio) -> Self {
        MCPServerConfig::Stdio(config)
    }
}

impl From<MCPServerHTTP> for MCPServerConfig {
    fn from(config: MCPServerHTTP) -> Self {
        MCPServerConfig::Http(config)
    }
}

impl From<MCPServerSSE> for MCPServerConfig {
    fn from(config: MCPServerSSE) -> Self {
        MCPServerConfig::Sse(config)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdio_config_new() {
        let config = MCPServerStdio::new("python");
        assert_eq!(config.command, "python");
        assert!(config.args.is_empty());
        assert!(config.env.is_none());
        assert!(config.tool_filter.is_none());
        assert!(!config.cache_tools_list);
    }

    #[test]
    fn test_stdio_config_builder() {
        let mut env = HashMap::new();
        env.insert("API_KEY".to_string(), "secret".to_string());

        let config = MCPServerStdio::new("npx")
            .with_args(vec!["-y".to_string(), "@mcp/server".to_string()])
            .with_env(env.clone())
            .with_cache_tools_list(true);

        assert_eq!(config.command, "npx");
        assert_eq!(config.args.len(), 2);
        assert_eq!(config.env.as_ref().unwrap().get("API_KEY").unwrap(), "secret");
        assert!(config.cache_tools_list);
    }

    #[test]
    fn test_stdio_config_clone() {
        let config = MCPServerStdio::new("node")
            .with_args(vec!["server.js".to_string()]);
        let cloned = config.clone();
        assert_eq!(cloned.command, "node");
        assert_eq!(cloned.args, vec!["server.js"]);
    }

    #[test]
    fn test_stdio_config_debug() {
        let config = MCPServerStdio::new("python");
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("MCPServerStdio"));
        assert!(debug_str.contains("python"));
    }

    #[test]
    fn test_stdio_config_server_identifier() {
        let config = MCPServerStdio::new("python")
            .with_args(vec!["server.py".to_string()]);
        assert_eq!(config.server_identifier(), "stdio:python:server.py");
    }

    #[test]
    fn test_http_config_new() {
        let config = MCPServerHTTP::new("https://example.com/mcp");
        assert_eq!(config.url, "https://example.com/mcp");
        assert!(config.headers.is_none());
        assert!(config.streamable);
        assert!(config.tool_filter.is_none());
        assert!(!config.cache_tools_list);
    }

    #[test]
    fn test_http_config_builder() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());

        let config = MCPServerHTTP::new("https://api.example.com/mcp")
            .with_headers(headers)
            .with_streamable(false)
            .with_cache_tools_list(true);

        assert_eq!(config.url, "https://api.example.com/mcp");
        assert!(config.headers.is_some());
        assert!(!config.streamable);
        assert!(config.cache_tools_list);
    }

    #[test]
    fn test_http_config_server_identifier() {
        let config = MCPServerHTTP::new("https://example.com/mcp");
        assert_eq!(config.server_identifier(), "http:https://example.com/mcp");
    }

    #[test]
    fn test_sse_config_new() {
        let config = MCPServerSSE::new("https://example.com/sse");
        assert_eq!(config.url, "https://example.com/sse");
        assert!(config.headers.is_none());
        assert!(config.tool_filter.is_none());
        assert!(!config.cache_tools_list);
    }

    #[test]
    fn test_sse_config_builder() {
        let config = MCPServerSSE::new("https://example.com/sse")
            .with_cache_tools_list(true);
        assert!(config.cache_tools_list);
    }

    #[test]
    fn test_sse_config_server_identifier() {
        let config = MCPServerSSE::new("https://example.com/sse");
        assert_eq!(config.server_identifier(), "sse:https://example.com/sse");
    }

    #[test]
    fn test_mcp_server_config_enum() {
        let stdio = MCPServerConfig::Stdio(MCPServerStdio::new("python"));
        assert!(!stdio.cache_tools_list());
        assert!(stdio.tool_filter().is_none());
        assert!(stdio.server_identifier().starts_with("stdio:"));

        let http = MCPServerConfig::Http(
            MCPServerHTTP::new("https://example.com").with_cache_tools_list(true),
        );
        assert!(http.cache_tools_list());
        assert!(http.server_identifier().starts_with("http:"));

        let sse = MCPServerConfig::Sse(MCPServerSSE::new("https://example.com/sse"));
        assert!(!sse.cache_tools_list());
        assert!(sse.server_identifier().starts_with("sse:"));
    }

    #[test]
    fn test_mcp_server_config_from_impls() {
        let stdio_config = MCPServerStdio::new("python");
        let config: MCPServerConfig = stdio_config.into();
        assert!(matches!(config, MCPServerConfig::Stdio(_)));

        let http_config = MCPServerHTTP::new("https://example.com");
        let config: MCPServerConfig = http_config.into();
        assert!(matches!(config, MCPServerConfig::Http(_)));

        let sse_config = MCPServerSSE::new("https://example.com/sse");
        let config: MCPServerConfig = sse_config.into();
        assert!(matches!(config, MCPServerConfig::Sse(_)));
    }

    #[test]
    fn test_stdio_config_with_tool_filter() {
        let filter: ArcToolFilter = Arc::new(|tool: &Value| {
            tool.get("name")
                .and_then(|n| n.as_str())
                .map(|name| name.starts_with("allowed_"))
                .unwrap_or(false)
        });

        let config = MCPServerStdio::new("python")
            .with_tool_filter(filter);

        assert!(config.tool_filter.is_some());
        let f = config.tool_filter.as_ref().unwrap();
        assert!(f(&serde_json::json!({"name": "allowed_tool"})));
        assert!(!f(&serde_json::json!({"name": "blocked_tool"})));
    }

    #[test]
    fn test_http_config_serde_roundtrip() {
        let config = MCPServerHTTP::new("https://example.com/mcp")
            .with_streamable(true)
            .with_cache_tools_list(true);

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: MCPServerHTTP = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.url, "https://example.com/mcp");
        assert!(deserialized.streamable);
        assert!(deserialized.cache_tools_list);
        // tool_filter is skipped in serde, so it should be None after roundtrip.
        assert!(deserialized.tool_filter.is_none());
    }

    #[test]
    fn test_stdio_config_serde_roundtrip() {
        let config = MCPServerStdio::new("python")
            .with_args(vec!["server.py".to_string()])
            .with_cache_tools_list(true);

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: MCPServerStdio = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.command, "python");
        assert_eq!(deserialized.args, vec!["server.py"]);
        assert!(deserialized.cache_tools_list);
    }

    #[test]
    fn test_sse_config_serde_roundtrip() {
        let config = MCPServerSSE::new("https://example.com/sse")
            .with_cache_tools_list(true);

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: MCPServerSSE = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.url, "https://example.com/sse");
        assert!(deserialized.cache_tools_list);
    }

    #[test]
    fn test_http_config_debug_masks_headers() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer secret_token".to_string());

        let config = MCPServerHTTP::new("https://example.com")
            .with_headers(headers);

        let debug_str = format!("{:?}", config);
        // The debug output should mask header values.
        assert!(!debug_str.contains("secret_token"));
        assert!(debug_str.contains("Authorization"));
    }
}
