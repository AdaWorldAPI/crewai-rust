//! OpenAI Agents adapter.
//!
//! Corresponds to `crewai/agents/agent_adapters/openai_agents/` Python package.
//!
//! Provides adapter implementations for integrating OpenAI Agents (Assistants API)
//! with CrewAI. The OpenAI adapter wraps OpenAI's agent pattern to work within
//! the CrewAI framework, providing:
//!
//! - `OpenAIAgentAdapter` - Main agent adapter for OpenAI Agents, extends
//!   `BaseAgentAdapter` with OpenAI-specific execution via `Runner.run()`,
//!   tool integration, and structured output support.
//! - `OpenAIAgentToolAdapter` - Tool conversion from CrewAI `BaseTool` to
//!   OpenAI's `FunctionTool` format with proper schema generation.
//! - `OpenAIConverterAdapter` - Structured output converter that enhances
//!   system prompts with schema instructions for OpenAI format.
//! - Protocol definitions for lazy-loaded OpenAI agents modules.

// Future modules:
// pub mod openai_adapter;
// pub mod openai_agent_tool_adapter;
// pub mod protocols;
// pub mod structured_output_converter;
