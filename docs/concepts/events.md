# Events

The events system provides a publish-subscribe architecture for monitoring and extending crewAI operations. Every significant action -- agent execution, task completion, tool usage, LLM calls, flow transitions, memory operations -- emits an event that listeners can observe. The system supports dependency-aware handler ordering, hierarchical scope tracking, and thread-safe emission.

**Source**: `src/events/` (corresponds to Python `crewai/events/`)

---

## Architecture Overview

```
CrewAIEventsBus (OnceLock singleton)
  |
  +-- handlers: RwLock<HashMap<TypeId, Vec<HandlerEntry>>>
  |     |
  |     +-- keyed by event TypeId for type-safe dispatch
  |     +-- each entry: HandlerId + SyncHandler + Vec<Depends>
  |
  +-- execution_plan_cache: RwLock<HashMap<TypeId, ExecutionPlan>>
  |     |
  |     +-- cached topological sort per event type
  |
  +-- runtime: tokio::runtime::Runtime (2 worker threads, "crewai-events")
  |     |
  |     +-- handlers dispatched as async tasks
  |
  +-- pending: Mutex<Vec<JoinHandle<()>>>
  |     |
  |     +-- tracks in-flight handler tasks for flush/shutdown
  |
  +-- shutting_down: RwLock<bool>

Event Context (thread-local)
  |
  +-- EVENT_ID_STACK: RefCell<Vec<(event_id, event_type)>>
  +-- LAST_EVENT_ID: RefCell<Option<String>>
  +-- TRIGGERING_EVENT_ID: RefCell<Option<String>>
  +-- EMISSION_COUNTER: AtomicU64
```

### Key Design Differences from Python

| Aspect | Python | Rust |
|--------|--------|------|
| Singleton | Module-level instance via `__init__` | `OnceLock<CrewAIEventsBus>` with `global()` accessor |
| Handler storage | Dict keyed by event class | `HashMap<TypeId, Vec<HandlerEntry>>` |
| Event dispatch | `asyncio` event loop | Dedicated `tokio::runtime::Runtime` (2 worker threads) |
| Scope tracking | `contextvars.ContextVar` | `thread_local!` with `RefCell` |
| Emission counter | `contextvars.ContextVar[int]` | `thread_local!` with `AtomicU64` |
| Error handling | Exception propagation | `std::panic::catch_unwind` per handler |
| Handler dependencies | `Depends` class | `Depends` struct with topological sort |
| Thread safety | GIL + asyncio | `RwLock`, `Mutex`, `Arc`, `Send + Sync` bounds |

---

## BaseEvent Trait

All events in the system implement the `BaseEvent` trait. This trait requires `Send + Sync + Debug` bounds for thread-safe dispatch:

```rust
pub trait BaseEvent: Send + Sync + std::fmt::Debug {
    /// Unique identifier for this event instance (UUID v4).
    fn event_id(&self) -> &str;

    /// UTC timestamp when the event was created.
    fn timestamp(&self) -> DateTime<Utc>;

    /// Event type discriminator string (e.g., "crew_kickoff_started").
    fn event_type(&self) -> &str;

    /// UUID string of the source entity fingerprint, if available.
    fn source_fingerprint(&self) -> Option<&str>;

    /// Source entity kind ("agent", "task", "crew", etc.).
    fn source_type(&self) -> Option<&str>;

    /// Arbitrary fingerprint metadata.
    fn fingerprint_metadata(&self) -> Option<&HashMap<String, serde_json::Value>>;

    /// Task ID associated with this event, if any.
    fn task_id(&self) -> Option<&str>;

    /// Task name associated with this event, if any.
    fn task_name(&self) -> Option<&str>;

    /// Agent ID associated with this event, if any.
    fn agent_id(&self) -> Option<&str>;

    /// Agent role associated with this event, if any.
    fn agent_role(&self) -> Option<&str>;

    /// Parent event ID for hierarchical scope tracking.
    fn parent_event_id(&self) -> Option<&str>;
    fn set_parent_event_id(&mut self, id: Option<String>);

    /// Previous event ID for linear chain tracking.
    fn previous_event_id(&self) -> Option<&str>;
    fn set_previous_event_id(&mut self, id: Option<String>);

    /// ID of the event that causally triggered this event.
    fn triggered_by_event_id(&self) -> Option<&str>;
    fn set_triggered_by_event_id(&mut self, id: Option<String>);

    /// Monotonically increasing emission sequence number.
    fn emission_sequence(&self) -> Option<u64>;
    fn set_emission_sequence(&mut self, seq: Option<u64>);
}
```

---

## BaseEventData

The concrete struct that implements `BaseEvent`. Most domain-specific events embed this struct and delegate trait methods to it:

```rust
use crewai::events::BaseEventData;

let event = BaseEventData::new("my_custom_event");
println!("ID: {}", event.event_id);        // UUID v4
println!("Time: {}", event.timestamp);      // UTC now
println!("Type: {}", event.event_type);     // "my_custom_event"
```

### Fields

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseEventData {
    pub event_id: String,                    // UUID v4
    pub timestamp: DateTime<Utc>,            // chrono::Utc::now()
    #[serde(rename = "type")]
    pub event_type: String,                  // Discriminator string
    pub source_fingerprint: Option<String>,
    pub source_type: Option<String>,
    pub fingerprint_metadata: Option<HashMap<String, serde_json::Value>>,
    pub task_id: Option<String>,
    pub task_name: Option<String>,
    pub agent_id: Option<String>,
    pub agent_role: Option<String>,
    pub parent_event_id: Option<String>,     // Hierarchical scope
    pub previous_event_id: Option<String>,   // Linear chain
    pub triggered_by_event_id: Option<String>, // Causal chain
    pub emission_sequence: Option<u64>,      // Per-thread counter
}
```

---

## impl_base_event! Macro

Domain events embed `BaseEventData` as a `base` field and use the `impl_base_event!` macro to delegate all `BaseEvent` trait methods:

```rust
use crewai::impl_base_event;
use crewai::events::base_event::{BaseEvent, BaseEventData};

#[derive(Debug, Clone)]
pub struct MyCustomEvent {
    pub base: BaseEventData,
    pub custom_field: String,
}

impl_base_event!(MyCustomEvent);

// Now MyCustomEvent implements BaseEvent, delegating to self.base
```

The macro is defined with `#[macro_export]` so it is available at the crate root. It generates implementations for all 18 methods of the `BaseEvent` trait, each delegating to the corresponding field or method on `self.base`.

---

## CrewAIEventsBus

The global event bus is a singleton stored in a `static OnceLock`:

```rust
use crewai::events::event_bus::{CrewAIEventsBus, CREWAI_EVENT_BUS};

// Access the singleton (initializes on first call)
let bus = CrewAIEventsBus::global();
```

### Initialization

On first access, the bus creates a dedicated Tokio runtime:

```rust
let runtime = Builder::new_multi_thread()
    .worker_threads(2)
    .thread_name("crewai-events")
    .enable_all()
    .build()
    .expect("failed to create CrewAI events runtime");
```

This runtime is independent of any application-level async runtime, ensuring event handlers never block the main execution path.

### Registering Handlers

```rust
use crewai::events::event_bus::{CrewAIEventsBus, Depends, HandlerId};

let bus = CrewAIEventsBus::global();

// Register a handler for a specific event type
let handler_id = bus.on::<MyCustomEvent>(
    "my_handler",
    |source, event| {
        println!("Event: {}", event.event_type());
    },
    None,  // no dependencies
);

// Register with an Arc<dyn Fn> handler
let handler_id = bus.register_handler::<MyCustomEvent>(
    "arc_handler",
    Arc::new(|source, event| {
        println!("Event via Arc: {}", event.event_type());
    }),
);
```

The handler type signature is:

```rust
pub type SyncHandler = Arc<dyn Fn(&dyn Any, &dyn BaseEvent) + Send + Sync>;
```

Handlers receive:
- `source: &dyn Any` -- the type-erased object that emitted the event
- `event: &dyn BaseEvent` -- the event data (serialized to `Arc<BaseEventData>` for cross-thread safety)

### Unregistering Handlers

```rust
bus.off::<MyCustomEvent>(&handler_id);
```

Removing a handler also invalidates the cached execution plan for that event type.

### Emitting Events

```rust
use std::sync::Arc;
use std::any::Any;

let bus = CrewAIEventsBus::global();

let source: Arc<dyn Any + Send + Sync> = Arc::new("my_source".to_string());
let mut event = MyCustomEvent {
    base: BaseEventData::new("my_custom_event"),
    custom_field: "hello".to_string(),
};

bus.emit(source, &mut event);
```

The `emit` method performs the following steps:

1. **Chain tracking** -- Sets `previous_event_id`, `triggered_by_event_id`, and `emission_sequence` from thread-local state.
2. **Scope tracking** -- Checks if the event is a scope-starting or scope-ending event:
   - **Starting**: pushes `(event_id, event_type)` onto the scope stack, sets `parent_event_id` to current stack top.
   - **Ending**: pops from the scope stack, validates the pair matches, sets `parent_event_id` to the enclosing scope.
   - **Other**: sets `parent_event_id` to current stack top without modifying the stack.
3. **Last event ID** -- Updates the thread-local `LAST_EVENT_ID`.
4. **Dispatch** -- Serializes the event to an `Arc<BaseEventData>` and spawns handler tasks on the background runtime.

### Dependency-Aware Dispatch

If any handler for an event type declares dependencies, the bus switches to dependency-aware dispatch:

```rust
let handler_a = bus.on::<MyEvent>(
    "handler_a",
    |_, _| { /* runs first */ },
    None,
);

let handler_b = bus.on::<MyEvent>(
    "handler_b",
    |_, _| { /* runs after handler_a */ },
    Some(vec![Depends::new(handler_a.clone())]),
);
```

The bus builds an execution plan using topological sort (via `build_execution_plan` from the `handler_graph` module). Handlers are grouped into levels:
- Handlers within the same level can run concurrently.
- Levels execute sequentially, with each level blocking until all its handlers complete.

The execution plan is cached per event `TypeId` and invalidated when handlers are added or removed.

### Flush and Shutdown

```rust
let bus = CrewAIEventsBus::global();

// Block until all pending handlers complete
let all_ok = bus.flush();  // returns true if no errors

// Graceful shutdown (flush first, then clear all handlers)
bus.shutdown(true);  // wait=true flushes before clearing

// Immediate shutdown (clears handlers without waiting)
bus.shutdown(false);
```

### Validate Dependencies

```rust
// Eagerly check for circular or unresolved dependencies
bus.validate_dependencies()?;
// Returns Err(CircularDependencyError) if cycles exist
```

---

## Handler Dependencies (Depends)

The `Depends` struct declares that one handler must execute after another:

```rust
use crewai::events::event_bus::Depends;

let dep = Depends::new(handler_a_id.clone());
```

Under the hood, the `handler_graph` module performs topological sorting on the dependency graph. If circular dependencies are detected, a `CircularDependencyError` is returned.

### HandlerId

Each handler receives a unique `HandlerId`:

```rust
pub struct HandlerId {
    pub name: String,  // Human-readable name
    id: u64,           // Monotonically increasing (AtomicU64)
}
```

`HandlerId` implements `PartialEq`, `Eq`, and `Hash` based on the numeric `id` field, not the name. This allows multiple handlers to have the same human-readable name while remaining distinct.

---

## Emission Sequence

Events carry an auto-incrementing emission sequence number, tracked per-thread:

```rust
use crewai::events::base_event::{get_next_emission_sequence, reset_emission_counter};

let seq1 = get_next_emission_sequence(); // 1
let seq2 = get_next_emission_sequence(); // 2
let seq3 = get_next_emission_sequence(); // 3

reset_emission_counter(); // resets to 1
let seq4 = get_next_emission_sequence(); // 1
```

The counter uses `thread_local!` with `AtomicU64` (using `Ordering::Relaxed` since it is thread-local). This corresponds to Python's `contextvars.ContextVar` for the emission counter.

---

## Event Context and Scope Tracking

The `event_context` module (`src/events/event_context.rs`) manages hierarchical parent-child relationships between events using thread-local state.

### Scope Stack

```rust
use crewai::events::event_context::{
    push_event_scope, pop_event_scope,
    get_current_parent_id, get_enclosing_parent_id,
};

// Push a scope (e.g., when crew_kickoff_started fires)
push_event_scope("event-uuid-1".to_string(), "crew_kickoff_started".to_string());

// Current parent is the top of the stack
assert_eq!(get_current_parent_id(), Some("event-uuid-1".to_string()));

// Nested scope
push_event_scope("event-uuid-2".to_string(), "task_started".to_string());
assert_eq!(get_current_parent_id(), Some("event-uuid-2".to_string()));
assert_eq!(get_enclosing_parent_id(), Some("event-uuid-1".to_string()));

// Pop when scope ends
pop_event_scope(); // removes task_started scope
```

### RAII Scope Guards

```rust
use crewai::events::event_context::{EventScopeGuard, TriggeredByScopeGuard};

// EventScopeGuard pushes on creation, pops on drop
{
    let _guard = EventScopeGuard::new(
        "event-uuid".to_string(),
        "crew_kickoff_started".to_string(),
    );
    // scope is active here
} // automatically pops when guard is dropped

// TriggeredByScopeGuard saves and restores triggering event ID
{
    let _guard = TriggeredByScopeGuard::new("trigger-event-uuid".to_string());
    // get_triggering_event_id() returns Some("trigger-event-uuid")
} // restores previous triggering event ID
```

### Linear Chain Tracking

```rust
use crewai::events::event_context::{
    get_last_event_id, set_last_event_id, reset_last_event_id,
    get_triggering_event_id, set_triggering_event_id,
};

// Track the last emitted event for previous_event_id chain
set_last_event_id("event-1".to_string());
assert_eq!(get_last_event_id(), Some("event-1".to_string()));

// Track causal triggering
set_triggering_event_id(Some("cause-event".to_string()));
assert_eq!(get_triggering_event_id(), Some("cause-event".to_string()));
```

### Mismatch Handling

The system detects mismatched event pairs (e.g., `task_completed` without a preceding `task_started`):

```rust
use crewai::events::event_context::{EventContextConfig, MismatchBehavior};

// Configuration (per-thread override or global default)
let config = EventContextConfig {
    max_stack_depth: 100,                      // 0 = unlimited
    mismatch_behavior: MismatchBehavior::Warn, // Warn, Raise, or Silent
    empty_pop_behavior: MismatchBehavior::Warn,
};
```

---

## BaseEventListener Trait

Create custom event listeners that register handlers on the global event bus:

```rust
use crewai::events::BaseEventListener;
use crewai::events::event_bus::CrewAIEventsBus;

pub trait BaseEventListener: Send + Sync {
    /// Whether this listener produces verbose output.
    fn verbose(&self) -> bool { false }

    /// Register event handlers on the provided event bus.
    fn setup_listeners(&self, bus: &CrewAIEventsBus);

    /// Initialize the listener (calls setup_listeners + validate_dependencies).
    fn init(&self) {
        let bus = CrewAIEventsBus::global();
        self.setup_listeners(bus);
        let _ = bus.validate_dependencies();
    }
}
```

### Implementing a Custom Listener

```rust
struct MyLogger;

impl BaseEventListener for MyLogger {
    fn verbose(&self) -> bool { true }

    fn setup_listeners(&self, bus: &CrewAIEventsBus) {
        bus.on::<CrewKickoffStartedEvent>(
            "my_logger_crew_start",
            |_, event| {
                println!("[LOG] Crew kicked off: {}", event.event_id());
            },
            None,
        );

        bus.on::<TaskCompletedEvent>(
            "my_logger_task_done",
            |_, event| {
                println!("[LOG] Task completed: {}", event.event_id());
            },
            None,
        );
    }
}

// Initialize and register
let logger = MyLogger;
logger.init();
```

---

## Scope-Starting and Scope-Ending Events

The system maintains comprehensive sets of event types that open and close scopes:

### Scope-Starting Events

| Event Type | Component |
|------------|-----------|
| `flow_started` | Flow |
| `method_execution_started` | Flow |
| `crew_kickoff_started` | Crew |
| `crew_train_started` | Crew |
| `crew_test_started` | Crew |
| `agent_execution_started` | Agent |
| `agent_evaluation_started` | Agent |
| `lite_agent_execution_started` | Agent |
| `task_started` | Task |
| `llm_call_started` | LLM |
| `llm_guardrail_started` | LLM |
| `tool_usage_started` | Tool |
| `mcp_connection_started` | MCP |
| `mcp_tool_execution_started` | MCP |
| `memory_retrieval_started` | Memory |
| `memory_save_started` | Memory |
| `memory_query_started` | Memory |
| `knowledge_query_started` | Knowledge |
| `knowledge_search_query_started` | Knowledge |
| `a2a_delegation_started` | A2A |
| `a2a_conversation_started` | A2A |
| `a2a_server_task_started` | A2A |
| `a2a_parallel_delegation_started` | A2A |
| `agent_reasoning_started` | Reasoning |

### Scope-Ending Events

Each starting event has one or more corresponding ending events (completed, failed, paused, error, canceled). The `VALID_EVENT_PAIRS` map connects each ending event to its expected starting event for validation.

---

## Event Types Reference

### Agent Events

| Event | Description |
|-------|-------------|
| `AgentExecutionStartedEvent` | Agent begins executing a task |
| `AgentExecutionCompletedEvent` | Agent finishes a task successfully |
| `AgentExecutionErrorEvent` | Agent encounters an error |
| `AgentEvaluationStartedEvent` | Agent evaluation begins |
| `AgentEvaluationCompletedEvent` | Agent evaluation completes |
| `AgentEvaluationFailedEvent` | Agent evaluation fails |
| `LiteAgentExecutionStartedEvent` | Lite agent execution begins |
| `LiteAgentExecutionCompletedEvent` | Lite agent execution completes |
| `LiteAgentExecutionErrorEvent` | Lite agent execution fails |

### Crew Events

| Event | Description |
|-------|-------------|
| `CrewKickoffStartedEvent` | Crew begins execution |
| `CrewKickoffCompletedEvent` | Crew completes successfully |
| `CrewKickoffFailedEvent` | Crew execution fails |
| `CrewTestStartedEvent` | Crew testing begins |
| `CrewTestCompletedEvent` | Crew testing completes |
| `CrewTestFailedEvent` | Crew testing fails |
| `CrewTestResultEvent` | Individual test result |
| `CrewTrainStartedEvent` | Crew training begins |
| `CrewTrainCompletedEvent` | Crew training completes |
| `CrewTrainFailedEvent` | Crew training fails |

### Task Events

| Event | Description |
|-------|-------------|
| `TaskStartedEvent` | Task execution begins |
| `TaskCompletedEvent` | Task completes successfully |
| `TaskFailedEvent` | Task execution fails |
| `TaskEvaluationEvent` | Task quality evaluation result |

### Tool Events

| Event | Description |
|-------|-------------|
| `ToolUsageEvent` | Tool invoked |
| `ToolUsageStartedEvent` | Tool execution begins |
| `ToolUsageFinishedEvent` | Tool execution completes |
| `ToolUsageErrorEvent` | Tool execution fails |
| `ToolSelectionErrorEvent` | Tool selection/matching fails |
| `ToolExecutionErrorEvent` | Tool runtime error |
| `ToolValidateInputErrorEvent` | Tool input validation fails |

### LLM Events

| Event | Description |
|-------|-------------|
| `LLMCallStartedEvent` | LLM API call initiated |
| `LLMCallCompletedEvent` | LLM API call completed |
| `LLMCallFailedEvent` | LLM API call failed |
| `LLMStreamChunkEvent` | Streaming chunk received |

### Flow Events

| Event | Description |
|-------|-------------|
| `FlowCreatedEvent` | Flow instance created |
| `FlowStartedEvent` | Flow execution begins |
| `FlowFinishedEvent` | Flow execution completes |
| `FlowPausedEvent` | Flow paused for feedback |
| `MethodExecutionStartedEvent` | Flow method begins |
| `MethodExecutionFinishedEvent` | Flow method completes |
| `MethodExecutionFailedEvent` | Flow method fails |
| `MethodExecutionPausedEvent` | Flow method paused |
| `HumanFeedbackRequestedEvent` | Human feedback requested |
| `HumanFeedbackReceivedEvent` | Human feedback received |

### Memory Events

| Event | Description |
|-------|-------------|
| `MemorySaveStartedEvent` | Memory save initiated |
| `MemorySaveCompletedEvent` | Memory save completed |
| `MemorySaveFailedEvent` | Memory save failed |
| `MemoryQueryStartedEvent` | Memory query initiated |
| `MemoryQueryCompletedEvent` | Memory query completed |
| `MemoryQueryFailedEvent` | Memory query failed |
| `MemoryRetrievalStartedEvent` | Memory retrieval begins |
| `MemoryRetrievalCompletedEvent` | Memory retrieval completes |
| `MemoryRetrievalFailedEvent` | Memory retrieval fails |

### Knowledge Events

| Event | Description |
|-------|-------------|
| `KnowledgeQueryStartedEvent` | Knowledge query initiated |
| `KnowledgeQueryCompletedEvent` | Knowledge query completed |
| `KnowledgeQueryFailedEvent` | Knowledge query failed |
| `KnowledgeRetrievalStartedEvent` | Knowledge retrieval begins |
| `KnowledgeRetrievalCompletedEvent` | Knowledge retrieval completes |
| `KnowledgeSearchQueryFailedEvent` | Knowledge search failed |

### Additional Event Categories

| Category | Module | Description |
|----------|--------|-------------|
| A2A Events | `types::a2a_events` | Agent-to-Agent protocol events |
| MCP Events | `types::mcp_events` | MCP server/tool events |
| LLM Guardrail Events | `types::llm_guardrail_events` | Output guardrail checks |
| Reasoning Events | `types::reasoning_events` | Agent reasoning steps |
| System Events | `types::system_events` | System-level events |
| Logging Events | `types::logging_events` | Log-related events |
| Tool Usage Events | `types::tool_usage_events` | Detailed tool usage tracking |

---

## Thread Safety Design

| Component | Synchronization | Purpose |
|-----------|-----------------|---------|
| Handler registry | `RwLock<HashMap<TypeId, Vec<HandlerEntry>>>` | Multiple concurrent readers, exclusive writes |
| Execution plan cache | `RwLock<HashMap<TypeId, ExecutionPlan>>` | Cached topological sorts |
| Pending handles | `Mutex<Vec<JoinHandle<()>>>` | Track in-flight handler tasks |
| Shutdown flag | `RwLock<bool>` | Prevent emission during shutdown |
| Scope stack | `thread_local! { RefCell<Vec<...>> }` | Per-thread hierarchical tracking |
| Emission counter | `thread_local! { AtomicU64 }` | Per-thread monotonic counter |
| Last event ID | `thread_local! { RefCell<Option<String>> }` | Per-thread linear chain |
| Triggering event ID | `thread_local! { RefCell<Option<String>> }` | Per-thread causal chain |
| Handler dispatch | `std::panic::catch_unwind` per handler | Isolate handler panics |
| Event data | `Arc<BaseEventData>` | Shared immutable event data across handler tasks |
