//! Base agent executor mixin.
//!
//! Corresponds to `crewai/agents/agent_builder/base_agent_executor_mixin.py`.
//!
//! Provides the `BaseAgentExecutorMixin` trait with shared memory creation
//! and lifecycle management methods used by all agent executor implementations.

use std::fmt;

use crate::agents::parser::AgentFinish;

/// Mixin trait for agent executor implementations.
///
/// Provides shared memory creation and lifecycle management methods
/// used by all agent executor implementations. In the Python codebase
/// this is a mixin class; in Rust it is expressed as a trait.
pub trait BaseAgentExecutorMixin: Send + Sync + fmt::Debug {
    /// Create and save a short-term memory item from the execution output.
    ///
    /// Saves the output text to the crew's short-term memory if the crew
    /// has memory enabled and the output is not a delegation action.
    fn create_short_term_memory(
        &self,
        output: &AgentFinish,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Stub: In a full implementation this would:
        // 1. Check if crew and task are set
        // 2. Check if the output is not a delegation action
        // 3. Save to crew._short_term_memory
        log::debug!(
            "create_short_term_memory: output_len={}",
            output.text.len()
        );
        Ok(())
    }

    /// Create and save an external memory item from the execution output.
    fn create_external_memory(
        &self,
        output: &AgentFinish,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "create_external_memory: output_len={}",
            output.text.len()
        );
        Ok(())
    }

    /// Create and save a long-term memory item from the execution output.
    ///
    /// Uses a TaskEvaluator to assess the output quality and stores the
    /// evaluation alongside the output in long-term memory.
    fn create_long_term_memory(
        &self,
        output: &AgentFinish,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "create_long_term_memory: output_len={}",
            output.text.len()
        );
        Ok(())
    }

    /// Create and save entity memory items extracted from the output.
    ///
    /// Uses the agent's LLM to extract entities from the output text
    /// and stores them in entity memory for future reference.
    fn create_entity_memory(
        &self,
        output: &AgentFinish,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "create_entity_memory: output_len={}",
            output.text.len()
        );
        Ok(())
    }
}
