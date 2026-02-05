//! Base converter adapter for structured output conversion.
//!
//! Corresponds to `crewai/agents/agent_adapters/base_converter_adapter.py`.
//!
//! Defines the common interface for converting agent outputs to structured
//! formats (JSON or Pydantic-like). All converter adapters must implement
//! the methods defined here.

use std::fmt;

use regex::Regex;
use serde_json::Value;

/// Output format specifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// JSON output format.
    Json,
    /// Pydantic model output format.
    Pydantic,
}

/// Abstract base trait for converter adapters in CrewAI.
///
/// Defines the common interface for converting agent outputs to structured
/// formats. All converter adapters must implement `configure_structured_output`
/// and `enhance_system_prompt`.
pub trait BaseConverterAdapter: Send + Sync + fmt::Debug {
    /// Get the current output format.
    fn output_format(&self) -> Option<OutputFormat>;

    /// Get the schema description for the expected output.
    fn schema(&self) -> Option<&Value>;

    /// Configure agents to return structured output.
    ///
    /// Must support both JSON and Pydantic output formats.
    fn configure_structured_output(
        &mut self,
        task: &dyn std::any::Any,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Enhance the system prompt with structured output instructions.
    ///
    /// # Arguments
    ///
    /// * `base_prompt` - The original system prompt.
    ///
    /// # Returns
    ///
    /// Enhanced prompt with structured output guidance.
    fn enhance_system_prompt(&self, base_prompt: &str) -> String;

    /// Post-process the result to ensure proper string format.
    ///
    /// Extracts valid JSON from text that may contain markdown or other formatting.
    fn post_process_result(&self, result: &str) -> String {
        if self.output_format().is_none() {
            return result.to_string();
        }

        extract_json_from_text(result)
    }
}

/// Validate if text is valid JSON and return it, or `None` if invalid.
pub fn validate_json(text: &str) -> Option<String> {
    serde_json::from_str::<Value>(text)
        .ok()
        .map(|_| text.to_string())
}

/// Extract valid JSON from text that may contain markdown or other formatting.
///
/// Handles cases where JSON may be wrapped in Markdown code blocks or
/// embedded in text.
pub fn extract_json_from_text(result: &str) -> String {
    // Try direct parse first
    if let Some(valid) = validate_json(result) {
        return valid;
    }

    // Try extracting from code blocks
    let code_block_re = Regex::new(r"```(?:json)?\s*([\s\S]*?)```").unwrap();
    for cap in code_block_re.captures_iter(result) {
        if let Some(m) = cap.get(1) {
            let trimmed = m.as_str().trim();
            if let Some(valid) = validate_json(trimmed) {
                return valid;
            }
        }
    }

    // Try extracting any JSON object
    let json_obj_re = Regex::new(r"\{[\s\S]*\}").unwrap();
    for m in json_obj_re.find_iter(result) {
        if let Some(valid) = validate_json(m.as_str()) {
            return valid;
        }
    }

    // Return original if no valid JSON found
    result.to_string()
}

/// Determine output format and schema from task requirements.
///
/// Examines the task's output requirements and returns the appropriate
/// format type and schema description.
pub fn configure_format_from_task(
    output_json: Option<&Value>,
    output_pydantic: Option<&Value>,
) -> (Option<OutputFormat>, Option<Value>) {
    if let Some(json_schema) = output_json {
        return (Some(OutputFormat::Json), Some(json_schema.clone()));
    }
    if let Some(pydantic_schema) = output_pydantic {
        return (Some(OutputFormat::Pydantic), Some(pydantic_schema.clone()));
    }
    (None, None)
}
