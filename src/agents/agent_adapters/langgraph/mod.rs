//! LangGraph agent adapter.
//!
//! Corresponds to `crewai/agents/agent_adapters/langgraph/` Python package.
//!
//! Provides adapter implementations for integrating LangGraph-based agents
//! with CrewAI. The LangGraph adapter wraps LangGraph's ReAct agent pattern
//! to work within the CrewAI framework, providing:
//!
//! - `LangGraphAgentAdapter` - Main agent adapter for LangGraph, extends
//!   `BaseAgentAdapter` with LangGraph-specific execution, memory persistence
//!   via checkpointing, tool integration, and structured output support.
//! - `LangGraphToolAdapter` - Tool conversion from CrewAI `BaseTool` to
//!   LangGraph-compatible tool format using `@tool` decorator equivalents.
//! - `LangGraphConverterAdapter` - Structured output converter that enhances
//!   system prompts with JSON/Pydantic schema instructions.
//! - Protocol definitions for lazy-loaded LangGraph modules.

// Future modules:
// pub mod langgraph_adapter;
// pub mod langgraph_tool_adapter;
// pub mod protocols;
// pub mod structured_output_converter;
