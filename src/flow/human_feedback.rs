//! Human feedback types for Flow methods.
//!
//! Corresponds to `crewai/flow/human_feedback.py`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result from a @human_feedback decorated method.
///
/// This struct captures all information about a human feedback interaction,
/// including the original method output, the human's feedback, and any
/// collapsed outcome for routing purposes.
///
/// Corresponds to `crewai.flow.human_feedback.HumanFeedbackResult`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanFeedbackResult {
    /// The original return value from the decorated method that was
    /// shown to the human for review.
    pub output: serde_json::Value,
    /// The raw text feedback provided by the human. Empty string
    /// if no feedback was provided.
    pub feedback: String,
    /// The collapsed outcome string when emit is specified.
    /// This is determined by the LLM based on the human's feedback.
    /// None if emit was not specified.
    pub outcome: Option<String>,
    /// When the feedback was received.
    pub timestamp: DateTime<Utc>,
    /// The name of the decorated method that triggered feedback.
    pub method_name: String,
    /// Optional metadata for enterprise integrations. Can be used
    /// to pass additional context like channel, assignee, etc.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for HumanFeedbackResult {
    fn default() -> Self {
        Self {
            output: serde_json::Value::Null,
            feedback: String::new(),
            outcome: None,
            timestamp: Utc::now(),
            method_name: String::new(),
            metadata: HashMap::new(),
        }
    }
}

impl HumanFeedbackResult {
    /// Create a new HumanFeedbackResult.
    pub fn new(
        output: serde_json::Value,
        feedback: String,
        method_name: String,
    ) -> Self {
        Self {
            output,
            feedback,
            outcome: None,
            timestamp: Utc::now(),
            method_name,
            metadata: HashMap::new(),
        }
    }

    /// Create a new HumanFeedbackResult with an outcome.
    pub fn with_outcome(
        output: serde_json::Value,
        feedback: String,
        outcome: String,
        method_name: String,
    ) -> Self {
        Self {
            output,
            feedback,
            outcome: Some(outcome),
            timestamp: Utc::now(),
            method_name,
            metadata: HashMap::new(),
        }
    }
}

/// Configuration for the @human_feedback decorator.
///
/// Stores the parameters passed to the decorator for later use during
/// method execution and for introspection by visualization tools.
///
/// Corresponds to `crewai.flow.human_feedback.HumanFeedbackConfig`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanFeedbackConfig {
    /// The message shown to the human when requesting feedback.
    pub message: String,
    /// Optional sequence of outcome strings for routing.
    pub emit: Option<Vec<String>>,
    /// The LLM model to use for collapsing feedback to outcomes.
    pub llm: Option<String>,
    /// The outcome to use when no feedback is provided.
    pub default_outcome: Option<String>,
    /// Optional metadata for enterprise integrations.
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    /// Optional custom feedback provider name for async workflows.
    pub provider: Option<String>,
}

impl HumanFeedbackConfig {
    /// Create a new HumanFeedbackConfig.
    pub fn new(message: String) -> Self {
        Self {
            message,
            emit: None,
            llm: None,
            default_outcome: None,
            metadata: None,
            provider: None,
        }
    }

    /// Create a new HumanFeedbackConfig with emit and LLM.
    pub fn with_routing(
        message: String,
        emit: Vec<String>,
        llm: String,
    ) -> Result<Self, String> {
        if emit.is_empty() {
            return Err("emit must not be empty when specified".to_string());
        }
        Ok(Self {
            message,
            emit: Some(emit),
            llm: Some(llm),
            default_outcome: None,
            metadata: None,
            provider: None,
        })
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.emit.is_some() && self.llm.is_none() {
            return Err(
                "llm is required when emit is specified. \
                 Provide an LLM model string (e.g., 'gpt-4o-mini')."
                    .to_string(),
            );
        }
        if let Some(ref default) = self.default_outcome {
            match &self.emit {
                Some(emit) => {
                    if !emit.contains(default) {
                        return Err(format!(
                            "default_outcome '{}' must be one of the emit options: {:?}",
                            default, emit
                        ));
                    }
                }
                None => {
                    return Err("default_outcome requires emit to be specified.".to_string());
                }
            }
        }
        Ok(())
    }
}
