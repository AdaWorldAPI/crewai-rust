//! Crew output representation.
//!
//! Corresponds to `crewai/crews/crew_output.py`.
//!
//! Represents the result of a crew execution, including raw output,
//! structured (Pydantic/JSON) output, individual task outputs, and
//! token usage metrics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::tasks::output_format::OutputFormat;
use crate::tasks::task_output::TaskOutput;
use crate::types::usage_metrics::UsageMetrics;

/// Class that represents the result of a crew.
///
/// Corresponds to `crewai.crews.crew_output.CrewOutput`.
///
/// # Fields
///
/// * `raw` - Raw output of crew (the final task's raw text).
/// * `pydantic` - Structured model output of Crew (serde_json::Value equivalent).
/// * `json_dict` - JSON dict output of Crew.
/// * `tasks_output` - Output of each task in execution order.
/// * `token_usage` - Processed token summary across all tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewOutput {
    /// Raw output of crew.
    pub raw: String,
    /// Pydantic/structured output of Crew.
    pub pydantic: Option<serde_json::Value>,
    /// JSON dict output of Crew.
    pub json_dict: Option<HashMap<String, serde_json::Value>>,
    /// Output of each task.
    pub tasks_output: Vec<TaskOutput>,
    /// Processed token summary.
    pub token_usage: UsageMetrics,
}

impl Default for CrewOutput {
    fn default() -> Self {
        Self {
            raw: String::new(),
            pydantic: None,
            json_dict: None,
            tasks_output: Vec::new(),
            token_usage: UsageMetrics::new(),
        }
    }
}

impl CrewOutput {
    /// Create a new CrewOutput.
    pub fn new(
        raw: String,
        tasks_output: Vec<TaskOutput>,
        token_usage: UsageMetrics,
    ) -> Self {
        Self {
            raw,
            pydantic: None,
            json_dict: None,
            tasks_output,
            token_usage,
        }
    }

    /// Get the JSON string representation of the crew output.
    ///
    /// # Errors
    ///
    /// Returns an error if the final task output format is not JSON.
    pub fn json(&self) -> Result<String, String> {
        if let Some(last) = self.tasks_output.last() {
            if last.output_format != OutputFormat::JSON {
                return Err(
                    "No JSON output found in the final task. \
                     Please make sure to set the output_json property in the final task in your crew."
                        .to_string(),
                );
            }
        }

        match &self.json_dict {
            Some(dict) => serde_json::to_string(dict).map_err(|e| e.to_string()),
            None => Ok("null".to_string()),
        }
    }

    /// Convert json_output and pydantic_output to a dictionary.
    ///
    /// Returns the json_dict if available, otherwise converts the pydantic
    /// value to a dictionary.
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

    /// Get a value by key (checks pydantic first, then json_dict).
    ///
    /// # Errors
    ///
    /// Returns an error if the key is not found.
    pub fn get(&self, key: &str) -> Result<serde_json::Value, String> {
        if let Some(ref pydantic) = self.pydantic {
            if let serde_json::Value::Object(map) = pydantic {
                if let Some(val) = map.get(key) {
                    return Ok(val.clone());
                }
            }
        }
        if let Some(ref json_dict) = self.json_dict {
            if let Some(val) = json_dict.get(key) {
                return Ok(val.clone());
            }
        }
        Err(format!("Key '{}' not found in CrewOutput.", key))
    }
}

impl fmt::Display for CrewOutput {
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
