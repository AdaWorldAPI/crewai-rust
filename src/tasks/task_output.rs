//! Task output representation and formatting.
//!
//! Corresponds to `crewai/tasks/task_output.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use super::output_format::OutputFormat;

/// Represents a message from the LLM during task execution.
///
/// Corresponds to `crewai/utilities/types.py::LLMMessage`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMMessage {
    /// Role of the message sender (e.g., "system", "user", "assistant").
    pub role: String,
    /// Content of the message.
    pub content: String,
}

/// Class that represents the result of a task.
///
/// # Fields
///
/// * `description` - Description of the task
/// * `name` - Optional name of the task
/// * `expected_output` - Expected output of the task
/// * `summary` - Summary of the task (auto-generated from description)
/// * `raw` - Raw output of the task
/// * `pydantic` - Structured model output (as serde_json::Value in Rust)
/// * `json_dict` - JSON dictionary output of the task
/// * `agent` - Agent that executed the task
/// * `output_format` - Output format of the task (JSON, Pydantic, or Raw)
/// * `messages` - Messages exchanged during the task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutput {
    /// Description of the task.
    pub description: String,
    /// Name of the task.
    pub name: Option<String>,
    /// Expected output of the task.
    pub expected_output: Option<String>,
    /// Summary of the task (auto-generated from description).
    pub summary: Option<String>,
    /// Raw output of the task.
    pub raw: String,
    /// Pydantic/structured output of task (serde_json::Value equivalent).
    pub pydantic: Option<serde_json::Value>,
    /// JSON dictionary of task.
    pub json_dict: Option<HashMap<String, serde_json::Value>>,
    /// Agent that executed the task.
    pub agent: String,
    /// Output format of the task.
    pub output_format: OutputFormat,
    /// Messages of the task.
    #[serde(default)]
    pub messages: Vec<LLMMessage>,
}

impl TaskOutput {
    /// Create a new TaskOutput with summary auto-generated from description.
    pub fn new(
        description: String,
        agent: String,
        raw: String,
        output_format: OutputFormat,
    ) -> Self {
        let summary = Self::generate_summary(&description);
        Self {
            description,
            name: None,
            expected_output: None,
            summary: Some(summary),
            raw,
            pydantic: None,
            json_dict: None,
            agent,
            output_format,
            messages: Vec::new(),
        }
    }

    /// Generate a summary from the description (first 10 words + "...").
    fn generate_summary(description: &str) -> String {
        let excerpt: String = description
            .split_whitespace()
            .take(10)
            .collect::<Vec<&str>>()
            .join(" ");
        format!("{}...", excerpt)
    }

    /// Set the summary field based on the description.
    pub fn set_summary(&mut self) {
        self.summary = Some(Self::generate_summary(&self.description));
    }

    /// Get the JSON string representation of the task output.
    ///
    /// # Errors
    ///
    /// Returns an error if output format is not JSON.
    pub fn json(&self) -> Result<String, String> {
        if self.output_format != OutputFormat::JSON {
            return Err(
                "Invalid output format requested. \
                 If you would like to access the JSON output, \
                 please make sure to set the output_json property for the task"
                    .to_string(),
            );
        }

        match &self.json_dict {
            Some(dict) => serde_json::to_string(dict).map_err(|e| e.to_string()),
            None => Ok("null".to_string()),
        }
    }

    /// Convert json_output and pydantic_output to a dictionary.
    ///
    /// Prioritizes json_dict over pydantic model dump if both are available.
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut output_dict = HashMap::new();
        if let Some(ref json_dict) = self.json_dict {
            output_dict.extend(json_dict.clone());
        } else if let Some(ref pydantic) = self.pydantic {
            if let serde_json::Value::Object(map) = pydantic {
                for (k, v) in map {
                    output_dict.insert(k.clone(), v.clone());
                }
            }
        }
        output_dict
    }
}

impl fmt::Display for TaskOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref pydantic) = self.pydantic {
            write!(f, "{}", pydantic)
        } else if let Some(ref json_dict) = self.json_dict {
            write!(f, "{:?}", json_dict)
        } else {
            write!(f, "{}", self.raw)
        }
    }
}
