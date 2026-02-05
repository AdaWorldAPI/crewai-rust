# MCP (Model Context Protocol)

MCP integration allows crewAI agents to discover and invoke tools from external MCP-compatible servers. The module provides server configuration types for three transport mechanisms, a client abstraction with retry logic and caching, and both static and dynamic tool filtering.

**Source**: `src/mcp/` (corresponds to Python `crewai/mcp/`)

---

## Architecture Overview

```
Agent
  |
  +-- mcps: Vec<MCPServerConfig>
        |
        +-- MCPServerStdio  --> StdioTransport     --> MCPClient
        +-- MCPServerHTTP   --> HTTPTransport       --> MCPClient
        +-- MCPServerSSE    --> SSETransport        --> MCPClient
                                    |
                                    +-- connect()
                                    +-- list_tools()
                                    +-- call_tool(name, args)
                                    +-- list_prompts()
                                    +-- get_prompt(name, args)
                                    +-- disconnect()
```

### Module Structure

```
src/mcp/
  mod.rs          -- module re-exports
  config.rs       -- MCPServerStdio, MCPServerHTTP, MCPServerSSE, MCPServerConfig
  client.rs       -- MCPClient with retry logic and caching
  filters.rs      -- StaticToolFilter, ToolFilter, DynamicToolFilter
  transports/
    mod.rs        -- BaseTransport trait, TransportType enum
    stdio.rs      -- StdioTransport (local child process)
    http.rs       -- HTTPTransport (HTTP/Streamable HTTP)
    sse.rs        -- SSETransport (Server-Sent Events)
```

---

## Transport Layer

### BaseTransport Trait

All transports implement the `BaseTransport` trait, which uses `#[async_trait]` for async methods:

```rust
use async_trait::async_trait;

#[async_trait]
pub trait BaseTransport: Send + Sync {
    /// Return the transport type.
    fn transport_type(&self) -> TransportType;

    /// Check if transport is currently connected.
    fn connected(&self) -> bool;

    /// Establish connection to the MCP server.
    async fn connect(&mut self) -> Result<(), anyhow::Error>;

    /// Close connection to the MCP server.
    async fn disconnect(&mut self) -> Result<(), anyhow::Error>;

    /// Return a string identifier for this server (for caching and logging).
    fn server_identifier(&self) -> String;
}
```

The `Send + Sync` bounds ensure transports can be owned by `MCPClient` and used across async tasks.

### TransportType Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportType {
    Stdio,          // Local process via stdin/stdout
    Http,           // HTTP/HTTPS (non-streaming)
    StreamableHttp, // HTTP/HTTPS with streaming support
    Sse,            // Server-Sent Events
}
```

`TransportType` implements `Display` and provides `value()` for string conversion and `from_str_opt()` for case-insensitive parsing.

### Concrete Transports

#### StdioTransport

Connects to local MCP servers running as child processes:

```rust
use crewai::mcp::transports::StdioTransport;

let transport = StdioTransport::new(
    "python",                                    // command
    Some(vec!["server.py".to_string()]),          // args
    None,                                        // env vars
);

assert_eq!(transport.transport_type(), TransportType::Stdio);
assert_eq!(transport.server_identifier(), "stdio:python:server.py");
```

#### HTTPTransport

Connects to remote MCP servers over HTTP/HTTPS:

```rust
use crewai::mcp::transports::HTTPTransport;

// Non-streaming HTTP
let transport = HTTPTransport::new(
    "https://api.example.com/mcp",
    None,          // headers
    Some(false),   // streamable = false
);
assert_eq!(transport.transport_type(), TransportType::Http);

// Streamable HTTP (default)
let transport = HTTPTransport::new(
    "https://api.example.com/mcp",
    None,
    Some(true),    // streamable = true
);
assert_eq!(transport.transport_type(), TransportType::StreamableHttp);
```

#### SSETransport

Connects to remote MCP servers using Server-Sent Events:

```rust
use crewai::mcp::transports::SSETransport;

let transport = SSETransport::new(
    "https://api.example.com/sse",
    None,  // headers
);
assert_eq!(transport.transport_type(), TransportType::Sse);
assert_eq!(transport.server_identifier(), "sse:https://api.example.com/sse");
```

---

## Server Configuration

Server configurations are high-level structs that describe how to connect to an MCP server. They are used to construct transports and clients. All configurations support serde serialization (with `tool_filter` skipped via `#[serde(skip)]`).

### MCPServerStdio

```rust
use crewai::mcp::config::MCPServerStdio;
use std::collections::HashMap;

let config = MCPServerStdio::new("python")
    .with_args(vec!["-m".to_string(), "mcp_server".to_string()])
    .with_cache_tools_list(true);

// With environment variables
let mut env = HashMap::new();
env.insert("API_KEY".to_string(), "secret123".to_string());

let config = MCPServerStdio::new("npx")
    .with_args(vec!["-y".to_string(), "@mcp/weather-server".to_string()])
    .with_env(env)
    .with_cache_tools_list(true);
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `command` | `String` | Required | Executable to run (`"python"`, `"node"`, `"npx"`, `"uvx"`) |
| `args` | `Vec<String>` | `[]` | Command arguments |
| `env` | `Option<HashMap<String, String>>` | `None` | Environment variables for the child process |
| `tool_filter` | `Option<ArcToolFilter>` | `None` | Tool filter function (`#[serde(skip)]`) |
| `cache_tools_list` | `bool` | `false` | Cache discovered tools for faster subsequent access |

### MCPServerHTTP

```rust
use crewai::mcp::config::MCPServerHTTP;
use std::collections::HashMap;

let mut headers = HashMap::new();
headers.insert("Authorization".to_string(), "Bearer token123".to_string());

let config = MCPServerHTTP::new("https://api.example.com/mcp")
    .with_headers(headers)
    .with_streamable(true)     // default: true
    .with_cache_tools_list(true);
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String` | Required | Server URL |
| `headers` | `Option<HashMap<String, String>>` | `None` | HTTP headers (e.g., authentication) |
| `streamable` | `bool` | `true` | Use streamable HTTP transport |
| `tool_filter` | `Option<ArcToolFilter>` | `None` | Tool filter function (`#[serde(skip)]`) |
| `cache_tools_list` | `bool` | `false` | Cache discovered tools |

### MCPServerSSE

```rust
use crewai::mcp::config::MCPServerSSE;

let config = MCPServerSSE::new("https://api.example.com/mcp/sse")
    .with_cache_tools_list(true);

// With authentication
let mut headers = HashMap::new();
headers.insert("Authorization".to_string(), "Bearer token".to_string());

let config = MCPServerSSE::new("https://api.example.com/sse")
    .with_headers(headers);
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String` | Required | Server URL |
| `headers` | `Option<HashMap<String, String>>` | `None` | HTTP headers |
| `tool_filter` | `Option<ArcToolFilter>` | `None` | Tool filter function (`#[serde(skip)]`) |
| `cache_tools_list` | `bool` | `false` | Cache discovered tools |

### MCPServerConfig Enum

A union type for any server configuration:

```rust
use crewai::mcp::config::{MCPServerConfig, MCPServerStdio, MCPServerHTTP, MCPServerSSE};

// Create from specific types using From
let config: MCPServerConfig = MCPServerStdio::new("python").into();
let config: MCPServerConfig = MCPServerHTTP::new("https://example.com").into();
let config: MCPServerConfig = MCPServerSSE::new("https://example.com/sse").into();

// Or construct directly
let config = MCPServerConfig::Stdio(MCPServerStdio::new("node"));

// Common methods delegated to the inner type
config.tool_filter();       // &Option<ArcToolFilter>
config.cache_tools_list();  // bool
config.server_identifier(); // String (for logging/caching)
```

---

## MCPClient

The client manages connections to MCP servers and provides a high-level interface for tool discovery, tool execution, and prompt operations.

### Creating a Client

```rust
use crewai::mcp::client::MCPClient;
use crewai::mcp::transports::StdioTransport;

let transport = StdioTransport::new("python", Some(vec!["server.py".into()]), None);
let mut client = MCPClient::new(Box::new(transport))
    .with_connect_timeout(60)
    .with_execution_timeout(120)
    .with_discovery_timeout(45)
    .with_max_retries(5)
    .with_cache_tools_list(true);
```

### Client Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `transport` | `Box<dyn BaseTransport>` | Required | Transport for MCP server communication |
| `connect_timeout` | `u64` | 30 | Connection timeout in seconds |
| `execution_timeout` | `u64` | 30 | Tool execution timeout in seconds |
| `discovery_timeout` | `u64` | 30 | Tool discovery timeout in seconds |
| `max_retries` | `u32` | 3 | Maximum retry attempts |
| `cache_tools_list` | `bool` | `false` | Cache tool list results |

### Connection Lifecycle

```rust
// Connect (auto-connects if needed when calling list_tools/call_tool)
client.connect().await?;
assert!(client.connected());

// Check connection state
let session = client.get_session()?;  // Error if not connected

// Disconnect
client.disconnect().await?;
```

The `connect()` method:
1. Checks if already connected (no-op if so).
2. Tracks whether this is a reconnection attempt.
3. Applies `connect_timeout` via `tokio::time::timeout`.
4. On success, sets `initialized = true` and `was_connected = true`.
5. On failure, calls `cleanup_on_error()` to reset state.

### Tool Operations

#### Listing Tools

```rust
// Discover available tools
let tools = client.list_tools(None).await?;
// Returns Vec<HashMap<String, Value>> with "name", "description", "inputSchema"

// Force cache usage
let tools = client.list_tools(Some(true)).await?;

// Force fresh fetch (bypass cache)
let tools = client.list_tools(Some(false)).await?;
```

#### Calling a Tool

```rust
let mut args = HashMap::new();
args.insert("query".to_string(), serde_json::json!("Rust programming"));

let result = client.call_tool("search", Some(args)).await?;
// Returns String with the tool's text output
```

Both `list_tools` and `call_tool` automatically connect if not already connected, and use the retry mechanism.

### Prompt Operations

```rust
// List available prompts
let prompts = client.list_prompts().await?;
// Returns Vec<HashMap<String, Value>> with "name", "description", "arguments"

// Get a specific prompt
let mut prompt_args = HashMap::new();
prompt_args.insert("topic".to_string(), serde_json::json!("Rust"));

let prompt = client.get_prompt("explain_topic", Some(prompt_args)).await?;
// Returns HashMap<String, Value> with "name", "messages", "arguments"
```

### Argument Cleaning

The client automatically cleans tool arguments before execution:

```rust
let mut args = HashMap::new();
args.insert("query".to_string(), serde_json::json!("test"));
args.insert("unused".to_string(), Value::Null);  // will be removed
args.insert("sources".to_string(), serde_json::json!(["web", "file"]));

let cleaned = MCPClient::clean_tool_arguments(&args);
// "unused" removed (null)
// "sources" converted: ["web", "file"] -> [{"type": "web"}, {"type": "file"}]
```

Cleaning rules:
1. Remove `null` values.
2. Convert `sources` arrays from string elements to `{"type": "..."}` objects.
3. Recursively clean nested objects and arrays.
4. Remove empty objects and arrays after cleaning.

### Retry Logic

The client uses exponential backoff for retryable operations:

```rust
// Retry schedule: 1s, 2s, 4s, ... (2^attempt seconds)
// Default: max 3 attempts

// Non-retryable errors (returned immediately):
// - Authentication failures ("authentication", "unauthorized")
// - Not found errors ("not found")

// Retryable errors:
// - Timeouts
// - Server errors
// - Transient network errors
```

### Schema Cache

The client maintains an in-memory cache with 5-minute TTL:

```rust
// Cache key format: "mcp:{server_identifier}:{resource_type}"
// Example: "mcp:stdio:python:server.py:tools"

// TTL: 300 seconds (5 minutes)
const CACHE_TTL: Duration = Duration::from_secs(300);
```

The cache stores `Vec<HashMap<String, Value>>` entries per resource type (e.g., `"tools"`). Cache entries are checked for expiration on access.

### Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `MCP_CONNECTION_TIMEOUT` | 30 | Connection timeout in seconds |
| `MCP_TOOL_EXECUTION_TIMEOUT` | 30 | Tool execution timeout in seconds |
| `MCP_DISCOVERY_TIMEOUT` | 30 | Tool discovery timeout in seconds |
| `MCP_MAX_RETRIES` | 3 | Maximum retry attempts |
| `CACHE_TTL` | 300s | Schema cache TTL (5 minutes) |

---

## Tool Filtering

### ArcToolFilter (Config-Level)

The `ArcToolFilter` type is used in server configurations. It wraps a filter function in `Arc` for cloneability:

```rust
use crewai::mcp::config::ArcToolFilter;
use std::sync::Arc;

// Only include tools that start with "allowed_"
let filter: ArcToolFilter = Arc::new(|tool: &serde_json::Value| {
    tool.get("name")
        .and_then(|n| n.as_str())
        .map(|name| name.starts_with("allowed_"))
        .unwrap_or(false)
});

let config = MCPServerStdio::new("python")
    .with_tool_filter(filter);
```

### StaticToolFilter

For simple allow/block list filtering based on tool names:

```rust
use crewai::mcp::filters::StaticToolFilter;

// Allow only specific tools
let filter = StaticToolFilter::new(
    Some(vec!["get_weather".to_string(), "search_files".to_string()]),
    None,
);
assert!(filter.filter(&serde_json::json!({"name": "get_weather"})));
assert!(!filter.filter(&serde_json::json!({"name": "delete_files"})));

// Block specific tools (block list takes precedence over allow list)
let filter = StaticToolFilter::new(
    None,
    Some(vec!["dangerous_tool".to_string()]),
);
assert!(filter.filter(&serde_json::json!({"name": "safe_tool"})));
assert!(!filter.filter(&serde_json::json!({"name": "dangerous_tool"})));

// Convert to a boxed ToolFilter function
let tool_filter: ToolFilter = filter.into_tool_filter();
```

The filter fields use `HashSet<String>` for O(1) lookup. Blocked tools take precedence: if a tool appears in both allow and block lists, it is blocked.

### ToolFilter Type

A simple boxed filter function:

```rust
pub type ToolFilter = Box<dyn Fn(&Value) -> bool + Send + Sync>;
```

### DynamicToolFilter

Context-aware filtering with access to agent and server information:

```rust
use crewai::mcp::filters::{
    create_dynamic_tool_filter, DynamicToolFilter, ToolFilterContext,
};

let filter = create_dynamic_tool_filter(|context, tool| {
    // Filter based on agent role
    let agent_role = context.agent.get("role")
        .and_then(|r| r.as_str())
        .unwrap_or("");

    // Only allow write tools for admin agents
    let tool_name = tool.get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("");

    if tool_name.starts_with("write_") {
        return agent_role == "admin";
    }
    true
});

// ToolFilterContext provides:
let context = ToolFilterContext::new(
    serde_json::json!({"role": "admin"}), // agent (serialized)
    "my_server".to_string(),               // server_name
    None,                                  // run_context (optional)
);
```

### Convenience Constructor

```rust
use crewai::mcp::filters::create_static_tool_filter;

let filter = create_static_tool_filter(
    Some(vec!["tool_a".to_string(), "tool_b".to_string()]),
    None,
);
```

---

## Server Identification

Each server config provides a unique identifier used for caching and logging:

```rust
let stdio = MCPServerStdio::new("python")
    .with_args(vec!["server.py".to_string()]);
assert_eq!(stdio.server_identifier(), "stdio:python:server.py");

let http = MCPServerHTTP::new("https://example.com/mcp");
assert_eq!(http.server_identifier(), "http:https://example.com/mcp");

let sse = MCPServerSSE::new("https://example.com/sse");
assert_eq!(sse.server_identifier(), "sse:https://example.com/sse");
```

---

## Debug Security

HTTP and SSE configurations mask header values in debug output to prevent accidental credential exposure:

```rust
let mut headers = HashMap::new();
headers.insert("Authorization".to_string(), "Bearer secret_token".to_string());

let config = MCPServerHTTP::new("https://example.com")
    .with_headers(headers);

println!("{:?}", config);
// Output: MCPServerHTTP { url: "https://example.com",
//   headers: Some(["Authorization=<masked>"]), ... }
// The value "Bearer secret_token" is never shown.
```

---

## Serialization

All server configs support serde serialization. The `tool_filter` field is annotated with `#[serde(skip)]` since function pointers cannot be serialized:

```rust
let config = MCPServerHTTP::new("https://example.com/mcp")
    .with_cache_tools_list(true);

let json = serde_json::to_string(&config)?;
let deserialized: MCPServerHTTP = serde_json::from_str(&json)?;

assert_eq!(deserialized.url, "https://example.com/mcp");
assert!(deserialized.cache_tools_list);
assert!(deserialized.tool_filter.is_none()); // always None after deserialization
```

---

## Python vs. Rust Comparison

| Aspect | Python | Rust |
|--------|--------|------|
| Config classes | Pydantic `BaseModel` | `struct` with `Serialize`/`Deserialize` |
| Tool filter | `Callable[[dict], bool]` | `Arc<dyn Fn(&Value) -> bool + Send + Sync>` |
| Transport trait | `ABC` | `#[async_trait] trait BaseTransport: Send + Sync` |
| Client session | `ClientSession` from MCP SDK | `Option<Value>` (pending SDK integration) |
| Error handling | Raises exceptions | Returns `Result<T, anyhow::Error>` |
| Retry | `tenacity` or manual | Custom `retry_operation()` with exponential backoff |
| Caching | Manual dict with timestamps | `Arc<Mutex<HashMap<String, CacheEntry>>>` |
| Secret masking | Custom `__repr__` | Custom `fmt::Debug` implementation |
| Union type | `MCPServerConfig = Type1 \| Type2 \| Type3` | `enum MCPServerConfig { Stdio, Http, Sse }` |
| Clone | Automatic (Pydantic) | Manual `Clone` impl (to handle `Arc` filter) |

---

## Complete Example

```rust
use crewai::mcp::config::{MCPServerStdio, MCPServerHTTP, MCPServerConfig, ArcToolFilter};
use crewai::mcp::client::MCPClient;
use crewai::mcp::transports::StdioTransport;
use crewai::mcp::filters::StaticToolFilter;
use std::sync::Arc;
use std::collections::HashMap;

// 1. Create a static tool filter
let filter = StaticToolFilter::new(
    Some(vec!["search".to_string(), "calculate".to_string()]),
    None,
);
let arc_filter: ArcToolFilter = Arc::new(move |tool| filter.filter(tool));

// 2. Configure a stdio server
let stdio_config = MCPServerStdio::new("python")
    .with_args(vec!["-m".to_string(), "my_mcp_server".to_string()])
    .with_tool_filter(arc_filter)
    .with_cache_tools_list(true);

// 3. Configure an HTTP server
let mut headers = HashMap::new();
headers.insert("Authorization".to_string(), "Bearer my-token".to_string());

let http_config = MCPServerHTTP::new("https://tools.example.com/mcp")
    .with_headers(headers)
    .with_cache_tools_list(true);

// 4. Store as MCPServerConfig for polymorphic usage
let configs: Vec<MCPServerConfig> = vec![
    stdio_config.into(),
    http_config.into(),
];

// 5. Create a client from a transport
let transport = StdioTransport::new(
    "python",
    Some(vec!["-m".to_string(), "my_mcp_server".to_string()]),
    None,
);
let mut client = MCPClient::new(Box::new(transport))
    .with_connect_timeout(60)
    .with_max_retries(5)
    .with_cache_tools_list(true);

// 6. Use the client (async context required)
// client.connect().await?;
// let tools = client.list_tools(None).await?;
// let result = client.call_tool("search", Some(args)).await?;
// client.disconnect().await?;
```

---

## Implementation Status

- Server configuration types and MCPServerConfig enum: **complete**
- Tool filtering (static, dynamic, arc-based): **complete**
- Serialization with secret masking: **complete**
- Transport trait and concrete transports (struct/methods): **complete**
- MCPClient with retry logic, caching, and argument cleaning: **complete**
- Transport I/O (actual stdio/HTTP/SSE communication): **stub** -- requires MCP SDK binding
- MCP session management (initialize, list_tools, call_tool): **stub** -- pending SDK integration
- Event emission (MCPConnectionStarted/Completed/Failed, MCPToolExecution*): **stub** -- log-level only

See the [Technical Debt Report](../../TECHNICAL_DEBT.md) for details on remaining integration work.
