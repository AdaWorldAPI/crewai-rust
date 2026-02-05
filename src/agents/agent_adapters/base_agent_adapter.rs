//! Base agent adapter trait.
//!
//! Corresponds to `crewai/agents/agent_adapters/base_agent_adapter.py`.
//!
//! Defines the common interface and functionality that all agent adapters
//! must implement. Extends the `BaseAgent` trait to maintain compatibility
//! with the CrewAI framework while adding adapter-specific requirements.

use std::any::Any;
use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::agents::agent_builder::base_agent_trait::BaseAgent;

/// Base trait for all agent adapters in CrewAI.
///
/// Extends `BaseAgent` to add adapter-specific methods like `configure_tools`
/// and `configure_structured_output`. All agent adapters (LangGraph, OpenAI
/// Agents, etc.) must implement this trait.
#[async_trait]
pub trait BaseAgentAdapter: BaseAgent {
    /// Whether this adapter supports structured output natively.
    fn adapted_structured_output(&self) -> bool {
        false
    }

    /// Get the adapter-specific agent configuration.
    fn agent_config(&self) -> Option<&HashMap<String, Value>> {
        None
    }

    /// Configure and adapt tools for the specific agent implementation.
    ///
    /// # Arguments
    ///
    /// * `tools` - Optional list of tool objects to be configured.
    fn configure_tools(
        &mut self,
        tools: Option<Vec<Box<dyn Any + Send + Sync>>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Configure the structured output for the specific agent implementation.
    ///
    /// # Arguments
    ///
    /// * `structured_output` - The structured output specification to be configured.
    fn configure_structured_output(
        &mut self,
        _structured_output: Option<Box<dyn Any + Send + Sync>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Default: no-op
        Ok(())
    }
}
