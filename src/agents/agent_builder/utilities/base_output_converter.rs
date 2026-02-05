//! Base output converter for transforming text into structured formats.
//!
//! Corresponds to `crewai/agents/agent_builder/utilities/base_output_converter.py`.
//!
//! Provides the `OutputConverter` trait that uses language models to transform
//! unstructured text into either Pydantic-like models or JSON objects.

use std::any::Any;
use std::collections::HashMap;
use std::fmt;

use serde_json::Value;

/// Abstract base trait for converting text to structured formats.
///
/// Uses language models to transform unstructured text into either structured
/// model instances or JSON objects based on provided instructions and target
/// schemas.
pub trait OutputConverter: Send + Sync + fmt::Debug {
    /// The input text to be converted.
    fn text(&self) -> &str;

    /// The language model used for conversion (type-erased).
    fn llm(&self) -> &dyn Any;

    /// Conversion instructions for the LLM.
    fn instructions(&self) -> &str;

    /// Maximum number of conversion attempts (default: 3).
    fn max_attempts(&self) -> u32 {
        3
    }

    /// Convert text to a structured model instance.
    ///
    /// Returns a `serde_json::Value` representing the structured output.
    /// In Python this returns a Pydantic `BaseModel`; in Rust we use
    /// `Value` as the generic structured type.
    ///
    /// # Arguments
    ///
    /// * `current_attempt` - Current attempt number for retry logic (1-indexed).
    fn to_pydantic(
        &self,
        current_attempt: u32,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;

    /// Convert text to a JSON dictionary.
    ///
    /// # Arguments
    ///
    /// * `current_attempt` - Current attempt number for retry logic (1-indexed).
    fn to_json(
        &self,
        current_attempt: u32,
    ) -> Result<HashMap<String, Value>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Data holder for `OutputConverter` implementations.
///
/// Concrete implementations can embed this to hold the common fields.
#[derive(Debug, Clone)]
pub struct OutputConverterData {
    /// The input text to be converted.
    pub text: String,
    /// Conversion instructions for the LLM.
    pub instructions: String,
    /// Maximum number of conversion attempts.
    pub max_attempts: u32,
}

impl OutputConverterData {
    /// Create a new `OutputConverterData`.
    pub fn new(
        text: impl Into<String>,
        instructions: impl Into<String>,
    ) -> Self {
        Self {
            text: text.into(),
            instructions: instructions.into(),
            max_attempts: 3,
        }
    }

    /// Builder method to set max attempts.
    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts;
        self
    }
}
