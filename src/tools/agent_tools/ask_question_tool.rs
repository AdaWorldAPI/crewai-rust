//! Ask question tool.
//!
//! Corresponds to `crewai/tools/agent_tools/ask_question_tool.py`.
//!
//! Allows an agent to ask questions to coworkers.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Schema for ask question tool arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskQuestionToolSchema {
    /// The question to ask.
    pub question: String,
    /// The context for the question.
    pub context: String,
    /// The role/name of the coworker to ask.
    pub coworker: String,
}

/// Tool for asking questions to coworkers.
///
/// Enables an agent to ask a specific question to another agent (coworker)
/// within the crew. The coworker agent will provide an answer based on
/// the question and context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskQuestionTool {
    /// Tool name.
    pub name: String,
    /// Tool description (includes available coworkers).
    pub description: String,
    /// Names/roles of available coworkers.
    pub coworker_names: Vec<String>,
}

impl AskQuestionTool {
    /// Create a new `AskQuestionTool`.
    pub fn new(description: impl Into<String>, coworker_names: Vec<String>) -> Self {
        Self {
            name: "Ask question to coworker".to_string(),
            description: description.into(),
            coworker_names,
        }
    }

    /// Get the JSON schema for the tool's arguments.
    pub fn args_schema() -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "The question to ask"
                },
                "context": {
                    "type": "string",
                    "description": "The context for the question"
                },
                "coworker": {
                    "type": "string",
                    "description": "The role/name of the coworker to ask"
                }
            },
            "required": ["question", "context", "coworker"]
        })
    }

    /// Execute the question.
    ///
    /// In a full implementation this would look up the coworker agent and
    /// execute the question. Returns a placeholder indicating the question was asked.
    pub fn run(
        &self,
        question: &str,
        context: &str,
        coworker: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let sanitized_coworker = sanitize_agent_name(coworker);

        // Verify the coworker exists
        let coworker_exists = self
            .coworker_names
            .iter()
            .any(|name| sanitize_agent_name(name) == sanitized_coworker);

        if !coworker_exists {
            let available = self
                .coworker_names
                .iter()
                .map(|n| format!("- {}", sanitize_agent_name(n)))
                .collect::<Vec<_>>()
                .join("\n");
            return Err(format!(
                "Coworker '{}' not found. Available coworkers:\n{}",
                sanitized_coworker, available
            )
            .into());
        }

        // In a full implementation, this would ask the question to the agent.
        Ok(format!(
            "Asked '{}' the question: {}\nContext: {}",
            sanitized_coworker, question, context
        ))
    }
}

/// Sanitize an agent role name by normalizing whitespace and converting to lowercase.
fn sanitize_agent_name(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('"', "")
        .to_lowercase()
}
