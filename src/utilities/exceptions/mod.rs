//! Custom exception types for CrewAI.
//!
//! Corresponds to `crewai/utilities/exceptions/`.

use thiserror::Error;

/// Error raised when the LLM context window is exceeded.
#[derive(Debug, Error)]
#[error("LLM context length exceeded: {message}")]
pub struct LLMContextLengthExceededError {
    /// Human-readable error message.
    pub message: String,
}

impl LLMContextLengthExceededError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Error raised when the maximum number of iterations is exceeded.
#[derive(Debug, Error)]
#[error("Maximum iterations exceeded: {message}")]
pub struct MaxIterationsExceededError {
    pub message: String,
}

impl MaxIterationsExceededError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Error raised when a task execution fails.
#[derive(Debug, Error)]
#[error("Task execution error: {message}")]
pub struct TaskExecutionError {
    pub message: String,
}

impl TaskExecutionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
