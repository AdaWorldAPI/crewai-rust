//! Hallucination Guardrail Placeholder for CrewAI.
//!
//! This is a no-op version of the HallucinationGuardrail for the open-source repository.
//!
//! Corresponds to `crewai/tasks/hallucination_guardrail.py`.

use super::task_output::TaskOutput;

/// Placeholder for the HallucinationGuardrail feature.
///
/// In the open-source version, this guardrail always returns that the output is valid.
///
/// # Fields
///
/// * `context` - The reference context that outputs would be checked against.
/// * `llm_model` - The language model identifier that would be used for evaluation.
/// * `threshold` - Optional minimum faithfulness score that would be required to pass.
/// * `tool_response` - Optional tool response information that would be used in evaluation.
#[derive(Debug, Clone)]
pub struct HallucinationGuardrail {
    /// The reference context that outputs would be checked against.
    pub context: String,
    /// The language model identifier that would be used for evaluation.
    pub llm_model: String,
    /// Optional minimum faithfulness score that would be required to pass.
    pub threshold: Option<f64>,
    /// Optional tool response information that would be used in evaluation.
    pub tool_response: String,
}

impl HallucinationGuardrail {
    /// Create a new HallucinationGuardrail placeholder.
    pub fn new(context: String, llm_model: String) -> Self {
        log::warn!(
            "Hallucination detection is a no-op in open source, \
             use it for free at https://app.crewai.com"
        );
        Self {
            context,
            llm_model,
            threshold: None,
            tool_response: String::new(),
        }
    }

    /// Create a new HallucinationGuardrail with threshold and tool_response.
    pub fn with_options(
        context: String,
        llm_model: String,
        threshold: Option<f64>,
        tool_response: String,
    ) -> Self {
        log::warn!(
            "Hallucination detection is a no-op in open source, \
             use it for free at https://app.crewai.com"
        );
        Self {
            context,
            llm_model,
            threshold,
            tool_response,
        }
    }

    /// Get a description of this guardrail for event logging.
    pub fn description(&self) -> &str {
        "HallucinationGuardrail (no-op)"
    }

    /// Validate a task output against hallucination criteria.
    ///
    /// In the open-source version, this method always returns that the output is valid.
    ///
    /// # Returns
    ///
    /// A tuple of (true, raw task output) -- always passes.
    pub fn call(&self, task_output: &TaskOutput) -> (bool, String) {
        log::warn!(
            "Premium hallucination detection skipped \
             (use for free at https://app.crewai.com)"
        );
        (true, task_output.raw.clone())
    }
}
