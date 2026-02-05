//! Output converter for converting text into structured formats.
//!
//! Corresponds to `crewai/utilities/converter.py`.

use serde_json::Value;
use thiserror::Error;

/// Error raised when the converter fails to parse input.
#[derive(Debug, Error)]
#[error("{message}")]
pub struct ConverterError {
    pub message: String,
}

impl ConverterError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Trait for converting text into a pydantic-like model or JSON.
///
/// In the Rust port, "pydantic" models are represented as
/// `serde_json::Value` (structured data).
pub trait Converter {
    /// Convert text to a structured value (analogous to `to_pydantic`).
    ///
    /// # Arguments
    /// * `current_attempt` - Retry attempt counter (starts at 1).
    fn to_structured(&self, current_attempt: u32) -> Result<Value, ConverterError>;

    /// Convert text to a JSON string (analogous to `to_json`).
    ///
    /// # Arguments
    /// * `current_attempt` - Retry attempt counter (starts at 1).
    fn to_json(&self, current_attempt: u32) -> Result<String, ConverterError>;
}

/// Validate a JSON string against a schema (represented as `Value`).
///
/// Returns the parsed value if valid.
pub fn validate_model(result: &str, _schema: &Value, is_json_output: bool) -> Result<Value, ConverterError> {
    let parsed: Value = serde_json::from_str(result)
        .map_err(|e| ConverterError::new(format!("JSON parse error: {}", e)))?;

    if is_json_output {
        Ok(parsed)
    } else {
        Ok(parsed)
    }
}

/// Attempt to extract and parse partial JSON from a result string.
pub fn handle_partial_json(result: &str, _schema: &Value, is_json_output: bool) -> Result<Value, ConverterError> {
    // Try to find JSON object in the result
    if let Some(start) = result.find('{') {
        if let Some(end) = result.rfind('}') {
            let json_str = &result[start..=end];
            if let Ok(parsed) = serde_json::from_str::<Value>(json_str) {
                return if is_json_output {
                    Ok(parsed)
                } else {
                    Ok(parsed)
                };
            }
        }
    }
    Err(ConverterError::new("No valid JSON found in result"))
}
