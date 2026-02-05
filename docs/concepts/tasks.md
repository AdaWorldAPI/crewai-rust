# Tasks

A **Task** represents a unit of work to be executed by an agent. Each task has a description, an expected output, and an assigned agent. Tasks support callbacks, guardrails, output formatting, file output, async execution, and input interpolation.

**Source**: `src/task.rs` (corresponds to Python `crewai/task.py`)

---

## Creating a Task

Use `Task::new()` with the two required fields:

```rust
use crewai::Task;

let mut task = Task::new(
    "Research the latest AI trends for 2024".to_string(),
    "A comprehensive bullet-point analysis".to_string(),
);
task.agent = Some("Senior Research Analyst".to_string());
```

## Task Struct Fields

### Core Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `Uuid` | Auto-generated | Unique task identifier |
| `description` | `String` | Required | What the task should accomplish |
| `expected_output` | `String` | Required | Clear definition of the expected result |
| `name` | `Option<String>` | `None` | Optional display name |
| `agent` | `Option<String>` | `None` | Agent role responsible for execution |

### Execution

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `async_execution` | `bool` | `false` | Run asynchronously |
| `tools` | `Vec<String>` | `[]` | Tool names available for this task |
| `context` | `Option<Vec<Uuid>>` | `None` | IDs of tasks providing context |
| `prompt_context` | `Option<String>` | `None` | Additional prompt context |
| `config` | `Option<HashMap<String, Value>>` | `None` | Task-specific config |

### Output Format

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `output_json` | `Option<String>` | `None` | Schema name for JSON output |
| `output_pydantic` | `Option<String>` | `None` | Schema name for structured output |
| `response_model` | `Option<String>` | `None` | Native provider structured output |
| `output_file` | `Option<String>` | `None` | File path for saving output |
| `create_directory` | `bool` | `true` | Create parent directories for output_file |
| `markdown` | `bool` | `false` | Instruct agent to format in Markdown |

### Guardrails

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `guardrail` | `Option<String>` | `None` | Single guardrail description |
| `guardrails` | `Option<Vec<String>>` | `None` | Multiple guardrail descriptions |
| `guardrail_max_retries` | `i32` | `3` | Max retries on guardrail failure |
| `guardrail_fn` | `Option<GuardrailFn>` | `None` | Compiled guardrail callback (not serialized) |
| `guardrails_fns` | `Vec<GuardrailFn>` | `[]` | Multiple compiled callbacks (not serialized) |

### Callbacks

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `callback` | `Option<TaskCallback>` | `None` | Called after task completion (not serialized) |

### Input Files

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input_files` | `HashMap<String, String>` | `{}` | Named input files (key -> path) |

### Security

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `security_config` | `SecurityConfig` | Default | Security fingerprinting |

### Human Input

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `human_input` | `bool` | `false` | Require human review of the output |

### Timing and Tracking

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `start_time` | `Option<DateTime<Utc>>` | `None` | Execution start time |
| `end_time` | `Option<DateTime<Utc>>` | `None` | Execution end time |
| `used_tools` | `i32` | `0` | Count of tools used |
| `tools_errors` | `i32` | `0` | Count of tool errors |
| `delegations` | `i32` | `0` | Count of delegations |
| `processed_by_agents` | `HashSet<String>` | `{}` | Agent roles that processed this task |

## TaskOutput

The result of a task execution:

```rust
use crewai::TaskOutput;

// TaskOutput fields:
// - description: String        -- task description
// - name: Option<String>       -- task name
// - expected_output: Option<String>
// - summary: Option<String>    -- first 10 words of description
// - raw: String                -- raw text output
// - pydantic: Option<Value>    -- structured output (serde Value)
// - json_dict: Option<Value>   -- JSON output
// - agent: String              -- agent role that produced this
// - output_format: OutputFormat -- Raw, JSON, or Pydantic
// - messages: Vec<Value>       -- LLM conversation messages
```

The `OutputFormat` enum has three variants:

```rust
pub enum OutputFormat {
    Raw,
    JSON,
    Pydantic,
}
```

## Task Execution

### Synchronous Execution

```rust
let mut task = Task::new(
    "Analyze the dataset".to_string(),
    "Summary of key findings".to_string(),
);
task.agent = Some("Data Analyst".to_string());

let output = task.execute_sync(
    Some("Data Analyst"),    // agent role
    Some("Previous context"), // optional context
    None,                     // optional tools
)?;

println!("Result: {}", output.raw);
println!("Duration: {:?}s", task.execution_duration());
```

### Asynchronous Execution

```rust
let handle = task.execute_async(
    Some("Data Analyst".to_string()),
    Some("Context data".to_string()),
    None,
);

// Do other work...

let output = handle.await??;
```

## Guardrails

### GuardrailFn Type

The `GuardrailFn` type is a callback that validates task output:

```rust
pub type GuardrailFn = Box<dyn Fn(&TaskOutput) -> (bool, String) + Send + Sync>;
```

It returns `(success: bool, result_or_error: String)`.

### Using Guardrails

```rust
let mut task = Task::new(
    "Generate a report".to_string(),
    "Professional report with citations".to_string(),
);

// String-based guardrail description (evaluated by LLM)
task.guardrail = Some(
    "Verify that the report contains at least 3 citations".to_string()
);
task.guardrail_max_retries = 5;

// Function-based guardrail (compiled callback)
task.guardrail_fn = Some(Box::new(|output: &TaskOutput| {
    if output.raw.len() > 100 {
        (true, output.raw.clone())
    } else {
        (false, "Output too short, needs more detail".to_string())
    }
}));
```

## Task Callbacks

Register a callback to be called after task completion:

```rust
task.callback = Some(Box::new(|output: &TaskOutput| {
    println!("Task completed by agent: {}", output.agent);
    println!("Output length: {} chars", output.raw.len());
}));
```

## Input Interpolation

Task descriptions and expected outputs support `{key}` placeholders:

```rust
let mut task = Task::new(
    "Research {topic} trends in {year}".to_string(),
    "A report about {topic}".to_string(),
);

let mut inputs = std::collections::HashMap::new();
inputs.insert("topic".to_string(), "quantum computing".to_string());
inputs.insert("year".to_string(), "2024".to_string());

task.interpolate_inputs(&inputs);
// description is now: "Research quantum computing trends in 2024"
```

## Task Prompt

Generate the task prompt that gets sent to the agent:

```rust
let prompt = task.prompt();
// Contains: description + expected output + optional markdown instructions
```

When `task.markdown = true`, the prompt includes formatting instructions.

## Task Key

Each task has a deterministic key based on its original description and expected output:

```rust
let key = task.key();
// MD5 hash of "description|expected_output"
```

## File Output

Save task output to a file:

```rust
task.output_file = Some("output/report.md".to_string());
task.create_directory = true;  // creates output/ if needed

// After execution:
task.save_file(&output.raw)?;
```

## Tracking Counters

Tasks track execution statistics:

```rust
task.increment_tools_errors();
task.increment_delegations(Some("Helper Agent"));

println!("Tools used: {}", task.used_tools);
println!("Tool errors: {}", task.tools_errors);
println!("Delegations: {}", task.delegations);
println!("Processed by: {:?}", task.processed_by_agents);
```
