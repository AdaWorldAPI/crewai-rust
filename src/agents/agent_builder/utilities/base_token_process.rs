//! Token usage tracking utilities.
//!
//! Corresponds to `crewai/agents/agent_builder/utilities/base_token_process.py`.
//!
//! Provides utilities for tracking token consumption and request metrics
//! during agent execution.

use serde::{Deserialize, Serialize};

use crate::types::usage_metrics::UsageMetrics;

/// Track token usage during agent processing.
///
/// Accumulates token counts across multiple LLM calls and provides
/// a summary via `get_summary()`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenProcess {
    /// Total number of tokens used (prompt + completion).
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

impl TokenProcess {
    /// Create a new `TokenProcess` with zero values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add prompt tokens to the running totals.
    pub fn sum_prompt_tokens(&mut self, tokens: i64) {
        self.prompt_tokens += tokens;
        self.total_tokens += tokens;
    }

    /// Add completion tokens to the running totals.
    pub fn sum_completion_tokens(&mut self, tokens: i64) {
        self.completion_tokens += tokens;
        self.total_tokens += tokens;
    }

    /// Add cached prompt tokens to the running total.
    pub fn sum_cached_prompt_tokens(&mut self, tokens: i64) {
        self.cached_prompt_tokens += tokens;
    }

    /// Add successful requests to the running total.
    pub fn sum_successful_requests(&mut self, requests: i64) {
        self.successful_requests += requests;
    }

    /// Get a summary of all tracked metrics.
    pub fn get_summary(&self) -> UsageMetrics {
        UsageMetrics {
            total_tokens: self.total_tokens,
            prompt_tokens: self.prompt_tokens,
            cached_prompt_tokens: self.cached_prompt_tokens,
            completion_tokens: self.completion_tokens,
            successful_requests: self.successful_requests,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_process_accumulation() {
        let mut tp = TokenProcess::new();
        tp.sum_prompt_tokens(100);
        tp.sum_completion_tokens(50);
        tp.sum_cached_prompt_tokens(20);
        tp.sum_successful_requests(1);

        assert_eq!(tp.total_tokens, 150);
        assert_eq!(tp.prompt_tokens, 100);
        assert_eq!(tp.completion_tokens, 50);
        assert_eq!(tp.cached_prompt_tokens, 20);
        assert_eq!(tp.successful_requests, 1);

        let summary = tp.get_summary();
        assert_eq!(summary.total_tokens, 150);
        assert_eq!(summary.prompt_tokens, 100);
        assert_eq!(summary.completion_tokens, 50);
        assert_eq!(summary.cached_prompt_tokens, 20);
        assert_eq!(summary.successful_requests, 1);
    }
}
