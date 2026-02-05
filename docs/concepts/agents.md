# Agents

An **Agent** is an autonomous unit in the crewAI system. Each agent has a role, a goal, and a backstory that define its persona and behavior. Agents can use tools, access memory, delegate work to other agents, and execute tasks assigned to them by a crew.

**Source**: `src/agent/core.rs` (corresponds to Python `crewai/agent/core.py`)

---

## Creating an Agent

Use `Agent::new()` with the three required fields:

```rust
use crewai::Agent;

let agent = Agent::new(
    "Senior Research Analyst".to_string(),
    "Uncover cutting-edge developments in AI".to_string(),
    "You are a senior research analyst at a leading tech think tank. \
     Your expertise lies in identifying emerging trends."
        .to_string(),
);
```

## Agent Struct Fields

### Core Identity

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `Uuid` | Auto-generated | Unique identifier |
| `role` | `String` | Required | The agent's role (e.g., "Senior Researcher") |
| `goal` | `String` | Required | The agent's objective |
| `backstory` | `String` | Required | Background context shaping behavior |

### Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `llm` | `Option<String>` | `None` | LLM model identifier (e.g., "gpt-4o") |
| `verbose` | `bool` | `false` | Enable detailed logging |
| `cache` | `bool` | `true` | Cache tool results |
| `max_iter` | `i32` | `25` | Maximum iterations per task |
| `max_tokens` | `Option<i32>` | `None` | Max tokens per LLM response |
| `max_rpm` | `Option<i32>` | `None` | Rate limit (requests per minute) |
| `max_execution_time` | `Option<i64>` | `None` | Timeout in seconds |
| `max_retry_limit` | `i32` | `2` | Retries on error |
| `config` | `Option<HashMap<String, Value>>` | `None` | Custom configuration |

### Capabilities

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `tools` | `Vec<String>` | `[]` | Tool names available to the agent |
| `allow_delegation` | `bool` | `false` | Can delegate to other agents |
| `allow_code_execution` | `bool` | `false` | Can execute code |
| `code_execution_mode` | `CodeExecutionMode` | `Safe` | Docker (`Safe`) or direct (`Unsafe`) |
| `multimodal` | `bool` | `false` | Supports image/audio inputs |
| `reasoning` | `bool` | `false` | Enable planning before execution |
| `max_reasoning_attempts` | `Option<i32>` | `None` | Max reasoning iterations |

### Templates

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `use_system_prompt` | `bool` | `true` | Include system prompt |
| `system_template` | `Option<String>` | `None` | Custom system prompt template |
| `prompt_template` | `Option<String>` | `None` | Custom prompt template |
| `response_template` | `Option<String>` | `None` | Custom response template |

### LLM Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `function_calling_llm` | `Option<String>` | `None` | Separate LLM for tool calling |
| `embedder` | `Option<HashMap<String, Value>>` | `None` | Embedder config for knowledge |

### Knowledge and Memory

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `knowledge_sources` | `Option<Vec<HashMap<String, Value>>>` | `None` | Knowledge source configs |
| `knowledge_storage` | `Option<Value>` | `None` | Knowledge storage config |
| `knowledge_config` | `Option<HashMap<String, Value>>` | `None` | Query behavior settings |

### Integration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mcps` | `Option<Vec<String>>` | `None` | MCP server references |
| `apps` | `Option<Vec<String>>` | `None` | Platform app references |
| `a2a` | `Option<Value>` | `None` | Agent-to-Agent config |

### Guardrails

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `guardrail` | `Option<String>` | `None` | Output validation description |
| `guardrail_max_retries` | `i32` | `3` | Retries on guardrail failure |

### Date Injection

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `inject_date` | `bool` | `false` | Auto-inject current date |
| `date_format` | `String` | `"%Y-%m-%d"` | Date format string |

### Security

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `security_config` | `SecurityConfig` | Default | Fingerprinting configuration |

### Callbacks (non-serialized)

| Field | Type | Description |
|-------|------|-------------|
| `step_callback` | `Option<StepCallback>` | Called after each execution step |

## Configuring Agents

```rust
let mut agent = Agent::new(
    "Data Analyst".to_string(),
    "Analyze datasets and extract insights".to_string(),
    "Expert data analyst with 10 years of experience".to_string(),
);

// Set LLM
agent.llm = Some("gpt-4o".to_string());

// Enable verbose logging
agent.verbose = true;

// Add tools
agent.tools = vec![
    "search_web".to_string(),
    "read_file".to_string(),
];

// Allow delegation to other agents
agent.allow_delegation = true;

// Set execution limits
agent.max_iter = 15;
agent.max_execution_time = Some(120);

// Enable reasoning
agent.reasoning = true;
agent.max_reasoning_attempts = Some(5);
```

## Agent Execution

### Execute a Task

```rust
let result = agent.execute_task(
    "Analyze Q4 revenue data and identify trends",
    Some("Revenue data: ..."),   // optional context
    Some(&["search_web".to_string()]),  // optional tools
)?;
```

### Standalone Kickoff

Agents can run independently without a crew:

```rust
let result = agent.kickoff("What are the latest trends in AI?")?;
```

### Async Execution

```rust
let result = agent.aexecute_task(
    "Research quantum computing advances",
    None,
    None,
).await?;
```

## Agent Delegation

When `allow_delegation` is set to `true`, the agent receives delegation tools that let it pass work to co-worker agents:

```rust
let agents = vec![analyst.clone(), writer.clone()];
let delegation_tools = researcher.get_delegation_tools(&agents);
// Returns: ["Delegate work to co-worker 'Data Analyst'",
//           "Ask question to co-worker 'Data Analyst'",
//           "Delegate work to co-worker 'Writer'",
//           "Ask question to co-worker 'Writer'"]
```

The delegation tools are:
- **DelegateWorkTool** -- Assign a task to a co-worker
- **AskQuestionTool** -- Ask a specific question to a co-worker

## Agent Tools Integration

### MCP Tools

Agents can discover and use tools from MCP servers:

```rust
agent.mcps = Some(vec![
    "https://mcp-server.example.com".to_string(),
]);
let mcp_tools = agent.get_mcp_tools(agent.mcps.as_deref().unwrap_or(&[]));
```

### Code Execution

```rust
agent.allow_code_execution = true;
agent.code_execution_mode = CodeExecutionMode::Safe; // Uses Docker
let code_tools = agent.get_code_execution_tools();
```

## Input Interpolation

Agent role, goal, and backstory support `{key}` placeholder interpolation:

```rust
let mut agent = Agent::new(
    "{department} Analyst".to_string(),
    "Analyze {topic} data".to_string(),
    "Expert in {domain}".to_string(),
);

let mut inputs = std::collections::HashMap::new();
inputs.insert("department".to_string(), "Finance".to_string());
inputs.insert("topic".to_string(), "Q4 revenue".to_string());
inputs.insert("domain".to_string(), "financial analysis".to_string());

agent.interpolate_inputs(&inputs);
// agent.role is now "Finance Analyst"
```

## Agent Key

Each agent has a deterministic key computed from its original role, goal, and backstory (MD5 hash):

```rust
let key = agent.key();
// Returns a hex string like "a3f2b1c4d5e6f7..."
```

## Python vs Rust Comparison

| Python | Rust |
|--------|------|
| `class Agent(BaseAgent)` with Pydantic | `pub struct Agent` with serde |
| `Optional[str]` | `Option<String>` |
| `@property` | `pub fn method(&self)` |
| `list[str]` | `Vec<String>` |
| `dict[str, Any]` | `HashMap<String, serde_json::Value>` |
| `BaseModel.model_validate()` | `Agent::new()` + field assignment |
| `agent.execute_task(task, context, tools)` | `agent.execute_task(desc, context, tools)` |
| Pydantic validators | Compile-time type checking |
