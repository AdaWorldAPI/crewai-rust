//! Agent adapters for different frameworks.
//!
//! Corresponds to `crewai/agents/agent_adapters/` Python package.
//!
//! Provides adapter traits and implementations for integrating different
//! agent frameworks (e.g., LangGraph, OpenAI Agents) with CrewAI.

pub mod base_agent_adapter;
pub mod base_converter_adapter;
pub mod base_tool_adapter;
pub mod langgraph;
pub mod openai_agents;

pub use base_agent_adapter::BaseAgentAdapter;
pub use base_converter_adapter::BaseConverterAdapter;
pub use base_tool_adapter::BaseToolAdapter;
