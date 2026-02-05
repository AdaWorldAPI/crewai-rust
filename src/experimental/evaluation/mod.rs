//! Evaluation framework for assessing agent performance.
//!
//! Corresponds to `crewai/experimental/evaluation/`.
//!
//! Provides a pluggable evaluation system with:
//! - Base evaluator trait and metric categories
//! - Goal alignment, semantic quality, reasoning, and tool usage metrics
//! - Experiment runner for batch evaluation over datasets
//! - Structured evaluation results and scoring

pub mod experiment;
pub mod metrics;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Metric categories
// ---------------------------------------------------------------------------

/// Categories of evaluation metrics.
///
/// Corresponds to `crewai.experimental.evaluation.base_evaluator.MetricCategory`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricCategory {
    /// How well the agent's output aligns with the task goal.
    GoalAlignment,
    /// Quality of the agent's output text.
    SemanticQuality,
    /// Efficiency of the agent's reasoning process.
    ReasoningEfficiency,
    /// Accuracy of tool selection decisions.
    ToolSelection,
    /// Quality of parameter extraction for tool calls.
    ParameterExtraction,
    /// Overall tool invocation correctness.
    ToolInvocation,
}

impl MetricCategory {
    /// Get a human-readable title for the metric category.
    pub fn title(&self) -> &'static str {
        match self {
            MetricCategory::GoalAlignment => "Goal Alignment",
            MetricCategory::SemanticQuality => "Semantic Quality",
            MetricCategory::ReasoningEfficiency => "Reasoning Efficiency",
            MetricCategory::ToolSelection => "Tool Selection",
            MetricCategory::ParameterExtraction => "Parameter Extraction",
            MetricCategory::ToolInvocation => "Tool Invocation",
        }
    }
}

impl std::fmt::Display for MetricCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title())
    }
}

// ---------------------------------------------------------------------------
// Evaluation score
// ---------------------------------------------------------------------------

/// A single evaluation score with feedback.
///
/// Corresponds to `crewai.experimental.evaluation.base_evaluator.EvaluationScore`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationScore {
    /// Numeric score from 0.0 to 10.0, or None if not applicable.
    pub score: Option<f64>,
    /// Detailed feedback explaining the evaluation score.
    pub feedback: String,
    /// Raw response from the evaluator (e.g., LLM output).
    pub raw_response: Option<String>,
}

impl Default for EvaluationScore {
    fn default() -> Self {
        Self {
            score: Some(5.0),
            feedback: String::new(),
            raw_response: None,
        }
    }
}

impl std::fmt::Display for EvaluationScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.score {
            Some(s) => write!(f, "Score: {:.1}/10 - {}", s, self.feedback),
            None => write!(f, "Score: N/A - {}", self.feedback),
        }
    }
}

// ---------------------------------------------------------------------------
// Base evaluator trait
// ---------------------------------------------------------------------------

/// Base trait for metric evaluators.
///
/// Corresponds to `crewai.experimental.evaluation.base_evaluator.BaseEvaluator`.
///
/// Implement this trait to create custom evaluation metrics that assess
/// agent performance on specific dimensions.
pub trait BaseEvaluator: Send + Sync {
    /// The metric category this evaluator measures.
    fn metric_category(&self) -> MetricCategory;

    /// Evaluate an agent's performance.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - Identifier for the agent being evaluated.
    /// * `execution_trace` - Execution trace data (tool calls, reasoning steps).
    /// * `final_output` - The agent's final output.
    /// * `task_description` - Optional task description for context.
    ///
    /// # Returns
    ///
    /// An `EvaluationScore` with the assessment.
    fn evaluate(
        &self,
        agent_id: &str,
        execution_trace: &serde_json::Value,
        final_output: &serde_json::Value,
        task_description: Option<&str>,
    ) -> EvaluationScore;
}

// ---------------------------------------------------------------------------
// Agent evaluation result
// ---------------------------------------------------------------------------

/// Evaluation results for a single agent.
///
/// Corresponds to `crewai.experimental.evaluation.base_evaluator.AgentEvaluationResult`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvaluationResult {
    /// ID of the evaluated agent.
    pub agent_id: String,
    /// ID of the task that was executed.
    pub task_id: String,
    /// Evaluation scores for each metric category.
    pub metrics: HashMap<String, EvaluationScore>,
}

impl AgentEvaluationResult {
    /// Create a new empty agent evaluation result.
    pub fn new(agent_id: String, task_id: String) -> Self {
        Self {
            agent_id,
            task_id,
            metrics: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Aggregation strategy
// ---------------------------------------------------------------------------

/// Strategy for aggregating evaluation scores across multiple tasks.
///
/// Corresponds to `crewai.experimental.evaluation.base_evaluator.AggregationStrategy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregationStrategy {
    /// Equal weight to all tasks.
    SimpleAverage,
    /// Weight by task complexity.
    WeightedByComplexity,
    /// Use best scores across tasks.
    BestPerformance,
    /// Use worst scores across tasks.
    WorstPerformance,
}

impl Default for AggregationStrategy {
    fn default() -> Self {
        Self::SimpleAverage
    }
}

// ---------------------------------------------------------------------------
// Aggregated evaluation result
// ---------------------------------------------------------------------------

/// Aggregated evaluation result for an agent across multiple tasks.
///
/// Corresponds to `crewai.experimental.evaluation.base_evaluator.AgentAggregatedEvaluationResult`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAggregatedEvaluationResult {
    /// ID of the agent.
    #[serde(default)]
    pub agent_id: String,
    /// Role of the agent.
    #[serde(default)]
    pub agent_role: String,
    /// Number of tasks included in this aggregation.
    #[serde(default)]
    pub task_count: usize,
    /// Strategy used for aggregation.
    #[serde(default)]
    pub aggregation_strategy: AggregationStrategy,
    /// Aggregated metrics across all tasks.
    #[serde(default)]
    pub metrics: HashMap<String, EvaluationScore>,
    /// IDs of tasks included in this aggregation.
    #[serde(default)]
    pub task_results: Vec<String>,
    /// Overall score for this agent.
    pub overall_score: Option<f64>,
}

impl std::fmt::Display for AgentAggregatedEvaluationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Agent Evaluation: {}", self.agent_role)?;
        writeln!(f, "Strategy: {:?}", self.aggregation_strategy)?;
        writeln!(f, "Tasks evaluated: {}", self.task_count)?;

        for (category, score) in &self.metrics {
            writeln!(f)?;
            writeln!(
                f,
                "- {}: {}/10",
                category.to_uppercase(),
                score
                    .score
                    .map(|s| format!("{:.1}", s))
                    .unwrap_or_else(|| "N/A".to_string())
            )?;
            if !score.feedback.is_empty() {
                writeln!(f, "  {}", score.feedback)?;
            }
        }

        Ok(())
    }
}
