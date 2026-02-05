//! Agent builder module.
//!
//! Corresponds to `crewai/agents/agent_builder/` Python package.
//!
//! Provides the base agent trait, base agent executor mixin, and builder
//! utilities for constructing agents with the appropriate configuration.

pub mod base_agent_executor;
pub mod base_agent_trait;
pub mod utilities;

pub use base_agent_trait::BaseAgent;
pub use base_agent_executor::BaseAgentExecutorMixin;
