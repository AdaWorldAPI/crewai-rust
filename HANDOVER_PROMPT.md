# Handover Prompt: Continue crewai-rust Development

> **Use this prompt to start a new Claude Code session in `AdaWorldAPI/crewai-rust`.**
> It provides full context for continuing the Rust port of crewAI.

---

## Context

You are continuing development of `crewai-rust`, a Rust port of the Python [crewAI](https://github.com/joaomdmoura/crewAI) multi-agent framework (v1.9.3). The source repo is `AdaWorldAPI/crewAI` (branch `claude/rust-port-HEyAb`), and this target repo is `AdaWorldAPI/crewai-rust`.

### Repository Layout

```
crewai-rust/
├── Cargo.toml              # Workspace: crewai (lib) + crewai-tools (lib)
├── CLAUDE.md               # Detailed implementation guide (READ THIS FIRST)
├── INTEGRATION_STATUS.md   # Current status report with prioritized open points
├── REALITY_CHECK.md        # Audit document from earlier phase
├── src/                    # Main library (crewai crate, ~48K LOC)
│   ├── lib.rs              # 44 module declarations
│   ├── crew.rs             # Crew orchestration (kickoff, execute_tasks)
│   ├── task.rs             # Task execution (execute_sync)
│   ├── process.rs          # Sequential/hierarchical process types
│   ├── llms/providers/     # LLM providers (openai, anthropic, xai, azure, bedrock, gemini)
│   ├── agents/             # Agent executor, parser, tools handler
│   ├── meta_agents/        # Meta-agent orchestration system (8 files, fully implemented)
│   ├── a2a/                # A2A protocol client + types
│   ├── mcp/                # MCP client + transports (partially stubbed)
│   ├── memory/             # Memory storage backends
│   ├── rag/                # RAG pipeline (embeddings, chromadb)
│   ├── knowledge/          # Knowledge sources
│   ├── flow/               # Flow orchestration + persistence
│   ├── tools/              # Tool wrappers
│   ├── events/             # Event bus (COMPLETE - do not modify)
│   ├── policy/             # Policy engine (COMPLETE - do not modify)
│   ├── interfaces/         # Interface definitions (COMPLETE - do not modify)
│   └── capabilities/       # Capability system (COMPLETE - do not modify)
└── lib/crewai-tools-rust/  # Tool implementations crate (~4.2K LOC)
    └── src/tools/          # 57+ tool stubs (search, file_ops, web_scraping, etc.)
```

### What's Working

The **core execution pipeline** is fully implemented and functional:

1. **LLM Providers** (3 of 6):
   - `OpenAIProvider` — real HTTP calls via reqwest, retry logic, function calling
   - `AnthropicProvider` — real HTTP, native tool use, Files API beta support
   - `XaiProvider` — real HTTP, OpenAI-compatible format, live search integration

2. **Agent Executor** (`agents/crew_agent_executor.rs`):
   - `invoke_loop()` dispatches to ReAct or native tool patterns
   - `invoke_loop_react()` — full ReAct loop (Action/Action Input → tool execution → observation → loop)
   - `invoke_loop_native_tools()` — native function calling with tool schemas

3. **Task/Crew Orchestration**:
   - `task.rs execute_sync()` — delegates to agent executor with callbacks
   - `crew.rs kickoff()` → `execute_tasks()` — sequential + hierarchical process

4. **Meta-Agent System** (fully implemented, 8 files):
   - `MetaOrchestrator` — auto-attended controller with event lifecycle
   - `SpawnerAgent` — multi-pass objective decomposition
   - `SkillEngine` — EMA-based proficiency adjustment, auto-discovery, cross-agent transfer
   - `DelegationProtocol` — Request → Dispatch → Response → Result
   - `CardBuilder` — A2A card generation from blueprints/state
   - `Savants` — 7 pre-built domain expert blueprints

5. **A2A Client** — real HTTP: get_agent_card, send_message, send_and_wait, cancel_task

6. **Storage** — RAG in-memory vector search, SQLite long-term memory, flow state persistence

### What's Not Working (Stubs)

1. **3 LLM providers**: Azure (`call()` returns error), Bedrock (error), Gemini (error)
2. **57+ tools**: ALL tools in crewai-tools-rust return `bail!("not yet implemented")`
3. **MCP integration**: Client structure exists but tool execution returns empty/error
4. **Experimental module**: 7 evaluation metrics use `todo!()`
5. **Various TODOs**: ~50 across agent/core.rs, mcp/client.rs, crew.rs, rag/

### Python Reference

For EVERY implementation, reference the corresponding Python source. The original crewAI Python code is at:
- **Source repo**: `https://github.com/AdaWorldAPI/crewAI` branch `claude/rust-port-HEyAb`
- **Python path**: `lib/crewai/src/crewai/` within that repo
- You can fetch specific files via the GitHub API:
  ```
  https://raw.githubusercontent.com/AdaWorldAPI/crewAI/claude/rust-port-HEyAb/lib/crewai/src/crewai/<path>
  ```

Key Python files to reference:
| Python File | Rust Equivalent | LOC |
|-------------|----------------|-----|
| `llms/providers/openai/completion.py` | `src/llms/providers/openai/mod.rs` | 2,301 |
| `llms/providers/anthropic/completion.py` | `src/llms/providers/anthropic/mod.rs` | 1,614 |
| `agents/crew_agent_executor.py` | `src/agents/crew_agent_executor.rs` | 630 |
| `task.py` | `src/task.rs` | 700 |
| `crew.py` | `src/crew.rs` | 1,300 |
| `mcp/` | `src/mcp/` | 1,649 |
| `a2a/` | `src/a2a/` | 10,962 |
| `knowledge/source/` | `src/knowledge/source/` | multiple |
| `utilities/` | `src/utilities/` | multiple |

---

## Prioritized Work Items

### P0 — Critical (Enable End-to-End Agent Workflows)

1. **Implement 6 Priority Tools** (~300 LOC total)
   - `SerperDevTool` — POST to google.serper.dev/search with SERPER_API_KEY
   - `FileReadTool` — std::fs::read_to_string
   - `FileWriterTool` — std::fs::write
   - `DirectoryReadTool` — std::fs::read_dir
   - `ScrapeWebsiteTool` — reqwest GET + strip HTML tags
   - `BraveSearchTool` — GET api.search.brave.com with BRAVE_API_KEY
   - Files: `lib/crewai-tools-rust/src/tools/search/mod.rs`, `tools/file_ops/mod.rs`, `tools/web_scraping/mod.rs`

2. **Wire MCP Client Transport** (~400 LOC)
   - 11 TODOs in `src/mcp/client.rs` blocking MCP tool discovery/execution
   - Implement HTTP transport in `src/mcp/transports/http.rs`
   - Implement SSE transport in `src/mcp/transports/sse.rs`
   - Python ref: `lib/crewai/src/crewai/mcp/`

3. **Fix LLM Capability Detection** (~20 LOC)
   - `src/agent/core.rs:374` — detect if LLM supports native function calling
   - Check provider type or model name to select ReAct vs native tool loop

### P1 — High (Broader Model & Feature Support)

4. **Azure OpenAI Provider** (~400 LOC) — `src/llms/providers/azure/mod.rs`
   - Fork from OpenAI, change URL pattern + api-version query param
   - `https://{resource}.openai.azure.com/openai/deployments/{deployment}/chat/completions?api-version=2024-08-01-preview`

5. **Gemini Provider** (~500 LOC) — `src/llms/providers/gemini/mod.rs`
   - `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`
   - Different request/response format

6. **Bedrock Provider** (~600 LOC) — `src/llms/providers/bedrock/mod.rs`
   - AWS Sig V4 auth required — consider `aws-sigv4` crate
   - Converse API endpoint

7. **Knowledge Integration** (~200 LOC)
   - `src/agent/core.rs:361` — Initialize Knowledge from knowledge_sources
   - `src/agent/utils.rs:138` — Implement actual knowledge retrieval

### P2 — Medium (Feature Completeness)

8. LLM Guardrail (`src/tasks/llm_guardrail.rs`) — second LLM call for output validation
9. Crew Utilities (`src/crews/utils.rs`) — 5 TODOs for crew setup automation
10. Timeout Implementation (`src/agent/core.rs:463`) — tokio::time::timeout
11. Usage Tracking (`src/crew.rs:484`) — aggregate token usage across agents
12. Message Persistence (`src/agent/utils.rs:228`) — save/load message history

### P3 — Low (Nice to Have)

13. Remaining 50+ tools in crewai-tools-rust
14. Experimental evaluation framework (7 evaluators)
15. RAG loaders (10 types), chunkers (4 types)
16. 8 embedding providers
17. ChromaDB integration (7 TODOs)
18. CLI (clap-based), training handler
19. Compiler warning cleanup (62 warnings)

---

## Build & Test Commands

```bash
# Must pass after every change
cargo check --all-features          # 0 errors required
cargo test                          # 192+ tests in crewai-rust

# Integration tests (require API keys)
cargo test --ignored

# Lint
cargo clippy --all-features

# Current state: 0 errors, 62 warnings, 192 tests passing
```

---

## Code Conventions

- **Error handling**: `anyhow::Result` for fallible ops. Never `unwrap()` in library code.
- **Async**: `async fn` with `#[async_trait]` for trait methods. `_sync` wrappers use `tokio::runtime::Runtime::block_on()`.
- **Logging**: `log::debug!`, `log::info!`, `log::warn!`, `log::error!` — not `println!`.
- **Tests**: Every public function gets `#[test]`. Integration tests with API keys use `#[ignore]`.
- **Naming**: Match Python names exactly: `kickoff()` not `kick_off()`, `execute_sync()` not `exec_sync()`.
- **Env vars**: Check in order: struct field → environment variable → error.

## Do NOT Modify

- `events/` module — complete
- `policy/` module — complete
- `interfaces/` module — complete
- `capabilities/` module — complete
- Public API surface (struct fields, trait signatures, module layout)
- Cargo.toml version (stays 1.9.3)

---

## Key Dependencies

```toml
reqwest = { version = "0.12", features = ["json", "stream"] }
tokio = { version = "1", features = ["full"] }
serde / serde_json
rusqlite = { version = "0.32", features = ["bundled"] }
opentelemetry / opentelemetry_sdk = "0.27"
async-trait = "0.1"
futures = "0.3"
anyhow / thiserror
```

Add as needed:
```toml
eventsource-client = "0.13"     # SSE streaming for MCP
tokio-stream = "0.1"            # Stream utilities
csv = "1"                       # CSV knowledge source
```

---

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
```

---

## Getting Started

1. Clone `AdaWorldAPI/crewai-rust` and checkout `claude/rust-port-HEyAb`
2. Read `CLAUDE.md` for the full implementation guide
3. Read `INTEGRATION_STATUS.md` for current state and open points
4. Run `cargo check && cargo test` to verify baseline
5. Start with P0 items — they unblock the most functionality

For Python reference code, fetch from the source repo:
```bash
# Example: get the Python OpenAI provider for reference
curl -H "Accept: application/vnd.github.raw" \
  "https://api.github.com/repos/AdaWorldAPI/crewAI/contents/lib/crewai/src/crewai/llms/providers/openai/completion.py?ref=claude/rust-port-HEyAb"
```

---

*This handover prompt was generated from the `AdaWorldAPI/crewAI` repository, branch `claude/rust-port-HEyAb`, commit `b54640a`.*
