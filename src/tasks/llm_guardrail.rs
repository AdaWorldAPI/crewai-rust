//! LLM-based guardrail for validating task outputs.
//!
//! Corresponds to `crewai/tasks/llm_guardrail.py`.

use serde::{Deserialize, Serialize};

use super::task_output::TaskOutput;

/// Result of an LLM guardrail validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMGuardrailResult {
    /// Whether the task output complies with the guardrail.
    pub valid: bool,
    /// Feedback about the task output if it is not valid.
    pub feedback: Option<String>,
}

/// Validates the output of another task using an LLM.
///
/// This struct is used to validate the output from a Task based on specified criteria.
/// It uses an LLM to validate the output and provides feedback if the output is not valid.
///
/// # Fields
///
/// * `description` - The description of the validation criteria.
/// * `llm_model` - The language model identifier to use for validation.
#[derive(Debug, Clone)]
pub struct LLMGuardrail {
    /// The description of the validation criteria.
    pub description: String,
    /// The language model identifier to use for validation.
    pub llm_model: String,
}

impl LLMGuardrail {
    /// Create a new LLMGuardrail.
    pub fn new(description: String, llm_model: String) -> Self {
        Self {
            description,
            llm_model,
        }
    }

    /// Validate the output of a task based on specified criteria.
    ///
    /// This is the Rust equivalent of the Python `__call__` method.
    ///
    /// # Returns
    ///
    /// A tuple of (success: bool, result_or_error: String).
    /// - If validation passes: (true, raw task output)
    /// - If validation fails: (false, feedback or error message)
    pub fn call(&self, task_output: &TaskOutput) -> (bool, String) {
        // In the full implementation this would:
        // 1. Create a guardrail agent
        // 2. Format a validation query
        // 3. Call the LLM to validate
        // For now, we provide the validation query template as documentation.
        let _query = format!(
            "Ensure the following task result complies with the given guardrail.\n\n\
             Task result:\n{}\n\n\
             Guardrail:\n{}\n\n\
             Your task:\n\
             - Confirm if the Task result complies with the guardrail.\n\
             - If not, provide clear feedback explaining what is wrong.\n\
             - Focus only on identifying issues -- do not propose corrections.\n\
             - If the Task result complies with the guardrail, say it is valid.",
            task_output.raw, self.description
        );

        // TODO: Implement actual LLM call through the agent system.
        // For now, return a placeholder indicating validation is pending.
        log::warn!(
            "LLMGuardrail.call() is a stub -- actual LLM validation not yet implemented"
        );
        (true, task_output.raw.clone())
    }
}
