//! Evaluator utilities for crew/task performance assessment.
//!
//! Corresponds to `crewai/utilities/evaluators/`.

use serde::{Deserialize, Serialize};

/// Summary of an evaluation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationSummary {
    /// Identifier of the entity being evaluated.
    pub entity_id: String,
    /// Human-readable label for the evaluation.
    pub label: String,
    /// Overall score (0.0 - 10.0).
    pub score: f64,
    /// Detailed feedback.
    pub feedback: String,
}

/// Trait for evaluation strategies.
pub trait Evaluator {
    /// Run the evaluation and return a summary.
    fn evaluate(&self) -> EvaluationSummary;
}
