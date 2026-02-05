//! Agent output parsing module for ReAct-style LLM responses.
//!
//! Corresponds to `crewai/agents/parser.py`.
//!
//! Provides parsing functionality for agent outputs that follow the ReAct
//! (Reasoning and Acting) format, converting them into structured
//! `AgentAction` or `AgentFinish` objects.

use std::fmt;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Constants (matching crewai/agents/constants.py patterns)
// ---------------------------------------------------------------------------

/// The text prefix for a final answer.
const FINAL_ANSWER_ACTION: &str = "Final Answer:";

/// Error message when action is missing after thought.
const MISSING_ACTION_AFTER_THOUGHT_ERROR_MESSAGE: &str =
    "I just got this: I couldn't find an Action after the Thought.";

/// Error message when action input is missing after action.
const MISSING_ACTION_INPUT_AFTER_ACTION_ERROR_MESSAGE: &str =
    "I just got this: I found an Action but couldn't find a valid Action Input right after it.";

// ---------------------------------------------------------------------------
// AgentAction
// ---------------------------------------------------------------------------

/// Represents an action to be taken by an agent.
///
/// Contains the parsed thought, tool name, tool input, raw text,
/// and optionally the result of executing the tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAction {
    /// The agent's reasoning/thought before taking the action.
    pub thought: String,
    /// The name of the tool to use.
    pub tool: String,
    /// The input to pass to the tool.
    pub tool_input: String,
    /// The raw text that was parsed.
    pub text: String,
    /// The result of executing the tool (populated after execution).
    pub result: Option<String>,
}

// ---------------------------------------------------------------------------
// AgentFinish
// ---------------------------------------------------------------------------

/// Represents the final answer from an agent.
///
/// Contains the parsed thought, the output (which may be a string or
/// structured data), and the raw text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentFinish {
    /// The agent's reasoning/thought before the final answer.
    pub thought: String,
    /// The final output. Can be a plain string or structured JSON.
    pub output: Value,
    /// The raw text that was parsed.
    pub text: String,
}

// ---------------------------------------------------------------------------
// OutputParserError
// ---------------------------------------------------------------------------

/// Exception raised when output parsing fails.
#[derive(Debug, Clone)]
pub struct OutputParserError {
    /// The error message describing what went wrong.
    pub error: String,
}

impl OutputParserError {
    /// Create a new `OutputParserError`.
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
        }
    }
}

impl fmt::Display for OutputParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OutputParserError: {}", self.error)
    }
}

impl std::error::Error for OutputParserError {}

// ---------------------------------------------------------------------------
// Parse function
// ---------------------------------------------------------------------------

/// Parse agent output text into `AgentAction` or `AgentFinish`.
///
/// Expects output to be in one of two formats:
///
/// **Action format** (results in `AgentAction`):
/// ```text
/// Thought: agent thought here
/// Action: search
/// Action Input: what is the temperature in SF?
/// ```
///
/// **Final answer format** (results in `AgentFinish`):
/// ```text
/// Thought: agent thought here
/// Final Answer: The temperature is 100 degrees
/// ```
///
/// # Errors
///
/// Returns `OutputParserError` if the text format is invalid.
pub fn parse(text: &str) -> Result<ParseResult, OutputParserError> {
    let thought = extract_thought(text);
    let includes_answer = text.contains(FINAL_ANSWER_ACTION);

    // Regex for Action + Action Input
    let action_input_re =
        Regex::new(r"(?s)Action\s*\d*\s*:\s*(.+?)\s*(?:\n|\r\n?)Action\s*\d*\s*Input\s*\d*\s*:\s*(.*)")
            .expect("Invalid regex");
    let action_re = Regex::new(r"Action\s*\d*\s*:").expect("Invalid regex");
    let action_input_only_re =
        Regex::new(r"Action\s*\d*\s*Input\s*\d*\s*:").expect("Invalid regex");

    if includes_answer {
        let final_answer = text
            .rsplit(FINAL_ANSWER_ACTION)
            .next()
            .unwrap_or("")
            .trim()
            .to_string();

        // Handle trailing triple backticks
        let final_answer = clean_trailing_backticks(&final_answer);

        return Ok(ParseResult::Finish(AgentFinish {
            thought,
            output: Value::String(final_answer),
            text: text.to_string(),
        }));
    }

    if let Some(caps) = action_input_re.captures(text) {
        let action = caps.get(1).map_or("", |m| m.as_str());
        let clean_action = clean_action(action);

        let action_input = caps.get(2).map_or("", |m| m.as_str()).trim();
        let tool_input = action_input.trim_matches('"');
        let safe_tool_input = safe_repair_json(tool_input);

        return Ok(ParseResult::Action(AgentAction {
            thought,
            tool: clean_action,
            tool_input: safe_tool_input,
            text: text.to_string(),
            result: None,
        }));
    }

    if !action_re.is_match(text) {
        return Err(OutputParserError::new(format!(
            "{}\nYou MUST use the following format:\n\
             Thought: [your thought]\n\
             Final Answer: [your final answer]",
            MISSING_ACTION_AFTER_THOUGHT_ERROR_MESSAGE
        )));
    }

    if !action_input_only_re.is_match(text) {
        return Err(OutputParserError::new(
            MISSING_ACTION_INPUT_AFTER_ACTION_ERROR_MESSAGE,
        ));
    }

    Err(OutputParserError::new(
        "Could not parse the output. Please use the correct format.",
    ))
}

/// Result of parsing agent output.
#[derive(Debug, Clone)]
pub enum ParseResult {
    /// The agent wants to take an action (use a tool).
    Action(AgentAction),
    /// The agent has a final answer.
    Finish(AgentFinish),
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Extract the thought portion from the text.
fn extract_thought(text: &str) -> String {
    let thought_index = text.find("\nAction").or_else(|| text.find("\nFinal Answer"));
    match thought_index {
        Some(idx) => {
            let thought = text[..idx].trim();
            thought.replace("```", "").trim().to_string()
        }
        None => String::new(),
    }
}

/// Clean action string by removing non-essential formatting characters.
fn clean_action(text: &str) -> String {
    text.trim().trim_matches('*').trim().to_string()
}

/// Clean trailing triple backticks from a final answer.
fn clean_trailing_backticks(text: &str) -> String {
    let mut result = text.to_string();
    if result.ends_with("```") {
        let count = result.matches("```").count();
        // If count is odd, it's an unmatched trailing set; remove it.
        if count % 2 != 0 {
            result = result[..result.len() - 3].trim_end().to_string();
        }
    }
    result
}

/// Safely attempt to repair JSON input.
///
/// Skips repair if the input is a JSON array (starts/ends with `[]`).
/// Replaces common LLM issues like triple quotes.
fn safe_repair_json(tool_input: &str) -> String {
    // Skip repair for array inputs
    if tool_input.starts_with('[') && tool_input.ends_with(']') {
        return tool_input.to_string();
    }

    // Replace triple quotes with single quotes
    let cleaned = tool_input.replace("\"\"\"", "\"");

    // Try to parse as JSON to verify it's valid
    if serde_json::from_str::<Value>(&cleaned).is_ok() {
        return cleaned;
    }

    // If the cleaned version is not valid JSON, return the original
    tool_input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_final_answer() {
        let text = "Thought: I know the answer\nFinal Answer: The temperature is 72 degrees.";
        let result = parse(text).unwrap();
        match result {
            ParseResult::Finish(finish) => {
                assert_eq!(finish.output, Value::String("The temperature is 72 degrees.".to_string()));
                assert_eq!(finish.thought, "Thought: I know the answer");
            }
            _ => panic!("Expected AgentFinish"),
        }
    }

    #[test]
    fn test_parse_action() {
        let text = "Thought: I need to search\nAction: search\nAction Input: temperature in SF";
        let result = parse(text).unwrap();
        match result {
            ParseResult::Action(action) => {
                assert_eq!(action.tool, "search");
                assert_eq!(action.tool_input, "temperature in SF");
            }
            _ => panic!("Expected AgentAction"),
        }
    }

    #[test]
    fn test_parse_missing_action() {
        let text = "Thought: I need to do something";
        let result = parse(text);
        assert!(result.is_err());
    }
}
