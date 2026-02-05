//! Experiment runner for batch evaluation.
//!
//! Corresponds to `crewai/experimental/evaluation/experiment/`.
//!
//! Provides the `ExperimentRunner` for running evaluation datasets against
//! crews or agents, and the `ExperimentResult`/`ExperimentResults` types
//! for structured result storage and comparison.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Experiment result types
// ---------------------------------------------------------------------------

/// Result of a single test case in an experiment.
///
/// Corresponds to `crewai.experimental.evaluation.experiment.result.ExperimentResult`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    /// Unique identifier for this test case.
    pub identifier: String,
    /// Input data that was provided to the crew/agent.
    pub inputs: HashMap<String, Value>,
    /// Actual score(s) achieved.
    pub score: Value,
    /// Expected score(s) for comparison.
    pub expected_score: Value,
    /// Whether the test case passed (actual >= expected).
    pub passed: bool,
    /// Per-agent evaluation results, if available.
    pub agent_evaluations: Option<HashMap<String, Value>>,
}

/// Collection of experiment results with metadata.
///
/// Corresponds to `crewai.experimental.evaluation.experiment.result.ExperimentResults`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResults {
    /// Individual test case results.
    pub results: Vec<ExperimentResult>,
    /// Additional metadata about the experiment run.
    pub metadata: HashMap<String, Value>,
    /// When the experiment was run.
    pub timestamp: DateTime<Utc>,
}

impl ExperimentResults {
    /// Create a new ExperimentResults from a list of results.
    pub fn new(results: Vec<ExperimentResult>) -> Self {
        Self {
            results,
            metadata: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    /// Serialize results to a JSON value.
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    /// Compare with a baseline experiment run.
    ///
    /// # Arguments
    ///
    /// * `baseline` - Previous experiment results to compare against.
    ///
    /// # Returns
    ///
    /// A map describing the changes between this run and the baseline.
    pub fn compare_with_baseline(&self, _baseline: &ExperimentResults) -> HashMap<String, Value> {
        todo!("Baseline comparison not yet implemented")
    }
}

// ---------------------------------------------------------------------------
// Experiment runner
// ---------------------------------------------------------------------------

/// Runner for executing evaluation experiments over datasets.
///
/// Corresponds to `crewai.experimental.evaluation.experiment.runner.ExperimentRunner`.
///
/// Takes a dataset of test cases (each with inputs and expected scores),
/// runs them through a crew or set of agents, and collects evaluation results.
#[derive(Debug)]
pub struct ExperimentRunner {
    /// Dataset of test cases to run.
    pub dataset: Vec<HashMap<String, Value>>,
}

impl ExperimentRunner {
    /// Create a new ExperimentRunner with a dataset.
    ///
    /// Each entry in the dataset should contain:
    /// - `inputs`: A map of input key/value pairs.
    /// - `expected_score`: The expected score (float or dict of floats).
    /// - `identifier` (optional): A unique identifier for the test case.
    pub fn new(dataset: Vec<HashMap<String, Value>>) -> Self {
        Self { dataset }
    }

    /// Run the experiment.
    ///
    /// Executes each test case in the dataset and returns collected results.
    ///
    /// # Returns
    ///
    /// An `ExperimentResults` containing all test case results.
    pub fn run(&self) -> ExperimentResults {
        todo!("ExperimentRunner::run not yet implemented")
    }
}
