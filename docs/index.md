# crewAI-rust Documentation

**Version**: 1.9.3 -- A 1:1 Rust port of the [crewAI Python framework](https://github.com/crewAIInc/crewAI) (v1.9.3)

---

## What is crewAI-rust?

crewAI-rust is a complete Rust reimplementation of the crewAI Python framework for AI agent orchestration. It provides the same data model, configuration surface, and module structure as the Python original while taking advantage of Rust's type safety, zero-cost abstractions, memory safety, and async capabilities via Tokio.

The crate compiles to a single static library with no Python runtime dependency. All 25+ top-level modules from the Python codebase have been ported, covering agents, tasks, crews, flows, tools, memory, knowledge, LLM providers, MCP integration, events, A2A, security, and telemetry.

## Key Benefits

| Benefit | Details |
|---------|---------|
| **Type safety** | All configuration validated at compile time via Rust structs and enums. No runtime `AttributeError` or `KeyError`. |
| **Zero-cost abstractions** | Trait-based polymorphism compiles to static dispatch where possible. No boxing overhead for monomorphized paths. |
| **Memory safety** | No garbage collector. Ownership and borrowing enforced by the compiler. `Arc<Mutex<T>>` for shared mutable state. |
| **Async via Tokio** | All I/O-bound operations (`kickoff_async`, `acall`, `arun`) use `async`/`await` with Tokio. CPU-bound work uses `tokio::spawn_blocking`. |
| **Serde-native** | All data structures derive `Serialize` and `Deserialize`. JSON round-tripping is first-class. |
| **Single binary** | No interpreter, no virtualenv. Ship one binary with all agent logic embedded. |

## Architecture Overview

crewAI-rust follows the same two-pillar architecture as the Python framework:

```
                    crewAI-rust
                   /            \
              Crews              Flows
             /     \            /     \
        Agents    Tasks    Start    Listen/Router
          |         |        |         |
        Tools    Output    State    Persistence
          |                  |
         LLM             Events
```

**Crews** orchestrate multiple agents executing tasks in sequential or hierarchical processes. Each agent has a role, goal, and backstory, and can use tools, memory, and knowledge to complete tasks.

**Flows** provide event-driven state machines with `start`, `listen`, and `router` methods, conditional execution (AND/OR), state persistence (SQLite), and human-in-the-loop feedback.

Both pillars share the underlying **LLM**, **Tools**, **Memory**, **Knowledge**, **Events**, and **MCP** subsystems.

## Documentation

### Getting Started

- [Installation](installation.md) -- Adding crewai-rust to your project
- [Quickstart](quickstart.md) -- Build your first crew in Rust

### Core Concepts

- [Agents](concepts/agents.md) -- Autonomous units with roles, goals, and backstories
- [Tasks](concepts/tasks.md) -- Units of work assigned to agents
- [Crews](concepts/crews.md) -- Groups of agents collaborating on tasks
- [Flows](concepts/flows.md) -- Event-driven state machines
- [Tools](concepts/tools.md) -- Capabilities agents can use
- [Memory](concepts/memory.md) -- Short-term, long-term, entity, and external memory
- [Knowledge](concepts/knowledge.md) -- RAG-backed knowledge sources
- [LLMs](concepts/llms.md) -- Language model configuration and providers
- [Events](concepts/events.md) -- Event bus architecture for monitoring
- [MCP](concepts/mcp.md) -- Model Context Protocol integration

### Tools

- [Tools Overview](tools/overview.md) -- Built-in and external tool categories

### Guides

- [Building Custom Tools](guides/custom-tools.md) -- Implement the BaseTool trait

### Migration

- [Python to Rust Migration](migration/python-to-rust.md) -- Translation patterns and key differences

### Implementation Status

- [Technical Debt Report](../TECHNICAL_DEBT.md) -- Current implementation gaps and roadmap

## Crate Structure

```
crewai (crate)
  src/
    agent/       -- Agent struct and execution logic
    agents/      -- Agent executor, adapters, cache handler
    crew.rs      -- Crew struct and orchestration
    crews/       -- CrewOutput, crew utilities
    task.rs      -- Task struct and execution
    tasks/       -- TaskOutput, output formats, guardrails
    flow/        -- Flow state machine, persistence, visualization
    tools/       -- BaseTool, Tool, ToolUsage, agent tools, MCP tools
    llm/         -- LLM struct, provider inference, context windows
    llms/        -- BaseLLM trait, provider implementations
    memory/      -- Memory types and storage backends
    knowledge/   -- Knowledge sources and storage
    events/      -- Event bus, base events, domain event types
    mcp/         -- MCP client, config, transports, filters
    a2a/         -- Agent-to-Agent protocol
    security/    -- SecurityConfig, fingerprinting
    telemetry/   -- OpenTelemetry integration
    utilities/   -- i18n, printer, rate limiter, evaluators
    process.rs   -- Process enum (Sequential, Hierarchical)
    context.rs   -- Execution context
    types/       -- Shared types (UsageMetrics, etc.)
    translations/ -- i18n translation files
    lib.rs       -- Crate root with re-exports
```

## Version

This documentation corresponds to crewAI-rust version **1.9.3**, which maps to Python crewAI v1.9.3.
