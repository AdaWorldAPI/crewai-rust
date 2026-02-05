# Building Custom Tools

This guide covers the three approaches for creating custom tools in crewAI-rust: implementing the `BaseTool` trait directly, using the `Tool` wrapper struct, and using `CrewStructuredTool::from_function`.

**Source**: `src/tools/base_tool.rs`, `src/tools/tool_types.rs`, `src/tools/structured_tool.rs`

---

## Approach 1: Implement the `BaseTool` Trait

For full control over tool behavior, implement the `BaseTool` trait directly:

```rust
use async_trait::async_trait;
use crewai::tools::base_tool::{BaseTool, EnvVar};
use crewai::tools::structured_tool::CrewStructuredTool;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug)]
struct WeatherTool {
    name: String,
    description: String,
    usage_count: u32,
    max_usage: Option<u32>,
}

impl WeatherTool {
    fn new() -> Self {
        Self {
            name: "get_weather".to_string(),
            description: "Get the current weather for a location".to_string(),
            usage_count: 0,
            max_usage: Some(20),
        }
    }
}

#[async_trait]
impl BaseTool for WeatherTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn args_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name or coordinates"
                },
                "units": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "Temperature units"
                }
            },
            "required": ["location"]
        })
    }

    fn env_vars(&self) -> &[EnvVar] {
        &[]  // No required environment variables
    }

    fn result_as_answer(&self) -> bool {
        false  // Tool result is not the final answer
    }

    fn max_usage_count(&self) -> Option<u32> {
        self.max_usage
    }

    fn current_usage_count(&self) -> u32 {
        self.usage_count
    }

    fn increment_usage_count(&mut self) {
        self.usage_count += 1;
    }

    fn reset_usage_count(&mut self) {
        self.usage_count = 0;
    }

    fn has_reached_max_usage_count(&self) -> bool {
        self.max_usage
            .map(|max| self.usage_count >= max)
            .unwrap_or(false)
    }

    fn should_cache(&self, _args: &Value, _result: &Value) -> bool {
        true  // Cache all results
    }

    fn run(
        &mut self,
        args: HashMap<String, Value>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let location = args
            .get("location")
            .and_then(|v| v.as_str())
            .ok_or("Missing required argument: location")?;

        let units = args
            .get("units")
            .and_then(|v| v.as_str())
            .unwrap_or("celsius");

        // In a real implementation, call a weather API here
        Ok(serde_json::json!({
            "location": location,
            "temperature": 22,
            "units": units,
            "conditions": "partly cloudy"
        }))
    }

    async fn arun(
        &mut self,
        args: HashMap<String, Value>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        // For async, you could use reqwest to call an actual API
        self.run(args)
    }

    fn to_structured_tool(&self) -> CrewStructuredTool {
        // Convert to a structured tool for the ToolUsage lifecycle
        CrewStructuredTool::new(
            self.name(),
            self.description(),
            self.args_schema(),
            std::sync::Arc::new(|args| {
                let location = args
                    .get("location")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                Ok(serde_json::json!({
                    "location": location,
                    "temperature": 22,
                    "conditions": "partly cloudy"
                }))
            }),
        )
    }
}
```

## Approach 2: Use the `Tool` Wrapper

For simpler tools that just wrap a function:

```rust
use crewai::tools::Tool;
use crewai::tools::base_tool::EnvVar;
use std::sync::Arc;

// Basic tool
let calculator = Tool::new(
    "calculator",
    "Evaluate a mathematical expression",
    Arc::new(|args| {
        let expression = args
            .get("expression")
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        // In practice, use a math expression parser
        Ok(serde_json::json!({"expression": expression, "result": "42"}))
    }),
);

// With full configuration
let api_tool = Tool::new(
    "fetch_api",
    "Fetch data from a REST API endpoint",
    Arc::new(|args| {
        let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
        Ok(serde_json::json!({"status": 200, "data": "response data"}))
    }),
)
.with_args_schema(serde_json::json!({
    "type": "object",
    "properties": {
        "url": {
            "type": "string",
            "description": "The API endpoint URL"
        },
        "method": {
            "type": "string",
            "enum": ["GET", "POST"],
            "description": "HTTP method"
        }
    },
    "required": ["url"]
}))
.with_env_vars(vec![
    EnvVar::new("API_TOKEN", "Authentication token for the API"),
])
.with_max_usage_count(Some(100))
.with_result_as_answer(false);
```

## Approach 3: Use `CrewStructuredTool::from_function`

The quickest way to create a tool:

```rust
use crewai::tools::CrewStructuredTool;
use std::sync::Arc;

let tool = CrewStructuredTool::from_function(
    "summarize",
    "Summarize a block of text",
    Arc::new(|args| {
        let text = args
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let max_words: usize = args
            .get("max_words")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;

        let words: Vec<&str> = text.split_whitespace().take(max_words).collect();
        Ok(serde_json::json!(words.join(" ")))
    }),
);

// With a custom schema
let tool = CrewStructuredTool::new(
    "database_query",
    "Execute a read-only SQL query",
    serde_json::json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "SQL SELECT query to execute"
            },
            "database": {
                "type": "string",
                "description": "Database name"
            }
        },
        "required": ["query", "database"]
    }),
    Arc::new(|args| {
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let db = args.get("database").and_then(|v| v.as_str()).unwrap_or("default");
        Ok(serde_json::json!({
            "database": db,
            "query": query,
            "rows": [],
            "count": 0
        }))
    }),
);
```

## Tool Result Handling

### ToolResult Type

The `ToolResult` struct carries the output and a flag indicating whether it should be the agent's final answer:

```rust
use crewai::tools::ToolResult;

// Normal result -- agent continues reasoning
let result = ToolResult::new("Found 5 matching documents");

// Final answer -- agent stops and returns this
let result = ToolResult::as_answer("The capital of France is Paris.");

// Check the flag
if result.result_as_answer {
    println!("This is a final answer: {}", result.output);
} else {
    println!("Tool output (agent continues): {}", result.output);
}
```

### Returning ToolResult from `run()`

The `run()` method returns `Result<Value, Error>`. To signal "result as answer", wrap the response:

```rust
fn run(
    &mut self,
    args: HashMap<String, Value>,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let answer = "42";
    Ok(serde_json::json!({
        "output": answer,
        "result_as_answer": true
    }))
}
```

## Tool Caching

By default, tools cache their results. Control caching behavior:

```rust
// In BaseTool implementation:
fn should_cache(&self, args: &Value, result: &Value) -> bool {
    // Don't cache errors
    if result.get("error").is_some() {
        return false;
    }
    // Don't cache empty results
    if result.as_str().map(|s| s.is_empty()).unwrap_or(false) {
        return false;
    }
    true
}
```

The cache key format is: `tool:{tool_name}|input:{arguments_json}`.

When a crew has `cache = true`, the `CacheTools` helper creates a tool that agents can use to look up previously cached results.

## Tool Usage Limits

Enforce a maximum number of invocations per tool:

```rust
let tool = Tool::new("expensive_api", "Premium API call", func)
    .with_max_usage_count(Some(10));

// The BaseTool trait tracks usage:
// - current_usage_count() -> u32
// - increment_usage_count()
// - reset_usage_count()
// - has_reached_max_usage_count() -> bool

// When the limit is reached, ToolUsage returns a
// "tool has reached its max usage count" error
```

## Environment Variables

Declare required environment variables:

```rust
use crewai::tools::base_tool::EnvVar;

// Required variable
let var = EnvVar::new(
    "OPENWEATHER_API_KEY",
    "API key for OpenWeatherMap",
);

// Optional variable with a default
let var = EnvVar::with_default(
    "WEATHER_CACHE_TTL",
    "Cache TTL in seconds",
    "300",
);

// Attach to a tool
let tool = Tool::new("weather", "Get weather data", func)
    .with_env_vars(vec![
        EnvVar::new("OPENWEATHER_API_KEY", "API key"),
    ]);
```

The tool system checks that required environment variables are set before executing the tool. Missing variables produce a descriptive error message.

## Registering Tools with Agents

```rust
// By tool name (agent resolves at runtime)
let mut agent = Agent::new(
    "Weather Reporter".to_string(),
    "Report weather conditions".to_string(),
    "Expert meteorologist".to_string(),
);
agent.tools = vec!["get_weather".to_string(), "calculator".to_string()];

// Task-specific tools
let mut task = Task::new(
    "Get the weather in Tokyo".to_string(),
    "Current temperature and conditions".to_string(),
);
task.tools = vec!["get_weather".to_string()];
```

## Complete Example

```rust
use crewai::tools::{Tool, CrewStructuredTool};
use crewai::tools::base_tool::EnvVar;
use std::sync::Arc;
use std::collections::HashMap;

// Create a custom file search tool
let file_search = Tool::new(
    "file_search",
    "Search for files matching a pattern in a directory",
    Arc::new(|args| {
        let pattern = args.get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("*");
        let directory = args.get("directory")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        // In a real implementation, use std::fs or glob
        Ok(serde_json::json!({
            "pattern": pattern,
            "directory": directory,
            "matches": ["file1.txt", "file2.txt"],
            "count": 2
        }))
    }),
)
.with_args_schema(serde_json::json!({
    "type": "object",
    "properties": {
        "pattern": {
            "type": "string",
            "description": "Glob pattern to match (e.g., *.rs)"
        },
        "directory": {
            "type": "string",
            "description": "Directory to search in"
        }
    },
    "required": ["pattern"]
}))
.with_max_usage_count(Some(50));

// Convert to structured tool for the ToolUsage lifecycle
let structured = file_search.to_structured_tool();

// Invoke directly
let result = structured.invoke(serde_json::json!({
    "pattern": "*.rs",
    "directory": "/src"
}))?;

println!("Found: {}", result);
```
