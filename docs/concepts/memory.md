# Memory

The memory system gives crewAI agents the ability to store and retrieve information across task executions. It provides four specialized memory types -- short-term, long-term, entity, and contextual -- each backed by a pluggable storage interface. The Rust port preserves the Python architecture while leveraging traits, `anyhow::Error` for propagation, `serde_json::Value` for metadata, and both sync and async APIs via `async_trait`.

**Source**: `src/memory/` (corresponds to Python `crewai/memory/`)

---

## Architecture Overview

```
Memory (base struct)
  |
  +-- ShortTermMemory     -- Transient data for immediate tasks (RAG-backed)
  +-- LongTermMemory      -- Cross-run data for crew execution history (SQLite-backed)
  +-- EntityMemory         -- Structured information about entities (RAG-backed)
  +-- ExternalMemory       -- Integration with external services (e.g., Mem0)
  +-- ContextualMemory     -- Aggregates all of the above into unified context
  |
  +-- Storage (trait)
        +-- RAGStorage              -- Vector similarity search
        +-- LTMSQLiteStorage        -- SQLite for long-term persistence
        +-- Mem0Storage             -- External Mem0 service integration
        +-- KickoffTaskOutputsSQLiteStorage -- Task output persistence
```

### Memory Types at a Glance

| Type | Storage | Scope | Use Case |
|------|---------|-------|----------|
| **Short-Term** | RAG (vector) | Single crew execution | Recent insights, task context |
| **Long-Term** | SQLite | Across executions | Historical task performance, quality scores |
| **Entity** | RAG (vector) | Single or cross execution | Facts about people, organizations, concepts |
| **External** | Mem0 or custom | External service | Integration with third-party memory stores |
| **Contextual** | Aggregator | Single task | Combines all sources for comprehensive context |

---

## The `Storage` Trait

All memory storage backends implement the `Storage` trait, defined in `memory::storage::interface`. This is the Rust equivalent of Python's `Storage` abstract class.

```rust
use async_trait::async_trait;
use std::collections::HashMap;
use serde_json::Value;

#[async_trait]
pub trait Storage: Send + Sync {
    /// Save a value with associated metadata.
    fn save(
        &self,
        value: &str,
        metadata: &HashMap<String, Value>,
    ) -> Result<(), anyhow::Error>;

    /// Async save. Default delegates to sync `save()`.
    async fn asave(
        &self,
        value: &str,
        metadata: &HashMap<String, Value>,
    ) -> Result<(), anyhow::Error> {
        self.save(value, metadata)
    }

    /// Search for entries matching the query.
    fn search(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error>;

    /// Async search. Default delegates to sync `search()`.
    async fn asearch(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        self.search(query, limit, score_threshold)
    }

    /// Remove all entries from storage.
    fn reset(&self) -> Result<(), anyhow::Error>;
}
```

### Key Rust Differences

- **`Send + Sync` bounds**: Storage backends must be safely shareable across threads (agents run concurrently on a Tokio runtime).
- **`anyhow::Error`**: Error handling uses `anyhow` for flexible error propagation instead of Python's exception hierarchy.
- **`async_trait`**: Async methods use the `async_trait` macro since Rust traits cannot have native `async fn`.
- **`Box<dyn Storage>`**: Memory types accept storage backends as trait objects, enabling runtime polymorphism.

### Available Backends

| Backend | Module | Storage Type | Description |
|---------|--------|--------------|-------------|
| `RAGStorage` | `memory::storage::rag_storage` | Vector | Embedding-based similarity search for STM and entity memory |
| `LTMSQLiteStorage` | `memory::storage::ltm_sqlite_storage` | SQLite | Relational storage for long-term memory |
| `Mem0Storage` | `memory::storage::mem0_storage` | External | Integration with the Mem0 external memory service |
| `KickoffTaskOutputsSQLiteStorage` | `memory::storage::kickoff_task_outputs_storage` | SQLite | Stores task outputs from crew kickoff runs |

---

## The `Memory` Base Struct

All memory types build on the `Memory` struct, which wraps a `Box<dyn Storage>` with optional embedder configuration.

```rust
use crewai::memory::Memory;

// Create with a storage backend
let memory = Memory::new(Box::new(my_storage_backend));

// Create with embedder configuration for vector-based storage
let memory = Memory::with_embedder(
    Box::new(rag_storage),
    Some(serde_json::json!({
        "provider": "openai",
        "config": { "model": "text-embedding-3-small" }
    })),
);
```

### Operations

```rust
use std::collections::HashMap;
use serde_json::Value;

// Save to memory (sync)
let mut metadata = HashMap::new();
metadata.insert("agent".to_string(), Value::from("researcher"));
memory.save("Important finding about AI trends", Some(metadata))?;

// Save to memory (async)
memory.asave("Another finding", None).await?;

// Search memory (sync)
let results = memory.search(
    "AI trends",  // query
    5,            // limit
    0.7,          // score_threshold
)?;

// Search memory (async)
let results = memory.asearch("AI trends", 5, 0.7).await?;
```

---

## Short-Term Memory

`ShortTermMemory` manages transient data related to immediate tasks and interactions. It uses `RAGStorage` by default for vector-based retrieval, or `Mem0Storage` when the provider is configured as `"mem0"`.

```rust
use crewai::memory::short_term::{ShortTermMemory, ShortTermMemoryItem};

// Create with default RAG storage
let stm = ShortTermMemory::new(
    Some(embedder_config),     // Optional embedder configuration
    None,                       // Optional pre-configured storage
    Some(vec!["Researcher".to_string()]), // Agent roles for collection naming
    None,                       // Optional persist directory
);

// Save a value
stm.save(
    "The quarterly revenue increased by 15%",
    Some(metadata),
    Some("Analyst"),  // agent_role
)?;

// Async save
stm.asave("Another insight", None, Some("Researcher")).await?;

// Search
let results = stm.search("revenue trends", 5, 0.6)?;

// Async search
let results = stm.asearch("revenue trends", 5, 0.6).await?;

// Reset all short-term memory
stm.reset()?;
```

### `ShortTermMemoryItem`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortTermMemoryItem {
    pub data: String,                        // The content
    pub agent: Option<String>,               // Agent role that created this
    pub metadata: HashMap<String, Value>,    // Associated metadata
}
```

---

## Long-Term Memory

`LongTermMemory` manages cross-run data related to overall crew execution and performance. It uses `LTMSQLiteStorage` for durable persistence.

```rust
use crewai::memory::long_term::{LongTermMemory, LongTermMemoryItem};

// Create with default SQLite storage
let ltm = LongTermMemory::new(
    None,                    // Optional pre-configured storage
    Some("/data/ltm.db".into()), // Optional database path
)?;

// Create a memory item
let item = LongTermMemoryItem::new(
    "Researcher".to_string(),            // agent
    "Analyze market trends".to_string(), // task
    "Comprehensive analysis".to_string(),// expected_output
    "2025-01-15T10:30:00Z".to_string(), // datetime
    Some(0.85),                          // quality score
    None,                                // metadata
);

// Save (sync)
ltm.save(&item)?;

// Save (async)
ltm.asave(&item).await?;

// Search by task description
let results = ltm.search("market trends", 5)?;

// Async search
let results = ltm.asearch("market trends", 5).await?;

// Reset
ltm.reset()?;
```

### `LongTermMemoryItem`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongTermMemoryItem {
    pub agent: String,                       // Agent role
    pub task: String,                        // Task description
    pub expected_output: String,             // Expected output description
    pub datetime: String,                    // ISO timestamp
    pub quality: Option<f64>,                // Quality score (0.0-1.0)
    pub metadata: HashMap<String, Value>,    // Additional metadata
}
```

---

## Entity Memory

`EntityMemory` manages structured information about entities (people, organizations, concepts) and their relationships. Like short-term memory, it uses RAG-based vector storage by default.

```rust
use crewai::memory::entity::{EntityMemory, EntityMemoryItem};

// Create entity memory
let em = EntityMemory::new(
    Some(embedder_config),  // Optional embedder configuration
    None,                    // Optional pre-configured storage
    None,                    // Agent roles
    None,                    // Persist directory
);

// Create entity items
let items = vec![
    EntityMemoryItem::new(
        "Acme Corp".to_string(),
        "organization".to_string(),
        "A technology company specializing in AI solutions".to_string(),
        "Partner of TechStart, competitor of Beta Inc".to_string(),
    ),
    EntityMemoryItem::new(
        "Jane Smith".to_string(),
        "person".to_string(),
        "CEO of Acme Corp since 2023".to_string(),
        "Reports to the board, manages 500 employees".to_string(),
    ),
];

// Save multiple entities (sync)
em.save(items.clone())?;

// Save multiple entities (async)
em.asave(items).await?;

// Search
let results = em.search("Acme Corp leadership", 5, 0.6)?;

// Async search
let results = em.asearch("Acme Corp leadership", 5, 0.6).await?;
```

### `EntityMemoryItem`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMemoryItem {
    pub name: String,                        // Entity name
    pub entity_type: String,                 // Category (person, org, concept)
    pub description: String,                 // Entity description
    pub metadata: HashMap<String, Value>,    // Includes "relationships" key
}
```

Note that the `relationships` string is automatically stored in the `metadata` HashMap under the key `"relationships"`.

---

## Contextual Memory

`ContextualMemory` aggregates and retrieves context from all memory sources for a given task. It queries short-term, long-term, entity, and external memory in parallel (in async mode) and combines the results.

```rust
use crewai::memory::contextual::ContextualMemory;

let ctx_memory = ContextualMemory::new(
    Some(short_term_memory),   // Optional ShortTermMemory
    Some(long_term_memory),    // Optional LongTermMemory
    Some(entity_memory),       // Optional EntityMemory
    Some(external_memory),     // Optional ExternalMemory
);

// Build context for a task (sync)
let context = ctx_memory.build_context_for_task(
    "Analyze the competitive landscape",  // task_description
    "Focus on AI startups",               // additional_context
)?;

// Build context for a task (async -- fetches all sources concurrently via tokio::join!)
let context = ctx_memory.abuild_context_for_task(
    "Analyze the competitive landscape",
    "Focus on AI startups",
).await?;
```

### How Context is Assembled

The context string is built from up to four sections:

1. **Historical Data** (from long-term memory) -- past task suggestions and quality data
2. **Recent Insights** (from short-term memory) -- top 5 results above 0.6 similarity
3. **Entities** (from entity memory) -- top 5 entity matches above 0.6 similarity
4. **External memories** (from external memory) -- top 5 results from external services

Sections with no results are omitted. In async mode, all four queries run concurrently using `tokio::join!`.

---

## External Memory

`ExternalMemory` integrates with external memory services such as Mem0. It follows the same `save`/`search`/`reset` pattern as other memory types.

```rust
use crewai::memory::external::ExternalMemory;

let exm = ExternalMemory::new(
    Some(embedder_config),
    None,
    None,
    None,
);

// Save
exm.save("Key insight about the project", Some(metadata), Some("PM"))?;

// Search
let results = exm.search("project insights", 5, 0.6)?;
```

---

## Configuring Memory in a Crew

Enable memory for a crew by setting the `memory` flag and optionally configuring the embedder:

```rust
use crewai::crew::Crew;

let mut crew = Crew::new(tasks, agents);
crew.memory = true;

// Configure the embedder for vector-based memory
crew.embedder = Some(serde_json::json!({
    "provider": "openai",
    "config": {
        "model": "text-embedding-3-small"
    }
}));

// Optional: configure a specific memory provider
crew.memory_config = Some(serde_json::json!({
    "provider": "mem0",
    "config": {
        "api_key": "your-mem0-api-key"
    }
}));
```

When `memory` is `true`, the crew automatically:
1. Creates `ShortTermMemory`, `LongTermMemory`, and `EntityMemory` instances
2. Wraps them in a `ContextualMemory` for task context building
3. Saves task outputs and entity information after each task completes

---

## Ownership and Lifetimes

Memory types own their storage backends via `Box<dyn Storage>`. This means:

- Each memory instance has exclusive ownership of its storage
- Storage backends do not need a specific lifetime -- they are heap-allocated
- The `Memory` struct stores the storage as `pub storage: Box<dyn Storage>`, allowing direct access when needed
- Context references (crew, agent, task) are set via `Box<dyn Any + Send + Sync>` to avoid tying memory lifetimes to specific struct lifetimes

```rust
// Memory owns its storage -- no lifetime issues
let memory = Memory::new(Box::new(RAGStorage::new(
    "short_term", true, None, None, None,
)));

// Access the underlying storage directly if needed
memory.storage.reset()?;
```

---

## Implementing a Custom Storage Backend

To create a custom storage backend, implement the `Storage` trait:

```rust
use async_trait::async_trait;
use crewai::memory::storage::Storage;
use std::collections::HashMap;
use serde_json::Value;

pub struct MyCustomStorage {
    // ... your fields
}

#[async_trait]
impl Storage for MyCustomStorage {
    fn save(
        &self,
        value: &str,
        metadata: &HashMap<String, Value>,
    ) -> Result<(), anyhow::Error> {
        // Persist value + metadata to your backend
        Ok(())
    }

    fn search(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        // Query your backend and return matching results
        Ok(vec![])
    }

    fn reset(&self) -> Result<(), anyhow::Error> {
        // Clear all data
        Ok(())
    }

    // Override async methods if your backend has native async support
    async fn asave(
        &self,
        value: &str,
        metadata: &HashMap<String, Value>,
    ) -> Result<(), anyhow::Error> {
        // Custom async implementation
        Ok(())
    }
}
```

Then use it with any memory type:

```rust
let storage = Box::new(MyCustomStorage { /* ... */ });
let stm = ShortTermMemory::new(None, Some(storage), None, None);
```

---

## Next Steps

- [Knowledge](knowledge.md) -- The RAG-based knowledge system for domain-specific information
- [Events](events.md) -- Memory operations emit events for observability
- [Migration Guide](../migration/python-to-rust.md) -- Migrating memory configuration from Python
