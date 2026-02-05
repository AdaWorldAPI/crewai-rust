# Python to Rust Migration Guide

This guide covers the key translation patterns between the Python crewAI framework and its Rust port, helping Python developers understand the Rust equivalents and migrate their workflows.

---

## Type Translation Table

| Python | Rust | Notes |
|--------|------|-------|
| `str` | `String` | Owned string. Use `&str` for borrowed references. |
| `int` | `i32` / `i64` | Choose based on range needed |
| `float` | `f64` | Always double precision |
| `bool` | `bool` | Same |
| `None` | `None` | As part of `Option<T>` |
| `Optional[T]` | `Option<T>` | `Some(value)` or `None` |
| `list[T]` | `Vec<T>` | Heap-allocated vector |
| `dict[str, Any]` | `HashMap<String, Value>` | `Value` is `serde_json::Value` |
| `dict[str, str]` | `HashMap<String, String>` | Typed map |
| `set[T]` | `HashSet<T>` | From `std::collections` |
| `tuple[A, B]` | `(A, B)` | Rust tuple |
| `Union[A, B]` | `enum { A(A), B(B) }` | Tagged union with variants |
| `Any` | `Box<dyn Any + Send + Sync>` | Type-erased trait object |
| `Callable[[A], B]` | `Arc<dyn Fn(A) -> B + Send + Sync>` | Thread-safe function pointer |
| `Callable[[A], Awaitable[B]]` | `Box<dyn Fn(A) -> BoxFuture<B>>` | Async callback |
| `UUID` | `Uuid` | From the `uuid` crate |
| `datetime` | `DateTime<Utc>` | From the `chrono` crate |
| `Path` | `PathBuf` / `&Path` | From `std::path` |

## Pattern Translation Table

| Python Pattern | Rust Equivalent |
|---------------|-----------------|
| `class Foo(BaseModel):` (Pydantic) | `#[derive(Debug, Clone, Serialize, Deserialize)] pub struct Foo { ... }` |
| `@dataclass` | `#[derive(Debug, Clone)] pub struct Foo { ... }` |
| `class Foo(ABC):` (abstract) | `pub trait Foo: Send + Sync { ... }` (with `#[async_trait]` for async methods) |
| `@abstractmethod def run()` | `fn run(&self) -> Result<T, E>;` (required trait method) |
| `@property def name(self)` | `pub fn name(&self) -> &str { ... }` |
| `@name.setter` | `pub fn set_name(&mut self, name: String) { ... }` |
| `def method(self)` | `pub fn method(&self) -> ... { ... }` |
| `async def amethod(self)` | `pub async fn amethod(&self) -> ... { ... }` |
| `try: ... except: ...` | `match result { Ok(v) => ..., Err(e) => ... }` or `result?` |
| `raise ValueError(...)` | `return Err(anyhow!("..."))` or `Err("...".into())` |
| `isinstance(x, Foo)` | `x.downcast_ref::<Foo>()` or pattern matching |
| `getattr(obj, "key", default)` | `obj.key.unwrap_or(default)` |
| `**kwargs` | `HashMap<String, Value>` or builder pattern |
| `super().__init__()` | Composition: embed parent struct as a field |
| `logging.getLogger()` | `log::debug!()`, `log::info!()`, etc. |
| `@classmethod` | `pub fn method() -> ...` (no `self`) |
| `__str__` / `__repr__` | `impl std::fmt::Display for Foo { ... }` |
| `__eq__` | `impl PartialEq for Foo { ... }` or `#[derive(PartialEq)]` |
| `f"Hello {name}"` | `format!("Hello {}", name)` |
| `with open(path) as f:` | `let content = std::fs::read_to_string(path)?;` |
| `json.dumps(obj)` | `serde_json::to_string(&obj)?` |
| `json.loads(s)` | `serde_json::from_str::<Value>(s)?` |
| `threading.Lock()` | `std::sync::Mutex<T>` or `parking_lot::Mutex<T>` |
| `asyncio.Lock()` | `tokio::sync::Mutex<T>` |
| `contextvars.ContextVar` | `thread_local!` or `tokio::task_local!` |
| `from enum import Enum` | `pub enum Foo { A, B, C }` |
| `Literal["a", "b"]` | `pub enum Foo { A, B }` with serde rename |

## Common crewAI Translation Patterns

### Agent Creation

**Python:**
```python
from crewai import Agent

agent = Agent(
    role="Senior Researcher",
    goal="Find cutting-edge AI research",
    backstory="Expert researcher...",
    verbose=True,
    llm="gpt-4o",
    tools=[search_tool],
)
```

**Rust:**
```rust
use crewai::Agent;

let mut agent = Agent::new(
    "Senior Researcher".to_string(),
    "Find cutting-edge AI research".to_string(),
    "Expert researcher...".to_string(),
);
agent.verbose = true;
agent.llm = Some("gpt-4o".to_string());
agent.tools = vec!["search_tool".to_string()];
```

### Task Creation

**Python:**
```python
from crewai import Task

task = Task(
    description="Research AI trends",
    expected_output="Bullet-point report",
    agent=researcher,
)
```

**Rust:**
```rust
use crewai::Task;

let mut task = Task::new(
    "Research AI trends".to_string(),
    "Bullet-point report".to_string(),
);
task.agent = Some("Senior Researcher".to_string());
```

### Crew Execution

**Python:**
```python
from crewai import Crew, Process

crew = Crew(
    agents=[researcher, writer],
    tasks=[research_task, write_task],
    process=Process.sequential,
    verbose=True,
)
result = crew.kickoff(inputs={"topic": "AI"})
```

**Rust:**
```rust
use crewai::{Crew, Process};

let mut crew = Crew::new(
    vec![research_task, write_task],
    vec!["Senior Researcher".to_string(), "Writer".to_string()],
);
crew.process = Process::Sequential;
crew.verbose = true;

let mut inputs = std::collections::HashMap::new();
inputs.insert("topic".to_string(), "AI".to_string());
let result = crew.kickoff(Some(inputs))?;
```

### LLM Configuration

**Python:**
```python
from crewai import LLM

llm = LLM(
    model="gpt-4o",
    temperature=0.7,
    max_tokens=2000,
    api_key="sk-...",
)
```

**Rust:**
```rust
use crewai::llm::LLM;

let llm = LLM::new("gpt-4o")
    .temperature(0.7)
    .max_tokens(2000)
    .api_key("sk-...");
```

### Error Handling

**Python:**
```python
try:
    result = crew.kickoff()
except Exception as e:
    print(f"Error: {e}")
```

**Rust:**
```rust
match crew.kickoff(None) {
    Ok(result) => println!("Success: {}", result.raw),
    Err(e) => eprintln!("Error: {}", e),
}

// Or with the ? operator in functions returning Result:
let result = crew.kickoff(None)?;
```

### Async/Await

**Python:**
```python
import asyncio

async def main():
    result = await crew.kickoff_async(inputs={"topic": "AI"})

asyncio.run(main())
```

**Rust:**
```rust
#[tokio::main]
async fn main() {
    let result = crew.kickoff_async(None).await.unwrap();
}
```

### Callbacks

**Python:**
```python
def my_callback(output):
    print(f"Task done: {output.raw}")

task = Task(
    description="...",
    expected_output="...",
    callback=my_callback,
)
```

**Rust:**
```rust
task.callback = Some(Box::new(|output: &TaskOutput| {
    println!("Task done: {}", output.raw);
}));
```

## Key Differences in the Rust Port

### 1. No Runtime Type Checking
Python uses Pydantic for runtime validation. Rust validates types at compile time. Invalid field types are caught during compilation, not at runtime.

### 2. Ownership and Borrowing
Rust enforces ownership rules. You cannot have multiple mutable references to the same data simultaneously. Use `Arc<Mutex<T>>` for shared mutable state.

### 3. No Inheritance
Rust uses composition and traits instead of class inheritance. Where Python has `class Agent(BaseAgent)`, Rust has a struct that implements traits and composes base structs.

### 4. Explicit Error Handling
All fallible operations return `Result<T, E>`. There are no exceptions. Use `?` for propagation or `match` for handling.

### 5. No Default Arguments
Rust does not have default function arguments. The port uses:
- Builder pattern (`LLM::new("gpt-4o").temperature(0.7)`)
- `Option<T>` fields with `Default` trait
- Separate constructor functions (`new()`, `with_config()`, etc.)

### 6. String Types
Python has one string type. Rust has `String` (owned, heap-allocated) and `&str` (borrowed reference). APIs typically accept `impl Into<String>` for flexibility.

### 7. Async Runtime
Python uses `asyncio`. Rust uses Tokio. The `#[tokio::main]` attribute sets up the runtime. All async methods require `.await`.

### 8. Serialization
Python uses `json.dumps()`/`json.loads()`. Rust uses serde with `#[derive(Serialize, Deserialize)]` and `serde_json::to_string()`/`serde_json::from_str()`.

### 9. Dynamic Dispatch
Python resolves methods at runtime. Rust uses static dispatch by default. Dynamic dispatch requires `dyn Trait` with `Box` or `Arc` wrapping.

### 10. Closures vs Lambdas
Python lambdas are limited to single expressions. Rust closures can contain full blocks and must specify capture semantics (`move`, `&`, `&mut`).

## Module Mapping

| Python Module | Rust Module |
|--------------|-------------|
| `crewai.agent` | `crewai::agent` |
| `crewai.task` | `crewai::task` |
| `crewai.crew` | `crewai::crew` |
| `crewai.flow` | `crewai::flow` |
| `crewai.tools` | `crewai::tools` |
| `crewai.llm` | `crewai::llm` |
| `crewai.llms` | `crewai::llms` |
| `crewai.memory` | `crewai::memory` |
| `crewai.knowledge` | `crewai::knowledge` |
| `crewai.events` | `crewai::events` |
| `crewai.mcp` | `crewai::mcp` |
| `crewai.a2a` | `crewai::a2a` |
| `crewai.security` | `crewai::security` |
| `crewai.telemetry` | `crewai::telemetry` |
| `crewai.utilities` | `crewai::utilities` |
| `crewai.process` | `crewai::process` |
| `crewai.types` | `crewai::types` |
