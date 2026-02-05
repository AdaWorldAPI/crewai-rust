//! Convenience re-exports of BaseAgent trait and data types.
//!
//! Corresponds to `crewai/agents/agent_builder/base_agent.py` (re-exported
//! at the agents package level for convenience).
//!
//! This module re-exports the core `BaseAgent` trait and `BaseAgentData`
//! struct from the `agent_builder` submodule so consumers can use them
//! directly from `agents::base_agent`.

pub use super::agent_builder::base_agent_trait::{
    BaseAgent, BaseAgentData, PlatformApp, PlatformAppOrAction,
};
