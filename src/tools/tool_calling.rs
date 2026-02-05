//! Tool calling data structures.
//!
//! Corresponds to `crewai/tools/tool_calling.py`.
//!
//! Provides the `ToolCalling` struct used to represent a tool invocation
//! with its name and arguments.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents a tool call with its name and arguments.
///
/// Used to capture the LLM's intent to invoke a specific tool with
/// the given arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCalling {
    /// The name of the tool to be called.
    pub tool_name: String,
    /// A dictionary of arguments to be passed to the tool.
    /// `None` if no arguments are provided.
    pub arguments: Option<HashMap<String, Value>>,
}

impl ToolCalling {
    /// Create a new `ToolCalling` instance.
    pub fn new(tool_name: impl Into<String>, arguments: Option<HashMap<String, Value>>) -> Self {
        Self {
            tool_name: tool_name.into(),
            arguments,
        }
    }
}

/// Instructor-compatible variant of `ToolCalling`.
///
/// Matches the Python `InstructorToolCalling` class which uses Pydantic's
/// `BaseModel` (as opposed to CrewAI's custom `BaseModel`). In Rust both
/// variants share the same structure.
pub type InstructorToolCalling = ToolCalling;
