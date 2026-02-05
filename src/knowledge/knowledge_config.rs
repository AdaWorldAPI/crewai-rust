//! Knowledge configuration for controlling query behavior.
//!
//! Port of crewai/knowledge/knowledge_config.py

use serde::{Deserialize, Serialize};

/// Configuration for knowledge query behavior.
///
/// Controls the maximum number of results returned and the minimum
/// relevance score threshold for knowledge queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeConfig {
    /// Maximum number of results to return from knowledge queries.
    /// Defaults to 3.
    #[serde(default = "default_results_limit")]
    pub results_limit: usize,

    /// Minimum similarity score threshold for knowledge query results.
    /// Results with scores below this threshold are filtered out.
    /// Defaults to 0.35.
    #[serde(default = "default_score_threshold")]
    pub score_threshold: f64,
}

fn default_results_limit() -> usize {
    3
}

fn default_score_threshold() -> f64 {
    0.35
}

impl Default for KnowledgeConfig {
    fn default() -> Self {
        Self {
            results_limit: default_results_limit(),
            score_threshold: default_score_threshold(),
        }
    }
}

impl KnowledgeConfig {
    /// Create a new KnowledgeConfig with custom values.
    pub fn new(results_limit: Option<usize>, score_threshold: Option<f64>) -> Self {
        Self {
            results_limit: results_limit.unwrap_or_else(default_results_limit),
            score_threshold: score_threshold.unwrap_or_else(default_score_threshold),
        }
    }
}
