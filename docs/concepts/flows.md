# Flows

A **Flow** is an event-driven state machine for orchestrating deterministic workflows. Flows complement crews by providing explicit control over execution order, conditional branching, state management, persistence, and human-in-the-loop feedback.

**Source**: `src/flow/` (corresponds to Python `crewai/flow/`)

---

## Core Concepts

In the Python framework, flows use decorators (`@start`, `@listen`, `@router`) to mark methods. In Rust, the equivalent is achieved through method registration with metadata structs.

### Method Types

| Type | Python Decorator | Rust Registration | Description |
|------|-----------------|-------------------|-------------|
| **Start** | `@start()` | `is_start_method: true` | Entry point(s) for the flow |
| **Listen** | `@listen(trigger)` | `trigger_methods: Some(vec![...])` | Triggered when another method completes |
| **Router** | `@router(trigger)` | `is_router: true` | Routes execution based on return value |

## FlowState

All flow state is stored in a `FlowState` struct with a unique ID and arbitrary key-value data:

```rust
use crewai::flow::FlowState;
use serde_json::Value;
use std::collections::HashMap;

// Create with default (auto-generated UUID)
let state = FlowState::new();

// Create with initial data
let mut data = HashMap::new();
data.insert("counter".to_string(), Value::from(0));
data.insert("name".to_string(), Value::from("my_flow"));
let state = FlowState::with_data(data);

// Access state
let counter = state.get("counter"); // Option<&Value>

// Modify state
let mut state = FlowState::new();
state.set("counter".to_string(), Value::from(42));

// Serialize to dictionary
let dict = state.to_dict(); // includes "id" key
```

## Creating a Flow

```rust
use crewai::flow::{Flow, FlowState, FlowMethodMeta, FlowConditionType};
use serde_json::Value;

// Create a flow with a name
let mut flow = Flow::with_name("DataPipeline");

// Create with initial state
let state = FlowState::with_data(/* ... */);
let mut flow = Flow::with_state(state);

// With persistence
// let flow = Flow::new().with_persistence(Box::new(sqlite_backend));
```

## Registering Methods

### Start Methods

```rust
use crewai::flow::FlowMethodMeta;

let start_meta = FlowMethodMeta {
    is_start_method: true,
    ..Default::default()
};
flow.register_method_meta("fetch_data", &start_meta);
```

### Listener Methods

```rust
use crewai::flow::{FlowMethodMeta, FlowMethodName, FlowConditionType};

// OR listener: triggers when ANY of the listed methods complete
let listen_meta = FlowMethodMeta {
    trigger_methods: Some(vec![FlowMethodName::new("fetch_data")]),
    condition_type: Some(FlowConditionType::OR),
    ..Default::default()
};
flow.register_method_meta("process_data", &listen_meta);

// AND listener: triggers only when ALL listed methods complete
let and_meta = FlowMethodMeta {
    trigger_methods: Some(vec![
        FlowMethodName::new("fetch_data"),
        FlowMethodName::new("validate_config"),
    ]),
    condition_type: Some(FlowConditionType::AND),
    ..Default::default()
};
flow.register_method_meta("merge_results", &and_meta);
```

### Router Methods

```rust
let router_meta = FlowMethodMeta {
    is_router: true,
    trigger_methods: Some(vec![FlowMethodName::new("process_data")]),
    condition_type: Some(FlowConditionType::OR),
    router_paths: Some(vec!["success_path".to_string(), "error_path".to_string()]),
    ..Default::default()
};
flow.register_method_meta("decide_next_step", &router_meta);
```

## Registering Callbacks

Since Rust does not have Python-style decorators, method callbacks are registered as async closures:

```rust
use futures::FutureExt;

flow.register_callback("fetch_data", Box::new(|state, _trigger| {
    async move {
        state.set("data".to_string(), serde_json::json!({"items": [1, 2, 3]}));
        Ok(serde_json::json!("fetch complete"))
    }.boxed()
}));

flow.register_callback("process_data", Box::new(|state, trigger_result| {
    async move {
        let data = state.get("data").cloned().unwrap_or_default();
        state.set("processed".to_string(), serde_json::json!(true));
        Ok(serde_json::json!("processing complete"))
    }.boxed()
}));
```

## Flow Execution

### Synchronous Kickoff

```rust
let result = flow.kickoff()?;
println!("Flow result: {}", result);
```

### Async Kickoff

```rust
let result = flow.kickoff_async().await?;
```

### State Initialization with Inputs

```rust
use std::collections::HashMap;
use serde_json::Value;

let mut inputs = HashMap::new();
inputs.insert("config_path".to_string(), Value::from("/etc/config.json"));
flow.initialize_state(inputs);

let result = flow.kickoff_async().await?;
```

## Condition Helpers

For compound conditions, use the `and_` and `or_` helpers:

```rust
use crewai::flow::{and_, or_, FlowMethodName};

// OR condition: trigger when method_a OR method_b completes
let or_condition = or_(vec![
    FlowMethodName::new("method_a"),
    FlowMethodName::new("method_b"),
]);

// AND condition: trigger when BOTH method_a AND method_b complete
let and_condition = and_(vec![
    FlowMethodName::new("method_a"),
    FlowMethodName::new("method_b"),
]);
```

## Flow Persistence

Flows support persistence backends for saving state across executions:

```rust
use crewai::flow::persistence::FlowPersistence;

// The FlowPersistence trait defines:
// - save_state(flow_id, method_name, state_data)
// - load_state(flow_id)
// - save_pending_feedback(flow_id, context)
// - load_pending_feedback(flow_id)
// - clear_pending_feedback(flow_id)

// SQLite persistence (when implemented):
// let persistence = SqliteFlowPersistence::new("flow_data.db")?;
// let flow = Flow::new().with_persistence(Box::new(persistence));
```

State is automatically persisted after each method completion when a persistence backend is configured.

## Human-in-the-Loop Feedback

Flows support pausing for human input and resuming:

```rust
// Resume a paused flow with feedback
let result = flow.resume("Approved, proceed with deployment")?;

// Async resume
let result = flow.resume_async("Looks good").await?;

// Restore a flow from a pending feedback state
let flow = Flow::from_pending("flow-uuid-123", Box::new(persistence))?;
let result = flow.resume("User feedback here")?;
```

Access feedback history:

```rust
if let Some(ref feedback) = flow.last_human_feedback {
    println!("Feedback: {}", feedback.feedback);
    println!("Outcome: {:?}", feedback.outcome);
}

for entry in &flow.human_feedback_history {
    println!("{}: {}", entry.method_name, entry.feedback);
}
```

## Flow Visualization

Generate an interactive HTML visualization of the flow structure:

```rust
let html_path = flow.plot(Some("my_flow_diagram"))?;
// Creates "my_flow_diagram.html" with an interactive graph
```

You can also build the structure programmatically:

```rust
use crewai::flow::{build_flow_structure, render_interactive};

let structure = build_flow_structure(&flow.methods);
let html = render_interactive(&structure, "output")?;
```

## Flow Reset

Reset the flow to its initial state:

```rust
flow.reset();
// Clears: method_outputs, completed_methods, pending listeners,
//         feedback history, execution data
```

## Execution Tracking

Access flow execution data:

```rust
// Method outputs in order
for output in &flow.method_outputs {
    println!("Output: {}", output);
}

// Execution data for serialization
let exec_data = &flow.execution_data;
println!("Flow ID: {}", exec_data.id);
println!("Completed methods: {}", exec_data.completed_methods.len());
```

## Complete Example

```rust
use crewai::flow::{Flow, FlowMethodMeta, FlowMethodName, FlowConditionType};
use futures::FutureExt;
use serde_json::Value;

let mut flow = Flow::with_name("ResearchPipeline");

// Register start method
flow.register_method_meta("gather_sources", &FlowMethodMeta {
    is_start_method: true,
    ..Default::default()
});

// Register listener
flow.register_method_meta("analyze", &FlowMethodMeta {
    trigger_methods: Some(vec![FlowMethodName::new("gather_sources")]),
    condition_type: Some(FlowConditionType::OR),
    ..Default::default()
});

// Register router
flow.register_method_meta("quality_check", &FlowMethodMeta {
    is_router: true,
    trigger_methods: Some(vec![FlowMethodName::new("analyze")]),
    condition_type: Some(FlowConditionType::OR),
    router_paths: Some(vec!["publish".to_string(), "revise".to_string()]),
    ..Default::default()
});

// Register callbacks
flow.register_callback("gather_sources", Box::new(|state, _| {
    async move {
        state.set("sources".to_string(), Value::from(vec!["paper1", "paper2"]));
        Ok(Value::from("gathered"))
    }.boxed()
}));

flow.register_callback("analyze", Box::new(|state, _| {
    async move {
        state.set("analysis_complete".to_string(), Value::from(true));
        Ok(Value::from("analyzed"))
    }.boxed()
}));

flow.register_callback("quality_check", Box::new(|state, _| {
    async move {
        // Router returns the path to take
        Ok(Value::from("publish"))
    }.boxed()
}));

// Execute
// let result = flow.kickoff()?;
```
