# Tools

Tools are the mechanism by which crewAI agents interact with the outside world -- searching the web, reading files, calling APIs, executing code, and more. The Rust port replaces Python's class-based inheritance hierarchy with a trait-based design built on `async_trait`, `serde_json::Value` for dynamic arguments, and `Arc`-based function pointers for thread-safe sharing across concurrent agents.

**Source**: `src/tools/` (corresponds to Python `crewai/tools/`)

---

## Architecture Overview

```
BaseTool (trait)                          -- The contract every tool satisfies
  |
  +-- Tool (struct)                       -- Wraps a plain Fn as a tool
  +-- MCPNativeTool                       -- MCP server tool via persistent session
  +-- MCPToolWrapper                      -- MCP server tool with on-demand connection
  +-- DelegateWorkTool / AskQuestionTool  -- Agent collaboration tools
  |
  +-- to_structured_tool() -----> CrewStructuredTool
                                    |
                                ToolUsage (lifecycle manager)
                                    |   parse -> select -> validate -> execute -> cache
                                    |
                                CacheTools (cache reader tool)
```

### Type Summary

| Rust Type | Python Equivalent | Purpose |
|-----------|-------------------|---------|
| `BaseTool` trait | `BaseTool` ABC | Contract every tool must satisfy |
| `Tool` struct | `Tool` class | Wraps a plain function as a named tool |
| `CrewStructuredTool` | `CrewStructuredTool` | Schema-validated tool with argument parsing |
| `ToolResult` | `ToolResult` | Carries output and `result_as_answer` flag |
| `ToolCalling` | `ToolCalling` | Parsed LLM tool-call intent (name + args) |
| `ToolUsage` | `ToolUsage` | Full lifecycle: parse, select, validate, execute, cache |
| `CacheTools` | `CacheTools` | Reads from the agent's tool-result cache |
| `EnvVar` | N/A (new in Rust) | Declares required or optional environment variables |

All public types are re-exported from the `crewai::tools` module:

```rust
use crewai::tools::{
    BaseTool, Tool, CrewStructuredTool, ToolResult,
    ToolCalling, ToolUsage, CacheTools, EnvVar,
};
```

---

## The `BaseTool` Trait

Every tool in crewAI must implement `BaseTool`. This is the Rust equivalent of Python's `BaseTool` abstract base class, but expressed as a trait with `Send + Sync` bounds so tools can be safely shared across async tasks and threads.

```rust
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

#[async_trait]
pub trait BaseTool: Send + Sync + fmt::Debug {
    /// Unique name that clearly communicates what the tool does.
    fn name(&self) -> &str;

    /// Description the LLM reads to decide when/how to use this tool.
    fn description(&self) -> &str;

    /// JSON Schema defining the arguments the tool accepts.
    /// Default: empty object `{}`.
    fn args_schema(&self) -> Value { Value::Object(Default::default()) }

    /// Environment variables required or optionally used by the tool.
    fn env_vars(&self) -> &[EnvVar] { &[] }

    /// If true, the tool's output becomes the agent's final answer
    /// (no further LLM reasoning).
    fn result_as_answer(&self) -> bool { false }

    /// Optional maximum number of invocations. None = unlimited.
    fn max_usage_count(&self) -> Option<u32> { None }

    /// Current invocation count.
    fn current_usage_count(&self) -> u32;
    fn increment_usage_count(&mut self);
    fn reset_usage_count(&mut self);

    /// Check if the tool has reached its usage limit.
    fn has_reached_max_usage_count(&self) -> bool {
        self.max_usage_count()
            .map(|max| self.current_usage_count() >= max)
            .unwrap_or(false)
    }

    /// Cache predicate: should the result be cached?
    fn should_cache(&self, _args: &Value, _result: &Value) -> bool { true }

    /// Synchronous execution -- the primary entry point.
    fn run(
        &mut self,
        args: HashMap<String, Value>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;

    /// Asynchronous execution. Default delegates to `run()`.
    async fn arun(
        &mut self,
        args: HashMap<String, Value>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        self.run(args)
    }

    /// Convert this tool into a CrewStructuredTool for schema-based invocation.
    fn to_structured_tool(&self) -> CrewStructuredTool;
}
```

### Key Rust Differences from Python

| Aspect | Python | Rust |
|--------|--------|------|
| **Concurrency safety** | GIL protects shared state | `Send + Sync` trait bounds enforced at compile time |
| **Mutability** | Pydantic models freely mutable | `&mut self` on `run` -- usage counter mutation is explicit |
| **Async** | `async def arun(...)` | `async_trait` macro (Rust traits cannot have native `async fn`) |
| **Arguments** | `dict[str, Any]` | `HashMap<String, serde_json::Value>` -- strongly typed JSON |
| **Inheritance** | `class MyTool(BaseTool)` | `impl BaseTool for MyTool` -- composition over inheritance |
| **Schema** | Pydantic `model_json_schema()` | Explicit `serde_json::json!({...})` or `#[derive(JsonSchema)]` |

---

## The `Tool` Struct

`Tool` is the most common way to create a tool. It wraps a function pointer stored as `Arc<dyn Fn(HashMap<String, Value>) -> Result<Value, ...> + Send + Sync>` with metadata.

```rust
use std::sync::Arc;
use crewai::tools::{Tool, EnvVar};

let search_tool = Tool::new(
    "web_search",
    "Search the web for current information on any topic",
    Arc::new(|args| {
        let query = args.get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        // ... perform search ...
        Ok(serde_json::json!({ "results": ["result1", "result2"] }))
    }),
)
.with_args_schema(serde_json::json!({
    "type": "object",
    "properties": {
        "query": {
            "type": "string",
            "description": "The search query to execute"
        }
    },
    "required": ["query"]
}))
.with_env_vars(vec![
    EnvVar::new("SEARCH_API_KEY", "API key for the search provider"),
])
.with_max_usage_count(Some(10));
```

### Builder Methods

| Method | Purpose |
|--------|---------|
| `with_args_schema(Value)` | Set the JSON Schema for input arguments |
| `with_env_vars(Vec<EnvVar>)` | Declare environment variable requirements |
| `with_result_as_answer(bool)` | Mark output as the agent's final answer |
| `with_max_usage_count(Option<u32>)` | Limit how many times the tool can be called per crew run |

### Thread Safety

The function pointer inside `Tool` is wrapped in `Arc<dyn Fn(...) + Send + Sync>`, making it safely shareable across threads and async tasks. This is critical because crewAI agents may execute concurrently.

---

## `CrewStructuredTool`

`CrewStructuredTool` is the runtime representation of a tool that the `ToolUsage` lifecycle manager works with. It wraps a callable function with a name, description, schema, and usage tracking. It replaces LangChain's `StructuredTool` from the Python codebase.

```rust
use crewai::tools::CrewStructuredTool;
use std::sync::Arc;

// Create with full configuration
let tool = CrewStructuredTool::new(
    "calculator",
    "Evaluate a mathematical expression and return the numeric result",
    serde_json::json!({
        "type": "object",
        "properties": {
            "expression": {
                "type": "string",
                "description": "Mathematical expression to evaluate"
            }
        },
        "required": ["expression"]
    }),
    Arc::new(|args| {
        let expr = args.get("expression")
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        // ... evaluate expression ...
        Ok(serde_json::json!(42))
    }),
);

// Create from function (minimal configuration)
let tool = CrewStructuredTool::from_function(
    "greet",
    "Generate a greeting message",
    Arc::new(|args| {
        let name = args.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("World");
        Ok(serde_json::json!(format!("Hello, {}!", name)))
    }),
);
```

### Invocation

```rust
// Synchronous invocation with a JSON object
let result = tool.invoke(serde_json::json!({"expression": "6 * 7"}))?;

// Asynchronous invocation
let result = tool.ainvoke(serde_json::json!({"expression": "6 * 7"})).await?;

// Parse and validate arguments from a JSON value
let args = tool.parse_args(serde_json::json!({"key": "value"}))?;
```

The `invoke` / `ainvoke` methods handle:
1. Parsing arguments (accepts JSON object or JSON string that gets parsed)
2. Checking usage limits (returns `ToolUsageLimitExceededError` if exceeded)
3. Incrementing the usage counter
4. Executing the function and returning the result

---

## `ToolResult`

`ToolResult` wraps a tool's output string with an optional `result_as_answer` flag. When `result_as_answer` is `true`, the agent skips further reasoning and returns the tool's output directly as its final answer.

```rust
use crewai::tools::ToolResult;

// Normal result -- agent continues reasoning
let result = ToolResult::new("Found 3 matching documents");

// Result that becomes the agent's final answer
let final_answer = ToolResult::as_answer("The capital of France is Paris.");

// Implicit conversion from strings
let result: ToolResult = "some output".into();
let result: ToolResult = String::from("some output").into();
```

`ToolResult` derives `Serialize`, `Deserialize`, `Clone`, `Debug`, and implements `Display` and `From<&str>` / `From<String>`.

---

## `ToolCalling`

Represents the LLM's intent to invoke a specific tool with a set of arguments. When the LLM outputs a tool-use block, `ToolUsage` parses it into a `ToolCalling` instance.

```rust
use crewai::tools::ToolCalling;
use std::collections::HashMap;

let calling = ToolCalling::new(
    "web_search",
    Some(HashMap::from([
        ("query".to_string(), serde_json::json!("Rust async programming")),
    ])),
);

assert_eq!(calling.tool_name, "web_search");
assert!(calling.arguments.is_some());
```

The type alias `InstructorToolCalling` is identical to `ToolCalling` in Rust (both map to the same struct).

---

## `ToolUsage` -- the Full Lifecycle

`ToolUsage` manages the complete tool execution lifecycle for an agent. This is the heart of the tools system.

### Lifecycle Steps

```
LLM Output -> parse -> select -> validate -> execute -> cache -> emit
```

1. **Parse**: Extract tool name and arguments from the LLM's raw text or JSON output
2. **Select**: Find the matching tool by name (exact match first, then fuzzy match with > 0.85 similarity threshold)
3. **Validate**: Check arguments against the tool's JSON Schema
4. **Execute**: Invoke the tool synchronously or asynchronously
5. **Cache**: Store the result in the `CacheHandler` to avoid re-executing identical calls
6. **Emit**: Log tool execution details including timing

```rust
use crewai::tools::ToolUsage;

let tools = vec![/* CrewStructuredTool instances */];
let mut usage = ToolUsage::new(tools, Some(cache_handler), Some("gpt-4o"));
usage.verbose = true;
usage.agent_role = Some("Researcher".to_string());

// Parse a tool call from LLM output
let calling = usage.parse_tool_calling(
    r#"{"tool_name": "search", "arguments": {"query": "rust"}}"#
)?;

// Execute the tool (sync)
let result = usage.use_tool(&calling, "raw output");

// Execute the tool (async)
let result = usage.ause_tool(&calling, "raw output").await;
```

### Fuzzy Tool Selection

When the LLM misspells a tool name, `ToolUsage` uses a character-level longest-common-subsequence ratio (mirroring Python's `SequenceMatcher.ratio()`) to find the closest match above a 0.85 threshold. If found, it proceeds with the best match rather than failing.

### Repeated-Usage Detection

If the same tool is called with identical arguments consecutively, `ToolUsage` returns a message asking the agent to try a different approach rather than re-executing.

### Configuration

| Field | Type | Description |
|-------|------|-------------|
| `max_parsing_attempts` | `usize` | Max retries for parsing (2 for large OpenAI models, 3 otherwise) |
| `remember_format_after_usages` | `usize` | Remind agent of correct format every N tool uses |
| `verbose` | `bool` | Print tool execution details to stdout |
| `fingerprint_context` | `Option<String>` | Security metadata added to tool arguments |

---

## `EnvVar`

Declares an environment variable that a tool requires or optionally uses. This is a Rust addition that makes tool dependencies explicit at the type level.

```rust
use crewai::tools::base_tool::EnvVar;

// Required environment variable
let required = EnvVar::new("API_KEY", "Authentication key for the API");

// Optional environment variable with a default
let optional = EnvVar::with_default(
    "TIMEOUT",
    "Request timeout in seconds",
    "30",
);
```

---

## `CacheTools`

Creates a structured tool that reads from the agent's tool-result cache, enabling agents to re-read previously computed results without re-executing the original tool.

```rust
use crewai::tools::CacheTools;
use crewai::agents::cache::CacheHandler;

let handler = CacheHandler::new();
let cache_tools = CacheTools::new(handler);
let tool = cache_tools.tool();

// The tool accepts keys in the format: "tool:{name}|input:{input}"
let result = tool.invoke(serde_json::json!({
    "key": "tool:web_search|input:query=rust programming"
}))?;
```

---

## MCP Tool Integration

Tools discovered via MCP (Model Context Protocol) servers are automatically wrapped as `BaseTool` implementations.

### `MCPNativeTool`

Reuses an existing MCP client session for persistent connections. The tool name is prefixed with the server name to avoid collisions.

```rust
use crewai::tools::mcp_native_tool::MCPNativeTool;

let tool = MCPNativeTool::new(
    mcp_client_session,    // Box<dyn Any + Send + Sync>
    "get_weather",         // original tool name on the MCP server
    &tool_schema_json,     // JSON schema from MCP tool discovery
    "weather_server",      // server name (used as prefix)
);
// tool.name() returns "weather_server_get_weather"
```

### `MCPToolWrapper`

Wraps an MCP server configuration for on-demand connections. Each invocation opens a connection, executes the tool, and closes the connection. Suitable for infrequent tool usage where persistent connections are not needed.

---

## Built-in Agent Tools

| Tool | Module | Description |
|------|--------|-------------|
| `DelegateWorkTool` | `agent_tools/delegate_work_tool.rs` | Delegate a task to a co-worker agent |
| `AskQuestionTool` | `agent_tools/ask_question_tool.rs` | Ask a question to a co-worker agent |
| `AddImageTool` | `agent_tools/add_image_tool.rs` | Add an image to the agent's context |
| `ReadFileTool` | `agent_tools/read_file_tool.rs` | Read a file from the filesystem |

---

## Converting Between Tool Types

```rust
use crewai::tools::base_tool::to_structured_tools;

// Convert a slice of trait objects to structured tools
let trait_objects: Vec<Box<dyn BaseTool>> = vec![/* ... */];
let structured: Vec<CrewStructuredTool> = to_structured_tools(&trait_objects);

// Convert a single tool
let structured = my_tool.to_structured_tool();
```

---

## Error Types

| Error | When |
|-------|------|
| `ToolUsageLimitExceededError` | Tool has reached its `max_usage_count` |
| `ToolUsageError` | Parsing, selection, or execution failure |

Both implement `std::error::Error`, `Display`, and `Debug`.

---

## Usage Limits

```rust
let tool = Tool::new("expensive_api", "Calls a paid API", func)
    .with_max_usage_count(Some(5));

// After 5 invocations, further calls return ToolUsageLimitExceededError
if tool.has_reached_max_usage_count() {
    println!("Tool {} has reached its limit", tool.name());
}
```

---

## Next Steps

- [Custom Tools Guide](../guides/custom-tools.md) -- Step-by-step walkthrough for building custom tools
- [Tool Framework Overview](../tools/overview.md) -- High-level overview and built-in tools catalog
- [MCP Integration](mcp.md) -- Tools exposed via Model Context Protocol servers
