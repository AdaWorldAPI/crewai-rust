//! Agents system for CrewAI.
//!
//! Corresponds to `crewai/agents/` Python package.
//!
//! This module provides the agent infrastructure including the base agent
//! trait, agent builder, executor, parser, tools handler, cache, and
//! agent adapters for different frameworks.

pub mod agent_adapters;
pub mod agent_builder;
pub mod base_agent;
pub mod cache;
pub mod crew_agent_executor;
pub mod parser;
pub mod tools_handler;

// Re-exports for convenience
pub use agent_builder::base_agent_trait::{BaseAgent, PlatformApp};
pub use base_agent::BaseAgentData;
pub use cache::cache_handler::CacheHandler;
pub use crew_agent_executor::CrewAgentExecutor;
pub use parser::{AgentAction, AgentFinish, OutputParserError};
pub use tools_handler::ToolsHandler;
