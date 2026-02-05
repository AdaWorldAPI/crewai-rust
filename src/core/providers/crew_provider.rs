//! Crew provider trait for crew execution abstraction.
//!
//! Corresponds to the crew execution interface extracted from `crewai/crew.py`.
//!
//! This provider trait abstracts the crew execution logic so that different
//! backends can implement the orchestration differently (e.g., sequential
//! vs hierarchical, local vs distributed).

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::crews::crew_output::CrewOutput;

/// Provider trait for crew execution logic.
///
/// This abstracts the crew execution so different backends can implement
/// the orchestration differently. The default implementation in `Crew`
/// handles sequential and hierarchical process flows.
///
/// # Methods
///
/// - `kickoff` - Execute the crew synchronously with given inputs.
/// - `kickoff_async` - Execute the crew asynchronously.
/// - `kickoff_for_each` - Execute the crew for each set of inputs.
/// - `kickoff_for_each_async` - Execute for each set of inputs asynchronously.
#[async_trait]
pub trait CrewProvider: Send + Sync {
    /// Execute the crew with the given inputs.
    ///
    /// # Arguments
    ///
    /// * `inputs` - Input key-value pairs to interpolate into tasks and agents.
    ///
    /// # Returns
    ///
    /// The crew output containing raw results, structured output, task outputs,
    /// and token usage metrics.
    async fn kickoff(
        &self,
        inputs: HashMap<String, Value>,
    ) -> Result<CrewOutput, anyhow::Error>;

    /// Execute the crew asynchronously (non-blocking).
    ///
    /// Returns a future that resolves to the crew output.
    async fn kickoff_async(
        &self,
        inputs: HashMap<String, Value>,
    ) -> Result<CrewOutput, anyhow::Error>;

    /// Execute the crew for each set of inputs.
    ///
    /// Runs the crew once per input set, collecting all outputs.
    ///
    /// # Arguments
    ///
    /// * `inputs` - List of input dictionaries, one per execution.
    async fn kickoff_for_each(
        &self,
        inputs: Vec<HashMap<String, Value>>,
    ) -> Result<Vec<CrewOutput>, anyhow::Error>;

    /// Execute the crew for each set of inputs asynchronously.
    ///
    /// Runs all executions concurrently and collects outputs.
    async fn kickoff_for_each_async(
        &self,
        inputs: Vec<HashMap<String, Value>>,
    ) -> Result<Vec<CrewOutput>, anyhow::Error>;
}
