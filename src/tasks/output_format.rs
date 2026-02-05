//! Task output format definitions for CrewAI.
//!
//! Corresponds to `crewai/tasks/output_format.py`.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Enum that represents the output format of a task.
///
/// # Variants
///
/// * `JSON` - Output as JSON dictionary format
/// * `Pydantic` - Output as Pydantic model instance (serde_json::Value in Rust)
/// * `Raw` - Output as raw unprocessed string
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Output as JSON dictionary format.
    #[serde(rename = "json")]
    JSON,
    /// Output as a structured model (Pydantic equivalent).
    #[serde(rename = "pydantic")]
    Pydantic,
    /// Output as raw unprocessed string.
    #[serde(rename = "raw")]
    Raw,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Raw
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::JSON => write!(f, "json"),
            OutputFormat::Pydantic => write!(f, "pydantic"),
            OutputFormat::Raw => write!(f, "raw"),
        }
    }
}
