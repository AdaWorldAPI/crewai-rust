# CLAUDE.md — crewai-rust Implementation Completion Guide

## Project Context

**Repository**: `AdaWorldAPI/crewAI` → `lib/crewai-rust/`
**Companion**: `lib/crewai-tools-rust/` (tool implementations)
**Reference**: `lib/crewai/src/crewai/` (Python original, v1.9.3)
**Current state**: 41K LOC Rust scaffolding, 225 files, 176 tests, 0 compile errors, 46 warnings
**Goal**: Make the Rust port functionally equivalent to the Python crewAI framework

## Architecture Overview

```
Crew.kickoff()
  → execute_tasks()          # src/crew.rs — loops tasks sequentially/hierarchically
    → Task.execute_sync()    # src/task.rs — prepares context, delegates to agent
      → Agent.execute_task() # src/agent/core.rs — builds executor, runs
        → CrewAgentExecutor.invoke()  # src/agents/crew_agent_executor.rs
          → invoke_loop()             # THE CORE LOOP
            → LLM.call(messages, tools)  # src/llms/base_llm.rs trait
              → OpenAIProvider.call()    # src/llms/providers/openai/mod.rs
            → parse response → AgentAction or AgentFinish
            → if AgentAction → execute tool → append result → loop
            → if AgentFinish → return output
```

Every `→` in this chain must work end-to-end. Currently, `invoke_loop()` returns `Err("not yet implemented")`.

## Critical Rule: Use Python as Reference

For EVERY implementation, open the corresponding Python file and port the logic faithfully. The Python source is at `lib/crewai/src/crewai/`. The Rust types, traits, and module structure already mirror Python — you are filling in method bodies, not designing new APIs.

Pattern:
```
Python: lib/crewai/src/crewai/llms/providers/openai/completion.py (2,301 lines)
Rust:   lib/crewai-rust/src/llms/providers/openai/mod.rs (406 lines → target ~800-1000)
```

## Dependencies Already in Cargo.toml

```toml
reqwest = { version = "0.12", features = ["json", "stream"] }  # HTTP client — USE THIS
tokio = { version = "1", features = ["full"] }                  # Async runtime
serde / serde_json                                               # Serialization
rusqlite = { version = "0.32", features = ["bundled"] }         # SQLite
opentelemetry / opentelemetry_sdk = "0.27"                       # OTEL
async-trait = "0.1"                                              # Async traits
futures = "0.3"                                                  # Stream combinators
anyhow / thiserror                                               # Error handling
```

Dependencies to ADD to Cargo.toml:
```toml
eventsource-client = "0.13"           # SSE streaming for MCP + LLM streaming
tokio-stream = "0.1"                  # Stream utilities
reqwest-eventsource = "0.6"           # SSE for reqwest (alternative)
csv = "1"                             # CSV knowledge source
```

---

## PHASE 1 — Core Execution Pipeline (P0 Blockers)

### 1.1 OpenAI Provider — `src/llms/providers/openai/mod.rs`

**Python ref**: `lib/crewai/src/crewai/llms/providers/openai/completion.py` (2,301 lines)
**Current Rust**: 406 lines — struct defs, enums, no HTTP calls

**Implement**:

```rust
impl BaseLLM for OpenAIProvider {
    fn call(
        &self,
        messages: Vec<LLMMessage>,
        tools: Option<Vec<Value>>,
        available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        // 1. Build request body:
        //    - model, messages, temperature, max_tokens, top_p
        //    - if tools.is_some() → add "tools" array with function schemas
        //    - if self.response_format is set → add "response_format"
        //
        // 2. POST to https://api.openai.com/v1/chat/completions
        //    Headers: Authorization: Bearer {api_key}, Content-Type: application/json
        //    Use reqwest::blocking::Client (or tokio::runtime::Runtime::block_on for async)
        //
        // 3. Parse response:
        //    - choices[0].message.content → text response
        //    - choices[0].message.tool_calls → Vec<ToolCall> if function calling
        //    - usage → update UsageMetrics
        //
        // 4. If tool_calls present:
        //    - For each tool_call: look up in available_functions, execute, collect results
        //    - Append tool results as messages with role="tool"
        //    - Recurse (call self again with extended messages)
        //
        // 5. Return final text as Value::String or structured output
    }
}
```

**Key details from Python** (`_call_completions` method):
- API URL: `self.base_url` or `https://api.openai.com/v1/chat/completions`
- Retry with exponential backoff on 429/500/503
- Handle `stop` sequences
- Reasoning models (o1, o3, o4-mini): use `max_completion_tokens` not `max_tokens`, no `temperature`
- Streaming: use SSE, yield `delta.content` chunks, collect `delta.tool_calls`
- Track `prompt_tokens`, `completion_tokens`, `cached_tokens` in UsageMetrics

**After OpenAI works**, port Anthropic (`completion.py` 1,614 lines) — same pattern but:
- API URL: `https://api.anthropic.com/v1/messages`
- Header: `x-api-key` instead of `Authorization: Bearer`
- Response format: `content[].text` instead of `choices[].message.content`
- Tool use: `content[].type == "tool_use"` with `input` field

### 1.2 Agent Executor — `src/agents/crew_agent_executor.rs`

**Python ref**: `lib/crewai/src/crewai/agents/crew_agent_executor.py` (lines 302-630)
**Current Rust**: `invoke_loop()` returns Err stub

**Implement `invoke_loop()`**:
```rust
fn invoke_loop(&mut self) -> Result<AgentFinish, Box<dyn std::error::Error + Send + Sync>> {
    // Check if LLM supports native function calling
    // For now, default to ReAct pattern
    self.invoke_loop_react()
}
```

**Implement `invoke_loop_react()`** (Python lines 325-471):
```rust
fn invoke_loop_react(&mut self) -> Result<AgentFinish, ...> {
    loop {
        if self.iterations >= self.max_iter {
            return self.force_final_answer();
        }

        // 1. Build messages from self.messages
        // 2. Call LLM: self.llm.call(messages, None, None)
        // 3. Parse response text for:
        //    - "Final Answer:" → return AgentFinish { output: text_after }
        //    - "Action:" + "Action Input:" → AgentAction { tool, tool_input }
        // 4. If AgentAction:
        //    a. Look up tool in self.tools_handler
        //    b. Execute tool with input
        //    c. Create observation message
        //    d. Append to self.messages:
        //       - Assistant message with the action text
        //       - Observation message with tool result
        // 5. Increment self.iterations
        // 6. Invoke step_callback if set
    }
}
```

**Implement `invoke_loop_native_tools()`** (Python lines 473-596):
```rust
fn invoke_loop_native_tools(&mut self) -> Result<AgentFinish, ...> {
    loop {
        if self.iterations >= self.max_iter {
            return self.force_final_answer();
        }

        // 1. Format tools as OpenAI function schemas
        let tool_schemas = self.format_tools_for_llm();

        // 2. Call LLM with tool schemas
        let response = self.llm.call(messages, Some(tool_schemas), None)?;

        // 3. Check response:
        //    - If response has tool_calls → execute each, append results, loop
        //    - If response has content only → AgentFinish
        //    - If response has both → execute tools, then check if done

        self.iterations += 1;
    }
}
```

**Parsing logic** — the ReAct parser (Python `agents/parser.py`):
```
Thought: <reasoning>
Action: <tool_name>
Action Input: <json_or_text>
```
or
```
Thought: <reasoning>
Final Answer: <the_answer>
```
Use regex: `r"Action\s*:\s*(.+?)\n"` and `r"Action\s*Input\s*:\s*(.+)"` (with dotall for multiline input).
The Rust file `src/agents/parser.rs` has the struct — fill the `parse()` method.

### 1.3 Task Execution — `src/task.rs`

**Python ref**: `lib/crewai/src/crewai/task.py` (lines 493-700)
**Current Rust**: `execute_sync()` returns placeholder TaskOutput

**Implement** (the existing method body has the right setup — extend after the agent role resolution):
```rust
pub fn execute_sync(&mut self, agent: Option<&str>, context: Option<&str>, tools: Option<&[String]>) -> Result<TaskOutput, String> {
    // ... existing agent validation code ...

    // NEW: Actually execute via agent
    // 1. Build task prompt: self.prompt() + context
    // 2. If self.human_input → ask for feedback loop
    // 3. Call agent.execute_task(prompt, tools, context) ← this is where the executor runs
    // 4. Parse result into TaskOutput { raw, pydantic, json_dict, agent, output_format }
    // 5. Store in self.output
    // 6. Run output guardrails if configured (self.guardrail)
    // 7. Emit TaskCompletedEvent via event bus
}
```

### 1.4 Crew Orchestration — `src/crew.rs`

**Python ref**: `lib/crewai/src/crewai/crew.py` (lines 695-1300)
**Current Rust**: `kickoff()` calls `execute_tasks()` which works but relies on task stubs

The `execute_tasks()` loop is already mostly correct. Main additions:
- Add `kickoff_for_each()` — iterate over a list of inputs, run crew per input
- Add `_run_hierarchical_process()` — create a manager agent that delegates
- Wire up `before_kickoff_callbacks` and `after_kickoff_callbacks`
- Emit `CrewStartedEvent`, `CrewFinishedEvent` via event bus
- Handle `self.memory` — save/load task outputs to memory backends
- Handle `self.planning` — if enabled, create a planning agent first

---

## PHASE 2 — Storage & I/O

### 2.1 MCP Transport — `src/mcp/transports/`

**Python ref**: `lib/crewai/src/crewai/mcp/` (1,649 lines)

**HTTP Transport** (`transports/http.rs`):
```rust
impl McpTransport for HttpTransport {
    async fn connect(&mut self) -> Result<()> {
        // POST to {base_url}/initialize with protocol version
        // Store session_id from response
    }

    async fn call_tool(&self, name: &str, args: Value) -> Result<Value> {
        // POST to {base_url}/tools/call
        // Body: { "name": name, "arguments": args }
        // Parse response content
    }

    async fn list_tools(&self) -> Result<Vec<McpTool>> {
        // GET {base_url}/tools/list
        // Parse into Vec<McpTool>
    }
}
```

**SSE Transport** (`transports/sse.rs`):
```rust
// Use eventsource-client or reqwest-eventsource
// Connect to SSE endpoint, parse event stream
// Handle: tool_call_result, progress, error events
```

**Stdio Transport** (`transports/stdio.rs`):
```rust
// Use tokio::process::Command to spawn MCP server
// Read/write JSON-RPC over stdin/stdout
// Use BufReader for line-based parsing
```

### 2.2 Memory Backends — `src/memory/storage/`

**RAG Storage** (`rag_storage.rs`):
```rust
impl Storage for RAGStorage {
    fn save(&self, value: &str, metadata: &HashMap<String, Value>) -> Result<()> {
        // 1. Generate embedding via configured embedder
        // 2. Store in vector DB (chromadb, qdrant, etc.)
        // For MVP: use a simple in-memory Vec<(String, Vec<f32>, HashMap)>
    }

    fn search(&self, query: &str, limit: usize, score_threshold: f64) -> Result<Vec<Value>> {
        // 1. Generate query embedding
        // 2. Cosine similarity search
        // 3. Filter by score_threshold
        // 4. Return top-k
    }
}
```

**SQLite LTM** (`ltm_sqlite_storage.rs`):
```rust
impl Storage for LtmSqliteStorage {
    fn save(&self, value: &str, metadata: &HashMap<String, Value>) -> Result<()> {
        // rusqlite: INSERT INTO long_term_memories (content, metadata, created_at)
    }

    fn search(&self, query: &str, limit: usize, _score: f64) -> Result<Vec<Value>> {
        // SELECT * FROM long_term_memories WHERE content LIKE '%query%' LIMIT limit
        // For better: use FTS5 virtual table
    }
}
```

**Schema** (create on first use):
```sql
CREATE TABLE IF NOT EXISTS long_term_memories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT NOT NULL,
    metadata TEXT,  -- JSON
    agent TEXT,
    task_description TEXT,
    quality REAL DEFAULT 0.0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE VIRTUAL TABLE IF NOT EXISTS ltm_fts USING fts5(content, metadata);
```

### 2.3 Knowledge Sources — `src/knowledge/source/mod.rs`

**Python ref**: `lib/crewai/src/crewai/knowledge/source/` (10 source types)

Implement at minimum:
- `StringKnowledgeSource` — direct string input, chunk by paragraphs
- `TextFileKnowledgeSource` — read .txt file, chunk
- `CsvKnowledgeSource` — use `csv` crate, each row = document
- `JsonKnowledgeSource` — parse JSON, each object = document

Pattern for all:
```rust
impl BaseKnowledgeSource for TextFileKnowledgeSource {
    fn load(&self) -> Result<Vec<String>> {
        let content = std::fs::read_to_string(&self.file_path)?;
        Ok(self.chunk_text(&content))
    }

    fn chunk_text(&self, text: &str) -> Vec<String> {
        // Split by double newline or fixed chunk size (default 4000 chars, 200 overlap)
        // Return Vec of chunks
    }
}
```

### 2.4 Flow Persistence — `src/flow/persistence/mod.rs`

**Python ref**: `lib/crewai/src/crewai/flow/persistence/` (SQLite-backed)

```rust
pub struct SqliteFlowPersistence {
    conn: rusqlite::Connection,
}

impl FlowPersistence for SqliteFlowPersistence {
    fn save_state(&self, flow_id: &str, state: &Value) -> Result<()> {
        // INSERT OR REPLACE INTO flow_states (flow_id, state_json, updated_at)
    }

    fn load_state(&self, flow_id: &str) -> Result<Option<Value>> {
        // SELECT state_json FROM flow_states WHERE flow_id = ?
    }

    fn list_flows(&self) -> Result<Vec<String>> {
        // SELECT DISTINCT flow_id FROM flow_states
    }
}
```

---

## PHASE 3 — Integrations

### 3.1 A2A Client — `src/a2a/client.rs`

**Python ref**: `lib/crewai/src/crewai/a2a/` (10,962 lines)

Implement the 4 stub methods:
```rust
impl A2AClient {
    pub async fn get_agent_card(&self) -> Result<AgentCard> {
        // GET {self.endpoint}/.well-known/agent.json
        let resp = reqwest::get(format!("{}/.well-known/agent.json", self.endpoint)).await?;
        Ok(resp.json::<AgentCard>().await?)
    }

    pub async fn send_message(&self, message: A2AMessage) -> Result<A2AResponse> {
        // POST {self.endpoint}/a2a/messages
        // Body: JSON message
        // Handle auth: self.auth.apply_to_request(&mut req)
    }

    pub async fn send_and_wait(&self, message: A2AMessage) -> Result<A2AResponse> {
        // send_message + poll for completion
    }

    pub async fn cancel_task(&self, task_id: &str) -> Result<()> {
        // POST {self.endpoint}/a2a/tasks/{task_id}/cancel
    }
}
```

### 3.2 Telemetry OTEL — `src/telemetry/mod.rs`

Wire the existing `opentelemetry` dependency:
```rust
impl Telemetry {
    pub fn initialize(&mut self) -> Result<()> {
        use opentelemetry_sdk::trace::TracerProvider;
        // 1. Create TracerProvider with OTLP exporter (if OTEL_EXPORTER_OTLP_ENDPOINT is set)
        //    or stdout exporter (for dev)
        // 2. Register global provider
        // 3. Set self.ready = true, self.trace_set = true
    }

    pub fn crew_creation(&self, crew_id: &str, agent_count: usize) {
        // Create span "crew.creation" with attributes
    }

    pub fn task_started(&self, task_id: &str, agent: &str) {
        // Create span "task.execution"
    }

    pub fn tool_usage(&self, tool_name: &str, agent: &str, attempts: u32) {
        // Create span "tool.usage"
    }
}
```

### 3.3 Additional LLM Providers

After OpenAI, implement in priority order:

1. **Anthropic** (`providers/anthropic/mod.rs`, 325 LOC → ~600)
   - Python ref: `completion.py` (1,614 lines)
   - API: `https://api.anthropic.com/v1/messages`
   - Key differences: `x-api-key` header, `content[]` array response, `tool_use` blocks

2. **Azure OpenAI** (`providers/azure/mod.rs`, 211 LOC → ~500)
   - Same as OpenAI but with Azure-specific URL pattern and API version query param
   - `https://{resource}.openai.azure.com/openai/deployments/{deployment}/chat/completions?api-version=2024-08-01-preview`

3. **Bedrock** (`providers/bedrock/mod.rs`, 266 LOC → ~500)
   - AWS Signature V4 auth
   - Converse API endpoint

4. **Gemini** (`providers/gemini/mod.rs`, 259 LOC → ~500)
   - `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`
   - Different tool calling format

---

## PHASE 4 — Tool Implementations (`lib/crewai-tools-rust/`)

Every tool `run()` currently returns `bail!("not yet implemented")`.

### Priority tools (enable basic agent workflows):

**SerperDevTool** (`tools/search/mod.rs`):
```rust
pub fn run(&self, args: HashMap<String, Value>) -> Result<Value> {
    let query = args.get("search_query").and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing search_query"))?;
    let api_key = self.api_key.as_ref()
        .or_else(|| std::env::var("SERPER_API_KEY").ok().as_ref())
        .ok_or_else(|| anyhow::anyhow!("Missing SERPER_API_KEY"))?;

    let client = reqwest::blocking::Client::new();
    let resp = client.post("https://google.serper.dev/search")
        .header("X-API-KEY", api_key)
        .json(&serde_json::json!({ "q": query, "num": self.max_results }))
        .send()?
        .json::<Value>()?;

    Ok(resp)
}
```

**FileReadTool** (`tools/file_ops/mod.rs`):
```rust
pub fn run(&self, args: HashMap<String, Value>) -> Result<Value> {
    let path = args.get("file_path").and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing file_path"))?;
    let content = std::fs::read_to_string(path)?;
    Ok(Value::String(content))
}
```

**ScrapeWebsiteTool** (`tools/web_scraping/mod.rs`):
```rust
pub fn run(&self, args: HashMap<String, Value>) -> Result<Value> {
    let url = args.get("website_url").and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing website_url"))?;
    let client = reqwest::blocking::Client::new();
    let body = client.get(url).send()?.text()?;
    // Strip HTML tags for plain text (basic: regex, better: use scraper crate)
    let text = regex::Regex::new(r"<[^>]+>")?.replace_all(&body, " ").to_string();
    Ok(Value::String(text))
}
```

### Remaining tools — implement using same pattern:
- Each tool's `run()` makes HTTP call to its respective API
- API key from struct field or environment variable
- Parse response into `Value`
- Return `Ok(Value)` or descriptive error

---

## PHASE 5 — Polish & Hardening

### 5.1 Guardrail Enforcement (`src/tasks/hallucination_guardrail.rs`, `llm_guardrail.rs`)
- Call LLM to evaluate output against task description
- Return `GuardrailResult { passed, feedback }`
- Python uses a second LLM call with specific evaluation prompts

### 5.2 Agent Utilities (`src/utilities/`)
Missing implementations:
- `agent_utils.rs` — reasoning handler integration (Python: 1,066 lines)
- `converter.rs` — output format conversion (Python: 428 lines)
- `streaming.rs` — streaming utilities (Python: 297 lines)

### 5.3 Training Handler (`src/utilities/training_handler.rs`)
- Serialize agent execution traces to YAML
- Load training data to improve prompts

### 5.4 Hooks (`src/hooks/mod.rs`)
Wire hook system to event bus — hooks should fire on agent/task lifecycle events.

### 5.5 CLI (`src/cli/mod.rs`)
Low priority. Add basic `create`, `run`, `train` commands using `clap`.

---

## Testing Strategy

### Existing: 176 `#[test]` functions (mostly struct construction and config validation)

### Required new tests:

**Integration tests** (create `tests/` directory):
```rust
#[tokio::test]
async fn test_openai_provider_call() {
    // Requires OPENAI_API_KEY env var
    let provider = OpenAIProvider::new("gpt-4o-mini");
    let messages = vec![LLMMessage::user("Say hello")];
    let result = provider.call(messages, None, None);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_crew_sequential_execution() {
    let researcher = Agent::new("researcher", "Research assistant", "gpt-4o-mini");
    let task = Task::new("What is Rust?", Some("researcher"));
    let mut crew = Crew::new(vec![researcher], vec![task]);
    let result = crew.kickoff(None);
    assert!(result.is_ok());
    assert!(!result.unwrap().raw.is_empty());
}

#[test]
fn test_react_parser() {
    let input = "Thought: I need to search\nAction: search\nAction Input: Rust programming";
    let parsed = parse_agent_output(input);
    assert!(matches!(parsed, AgentOutput::Action { .. }));
}
```

---

## File-by-File Implementation Checklist

### P0 — Must complete (4 files, ~2,500 LOC estimated)
- [ ] `src/llms/providers/openai/mod.rs` — implement `call()`, `acall()`, streaming
- [ ] `src/agents/crew_agent_executor.rs` — implement `invoke_loop()`, `invoke_loop_react()`, `invoke_loop_native_tools()`
- [ ] `src/task.rs` — implement `execute_sync()` body (delegate to agent executor)
- [ ] `src/agents/parser.rs` — implement ReAct output parser

### P1 — Core functionality (10 files, ~3,000 LOC)
- [ ] `src/llms/providers/anthropic/mod.rs` — implement `call()`
- [ ] `src/mcp/transports/http.rs` — implement HTTP transport with reqwest
- [ ] `src/mcp/transports/sse.rs` — implement SSE transport
- [ ] `src/mcp/transports/stdio.rs` — implement subprocess transport
- [ ] `src/mcp/client.rs` — wire `connect()`, `call_tool()`, `list_tools()`
- [ ] `src/memory/storage/rag_storage.rs` — in-memory vector search MVP
- [ ] `src/memory/storage/ltm_sqlite_storage.rs` — SQLite via rusqlite
- [ ] `src/knowledge/source/mod.rs` — StringSource, TextFileSource, CsvSource
- [ ] `src/flow/persistence/mod.rs` — SQLite flow state persistence
- [ ] `src/a2a/client.rs` — HTTP calls for 4 methods

### P1 — Tools (8 files, ~2,000 LOC)
- [ ] `lib/crewai-tools-rust/src/tools/search/mod.rs` — SerperDevTool, BraveSearchTool
- [ ] `lib/crewai-tools-rust/src/tools/file_ops/mod.rs` — FileReadTool, FileWriterTool, DirectoryReadTool
- [ ] `lib/crewai-tools-rust/src/tools/web_scraping/mod.rs` — ScrapeWebsiteTool
- [ ] `lib/crewai-tools-rust/src/tools/database/mod.rs` — at least one vector search tool
- [ ] `lib/crewai-tools-rust/src/tools/ai_ml/mod.rs` — RagTool, DalleTool
- [ ] `lib/crewai-tools-rust/src/tools/browser/mod.rs` — BrowserbaseLoadTool
- [ ] `lib/crewai-tools-rust/src/tools/automation/mod.rs` — ComposioTool
- [ ] `lib/crewai-tools-rust/src/tools/cloud_storage/mod.rs` — S3ReaderTool

### P2 — Polish (8 files, ~1,500 LOC)
- [ ] `src/llms/providers/azure/mod.rs` — Azure OpenAI
- [ ] `src/llms/providers/bedrock/mod.rs` — AWS Bedrock
- [ ] `src/llms/providers/gemini/mod.rs` — Google Gemini
- [ ] `src/telemetry/mod.rs` — OTEL initialization
- [ ] `src/tasks/hallucination_guardrail.rs` — LLM-based guardrail
- [ ] `src/tasks/llm_guardrail.rs` — content guardrail
- [ ] `src/utilities/training_handler.rs` — YAML training data
- [ ] `src/cli/mod.rs` — clap-based CLI

---

## Code Style & Conventions

- **Error handling**: Use `anyhow::Result` for fallible operations. Never `unwrap()` in library code.
- **Async**: Use `async fn` with `#[async_trait]` for trait methods. All I/O (HTTP, file, DB) should be async where possible. Provide `_sync` wrappers using `tokio::runtime::Runtime::block_on()` for the synchronous API surface.
- **Logging**: Use `log::debug!`, `log::info!`, `log::warn!`, `log::error!` — not `println!`.
- **Tests**: Every new public function gets a `#[test]`. Integration tests requiring API keys use `#[ignore]` and document the required env var.
- **Documentation**: Every public type and function has `///` doc comments with at least one usage example.
- **Naming**: Match Python names exactly where possible. `kickoff()` not `kick_off()`, `execute_sync()` not `exec_sync()`.
- **Env vars**: API keys always checked in order: struct field → environment variable → error.
- **Warnings**: Fix the 46 existing warnings (38 unused imports, 5 unused vars, 3 dead code) as you touch each file.

## Environment Variables

```bash
OPENAI_API_KEY        # OpenAI provider
ANTHROPIC_API_KEY     # Anthropic provider  
AZURE_OPENAI_API_KEY  # Azure provider
AZURE_OPENAI_ENDPOINT # Azure resource URL
SERPER_API_KEY        # SerperDevTool
BRAVE_API_KEY         # BraveSearchTool
AWS_ACCESS_KEY_ID     # Bedrock provider
AWS_SECRET_ACCESS_KEY # Bedrock provider
GOOGLE_API_KEY        # Gemini provider
OTEL_EXPORTER_OTLP_ENDPOINT  # Optional OTEL collector
```

## Validation

After each phase, run:
```bash
cargo check --all-features          # Must compile with 0 errors
cargo test                          # All existing 176 tests must pass
cargo test --ignored                # Integration tests (with API keys)
cargo clippy --all-features         # No new warnings
```

## What NOT to Change

- Do not modify the existing public API surface (struct fields, trait signatures, module layout)
- Do not remove or rename existing types — only add implementations
- Do not change Cargo.toml version (stays 1.9.3)
- Do not add dependencies without documenting them above
- Do not refactor the module structure — it mirrors Python intentionally
- The `policy/`, `interfaces/`, `capabilities/` modules are COMPLETE — do not touch
- The `events/` module is COMPLETE — do not modify event types
