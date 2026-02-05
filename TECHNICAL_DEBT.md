# crewAI-rust Technical Debt Report

**Version**: 1.9.3 (Rust port)
**Date**: 2026-02-05
**Source**: crewAI Python v1.9.3 (483 files, ~3.7 MB)
**Target**: crewAI-rust (215 files, ~39K lines)

---

## Overview

The Rust port covers all 25+ top-level Python modules with data structures,
trait definitions, and configuration logic. Compilation succeeds with zero
errors. However, many runtime paths remain stub implementations that need
provider integrations and I/O backends to become functional.

This report categorises every gap as **P0** (blocks execution),
**P1** (limits functionality), or **P2** (polish / enhancement).

---

## P0 - Blocking: Must Implement for Any Agent Execution

### 1. LLM Provider Integration (`src/llm/mod.rs`, `src/llms/providers/`)

- `LLM::call()` and `LLM::acall()` return `unimplemented!`
- Provider routing (`infer_provider()`) resolves to no-ops
- Affected providers: OpenAI, Anthropic, Azure, Bedrock, Gemini
- **Impact**: No agent can reason, plan, or produce output

### 2. Agent Executor (`src/agents/crew_agent_executor.rs`)

- `CrewAgentExecutor` struct defined with all fields but no `invoke()` logic
- The ReAct / tool-use loop that drives agent behaviour is absent
- **Impact**: Agents cannot execute tasks, use tools, or produce results

### 3. Crew Execution (`src/crew.rs`)

- `Crew` struct has all configuration fields (488 lines) but no `kickoff()`,
  `kickoff_async()`, or process orchestration methods
- Python: 2054 lines of orchestration logic
- **Impact**: Crews cannot run; entire framework is non-operational

### 4. Task Execution (`src/task.rs`)

- `Task` struct fields and type aliases present but no
  `execute_sync()`, `execute_async()`, output formatting, or guardrail hooks
- **Impact**: Tasks cannot produce `TaskOutput`

---

## P1 - Functional: Limits Major Features

### 5. MCP Transport Layer (`src/mcp/transports/`)

| Transport | Status |
|-----------|--------|
| HTTP      | Struct defined, no `reqwest` calls |
| SSE       | Struct defined, no event-stream parsing |
| Stdio     | Struct defined, no subprocess spawning |

- `MCPClient::connect()` / `MCPClient::call_tool()` are stubs
- **Impact**: Agents cannot use MCP-connected tools

### 6. Memory Backends (`src/memory/storage/`)

| Backend | Python File | Rust Status |
|---------|------------|-------------|
| RAG Storage | rag_storage.py | Stub (no vector DB calls) |
| SQLite LTM | ltm_sqlite_storage.py | Stub (no rusqlite integration) |
| Mem0 | mem0_storage.py | Stub |
| Kickoff Outputs | kickoff_task_outputs_storage.py | Stub |

- Short-term, long-term, entity, and contextual memory modules reference
  these backends but cannot persist or retrieve data
- **Impact**: Agents have no memory across tasks or sessions

### 7. Knowledge Document Ingestion (`src/knowledge/source/`)

- Python has 10 source types: PDF, CSV, Excel, JSON, String, Text,
  CrewDocling, GitHub, YouTube, custom
- Rust has a single `mod.rs` with `BaseKnowledgeSource` trait and
  placeholder `KnowledgeSource` struct
- Actual file parsing (PDF via crew-docling, Excel via calamine) absent
- **Impact**: Knowledge RAG pipeline non-functional

### 8. Flow Persistence (`src/flow/persistence/`)

- Python uses SQLite for flow state snapshots and resumption
- Rust has trait + struct definitions but no `rusqlite` integration
- **Impact**: Flows cannot be paused and resumed

### 9. Telemetry (`src/telemetry/mod.rs`)

- `Telemetry` singleton pattern defined
- OpenTelemetry SDK initialization not wired
- No trace/span/metric emission
- **Impact**: No observability; deployment monitoring absent

### 10. A2A Client (`src/a2a/client.rs`)

- `A2AClient` struct with auth and transport fields defined
- `send_task()`, `get_task()`, `cancel_task()` are stubs
- Auth schemes (API key, OAuth, JWT) defined but not validated
- **Impact**: Cannot communicate with external A2A agents

---

## P2 - Enhancement: Polish and Completeness

### 11. Guardrail Enforcement (`src/utilities/guardrail_types.rs`)

- `GuardrailResult` type defined
- No actual hallucination detection or LLM guardrail evaluation logic
- Python uses LLM-based checks; Rust needs equivalent

### 12. Training Handler (`src/utilities/training_handler.rs`)

- `TrainingHandler` struct defined
- No training data serialization or human feedback collection
- Python persists YAML training files

### 13. Evaluation Framework (`src/utilities/evaluators/`)

- `CrewEvaluatorHandler` and `TaskEvaluator` structures defined
- No scoring or benchmark logic

### 14. i18n Translation Loading (`src/utilities/i18n.rs`)

- `I18N` struct defined with `Default` impl
- `src/translations/en.json` exists
- Dynamic locale switching and fallback not implemented

### 15. RPM Rate Limiter (`src/utilities/rpm_controller.rs`)

- `RPMController` struct defined
- No actual rate-limiting / backpressure logic

### 16. Flow Visualization (`src/flow/visualization/`)

- `FlowStructure`, `NodeMetadata`, `EdgeMetadata` types complete
- `generate_html()` renders inline HTML/JS/CSS
- Interactive renderers (Python uses Jinja2 templates + D3.js) simplified

### 17. Agent Adapters (`src/agents/agent_adapters/`)

- LangGraph and OpenAI Agents adapters are stub modules
- Need integration with respective SDKs

### 18. Missing Utility: `import_utils`

- Python `import_utils.py` handles dynamic module loading
- No Rust equivalent (less relevant due to static compilation,
  but `libloading` or feature gates could fill the role)

---

## Module Completeness Matrix

| Module | Data Model | Config | Execution | I/O | Tests |
|--------|-----------|--------|-----------|-----|-------|
| agent/ | Full | Full | Stub | N/A | Partial |
| agents/ | Full | Full | Stub | N/A | Partial |
| crew | Full | Full | Stub | N/A | Partial |
| task | Full | Full | Stub | N/A | Partial |
| process | Full | Full | Full | N/A | Yes |
| llm/ | Full | Full | Stub | Stub | Partial |
| memory/ | Full | Partial | Stub | Stub | Partial |
| flow/ | Full | Full | Partial | Stub | Yes |
| knowledge/ | Full | Full | Stub | Stub | Yes |
| events/ | Full | Full | Full | N/A | Yes |
| mcp/ | Full | Full | Stub | Stub | Yes |
| a2a/ | Full | Full | Stub | Stub | Partial |
| security/ | Full | Full | Full | N/A | Partial |
| telemetry/ | Partial | Stub | Stub | Stub | No |
| utilities/ | Full | Full | Partial | Stub | Partial |
| tools/ | Full | Full | Partial | Partial | Yes |

**Legend**: Full = production-ready | Partial = some logic | Stub = types only

---

## Recommended Implementation Order

```
Phase 1: Core Execution Pipeline
  1. LLM provider (OpenAI first, then Anthropic)
  2. CrewAgentExecutor.invoke() / ReAct loop
  3. Task.execute_sync() / execute_async()
  4. Crew.kickoff() orchestration

Phase 2: Storage & I/O
  5. MCP HTTP transport (reqwest)
  6. Memory RAG storage backend
  7. Knowledge document loaders (text, JSON, CSV first)
  8. Flow SQLite persistence

Phase 3: Integrations
  9. A2A client HTTP calls
 10. Telemetry OTEL initialization
 11. Remaining LLM providers (Azure, Bedrock, Gemini)

Phase 4: Polish
 12. Guardrail enforcement
 13. Training/evaluation pipeline
 14. Agent adapters (LangGraph, OpenAI)
 15. i18n dynamic loading
```

---

## Dependency Additions Needed

| Crate | Purpose | Phase |
|-------|---------|-------|
| `reqwest` | HTTP calls (LLM, MCP, A2A) | 1 |
| `rusqlite` | Flow persistence, LTM storage | 2 |
| `eventsource-client` | SSE transport for MCP | 2 |
| `opentelemetry` | Telemetry traces/metrics | 3 |
| `pdf-extract` / `lopdf` | PDF knowledge source | 3 |
| `calamine` | Excel knowledge source | 3 |
| `csv` | CSV knowledge source | 2 |

---

## Warnings Summary

Current `cargo check` produces 46 warnings:
- 38 unused imports (cleanup pass needed)
- 5 unused variables
- 3 dead code warnings

These are cosmetic and do not affect compilation or correctness.
