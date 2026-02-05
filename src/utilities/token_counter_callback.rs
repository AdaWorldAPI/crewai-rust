//! Token counter callback handler.
//!
//! Corresponds to `crewai/utilities/token_counter_callback.py`.

use serde::{Deserialize, Serialize};

/// Tracks token usage across LLM calls.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenCalcHandler {
    /// Total number of prompt (input) tokens.
    pub total_prompt_tokens: u64,
    /// Total number of completion (output) tokens.
    pub total_completion_tokens: u64,
    /// Total number of successful LLM requests.
    pub successful_requests: u64,
}

impl TokenCalcHandler {
    /// Create a new `TokenCalcHandler`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record token usage from an LLM call.
    pub fn on_llm_end(&mut self, prompt_tokens: u64, completion_tokens: u64) {
        self.total_prompt_tokens += prompt_tokens;
        self.total_completion_tokens += completion_tokens;
        self.successful_requests += 1;
    }

    /// Get the total number of tokens (prompt + completion).
    pub fn total_tokens(&self) -> u64 {
        self.total_prompt_tokens + self.total_completion_tokens
    }

    /// Reset all counters to zero.
    pub fn reset(&mut self) {
        self.total_prompt_tokens = 0;
        self.total_completion_tokens = 0;
        self.successful_requests = 0;
    }
}
