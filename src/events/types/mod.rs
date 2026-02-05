//! Domain-specific event type definitions for the CrewAI event system.
//!
//! Corresponds to `crewai/events/types/__init__.py`.
//!
//! Each sub-module defines the event structs for a particular domain
//! (agents, crews, tasks, tools, LLM, flows, knowledge, memory, etc.).

/// Agent execution and evaluation events.
pub mod agent_events;

/// Crew lifecycle events (kickoff, train, test).
pub mod crew_events;

/// Task lifecycle events (started, completed, failed, evaluation).
pub mod task_events;

/// Tool usage events (started, finished, errors).
pub mod tool_events;

/// LLM call and streaming events.
pub mod llm_events;

/// Flow and method execution events, human feedback events.
pub mod flow_events;

/// Knowledge retrieval and query events.
pub mod knowledge_events;

/// Memory query, save, and retrieval events.
pub mod memory_events;

// ---------------------------------------------------------------------------
// Additional event modules (part of the existing codebase)
// ---------------------------------------------------------------------------

/// LLM guardrail events.
pub mod llm_guardrail_events;

/// Agent logging events.
pub mod logging_events;

/// MCP (Model Context Protocol) events.
pub mod mcp_events;

/// Agent reasoning events.
pub mod reasoning_events;

/// System signal events (SIGTERM, SIGINT, etc.).
pub mod system_events;

/// A2A (Agent-to-Agent) delegation events.
pub mod a2a_events;

/// Tool usage events under their original module name (backward-compat alias).
///
/// New code should prefer [`tool_events`].
pub mod tool_usage_events;
