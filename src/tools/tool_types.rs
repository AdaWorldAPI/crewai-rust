//! Tool result types for CrewAI.
//!
//! Corresponds to `crewai/tools/tool_types.py`.
//!
//! Provides the `ToolResult` type which carries the output of a tool
//! execution along with a flag indicating whether the result should be
//! treated as the agent's final answer.

use serde::{Deserialize, Serialize};

/// Result of a tool execution.
///
/// Wraps the string output of a tool together with an optional flag
/// that tells the agent executor to treat this result as the final
/// answer, bypassing further reasoning steps.
///
/// Corresponds to `crewai.tools.tool_types.ToolResult`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The string output produced by the tool.
    pub result: String,
    /// When `true`, the agent executor should use this result directly
    /// as the final answer instead of continuing the thought loop.
    #[serde(default)]
    pub result_as_answer: bool,
}

impl ToolResult {
    /// Create a new `ToolResult` with the given output.
    pub fn new(result: impl Into<String>) -> Self {
        Self {
            result: result.into(),
            result_as_answer: false,
        }
    }

    /// Create a new `ToolResult` that should be treated as the final answer.
    pub fn as_answer(result: impl Into<String>) -> Self {
        Self {
            result: result.into(),
            result_as_answer: true,
        }
    }
}

impl std::fmt::Display for ToolResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.result)
    }
}

impl From<String> for ToolResult {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for ToolResult {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_result_new() {
        let result = ToolResult::new("hello");
        assert_eq!(result.result, "hello");
        assert!(!result.result_as_answer);
    }

    #[test]
    fn test_tool_result_as_answer() {
        let result = ToolResult::as_answer("final");
        assert_eq!(result.result, "final");
        assert!(result.result_as_answer);
    }

    #[test]
    fn test_tool_result_display() {
        let result = ToolResult::new("output");
        assert_eq!(format!("{}", result), "output");
    }

    #[test]
    fn test_tool_result_from_string() {
        let result: ToolResult = "test".into();
        assert_eq!(result.result, "test");
        assert!(!result.result_as_answer);
    }

    #[test]
    fn test_tool_result_serde_roundtrip() {
        let result = ToolResult::as_answer("data");
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ToolResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.result, "data");
        assert!(deserialized.result_as_answer);
    }
}
