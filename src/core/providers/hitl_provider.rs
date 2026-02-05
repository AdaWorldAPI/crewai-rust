//! Human-in-the-loop (HITL) provider trait.
//!
//! Corresponds to the HITL pattern extracted from `crewai/crew.py` and
//! `crewai/core/providers/human_input.py`.
//!
//! Provides the trait for pausing crew execution to request human input
//! and resuming with the provided response. This enables interactive
//! workflows where human review/approval is required.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

/// Provider trait for human-in-the-loop interactions.
///
/// Allows pausing crew execution to request human input and resuming
/// with the provided response. Implementations can be console-based
/// (default), web-based, or API-based.
///
/// # Examples
///
/// The default `ConsoleHITLProvider` reads from stdin:
///
/// ```rust,no_run
/// use crewai::core::providers::hitl_provider::ConsoleHITLProvider;
/// ```
#[async_trait]
pub trait HITLProvider: Send + Sync {
    /// Request human input for a given prompt.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The prompt to display to the human.
    /// * `context` - Additional context information for the reviewer.
    ///
    /// # Returns
    ///
    /// The human's input string.
    async fn request_input(
        &self,
        prompt: &str,
        context: &HashMap<String, Value>,
    ) -> Result<String, anyhow::Error>;

    /// Resume execution with human-provided input.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The ID of the task that was paused.
    /// * `input` - The human's input to resume with.
    ///
    /// # Returns
    ///
    /// The result value after incorporating human input.
    async fn resume_with_input(
        &self,
        task_id: &str,
        input: &str,
    ) -> Result<Value, anyhow::Error>;

    /// Check if HITL is enabled for this provider.
    fn is_enabled(&self) -> bool;
}

/// Default console-based HITL provider.
///
/// Reads input from stdin when human input is requested. Displays
/// prompts to stdout and waits for user input.
#[derive(Debug, Default)]
pub struct ConsoleHITLProvider;

#[async_trait]
impl HITLProvider for ConsoleHITLProvider {
    async fn request_input(
        &self,
        prompt: &str,
        _context: &HashMap<String, Value>,
    ) -> Result<String, anyhow::Error> {
        println!("{}", prompt);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }

    async fn resume_with_input(
        &self,
        _task_id: &str,
        input: &str,
    ) -> Result<Value, anyhow::Error> {
        Ok(Value::String(input.to_string()))
    }

    fn is_enabled(&self) -> bool {
        true
    }
}
