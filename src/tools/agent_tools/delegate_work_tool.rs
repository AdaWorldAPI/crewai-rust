//! Delegate work tool.
//!
//! Corresponds to `crewai/tools/agent_tools/delegate_work_tool.py`.
//!
//! Allows an agent to delegate tasks to coworkers.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Schema for delegate work tool arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateWorkToolSchema {
    /// The task to delegate.
    pub task: String,
    /// The context for the task.
    pub context: String,
    /// The role/name of the coworker to delegate to.
    pub coworker: String,
}

/// Tool for delegating work to coworkers.
///
/// Enables an agent to delegate a specific task to another agent (coworker)
/// within the crew. The delegated agent will execute the task and return
/// the result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateWorkTool {
    /// Tool name.
    pub name: String,
    /// Tool description (includes available coworkers).
    pub description: String,
    /// Names/roles of available coworkers.
    pub coworker_names: Vec<String>,
}

impl DelegateWorkTool {
    /// Create a new `DelegateWorkTool`.
    pub fn new(description: impl Into<String>, coworker_names: Vec<String>) -> Self {
        Self {
            name: "Delegate work to coworker".to_string(),
            description: description.into(),
            coworker_names,
        }
    }

    /// Get the JSON schema for the tool's arguments.
    pub fn args_schema() -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "The task to delegate"
                },
                "context": {
                    "type": "string",
                    "description": "The context for the task"
                },
                "coworker": {
                    "type": "string",
                    "description": "The role/name of the coworker to delegate to"
                }
            },
            "required": ["task", "context", "coworker"]
        })
    }

    /// Execute the delegation.
    ///
    /// In a full implementation this would look up the coworker agent and
    /// execute the task. Here we return a placeholder indicating delegation.
    pub fn run(
        &self,
        task: &str,
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

        // In a full implementation, this would delegate the task to the agent.
        // For now, return a structured representation of the delegation.
        Ok(format!(
            "Delegated task to '{}': {}\nContext: {}",
            sanitized_coworker, task, context
        ))
    }
}

/// Sanitize an agent role name by normalizing whitespace and converting to lowercase.
fn sanitize_agent_name(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }
    // Normalize all whitespace to single spaces, remove quotes, lowercase
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('"', "")
        .to_lowercase()
}
