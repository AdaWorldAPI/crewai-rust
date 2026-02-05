# Knowledge

The knowledge system enables crewAI agents to access domain-specific information through a RAG (Retrieval-Augmented Generation) pipeline. Knowledge sources are ingested, chunked, optionally embedded, and stored in a searchable vector backend. The Rust port faithfully mirrors the Python architecture while expressing the source abstraction as `async_trait`-based traits, using `Arc<KnowledgeStorage>` for shared ownership, and providing builder patterns for ergonomic configuration.

**Source**: `src/knowledge/` (corresponds to Python `crewai/knowledge/`)

---

## Architecture Overview

```
Knowledge (manager struct)
  |
  +-- sources: Vec<Box<dyn BaseKnowledgeSource>>
  |     +-- StringKnowledgeSource       -- Raw string content
  |     +-- TextFileKnowledgeSource     -- Plain text files
  |     +-- CSVKnowledgeSource          -- CSV files (row-per-chunk)
  |     +-- JSONKnowledgeSource         -- JSON files (recursive flattening)
  |     +-- PDFKnowledgeSource          -- PDF files (stub -- needs pdf-extract)
  |     +-- ExcelKnowledgeSource        -- Excel files (stub -- needs calamine)
  |     +-- BaseFileKnowledgeSource     -- Trait for file-based sources
  |
  +-- storage: Arc<KnowledgeStorage>
        +-- implements BaseKnowledgeStorage (trait)
        +-- search(), save(), save_chunks(), reset()
        +-- Delegates to RAG vector backend
```

---

## The `Knowledge` Struct

`Knowledge` is the top-level manager that holds a list of sources and a shared storage backend.

```rust
use crewai::knowledge::{Knowledge, StringKnowledgeSource, KnowledgeStorage};

// Create a string-based knowledge source
let source = StringKnowledgeSource::new(
    "Rust is a systems programming language focused on safety, \
     concurrency, and performance. It achieves memory safety \
     without garbage collection."
        .to_string(),
);

// Create the knowledge manager
let knowledge = Knowledge::new(
    vec![Box::new(source)],               // sources
    None,                                  // embedder_config
    Some("rust_docs".to_string()),        // collection_name
    None,                                  // pre-configured storage
);
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `sources` | `Vec<Box<dyn BaseKnowledgeSource>>` | Knowledge sources to manage |
| `storage` | `Arc<KnowledgeStorage>` | Shared storage backend |
| `embedder_config` | `Option<Value>` | Embedder provider configuration |
| `collection_name` | `Option<String>` | Collection name (default: `"knowledge"`) |

### Querying Knowledge

```rust
// Synchronous query
let results = knowledge.query(
    "What is Rust?",  // query string
    Some(5),          // max results (default: 3)
    Some(0.5),        // min score threshold (default: 0.35)
)?;

for result in &results {
    println!("Match: {}", result);
}

// Async query
let results = knowledge.aquery("memory safety", None, None).await?;
```

### Ingesting Sources

```rust
// Add all configured sources to the storage (sync)
knowledge.add_sources()?;

// Async ingestion
knowledge.aadd_sources().await?;
```

### Resetting Knowledge

```rust
// Clear all stored knowledge
knowledge.reset()?;
```

---

## The `BaseKnowledgeSource` Trait

All knowledge sources implement this trait. It defines the lifecycle for loading content, chunking it, and saving it to storage.

```rust
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;

#[async_trait]
pub trait BaseKnowledgeSource: Send + Sync {
    /// Human-readable source name (for debugging / logging).
    fn source_name(&self) -> &str;

    /// Validate the content source (e.g., check file existence).
    /// Default: always succeeds.
    fn validate_content(&self) -> Result<(), anyhow::Error> { Ok(()) }

    /// Load content from the source, returning text chunks.
    fn load_content(&self) -> Result<Vec<String>, anyhow::Error>;

    /// Add loaded content to knowledge storage (sync).
    fn add(&self, storage: &Arc<KnowledgeStorage>) -> Result<(), anyhow::Error>;

    /// Add loaded content to knowledge storage (async).
    /// Default: delegates to sync `add()`.
    async fn aadd(&self, storage: &Arc<KnowledgeStorage>) -> Result<(), anyhow::Error> {
        self.add(storage)
    }

    /// Optional metadata for this source.
    fn metadata(&self) -> HashMap<String, Value> { HashMap::new() }

    /// Chunk text using a sliding window approach.
    fn chunk_text(
        &self,
        text: &str,
        chunk_size: Option<usize>,     // default: 4000
        chunk_overlap: Option<usize>,  // default: 200
    ) -> Vec<String>;
}
```

### Key Rust Design Choices

- **`Send + Sync`**: Sources must be thread-safe since `Knowledge` may add sources from async contexts.
- **`Arc<KnowledgeStorage>`**: Storage is shared via `Arc` so multiple sources can write to the same backend concurrently.
- **`async_trait`**: The `aadd` method enables async ingestion pipelines.
- **Default chunk parameters**: `chunk_size = 4000`, `chunk_overlap = 200` -- matching the Python defaults.

---

## Built-in Knowledge Sources

### `StringKnowledgeSource`

The simplest source -- raw string content. Supports builder-style configuration.

```rust
use crewai::knowledge::source::StringKnowledgeSource;
use std::collections::HashMap;
use serde_json::Value;

let source = StringKnowledgeSource::new(
    "Detailed content about your domain...".to_string(),
)
.with_chunking(2000, 100)           // custom chunk size and overlap
.with_metadata({
    let mut m = HashMap::new();
    m.insert("source".to_string(), Value::from("internal_docs"));
    m
})
.with_collection_name("my_collection".to_string());
```

### `TextFileKnowledgeSource`

Reads and ingests the contents of one or more plain text files.

```rust
use crewai::knowledge::source::TextFileKnowledgeSource;
use std::path::PathBuf;

let source = TextFileKnowledgeSource::new(vec![
    PathBuf::from("docs/guide.txt"),
    PathBuf::from("docs/reference.txt"),
]);

// Validates that files exist
source.validate_paths()?;

// Load and chunk content
let chunks = source.load_content()?;
```

### `CSVKnowledgeSource`

Each row of the CSV becomes a separate chunk.

```rust
use crewai::knowledge::source::CSVKnowledgeSource;
use std::path::PathBuf;

let source = CSVKnowledgeSource::new(vec![
    PathBuf::from("data/products.csv"),
]);
```

### `JSONKnowledgeSource`

Recursively flattens JSON structures into readable text, then chunks and ingests them.

```rust
use crewai::knowledge::source::JSONKnowledgeSource;
use std::path::PathBuf;

let source = JSONKnowledgeSource::new(vec![
    PathBuf::from("data/config.json"),
]);

// The JSON flattener converts nested structures:
// {"name": "Alice", "age": 30} -> "name: Alice\nage: 30"
```

### `PDFKnowledgeSource` (Stub)

Structure is in place but requires a PDF parsing crate like `pdf-extract` or `lopdf`.

```rust
use crewai::knowledge::source::PDFKnowledgeSource;
use std::path::PathBuf;

let source = PDFKnowledgeSource::new(vec![
    PathBuf::from("docs/whitepaper.pdf"),
]);
// Currently returns an error explaining the dependency requirement
```

### `ExcelKnowledgeSource` (Stub)

Structure is in place but requires an Excel parsing crate like `calamine`.

```rust
use crewai::knowledge::source::ExcelKnowledgeSource;
use std::path::PathBuf;

let source = ExcelKnowledgeSource::new(vec![
    PathBuf::from("data/financials.xlsx"),
]);
// Currently returns an error explaining the dependency requirement
```

---

## The `BaseFileKnowledgeSource` Trait

File-based sources extend `BaseKnowledgeSource` with path validation.

```rust
#[async_trait]
pub trait BaseFileKnowledgeSource: BaseKnowledgeSource {
    /// Get the file paths for this source.
    fn file_paths(&self) -> &[PathBuf];

    /// Validate that all file paths exist and are accessible.
    fn validate_paths(&self) -> Result<(), anyhow::Error> {
        for path in self.file_paths() {
            if !path.exists() {
                return Err(anyhow::anyhow!(
                    "File not found: {}", path.display()
                ));
            }
        }
        Ok(())
    }
}
```

Implemented by: `TextFileKnowledgeSource`, `CSVKnowledgeSource`, `JSONKnowledgeSource`, `PDFKnowledgeSource`, `ExcelKnowledgeSource`.

---

## The `BaseKnowledgeStorage` Trait

Storage backends for knowledge implement this trait, which is distinct from the `memory::storage::Storage` trait because knowledge storage has chunk-aware methods.

```rust
#[async_trait]
pub trait BaseKnowledgeStorage: Send + Sync {
    /// Search for relevant content.
    fn search(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error>;

    /// Save raw document strings.
    fn save(&self, documents: &[String]) -> Result<(), anyhow::Error>;

    /// Save text chunks with metadata.
    fn save_chunks(
        &self,
        chunks: &[String],
        metadata: &HashMap<String, Value>,
    ) -> Result<(), anyhow::Error>;

    /// Reset the storage (remove all data).
    fn reset(&self) -> Result<(), anyhow::Error>;

    // Async variants with default sync delegation:
    async fn asearch(...) -> Result<Vec<Value>, anyhow::Error>;
    async fn asave(...) -> Result<(), anyhow::Error>;
    async fn asave_chunks(...) -> Result<(), anyhow::Error>;
    async fn areset(...) -> Result<(), anyhow::Error>;
}
```

### `KnowledgeStorage` -- The Default Implementation

```rust
use crewai::knowledge::storage::KnowledgeStorage;

let storage = KnowledgeStorage::new(
    Some(serde_json::json!({
        "provider": "openai",
        "config": { "model": "text-embedding-3-small" }
    })),
    Some("my_collection".to_string()),
);

// Collection names are prefixed: "knowledge_my_collection"
// Default: "knowledge" (when no collection_name is set)
assert_eq!(storage.effective_collection_name(), "knowledge_my_collection");
```

Configuration:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `embedder_config` | `Option<Value>` | `None` | Embedder provider spec |
| `collection_name` | `Option<String>` | `None` | Collection name (prefixed with `"knowledge_"`) |
| `default_limit` | `usize` | `5` | Max results per query |
| `default_score_threshold` | `f64` | `0.6` | Min similarity score |

---

## `KnowledgeConfig`

Controls chunking and query parameters at the crew or agent level.

```rust
use crewai::knowledge::KnowledgeConfig;

let config = KnowledgeConfig {
    results_limit: 5,          // Max results per query (default: 3)
    score_threshold: 0.5,      // Min similarity score (default: 0.35)
    chunk_size: 4000,          // Text chunk size for ingestion
    chunk_overlap: 200,        // Overlap between chunks
};
```

---

## Chunking Algorithm

The `chunk_text` method uses a sliding window approach:

1. If the text length is less than or equal to `chunk_size`, return it as a single chunk
2. Otherwise, slide a window of size `chunk_size` across the text with steps of `chunk_size - chunk_overlap`
3. Each window becomes a chunk

```
Text: [==================================]
Chunk 1: [========]
Chunk 2:      [========]     (overlap)
Chunk 3:           [========]
```

This is a default trait method, so all sources share the same chunking logic unless they override it.

---

## Using Knowledge with Agents

```rust
use crewai::agents::Agent;

let mut agent = Agent::new(
    "Domain Expert".to_string(),
    "Answer questions using the knowledge base".to_string(),
    "Expert with access to internal documentation".to_string(),
);

agent.knowledge_sources = Some(vec![
    // Knowledge source configurations as JSON
]);

agent.embedder = Some(serde_json::json!({
    "provider": "openai",
    "config": { "model": "text-embedding-3-small" }
}));
```

## Using Knowledge with Crews

```rust
use crewai::crew::Crew;

let mut crew = Crew::new(tasks, agents);
crew.knowledge_sources = Some(vec![/* source configs */]);
crew.embedder = Some(serde_json::json!({
    "provider": "openai",
    "config": { "model": "text-embedding-3-small" }
}));
```

---

## Implementing a Custom Knowledge Source

```rust
use async_trait::async_trait;
use crewai::knowledge::source::{BaseKnowledgeSource, KnowledgeStorage};
use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;

pub struct APIKnowledgeSource {
    pub api_url: String,
    pub metadata: HashMap<String, Value>,
}

#[async_trait]
impl BaseKnowledgeSource for APIKnowledgeSource {
    fn source_name(&self) -> &str {
        "APIKnowledgeSource"
    }

    fn validate_content(&self) -> Result<(), anyhow::Error> {
        if self.api_url.is_empty() {
            return Err(anyhow::anyhow!("API URL is required"));
        }
        Ok(())
    }

    fn load_content(&self) -> Result<Vec<String>, anyhow::Error> {
        // Fetch from API and chunk
        let response = reqwest::blocking::get(&self.api_url)?
            .text()?;
        Ok(self.chunk_text(&response, Some(4000), Some(200)))
    }

    fn add(&self, storage: &Arc<KnowledgeStorage>) -> Result<(), anyhow::Error> {
        self.validate_content()?;
        let chunks = self.load_content()?;
        storage.save_chunks(&chunks, &self.metadata)
    }

    fn metadata(&self) -> HashMap<String, Value> {
        self.metadata.clone()
    }
}
```

---

## Python Source Types vs. Rust

| Python Source | Rust Equivalent | Status |
|---------------|-----------------|--------|
| `StringKnowledgeSource` | `StringKnowledgeSource` | Complete |
| `TextFileKnowledgeSource` | `TextFileKnowledgeSource` | Complete |
| `CSVKnowledgeSource` | `CSVKnowledgeSource` | Complete |
| `JSONKnowledgeSource` | `JSONKnowledgeSource` | Complete |
| `PDFKnowledgeSource` | `PDFKnowledgeSource` | Stub (needs `pdf-extract`) |
| `ExcelKnowledgeSource` | `ExcelKnowledgeSource` | Stub (needs `calamine`) |
| `CrewDoclingSource` | -- | Not yet ported |
| `GitHubKnowledgeSource` | -- | Not yet ported |
| `YouTubeKnowledgeSource` | -- | Not yet ported |

---

## Next Steps

- [Memory](memory.md) -- The memory system that uses similar storage abstractions
- [Tools](tools.md) -- Knowledge can be queried through tool interfaces
- [Custom Tools Guide](../guides/custom-tools.md) -- Build a tool that queries your knowledge base
