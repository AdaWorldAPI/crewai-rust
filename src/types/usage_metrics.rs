//! Usage metrics tracking for CrewAI execution.
//!
//! Corresponds to `crewai/types/usage_metrics.py`.

use serde::{Deserialize, Serialize};

/// Track usage metrics for crew execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageMetrics {
    /// Total number of tokens used.
    pub total_tokens: i64,
    /// Number of tokens used in prompts.
    pub prompt_tokens: i64,
    /// Number of cached prompt tokens used.
    pub cached_prompt_tokens: i64,
    /// Number of tokens used in completions.
    pub completion_tokens: i64,
    /// Number of successful requests made.
    pub successful_requests: i64,
}

impl UsageMetrics {
    /// Create a new empty UsageMetrics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add usage metrics from another UsageMetrics object.
    pub fn add_usage_metrics(&mut self, other: &UsageMetrics) {
        self.total_tokens += other.total_tokens;
        self.prompt_tokens += other.prompt_tokens;
        self.cached_prompt_tokens += other.cached_prompt_tokens;
        self.completion_tokens += other.completion_tokens;
        self.successful_requests += other.successful_requests;
    }
}
