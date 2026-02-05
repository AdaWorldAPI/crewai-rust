# Crews

A **Crew** represents a group of agents collaborating to complete a set of tasks. The crew defines the process (sequential or hierarchical), manages execution, and produces a combined `CrewOutput`.

**Source**: `src/crew.rs` (corresponds to Python `crewai/crew.py`)

---

## Creating a Crew

Use `Crew::new()` with tasks and agent role strings:

```rust
use crewai::{Crew, Task, Process};

let research_task = Task::new(
    "Research AI trends".to_string(),
    "Bullet-point analysis".to_string(),
);

let writing_task = Task::new(
    "Write a blog post about the findings".to_string(),
    "Full blog post, 4+ paragraphs".to_string(),
);

let mut crew = Crew::new(
    vec![research_task, writing_task],
    vec![
        "Senior Researcher".to_string(),
        "Content Writer".to_string(),
    ],
);
crew.process = Process::Sequential;
crew.verbose = true;
```

## Crew Struct Fields

### Identity

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `Uuid` | Auto-generated | Unique crew identifier |
| `name` | `Option<String>` | `Some("crew")` | Crew display name |

### Core Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `tasks` | `Vec<Task>` | Required | Tasks to execute |
| `agents` | `Vec<String>` | Required | Agent role strings |
| `process` | `Process` | `Sequential` | Execution process type |
| `verbose` | `bool` | `false` | Enable detailed logging |
| `cache` | `bool` | `true` | Enable tool result caching |

### Memory

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `memory` | `bool` | `false` | Enable memory subsystem |
| `short_term_memory` | `Option<HashMap<String, Value>>` | `None` | Short-term memory config |
| `long_term_memory` | `Option<HashMap<String, Value>>` | `None` | Long-term memory config |
| `entity_memory` | `Option<HashMap<String, Value>>` | `None` | Entity memory config |
| `external_memory` | `Option<HashMap<String, Value>>` | `None` | External memory config |
| `embedder` | `Option<HashMap<String, Value>>` | `None` | Embedder configuration |

### Manager (Hierarchical Process)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `manager_llm` | `Option<String>` | `None` | LLM for the manager agent |
| `manager_agent` | `Option<String>` | `None` | Custom manager agent role |

### LLM

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `function_calling_llm` | `Option<String>` | `None` | Global tool-calling LLM |
| `chat_llm` | `Option<String>` | `None` | LLM for crew chat mode |

### Planning

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `planning` | `bool` | `false` | Enable crew execution planning |
| `planning_llm` | `Option<String>` | `None` | LLM for the planner agent |

### Execution Control

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_rpm` | `Option<i32>` | `None` | Global rate limit |
| `stream` | `bool` | `false` | Enable output streaming |
| `share_crew` | `bool` | `false` | Share execution data with crewAI |

### Knowledge

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `knowledge_sources` | `Option<Vec<HashMap<String, Value>>>` | `None` | Knowledge source configs |
| `knowledge` | `Option<HashMap<String, Value>>` | `None` | Knowledge instance config |

### Callbacks (non-serialized)

| Field | Type | Description |
|-------|------|-------------|
| `step_callback` | `Option<Box<dyn Fn(&str)>>` | Called after each agent step |
| `task_callback` | `Option<Box<dyn Fn(&TaskOutput)>>` | Called after each task |
| `before_kickoff_callbacks` | `Vec<...>` | Transform inputs before execution |
| `after_kickoff_callbacks` | `Vec<...>` | Transform output after execution |

### Observability

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `tracing` | `Option<bool>` | `None` | Enable tracing |
| `output_log_file` | `Option<String>` | `None` | Log file path |
| `prompt_file` | `Option<String>` | `None` | Prompt JSON file path |

### Metrics

| Field | Type | Description |
|-------|------|-------------|
| `usage_metrics` | `Option<UsageMetrics>` | Aggregated LLM usage |
| `token_usage` | `Option<UsageMetrics>` | Token counts |
| `execution_logs` | `Vec<HashMap<String, Value>>` | Per-task logs |

## Process Types

```rust
pub enum Process {
    Sequential,    // Tasks execute in order
    Hierarchical,  // Manager agent delegates tasks
}
```

### Sequential Process

Tasks execute one after another. Each task receives the output of all previous tasks as context:

```rust
crew.process = Process::Sequential;
```

### Hierarchical Process

A manager agent decides which agent should handle each task:

```rust
crew.process = Process::Hierarchical;
crew.manager_llm = Some("gpt-4o".to_string());
```

## Crew Execution

### Synchronous Kickoff

```rust
let result = crew.kickoff(None)?;
println!("Final output: {}", result.raw);
```

### With Inputs

```rust
use std::collections::HashMap;

let mut inputs = HashMap::new();
inputs.insert("topic".to_string(), "AI Safety".to_string());

let result = crew.kickoff(Some(inputs))?;
```

### Async Kickoff

```rust
let result = crew.kickoff_async(None).await?;
```

## CrewOutput

The result of crew execution:

```rust
pub struct CrewOutput {
    /// Raw text from the final task
    pub raw: String,
    /// Structured output (if configured)
    pub pydantic: Option<Value>,
    /// JSON output (if configured)
    pub json_dict: Option<Value>,
    /// All task outputs in order
    pub tasks_output: Vec<TaskOutput>,
    /// Aggregated token usage
    pub token_usage: UsageMetrics,
}
```

Access individual task results:

```rust
let result = crew.kickoff(None)?;

for task_output in &result.tasks_output {
    println!("Task: {}", task_output.description);
    println!("Agent: {}", task_output.agent);
    println!("Output: {}", task_output.raw);
}
```

## Callbacks

### Before Kickoff

Transform or validate inputs before execution:

```rust
crew.before_kickoff_callbacks.push(Box::new(|inputs| {
    if let Some(mut inp) = inputs {
        inp.insert("timestamp".to_string(), "2024-01-01".to_string());
        Some(inp)
    } else {
        inputs
    }
}));
```

### After Kickoff

Transform or log the output:

```rust
crew.after_kickoff_callbacks.push(Box::new(|output| {
    println!("Crew completed. Token usage: {:?}", output.token_usage);
    output
}));
```

### Task Callback

Called after every task completes:

```rust
crew.task_callback = Some(Box::new(|output: &TaskOutput| {
    println!("Task '{}' completed by {}", output.description, output.agent);
}));
```

## Memory Management

Reset specific or all memory types:

```rust
// Reset all memories
crew.reset_memories("all")?;

// Reset specific memory type
crew.reset_memories("short")?;
crew.reset_memories("long")?;
crew.reset_memories("entity")?;
crew.reset_memories("external")?;
crew.reset_memories("knowledge")?;
crew.reset_memories("kickoff_outputs")?;
```

## Crew Key

Deterministic key based on agent keys and task keys:

```rust
let key = crew.key();
// MD5 hash of "agent1_key|agent2_key|task1_key|task2_key"
```

## Crew Copy

Create a deep copy with a new ID:

```rust
let crew_copy = crew.copy();
assert_ne!(crew.id, crew_copy.id);
```

## Usage Metrics

After execution, access aggregated metrics:

```rust
let result = crew.kickoff(None)?;

if let Some(ref metrics) = crew.usage_metrics {
    println!("Total tokens: {}", metrics.total_tokens);
    println!("Prompt tokens: {}", metrics.prompt_tokens);
    println!("Completion tokens: {}", metrics.completion_tokens);
    println!("Successful requests: {}", metrics.successful_requests);
}
```
